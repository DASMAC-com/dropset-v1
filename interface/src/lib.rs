#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod instructions;
pub mod state;
pub mod utils;

pub mod program {
    pinocchio_pubkey::declare_id!("TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q");
}
