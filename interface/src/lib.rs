#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(all(feature = "pinocchio-invoke", feature = "solana-sdk-invoke"))]
compile_error!("pinocchio-invoke and solana-sdk-invoke are mutually exclusive features");

#[cfg(all(feature = "pinocchio-invoke", feature = "client"))]
compile_error!("pinocchio-invoke and client are mutually exclusive features");

#[cfg(all(feature = "solana-sdk-invoke", feature = "client"))]
compile_error!("solana-sdk-invoke and client are mutually exclusive features");

pub mod error;
pub mod instructions;
pub mod state;
pub mod utils;

pub mod program {
    pinocchio_pubkey::declare_id!("TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q");
}
