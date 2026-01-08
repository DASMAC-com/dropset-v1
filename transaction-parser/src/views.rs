//! Read-only view helpers for decoding `dropset` on-chain market accounts into ergonomic Rust
//! structs.

use dropset_interface::state::{
    market::MarketRef,
    market_header::MarketHeader,
    market_seat::MarketSeat,
    node::Node,
    sector::SectorIndex,
    transmutable::Transmutable,
};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug)]
pub struct MarketHeaderView {
    pub discriminant: u64,
    pub num_seats: u32,
    pub num_free_sectors: u32,
    pub free_stack_top: SectorIndex,
    pub seats_dll_head: SectorIndex,
    pub seats_dll_tail: SectorIndex,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub market_bump: u8,
    pub nonce: u64,
    pub _padding: [u8; 3],
}

/// A view on a market account's data with the collection of type T nodes.
#[derive(Debug)]
pub struct MarketView<T> {
    pub header: MarketHeaderView,
    pub sectors: Vec<T>,
}

/// A view on a market account's data showing all collections of all node types.
#[derive(Debug)]
pub struct MarketViewAll {
    pub header: MarketHeaderView,
    pub seats: Vec<MarketSeatView>,
}

/// Attempts to parse a Dropset market account from raw Solana account fields and convert it into a
/// fully-typed market view.
///
/// Validates that:
/// - `account_owner` matches the Dropset program id, and
/// - `account_data` is at least [`MarketHeader::LEN`] bytes (i.e., initialized enough to contain a
///   header).
///
/// On success, returns a [`MarketViewAll`] over `account_data` (header + sector bytes).
///
/// # Errors
/// Returns an error if the account is not owned by the Dropset program or if the data is too short.
pub fn try_market_view_all_from_owner_and_data(
    account_owner: Pubkey,
    account_data: &[u8],
) -> Result<MarketViewAll, anyhow::Error> {
    if account_owner != dropset::ID.into() {
        return Err(anyhow::Error::msg("Account isn't owned by dropset program"));
    }

    if account_data.len() < MarketHeader::LEN {
        return Err(anyhow::Error::msg("Account is uninitialized"));
    }

    // Safety: Length was just checked.
    let market = unsafe { MarketRef::from_bytes(account_data) };

    Ok(market.into())
}

#[derive(Debug)]
pub struct MarketSeatView {
    pub prev_index: SectorIndex,
    pub index: SectorIndex,
    pub next_index: SectorIndex,
    pub user: Pubkey,
    pub base_deposited: u64,
    pub quote_deposited: u64,
    pub base_available: u64,
    pub quote_available: u64,
}

impl From<(SectorIndex, &Node)> for MarketSeatView {
    fn from(index_and_seat: (SectorIndex, &Node)) -> Self {
        let (sector_index, node) = index_and_seat;
        let seat = node.load_payload::<MarketSeat>();
        Self {
            prev_index: node.prev(),
            index: sector_index,
            next_index: node.next(),
            user: seat.user.into(),
            base_deposited: seat.base_deposited(),
            quote_deposited: seat.quote_deposited(),
            base_available: seat.base_available(),
            quote_available: seat.quote_available(),
        }
    }
}

impl From<&MarketHeader> for MarketHeaderView {
    fn from(header: &MarketHeader) -> Self {
        Self {
            discriminant: header.discriminant(),
            num_seats: header.num_seats(),
            num_free_sectors: header.num_free_sectors(),
            free_stack_top: header.free_stack_top(),
            seats_dll_head: header.seats_dll_head(),
            seats_dll_tail: header.seats_dll_tail(),
            base_mint: header.base_mint.into(),
            quote_mint: header.quote_mint.into(),
            market_bump: header.market_bump,
            nonce: header.num_events(),
            _padding: [0; 3],
        }
    }
}

impl From<MarketRef<'_>> for MarketView<MarketSeatView> {
    fn from(market: MarketRef) -> Self {
        Self {
            header: market.header.into(),
            sectors: market.iter_seats().map(MarketSeatView::from).collect(),
        }
    }
}

impl From<MarketRef<'_>> for MarketViewAll {
    fn from(market: MarketRef<'_>) -> Self {
        Self {
            header: market.header.into(),
            seats: market.iter_seats().map(MarketSeatView::from).collect(),
        }
    }
}
