//! See [`EventBuffer`].

use core::mem::MaybeUninit;

use dropset_interface::{
    events::{
        HeaderEventInstructionData,
        PackIntoSlice,
    },
    instructions::DropsetInstruction,
    program,
    seeds::event_authority,
    syscalls,
};
use pinocchio::{
    account_info::AccountInfo,
    cpi::invoke_signed,
    hint::unlikely,
    instruction::{
        AccountMeta,
        Instruction,
    },
    ProgramResult,
};

use crate::{
    event_authority_signer,
    validation::market_account_info::MarketAccountInfo,
};

/// The stack-allocated event buffer length.
pub const EVENT_BUFFER_LEN: usize = 1024;

/// The stack-based, buffer of event data emitted through self-CPIs.
///
/// Self-CPIs on event instruction data facilitates emitting events
/// without having to store it in account data.
///
/// The buffer `data` always begins with a [`HeaderInstructionData`]
/// event, tracking the number of events currently stored in the buffer
/// and other various info about the transaction/instruction.
///
/// The data that comes after the header data is one or more contiguous
/// event instruction data bytes.
///
/// The buffer avoids overflow by flushing all of its current data if
/// the remaining unused buffer space isn't sufficient for an incoming
/// event's data.
///
/// The length of the buffer is tracked internally.
pub struct EventBuffer {
    /// The stack-allocated, possibly initialized buffer bytes.
    ///
    /// The layout for the data is:
    /// - `[0]`: the instruction tag of the instruction that created this event buffer.
    /// - `[1..HeaderInstructionData::LEN_WITH_TAG]`: the header instruction data.
    /// - `[HeaderInstructionData::LEN_WITH_TAG..]`: the byte data for the other non-header events
    ///   in the buffer.
    pub data: [MaybeUninit<u8>; EVENT_BUFFER_LEN],
    /// The number of events in the buffer that come after the header.
    emitted_count: u16,
    /// The amount of initialized bytes. The index at `len` is the first uninitialized byte.
    len: usize,
    /// The instruction tag for the instruction responsible for the `dropset` program's invocation.
    pub instruction_tag: DropsetInstruction,
}

/// The header data begins after the initial invoking instruction tag byte.
const HEADER_DATA_OFFSET: usize = 1;

/// The total size of the dropset FlushEvents instruction tag + the header instruction data
/// tag + the header instruction event data.
///
/// This is essentially where the emitted events data actually starts.
const HEADER_SIZE_WITH_TAGS: usize =
    size_of::<DropsetInstruction>() + HeaderEventInstructionData::LEN_WITH_TAG;

impl EventBuffer {
    #[inline(never)] // The compiler inlines this otherwise and doubles the stack frame size.
    pub fn new(instruction_tag: DropsetInstruction) -> Self {
        let mut buf = Self {
            data: [MaybeUninit::uninit(); EVENT_BUFFER_LEN],
            emitted_count: 0,
            // The length after writing to the first byte + all the header data bytes.
            len: HEADER_SIZE_WITH_TAGS,
            instruction_tag,
        };

        // Manually pack the instruction tag for the CPI invocation.
        buf.data[0].write(DropsetInstruction::FlushEvents as u8);

        // Zero out the space reserved for the header event data.
        // Safety: `data` is valid for EVENT_BUFFER_LEN - HEADER_DATA_OFFSET bytes and is align 1.
        unsafe {
            syscalls::sol_memset_(
                buf.data.as_mut_ptr().add(HEADER_DATA_OFFSET) as *mut u8,
                0,
                HeaderEventInstructionData::LEN_WITH_TAG as u64,
            )
        };

        buf
    }

    /// Flush the event buffer by invoking a CPI with the buffer data.
    ///
    /// Safety:
    ///
    /// Caller guarantees `market_account` is not currently borrowed in any capacity.
    pub unsafe fn flush_events<'a>(
        &mut self,
        event_authority: &'a AccountInfo,
        mut market_account: MarketAccountInfo<'a>,
    ) -> ProgramResult {
        let emitted_count = self.emitted_count as u64;
        if unlikely(emitted_count == 0) {
            return Ok(());
        }

        let market_pubkey = *market_account.info().key();
        // Safety: `market_account` is not currently borrowed in any capacity.
        let market_ref_mut = unsafe { market_account.load_unchecked_mut() };
        market_ref_mut.header.increment_num_events_by(emitted_count);

        // Safety:
        // The header prefix bytes have already been zeroed, so `self.data` is long enough.
        // Updating `self.len` is not appropriate here as this is just updating the header
        // prefix prior to emission.
        unsafe {
            HeaderEventInstructionData::new(
                self.instruction_tag as u8,
                self.emitted_count,
                market_ref_mut.header.num_events(),
                market_pubkey,
            )
            .pack_into_slice(&mut self.data, HEADER_DATA_OFFSET);
        }

        // Safety: `data` has exactly `self.len` initialized, contiguous bytes.
        let data =
            unsafe { core::slice::from_raw_parts(self.data.as_ptr() as *const u8, self.len) };

        invoke_signed(
            &Instruction {
                program_id: &program::ID,
                data,
                accounts: &[AccountMeta::readonly_signer(&event_authority::ID)],
            },
            &[event_authority],
            &[event_authority_signer!()],
        )?;

        // Effectively "truncate" the buffer back down to the header size.
        self.len = HEADER_SIZE_WITH_TAGS;
        // Reset the count.
        self.emitted_count = 0;

        Ok(())
    }

    #[inline(always)]
    pub fn add_to_buffer<'a, T: PackIntoSlice>(
        &mut self,
        packable_event: T,
        event_authority: &'a AccountInfo,
        market_account: MarketAccountInfo<'a>,
    ) -> ProgramResult {
        let len = self.len;
        if len + T::LEN_WITH_TAG > EVENT_BUFFER_LEN {
            // Safety: `market_account` is not currently borrowed in any capacity.
            unsafe { self.flush_events(event_authority, market_account) }?;
        }

        // Since the length isn't checked again after flushing, check the very unlikely
        // edge case that we've defined an event that's larger than the size of a newly
        // flushed event buffer.
        debug_assert!(
            T::LEN_WITH_TAG < EVENT_BUFFER_LEN - HEADER_SIZE_WITH_TAGS,
            "Event is way too big"
        );

        // Safety: The buffer length is either sufficient or has recently been flushed.
        // The tracked length is incremented below.
        unsafe { packable_event.pack_into_slice(&mut self.data, len) };

        self.emitted_count += 1;
        self.len += T::LEN_WITH_TAG;

        Ok(())
    }
}

#[test]
fn test_max_cpi_len() {
    pub const MAX_CPI_INSTRUCTION_DATA_LEN: usize = 10 * 1024;

    static_assertions::const_assert!(EVENT_BUFFER_LEN <= MAX_CPI_INSTRUCTION_DATA_LEN);

    assert_eq!(
        solana_sdk::syscalls::MAX_CPI_INSTRUCTION_DATA_LEN as usize,
        MAX_CPI_INSTRUCTION_DATA_LEN
    );
}
