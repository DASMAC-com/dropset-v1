//! Test fixtures for verifying macro expansion across feature namespaces.
//!
//! This crate provides isolated environments for testing generated instruction
//! code under different compilation features (`client`, `pinocchio`, and
//! `solana-program`).

#![allow(dead_code)]
#![allow(unused_imports)]

mod client;
mod events;
mod pinocchio;
mod solana_program;

use pinocchio_pubkey::pubkey;

pub const ID: [u8; 32] = pubkey!("TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q");
