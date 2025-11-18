//! See [`EventBuffer`].

use core::mem::{
    offset_of,
    MaybeUninit,
};

use dropset_interface::{
    events::{
        HeaderInstructionData,
        PackIntoSlice,
    },
    instructions::DropsetInstruction,
    program,
};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{
        AccountMeta,
        Instruction,
    },
    program::invoke_signed_unchecked,
    pubkey::Pubkey,
};

/// The event buffer length, also exactly the max CPI instruction data length.
/// That value is checked below in unit tests.
pub const EVENT_BUFFER_LEN: usize = 10 * 1024;

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
pub struct EventBuffer<'a> {
    pub event_authority: &'a AccountInfo,
    pub event_authority_meta: AccountMeta<'a>,
    /// The stack-allocated, possibly initialized buffer bytes.
    ///
    /// The layout for the data is:
    /// - [0]: the instruction tag of the instruction that created this event buffer.
    /// - [1..HeaderInstructionData::LEN_WITH_TAG]: the header instruction data.
    /// - [HeaderInstructionData::LEN_WITH_TAG..]: the byte data for the other non-header events in
    ///   the buffer.
    pub data: [MaybeUninit<u8>; EVENT_BUFFER_LEN],
    /// The amount of initialized bytes. The index at `len` is the first uninitialized byte.
    len: usize,
}

const EMITTED_COUNT_OFFSET: usize = offset_of!(HeaderInstructionData, emitted_count);
const EMITTED_COUNT_SIZE: usize = size_of::<u16>();
const NONCE_OFFSET: usize = offset_of!(HeaderInstructionData, nonce);
const NONCE_SIZE: usize = size_of::<u64>();

/// The total size of the dropset FlushEvents instruction tag + the header instruction data
/// tag + the header instruction event data.
///
/// This is essentially where the emitted events data actually starts.
const HEADER_SIZE_WITH_TAGS: usize =
    size_of::<DropsetInstruction>() + HeaderInstructionData::LEN_WITH_TAG;

impl<'a> EventBuffer<'a> {
    pub fn new(
        instruction_tag: DropsetInstruction,
        market: Pubkey,
        event_authority: &'a AccountInfo,
    ) -> Self {
        let mut data: [MaybeUninit<u8>; 10240] = [MaybeUninit::uninit(); EVENT_BUFFER_LEN];
        // Manually pack the instruction tag for the CPI invocation.
        data[0].write(DropsetInstruction::FlushEvents as u8);
        let mut len = 1;
        // Then pack the event header.
        let header = HeaderInstructionData::new(instruction_tag as u8, 0, 0, market);
        // HeaderInstructionData::pack(&self)

        // Safety: data's length is sufficient and `len` increments by the header's length below.
        unsafe { header.pack_into_slice(&mut data, len) };

        len += HeaderInstructionData::LEN_WITH_TAG;

        debug_assert_eq!(len, HEADER_SIZE_WITH_TAGS,);

        Self {
            event_authority,
            event_authority_meta: AccountMeta::readonly_signer(event_authority.key()),
            data,
            len,
        }
    }

    /// Flush the event buffer by invoking a CPI with the buffer data.
    pub fn flush_events(&mut self) {
        // Safety; `data` has exactly `self.len()` initialized, contiguous bytes.
        let data =
            unsafe { core::slice::from_raw_parts(self.data.as_ptr() as *const u8, self.len()) };

        // Safety: The only account in the instruction is `event_authority`
        // and it has no data, so it is never borrowed in any context.
        unsafe {
            invoke_signed_unchecked(
                &Instruction {
                    program_id: &program::ID,
                    data,
                    accounts: &[self.event_authority_meta],
                },
                &[self.event_authority.into()],
                &[seeds::event_authority::SEEDS],
            )
        };

        // Effectively "truncate" the buffer back down to the header size.
        self.len = HEADER_SIZE_WITH_TAGS;
        // Reset the count.
        self.set_emitted_count(0);
    }

