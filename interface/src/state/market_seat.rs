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
    /// The user's public key.
    pub user: Pubkey,
    /// Amount of base token deposited.
    base_deposited: [u8; U64_SIZE],
    /// Amount of quote token deposited.
    quote_deposited: [u8; U64_SIZE],
    /// Amount of base token available.
    base_available: [u8; U64_SIZE],
    /// Amount of quote token available.
    quote_available: [u8; U64_SIZE],
}

impl MarketSeat {
    pub fn new(user: Pubkey, base: u64, quote: u64) -> Self {
        MarketSeat {
            user,
            base_deposited: base.to_le_bytes(),
            quote_deposited: quote.to_le_bytes(),
            base_available: base.to_le_bytes(),
            quote_available: quote.to_le_bytes(),
        }
    }
}

unsafe impl Transmutable for MarketSeat {
    const LEN: usize = NODE_PAYLOAD_SIZE;
}

impl NodePayload for MarketSeat {}

impl Pack<NODE_PAYLOAD_SIZE> for MarketSeat {
    fn pack_into_slice(&self, dst: &mut [core::mem::MaybeUninit<u8>; NODE_PAYLOAD_SIZE]) {
        write_bytes(dst, &self.user);
        write_bytes(dst, &self.base_deposited);
        write_bytes(dst, &self.quote_deposited);
    }
}

const_assert_eq!(core::mem::size_of::<MarketSeat>(), NODE_PAYLOAD_SIZE);
const_assert_eq!(align_of::<MarketSeat>(), 1);
