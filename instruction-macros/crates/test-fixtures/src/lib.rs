//! Test fixtures for verifying macro expansion across feature namespaces.
//!
//! This crate provides isolated environments for testing generated instruction
//! code under the two different compilation features: `client` and `program`.

#![allow(dead_code)]
#![allow(unused_imports)]

mod client;
mod events;
mod pack;
mod program;

use solana_address::Address;

pub const ID: Address = Address::from_str_const("TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q");
