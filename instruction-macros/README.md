# Overview
This crate is a `shank`-like macro to
generate structs for instruction invocation in various contexts.

It always generates an instruction tag enum with a `TryFrom<u8>` impl and its
instruction data struct with `pack` and `unpack` methods.

There are three types of instruction invocation structs that can be generated,
each enabled with a mutually exclusive feature flag:

1. `#[cfg(feature = "pinocchio-invoke")]`
  - Uses the `pinocchio` SDK to expose `invoke` and `invoke_signed` for
  extremely efficient CPIs.
2. `#[cfg(feature = "solana-sdk-invoke")]`
  - Uses the standard `solana_sdk` to expose `invoke` and `invoke_signed` for
  CPIs.
3. `#[cfg(feature = "client")]`
  - Exposes a simple `solana_instruction::Instruction` creation helper that
  reduces boilerplate for account names and instruction data.

# Example

> **Note**: The code examples below are illustrative excerpts.
> The generator output may differ slightly in formatting or naming as the macro evolves.

To view output, install `cargo expand` and run it on a relevant struct, for
example:

```shell
cargo expand instructions --package dropset-interface
```

```rust
#[derive(ProgramInstruction)]
#[program_instruction(error = ProgramError::InvalidInstructionData)]
#[repr(u8)]
#[rustfmt::skip]
pub enum DropsetInstruction {
    #[account(0, signer,   name = "user",             desc = "The user closing their seat.")]
    #[account(1, writable, name = "market_account",   desc = "The market account PDA.")]
    #[account(2, writable, name = "quote_market_ata", desc = "The market's associated quote mint token account.")]
    #[account(3,           name = "base_mint",        desc = "The base token mint account.")]
    #[args(sector_index_hint: u32, "A hint indicating which sector the user's seat resides in.")]
    #[args(second_arg: u128, "A big argument")]
    #[args(third: u128, "Another big argument")]
    // Implicit discriminant of 0
    CloseSeat,

    #[account(0, signer,   name = "user",           desc = "The user depositing or registering their seat.")]
    #[account(1, writable, name = "market_account", desc = "The market account PDA.")]
    #[args(amount: u64, "The amount to deposit.")]
    #[args(sector_index_hint: u32, "A hint indicating which sector the user's seat resides in (pass `NIL` when registering a new seat).")]
    Deposit = 50,

    #[account(0, signer, writable, name = "user",        desc = "The user registering the market.")]
    RegisterMarket = 40,

    // Implicit discriminant of 41
    #[account(0, signer, name = "user",           desc = "The user withdrawing.")]
    Withdraw,

    // Implicit discriminant of 42
    #[account(0, signer, name = "payer")]
    Resize,
}
```


## Code generated

### The instruction enum and its `try_from`

The enum's `TryFrom<u8>` error type is determined by the
`#[program_instruction]` attribute:
```rust
// The error defaults to the solana program error type above, but a different
// one can be specified.
#[program_instruction(error = ProgramError::InvalidInstructionData)]
```

The `enum` and its `TryFrom<u8>` implementation:

```rust
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(test, derive(strum_macros::FromRepr, strum_macros::EnumIter))]
pub enum DropsetInstruction {
    CloseSeat = 0,
    Deposit = 50,
    RegisterMarket = 40,
    Withdraw = 41,
    Resize = 42,
}

impl TryFrom<u8> for DropsetInstruction {
    type Error = ProgramError;
    
    fn try_from(tag: u8) -> Result<Self, Self::Error> {
        match tag {
            0 | 40..=42 | 50 => Ok(unsafe { core::mem::transmute::<u8, Self>(tag) }),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
```

### The instruction data and its `pack` and `unpack` methods

For example, for `CloseSeat`:

