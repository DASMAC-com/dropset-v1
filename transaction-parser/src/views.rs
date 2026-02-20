//! Read-only view helpers for decoding `dropset` on-chain market accounts into ergonomic Rust
//! structs.

use std::collections::HashMap;

use dropset_interface::state::{
    market::MarketRef,
    market_header::MarketHeader,
    market_seat::MarketSeat,
    order::Order,
    sector::{
        Sector,
        SectorIndex,
    },
    transmutable::Transmutable,
    user_order_sectors::UserOrderSectors,
};
use itertools::Itertools;
use price::EncodedPrice;
use solana_address::Address;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarketHeaderView {
    pub discriminant: u64,
    pub num_seats: u32,
    pub num_bids: u32,
    pub num_asks: u32,
    pub num_free_sectors: u32,
    pub free_stack_top: SectorIndex,
    pub seats_dll_head: SectorIndex,
    pub seats_dll_tail: SectorIndex,
    pub bids_dll_head: SectorIndex,
    pub bids_dll_tail: SectorIndex,
    pub asks_dll_head: SectorIndex,
    pub asks_dll_tail: SectorIndex,
    pub base_mint: Address,
    pub quote_mint: Address,
    pub market_bump: u8,
    pub nonce: u64,
    pub _padding: [u8; 3],
}

/// A view on a market account's data with the collection of type T sectors.
#[derive(Clone, Debug)]
pub struct MarketView<T> {
    pub header: MarketHeaderView,
    pub sectors: Vec<T>,
}

/// The various data associated with a single user for each market.
#[derive(Clone, Debug)]
pub struct MarketUserData {
    pub seat: MarketSeatView,
    pub bids: Vec<OrderView>,
    pub asks: Vec<OrderView>,
}

/// A view on a market account's data showing all collections of all sector types.
#[derive(Clone, Debug)]
pub struct MarketViewAll {
    pub header: MarketHeaderView,
    pub seats: Vec<MarketSeatView>,
    pub bids: Vec<OrderView>,
    pub asks: Vec<OrderView>,
    pub users: HashMap<Address, MarketUserData>,
}

/// Attempts to parse a Dropset market account from raw Solana account fields and convert it into a
/// fully-typed market view.
///
/// Checks account data length with [try_market_view_all] in addition to ensuring that the passed
/// `account_owner` is [dropset::ID].
pub fn try_market_view_all_from_owner_and_data(
    account_owner: Address,
    account_data: &[u8],
) -> anyhow::Result<MarketViewAll> {
    if account_owner != dropset::ID {
        return Err(anyhow::Error::msg("Account isn't owned by dropset program"));
    }

    try_market_view_all(account_data)
}

