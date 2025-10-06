#![no_std]

use pinocchio::{no_allocator, nostd_panic_handler, program_entrypoint};

mod entrypoint;
mod instructions;
mod shared;

program_entrypoint!(entrypoint::process_instruction);
no_allocator!();
nostd_panic_handler!();

pinocchio_pubkey::declare_id!("TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q");