```rust
/// Instruction data for `DropsetInstruction::CloseSeat`.
///
/// - `sector_index_hint` — A hint indicating which sector the user's seat resides in.
/// - `second_arg` — A big argument
/// - `third` — Another big argument
pub struct CloseSeatInstructionData {
    /// A hint indicating which sector the user's seat resides in.
    pub sector_index_hint: u32,
    /// A big argument
    pub second_arg: u128,
    /// Another big argument
    pub third: u128,
}

/// Compile time assertion that the size with the tag == the sum of the field sizes.
const _: [(); 37] = [(); 1 + 4 + 16 + 16];

impl CloseSeatInstructionData {
    /// Instruction data layout:
    /// - [0]: the discriminant `DropsetInstruction::CloseSeat` (u8, 1 byte)
    /// - [1..5]: the `sector_index_hint` (u32, 4 bytes)
    /// - [5..21]: the `second_arg` (u128, 16 bytes)
    /// - [21..37]: the `third` (u128, 16 bytes)
    #[inline(always)]
    pub fn pack(&self) -> [u8; 37] {
        let mut data: [core::mem::MaybeUninit<u8>; 37] = [core::mem::MaybeUninit::uninit(); 37];
        data[0].write(DropsetInstruction::CloseSeat as u8);
        unsafe {
            core::ptr::copy_nonoverlapping(
                (&self.sector_index_hint.to_le_bytes()).as_ptr(),
                (&mut data[1..5]).as_mut_ptr() as *mut u8,
                4,
            );
            core::ptr::copy_nonoverlapping(
                (&self.second_arg.to_le_bytes()).as_ptr(),
                (&mut data[5..21]).as_mut_ptr() as *mut u8,
                16,
            );
            core::ptr::copy_nonoverlapping(
                (&self.third.to_le_bytes()).as_ptr(),
                (&mut data[21..37]).as_mut_ptr() as *mut u8,
                16,
            );
        }
        unsafe { *(data.as_ptr() as *const [u8; 37]) }
    }

    /// This method unpacks the instruction data that comes *after* the discriminant has
    /// already been peeled off of the front of the slice.
    /// Trailing bytes are ignored; the length must be sufficient, not exact.
    #[inline(always)]
    pub fn unpack(instruction_data: &[u8]) -> Result<Self, ProgramError> {
        if instruction_data.len() < 36 {
            return Err(ProgramError::InvalidInstructionData);
        }
        unsafe {
            let p = instruction_data.as_ptr();
            let sector_index_hint = u32::from_le_bytes(*(p as *const [u8; 4]));
            let second_arg = u128::from_le_bytes(*(p.add(4) as *const [u8; 16]));
            let third = u128::from_le_bytes(*(p.add(20) as *const [u8; 16]));
            Ok(Self {
                sector_index_hint,
                second_arg,
                third,
            })
        }
    }
}
```

### The feature-flag based instruction accounts struct and its helpers

For the `CloseSeat` example above, there are three types of generated account
info structs, each feature flagged.

#### `#[cfg(feature = "pinocchio-invoke")]`

```rust
#[cfg(feature = "pinocchio-invoke")]
pub struct CloseSeat<'a> {
    /// The user closing their seat.
    pub user: &'a pinocchio::account_info::AccountInfo,
    /// The market account PDA.
    pub market_account: &'a pinocchio::account_info::AccountInfo,
    /// The market's associated quote mint token account.
    pub quote_market_ata: &'a pinocchio::account_info::AccountInfo,
    /// The base token mint account.
    pub base_mint: &'a pinocchio::account_info::AccountInfo,
}

#[cfg(feature = "pinocchio-invoke")]
impl CloseSeat<'_> {
    #[inline(always)]
    pub fn invoke(self, data: CloseSeatInstructionData) -> pinocchio::ProgramResult {
        self.invoke_signed(&[], data)
    }
    #[inline(always)]
    pub fn invoke_signed(
        self,
        signers_seeds: &[pinocchio::instruction::Signer],
        data: CloseSeatInstructionData,
    ) -> pinocchio::ProgramResult {
        let accounts = &[
            pinocchio::instruction::AccountMeta::readonly_signer(self.user.key()),
            pinocchio::instruction::AccountMeta::writable(self.market_account.key()),
            pinocchio::instruction::AccountMeta::writable(
                self.quote_market_ata.key(),
            ),
            pinocchio::instruction::AccountMeta::readonly(self.base_mint.key()),
        ];
        let Self { user, market_account, quote_market_ata, base_mint } = self;
        pinocchio::cpi::invoke_signed(
            &pinocchio::instruction::Instruction {
                program_id: &crate::program::ID,
                accounts,
                data: &data.pack(),
            },
            &[user, market_account, quote_market_ata, base_mint],
            signers_seeds,
        )
    }
}
```

