use pinocchio::pubkey::Pubkey;
use static_assertions::const_assert_eq;

use crate::{
    pack::{write_bytes, Pack},
    state::{
        node::{NodePayload, NODE_PAYLOAD_SIZE},
        transmutable::Transmutable,
        U64_SIZE,
    },
};

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketSeat {
    pub trader: Pubkey,
    base: [u8; U64_SIZE],
    quote: [u8; U64_SIZE],
}

impl MarketSeat {
    pub fn new(trader: Pubkey, base: u64, quote: u64) -> Self {
        MarketSeat {
            trader,
            base: base.to_le_bytes(),
            quote: quote.to_le_bytes(),
        }
    }
}

unsafe impl Transmutable for MarketSeat {
    const LEN: usize = NODE_PAYLOAD_SIZE;
}

impl NodePayload for MarketSeat {}

impl Pack<NODE_PAYLOAD_SIZE> for MarketSeat {
    fn pack_into_slice(&self, dst: &mut [core::mem::MaybeUninit<u8>; NODE_PAYLOAD_SIZE]) {
        write_bytes(dst, &self.trader);
        write_bytes(dst, &self.base);
        write_bytes(dst, &self.quote);
    }
}

const_assert_eq!(core::mem::size_of::<MarketSeat>(), NODE_PAYLOAD_SIZE);
const_assert_eq!(align_of::<MarketSeat>(), 1);
