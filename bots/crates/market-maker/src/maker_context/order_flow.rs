use std::collections::HashMap;

use dropset_interface::{
    instructions::{
        CancelOrderInstructionData,
        PostOrderInstructionData,
    },
    state::sector::SectorIndex,
};
use itertools::Itertools;
use price::{
    client_helpers::to_order_info_args,
    to_order_info,
    OrderInfoArgs,
};
use rust_decimal::Decimal;
use transaction_parser::views::OrderView;

use crate::maker_context::{
    order_as_key::OrderAsKey,
    utils::split_symmetric_difference,
};

/// Given the collections of bids/asks to cancel and bids/asks to post, determine which orders would
/// be redundant and then filter them out from the set of resulting instructions.
///
/// That is, if an order would be canceled and then reposted, the cancel and post instruction are
/// both redundant and should be filtered out.
///
/// The bids and asks in the latest stored state might be stale due to fills.
/// This will cause the cancel order attempts to fail and should be expected intermittently.
pub fn get_non_redundant_order_flow(
    bids_to_cancel: Vec<OrderView>,
    asks_to_cancel: Vec<OrderView>,
    bids_to_post: Vec<(Decimal, u64)>, // (price, size) tuples.
    asks_to_post: Vec<(Decimal, u64)>, // (price, size) tuples.
    maker_seat_index: SectorIndex,
) -> anyhow::Result<(
    Vec<CancelOrderInstructionData>,
    Vec<PostOrderInstructionData>,
)> {
    // Map the existing maker's key-able order infos to their respective orders.
    // These will be the orders that are canceled.
    let bid_cancels = to_order_view_map(bids_to_cancel);
    let ask_cancels = to_order_view_map(asks_to_cancel);

    // Map the incoming (to-be-posted) key-able order infos to their respective order info args.
    let bid_posts = to_order_args_map(bids_to_post)?;
    let ask_posts = to_order_args_map(asks_to_post)?;

    // Retain only the unique values in two hash maps `a` and `b`, where each item in `a` does not
    // have a corresponding matching key in `b`.
    let (c_ask, p_ask, c_bid, p_bid) = (&ask_cancels, &ask_posts, &bid_cancels, &bid_posts);
    let (unique_bid_posts, unique_bid_cancels) = split_symmetric_difference(p_bid, c_bid);
    let (unique_ask_posts, unique_ask_cancels) = split_symmetric_difference(p_ask, c_ask);

    let cancels = unique_bid_cancels
        .iter()
        .map(|c| CancelOrderInstructionData::new(c.encoded_price, true, maker_seat_index))
        .chain(
            unique_ask_cancels
                .iter()
                .map(|c| CancelOrderInstructionData::new(c.encoded_price, false, maker_seat_index)),
        )
        .collect_vec();

    let posts = unique_bid_posts
        .into_iter()
        .map(|p| PostOrderInstructionData::new(p.clone(), true, maker_seat_index))
        .chain(
            unique_ask_posts
                .into_iter()
                .map(|p| PostOrderInstructionData::new(p.clone(), false, maker_seat_index)),
        )
        .collect_vec();

    Ok((cancels, posts))
}

pub fn to_order_args_map(
    prices_and_sizes: Vec<(Decimal, u64)>,
) -> anyhow::Result<HashMap<OrderAsKey, OrderInfoArgs>> {
    prices_and_sizes
        .into_iter()
        .map(|(price, size)| {
            let args = to_order_info_args(price, size)?;
            let order_info = to_order_info(args.clone())?;
            Ok((order_info.into(), args))
        })
        .collect()
}

pub fn to_order_view_map(orders: Vec<OrderView>) -> HashMap<OrderAsKey, OrderView> {
    orders
        .into_iter()
        .map(|order| (order.clone().into(), order))
        .collect()
}

#[cfg(test)]
mod tests {
    use price::EncodedPrice;
    use rust_decimal::dec;

    use super::*;

    const MAKER_SEAT_INDEX: SectorIndex = 0;

    /// Helper to create an OrderView stub based on the input price and size.
    fn to_order_view_stub(price: Decimal, size: u64) -> OrderView {
        let args = to_order_info_args(price, size).unwrap();
        let info = to_order_info(args).unwrap();
        OrderView {
            prev_index: 0,
            index: 0,
            next_index: 0,
            encoded_price: info.encoded_price.as_u32(),
            user_seat: MAKER_SEAT_INDEX,
            base_remaining: info.base_atoms,
            quote_remaining: info.quote_atoms,
        }
    }