    pub fn add_to_buffer<T: PackIntoSlice>(&mut self, packable_event: T) {
        let len = self.len();
        if len + T::LEN_WITH_TAG > EVENT_BUFFER_LEN {
            self.flush_events();
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

        self.increment_emitted_count();
        self.increment_len_by(T::LEN_WITH_TAG);
    }

    #[inline(always)]
    fn increment_len_by(&mut self, amount: usize) {
        self.len += amount;
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    /// Safety:
    ///
    /// Caller guarantees that the pointer returned doesn't outlive the `self.data` slice.
    ///
    /// The simplest way to enforce this is to immediately use and drop the pointer.
    unsafe fn emitted_count_slice_mut_ptr(&mut self) -> *mut [u8; 2] {
        // Sanity check to ensure that there's no way the `.add` call below could ever result in UB.
        debug_assert!(
            size_of::<DropsetInstruction>() + EMITTED_COUNT_OFFSET + EMITTED_COUNT_SIZE
                <= EVENT_BUFFER_LEN
        );
        // Safety: `&mut` requires aliasing rules to be upheld. The `.add` call
        // can't result in undefined behavior because the slice size is always much larger
        // than the offset computed.
        unsafe {
            self.data
                .as_mut_ptr()
                // The first byte is the `FlushEvents` tag.
                .add(size_of::<DropsetInstruction>() + EMITTED_COUNT_OFFSET)
                as *mut [u8; EMITTED_COUNT_SIZE]
        }
    }

    /// Set the emitted count through raw pointer dereferencing using
    /// the compile-time checked offset and byte size.
    fn set_emitted_count(&mut self, new_count: u16) {
        // Safety:
        // No other reference to this data is currently held.
        unsafe {
            // Safety: The slice pointer is used and dropped immediately.
            let emitted_count_slice_ptr = self.emitted_count_slice_mut_ptr();
            core::ptr::copy_nonoverlapping(
                new_count.to_le_bytes().as_ptr(),
                emitted_count_slice_ptr as _,
                EMITTED_COUNT_SIZE,
            );
        };
    }

    /// Increment the emitted count.
    fn increment_emitted_count(&mut self) {
        // Safety: The slice pointer is used and dropped immediately.
        // The slice pointer always points to fully initialized data because
        // the header is initialized upon construction.
        let new_emitted_count =
            unsafe { u16::from_le_bytes(*self.emitted_count_slice_mut_ptr()) } + 1;
        self.set_emitted_count(new_emitted_count);
    }

    /// Increment the nonce through raw pointer dereferencing using
    /// the compile-time checked offset and byte size.
    fn increment_nonce(&mut self) {
        // Safety:
        // The first 1 + `HeaderInstructionData::LEN_WITH_TAG` bytes are always initialized.
        // No other reference to this data is currently held.
        unsafe {
            let nonce_slice = self
                .data
                .as_mut_ptr()
                // The first byte is the `FlushEvents` tag.
                .add(size_of::<DropsetInstruction>() + NONCE_OFFSET)
                as *mut [u8; NONCE_SIZE];
            let emitted_count = u64::from_le_bytes(*nonce_slice);
            let incremented = emitted_count + 1;
            core::ptr::copy_nonoverlapping(
                incremented.to_le_bytes().as_ptr(),
                nonce_slice as _,
                NONCE_SIZE,
            );
        };
    }
}

// Ensure `emitted_count` and `nonce` are the expected types and size.
const _: () = {
    fn assert_types(val: &HeaderInstructionData) {
        let _: &u16 = &val.emitted_count;
        let _: &u64 = &val.nonce;
        let _: [u8; EMITTED_COUNT_SIZE] = [0; size_of::<u16>()];
        let _: [u8; NONCE_SIZE] = [0; size_of::<u64>()];
    }
};

#[test]
fn test_max_cpi_len() {
    let _: [u8; solana_sdk::syscalls::MAX_CPI_INSTRUCTION_DATA_LEN as usize] =
        [0u8; EVENT_BUFFER_LEN];
    assert_eq!(
        solana_sdk::syscalls::MAX_CPI_INSTRUCTION_DATA_LEN as usize,
        EVENT_BUFFER_LEN
    );
}
