//! Public interface layer defining instruction schemas, program state, and shared utilities for
//! on-chain and client integration.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod events;
pub mod instructions;
pub mod seeds;
pub mod state;
pub mod utils;

pub mod program {
    use solana_address::Address;

    pub const ID: Address = Address::from_str_const("TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q");
}