/// Attempts to parse a Dropset market account from raw Solana account data and convert it into a
/// fully-typed market view.
///
/// Validates that:
/// - `account_data` is at least [`MarketHeader::LEN`] bytes (i.e., initialized enough to contain a
///   header).
/// - `account_data` is properly aligned for a market with a header + some number of [`Sector`]s.
///
/// On success, returns a [`MarketViewAll`] over `account_data` (header + sector bytes).
pub fn try_market_view_all(account_data: &[u8]) -> anyhow::Result<MarketViewAll> {
    if account_data.len() < MarketHeader::LEN {
        return Err(anyhow::Error::msg("Account is uninitialized"));
    }

    let bytes_past_header = account_data.len() - MarketHeader::LEN;
    if bytes_past_header % Sector::LEN != 0 {
        let msg = format!("Account has an invalid amount of bytes, got {bytes_past_header}");
        return Err(anyhow::Error::msg(msg));
    }

    // Safety: Length was just checked.
    let market = unsafe { MarketRef::from_bytes(account_data) };

    Ok(market.into())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarketSeatView {
    pub prev_index: SectorIndex,
    pub index: SectorIndex,
    pub next_index: SectorIndex,
    pub user: Address,
    pub base_available: u64,
    pub quote_available: u64,
    pub user_order_sectors: UserOrderSectors,
}

#[derive(Clone, Debug)]
pub struct OrderView {
    pub prev_index: SectorIndex,
    pub index: SectorIndex,
    pub next_index: SectorIndex,
    pub encoded_price: EncodedPrice,
    pub user_seat: SectorIndex,
    pub base_remaining: u64,
    pub quote_remaining: u64,
}

impl From<(SectorIndex, &Sector)> for MarketSeatView {
    fn from(index_and_seat: (SectorIndex, &Sector)) -> Self {
        let (sector_index, sector) = index_and_seat;
        let seat = sector.load_payload::<MarketSeat>();
        Self {
            prev_index: sector.prev(),
            index: sector_index,
            next_index: sector.next(),
            user: seat.user,
            base_available: seat.base_available(),
            quote_available: seat.quote_available(),
            user_order_sectors: seat.user_order_sectors.clone(),
        }
    }
}

impl From<(SectorIndex, &Sector)> for OrderView {
    fn from(index_and_order: (SectorIndex, &Sector)) -> Self {
        let (sector_index, sector) = index_and_order;
        let order = sector.load_payload::<Order>();
        Self {
            prev_index: sector.prev(),
            index: sector_index,
            next_index: sector.next(),
            encoded_price: order.encoded_price(),
            user_seat: order.user_seat(),
            base_remaining: order.base_remaining(),
            quote_remaining: order.quote_remaining(),
        }
    }
}

impl From<&MarketHeader> for MarketHeaderView {
    fn from(header: &MarketHeader) -> Self {
        Self {
            discriminant: header.discriminant(),
            num_seats: header.num_seats(),
            num_bids: header.num_bids(),
            num_asks: header.num_asks(),
            num_free_sectors: header.num_free_sectors(),
            free_stack_top: header.free_stack_top(),
            seats_dll_head: header.seats_dll_head(),
            seats_dll_tail: header.seats_dll_tail(),
            bids_dll_head: header.bids_dll_head(),
            bids_dll_tail: header.bids_dll_tail(),
            asks_dll_head: header.asks_dll_head(),
            asks_dll_tail: header.asks_dll_tail(),
            base_mint: header.base_mint,
            quote_mint: header.quote_mint,
            market_bump: header.market_bump,
            nonce: header.num_events(),
            _padding: [0; 3],
        }
    }
}

impl From<MarketRef<'_>> for MarketViewAll {
    fn from(market: MarketRef<'_>) -> Self {
        let seats = market.iter_seats().map(MarketSeatView::from).collect_vec();
        let bids = market.iter_bids().map(OrderView::from).collect_vec();
        let asks = market.iter_asks().map(OrderView::from).collect_vec();

        // Map seat indices to user addresses to O(1) get a user's address from an order.
        let seat_index_to_user: HashMap<SectorIndex, &Address> =
            seats.iter().map(|s| (s.index, &s.user)).collect();

        // Instantiate all user data with their corresponding seat and empty bids/asks.
        let mut users: HashMap<Address, MarketUserData> = seats
            .iter()
            .map(|s| {
                let new_user_data = MarketUserData {
                    seat: s.clone(),
                    bids: vec![],
                    asks: vec![],
                };
                (s.user, new_user_data)
            })
            .collect();

        // Update each user entry's bids.
        for bid in bids.iter() {
            let user = seat_index_to_user
                .get(&bid.user_seat)
                .expect("Should find user in seat index map");
            users
                .get_mut(user)
                .expect("Should find user in user map")
                .bids
                .push(bid.clone());
        }

        // Update each user entry's asks.
        for ask in asks.iter() {
            let user = seat_index_to_user
                .get(&ask.user_seat)
                .expect("Should find user in seat index map");
            users
                .get_mut(user)
                .expect("Should find user in user map")
                .asks
                .push(ask.clone());
        }

        Self {
            header: market.header.into(),
            seats,
            bids,
            asks,
            users,
        }
    }
}
