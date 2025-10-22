#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod context;
mod debug;
mod instructions;
mod shared;
mod validation;

pub use shared::seeds::market::MARKET_SEED_STR;
#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

pinocchio_pubkey::declare_id!("TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q");