    /// Helper function to convert [`PostOrderInstructionData`] to an encoded price.
    fn post_data_to_encoded_price(data: PostOrderInstructionData) -> EncodedPrice {
        to_order_info(data.order_info_args)
            .expect("Should create order info")
            .encoded_price
    }

    #[test]
    fn filters_redundant_orders() {
        // All order sizes are equal.
        // For bids and asks: cancels at prices 1, 2, 3 and posts at 3, 4, 5.
        // The orders with price 3 are thus redundant.
        let size = 1;

        let cancel_1 = to_order_view_stub(dec!(1.00), size);
        let cancel_2 = to_order_view_stub(dec!(2.00), size);
        let cancel_3 = to_order_view_stub(dec!(3.00), size);
        let post_3 = (dec!(3.00), size);
        let post_4 = (dec!(4.00), size);
        let post_5 = (dec!(5.00), size);

        let (cancels, posts) = get_non_redundant_order_flow(
            vec![cancel_1.clone(), cancel_2.clone(), cancel_3.clone()],
            vec![cancel_1, cancel_2, cancel_3],
            vec![post_3, post_4, post_5],
            vec![post_3, post_4, post_5],
            MAKER_SEAT_INDEX,
        )
        .unwrap();

        // 2 unique bid cancels + 2 unique ask cancels = 4 (price 3 filtered out)
        assert_eq!(cancels.len(), 4);
        // 2 unique bid posts + 2 unique ask posts = 4 (price 3 filtered out)
        assert_eq!(posts.len(), 4);

        // Verify price 3 was filtered out.
        let price_3_info = to_order_info(to_order_info_args(dec!(3.00), size).unwrap()).unwrap();
        let price_3_encoded = price_3_info.encoded_price.as_u32();

        // Ensure that both cancels and posts don't have any orders with price 3.
        assert!(!cancels.iter().any(|c| c.encoded_price == price_3_encoded));
        assert!(!posts
            .into_iter()
            .any(|p| post_data_to_encoded_price(p).as_u32() == price_3_encoded));
    }

    #[test]
    fn empty_inputs_returns_empty() {
        let (cancels, posts) =
            get_non_redundant_order_flow(vec![], vec![], vec![], vec![], MAKER_SEAT_INDEX).unwrap();

        assert!(cancels.is_empty());
        assert!(posts.is_empty());
    }

    #[test]
    fn redundancy_requires_matching_price_and_size() {
        // Orders are only redundant if both price AND size match.
        // cancel_1 and post_1 match in price and size → redundant
        // cancel_2 and post_2 have unique (price, size) tuples → not redundant
        let cancel_1 = to_order_view_stub(dec!(1.00), 10000);
        let cancel_2 = to_order_view_stub(dec!(1.00), 11111); // different size
        let post_1 = (dec!(1.00), 10000);
        let post_2 = (dec!(1.11), 10000); // different price

        let (cancels, posts) = get_non_redundant_order_flow(
            vec![cancel_1.clone(), cancel_2.clone()],
            vec![cancel_1.clone(), cancel_2.clone()],
            vec![post_1, post_2],
            vec![post_1, post_2],
            MAKER_SEAT_INDEX,
        )
        .unwrap();

        // The first cancel was filtered out.
        // Only the second cancel (for both bid and ask) should remain.
        assert_eq!(
            cancels,
            vec![
                CancelOrderInstructionData::new(
                    cancel_2.clone().encoded_price,
                    true,
                    MAKER_SEAT_INDEX
                ),
                CancelOrderInstructionData::new(cancel_2.encoded_price, false, MAKER_SEAT_INDEX),
            ]
        );

        // The first post was filtered out.
        // Only the second post (for both bid and ask) should remain.
        let p2 = to_order_info_args(post_2.0, post_2.1).expect("Should convert to order info args");
        assert_eq!(
            posts,
            vec![
                PostOrderInstructionData::new(p2.clone(), true, MAKER_SEAT_INDEX),
                PostOrderInstructionData::new(p2, false, MAKER_SEAT_INDEX),
            ]
        );
    }
}
