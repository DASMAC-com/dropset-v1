//! Creates a market making bot that utilizes the strategy defined in [`crate::calculate_spreads`].

mod calculate_spreads;
pub mod maker_context;

use calculate_spreads::*;

fn main() {
    println!("Hello, world!");
}
