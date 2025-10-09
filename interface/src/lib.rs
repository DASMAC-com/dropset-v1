#![no_std]

pub mod error;
pub mod instructions;
pub mod pack;
pub mod state;
pub mod utils;

pub mod program {
    pinocchio_pubkey::declare_id!("TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q");
}
