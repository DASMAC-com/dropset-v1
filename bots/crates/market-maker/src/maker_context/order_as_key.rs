use std::hash::Hash;

use price::{
    EncodedPrice,
    OrderInfo,
};
use transaction_parser::views::OrderView;

#[derive(Hash, Eq, PartialEq)]
pub struct OrderAsKey {
    encoded_price: EncodedPrice,
    base: u64,
    quote: u64,
}

impl From<OrderInfo> for OrderAsKey {
    fn from(o: OrderInfo) -> Self {
        Self {
            encoded_price: o.encoded_price,
            base: o.base_atoms,
            quote: o.quote_atoms,
        }
    }
}

impl From<OrderView> for OrderAsKey {
    fn from(o: OrderView) -> Self {
        Self {
            encoded_price: o.encoded_price,
            base: o.base_remaining,
            quote: o.quote_remaining,
        }
    }
}