#### `#[cfg(feature = "solana-sdk-invoke")]`

```rust
#[cfg(feature = "solana-sdk-invoke")]
pub struct CloseSeat<'a> {
    /// The user closing their seat.
    pub user: solana_sdk::account_info::AccountInfo<'a>,
    /// The market account PDA.
    pub market_account: solana_sdk::account_info::AccountInfo<'a>,
    /// The market's associated quote mint token account.
    pub quote_market_ata: solana_sdk::account_info::AccountInfo<'a>,
    /// The base token mint account.
    pub base_mint: solana_sdk::account_info::AccountInfo<'a>,
}

#[cfg(feature = "solana-sdk-invoke")]
impl CloseSeat<'_> {
    #[inline(always)]
    pub fn invoke(
        self,
        data: CloseSeatInstructionData,
    ) -> solana_sdk::entrypoint::ProgramResult {
        self.invoke_signed(&[], data)
    }
    #[inline(always)]
    pub fn invoke_signed(
        self,
        signers_seeds: &[&[&[u8]]],
        data: CloseSeatInstructionData,
    ) -> solana_sdk::entrypoint::ProgramResult {
        let accounts = [
            solana_instruction::AccountMeta::new_readonly(*self.user.key, true),
            solana_instruction::AccountMeta::new(*self.market_account.key, false),
            solana_instruction::AccountMeta::new(*self.quote_market_ata.key, false),
            solana_instruction::AccountMeta::new_readonly(*self.base_mint.key, false),
        ]
            .to_vec();
        let Self { user, market_account, quote_market_ata, base_mint } = self;
        solana_cpi::invoke_signed(
            &solana_instruction::Instruction {
                program_id: crate::program::ID.into(),
                accounts,
                data: data.pack().to_vec(),
            },
            &[user, market_account, quote_market_ata, base_mint],
            signers_seeds,
        )
    }
}
```

#### `#[cfg(feature = "client")]`

```rust
#[cfg(feature = "client")]
pub struct CloseSeat {
    /// The user closing their seat.
    pub user: solana_sdk::pubkey::Pubkey,
    /// The market account PDA.
    pub market_account: solana_sdk::pubkey::Pubkey,
    /// The market's associated quote mint token account.
    pub quote_market_ata: solana_sdk::pubkey::Pubkey,
    /// The base token mint account.
    pub base_mint: solana_sdk::pubkey::Pubkey,
}
#[cfg(feature = "client")]
impl CloseSeat {
    #[inline(always)]
    pub fn create_instruction(
        &self,
        data: CloseSeatInstructionData,
    ) -> solana_instruction::Instruction {
        let accounts = [
            solana_instruction::AccountMeta::new_readonly(self.user, true),
            solana_instruction::AccountMeta::new(self.market_account, false),
            solana_instruction::AccountMeta::new(self.quote_market_ata, false),
            solana_instruction::AccountMeta::new_readonly(self.base_mint, false),
        ]
            .to_vec();
        solana_instruction::Instruction {
            program_id: crate::program::ID.into(),
            accounts,
            data: data.pack().to_vec(),
        }
    }
}
```

The doc comment for the struct is the same for all three types:

```rust
/// The invocation struct for a `DropsetInstruction::CloseSeat` instruction.
///
/// # Caller Guarantees
///
/// When invoking this instruction as a cross-program invocation, caller must ensure that:
/// - WRITE accounts are not currently borrowed in *any* capacity.
/// - READ accounts are not currently mutably borrowed.
///
/// ### Accounts
/// 0. `[READ, SIGNER]` `user` The user closing their seat.
/// 1. `[WRITE]` `market_account` The market account PDA.
/// 2. `[WRITE]` `quote_market_ata` The market's associated quote mint token account.
/// 3. `[READ]` `base_mint` The base token mint account.
pub struct ... {
    // ...
}
```
