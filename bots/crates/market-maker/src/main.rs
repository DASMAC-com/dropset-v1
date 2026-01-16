//! Creates a market making bot that utilizes the strategy defined in [`crate::calculate_spreads`].

pub mod calculate_spreads;
pub mod maker_context;
pub mod oanda;

fn main() {
    println!("Hello, world!");
}
