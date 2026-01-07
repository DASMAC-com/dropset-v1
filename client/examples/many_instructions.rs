use std::collections::{
    HashMap,
    HashSet,
};

use client::{
    e2e_helpers::{
        test_accounts,
        E2e,
        Trader,
    },
    transactions::{
        CustomRpcClient,
        SendTransactionConfig,
    },
};
use dropset_interface::state::sector::SectorIndex;
use itertools::Itertools;
use solana_instruction::Instruction;
use solana_sdk::{
    pubkey::Pubkey,
    signer::Signer,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = CustomRpcClient::new(
        None,
        Some(SendTransactionConfig {
            compute_budget: Some(2000000),
            debug_logs: Some(true),
            program_id_filter: HashSet::from([dropset_interface::program::ID.into()]),
        }),
    );
    // Create the collection of traders out of order so that the order must change when they're
    // sorted on insert later.
    let traders = [
        Trader::new(test_accounts::acc_5555(), 10000, 10000),
        Trader::new(test_accounts::acc_2222(), 10000, 10000),
        Trader::new(test_accounts::acc_4444(), 10000, 10000),
        Trader::new(test_accounts::acc_1111(), 10000, 10000),
        Trader::new(test_accounts::acc_3333(), 10000, 10000),
    ];
    let e2e = E2e::new_traders_and_market(Some(rpc), &traders).await?;

    // Create the seats for each trader.
    let seat_creations: Vec<Instruction> = traders
        .iter()
        .map(|pk| -> Instruction { e2e.market.create_seat(pk.pubkey()).into() })
        .collect();
    e2e.rpc
        .send_and_confirm_txn(
            test_accounts::default_payer(),
            &traders.iter().map(|tr| tr.keypair).collect_vec(),
            &seat_creations,
        )
        .await?;

    let seats: Vec<SectorIndex> = traders
        .iter()
        .map(|trader| {
            e2e.find_seat(&trader.pubkey())
                .ok()
                .flatten()
                .expect("Trader should have a seat")
                .index
        })
        .collect();

    // HashMap<Pubkey, (deposit_amount, withdraw_amount)>
    let base_amounts: HashMap<Pubkey, (u64, u64)> = HashMap::from([
        (test_accounts::acc_1111().pubkey(), (100, 10)),
        (test_accounts::acc_2222().pubkey(), (100, 20)),
        (test_accounts::acc_3333().pubkey(), (100, 30)),
        (test_accounts::acc_4444().pubkey(), (100, 40)),
        (test_accounts::acc_5555().pubkey(), (100, 50)),
    ]);

    let deposits_and_withdraws: Vec<Instruction> = traders
        .iter()
        .zip(seats)
        .flat_map(|(trader, seat)| {
            let trader_addr = trader.pubkey();
            let (deposit, withdraw) = base_amounts.get(&trader_addr).unwrap();
            [
                e2e.market.deposit_base(trader_addr, *deposit, seat).into(),
                e2e.market
                    .withdraw_base(trader_addr, *withdraw, seat)
                    .into(),
            ]
        })
        .collect();

    e2e.rpc
        .send_and_confirm_txn(
            test_accounts::default_payer(),
            &traders.iter().map(|tr| tr.keypair).collect_vec(),
            &deposits_and_withdraws,
        )
        .await?;

    let expected_base = base_amounts
        .into_iter()
        .map(|pk_and_amts| {
            let (pubkey, (deposit, withdraw)) = pk_and_amts;
            (pubkey, deposit, withdraw)
        })
        // Sort by the pubkey.
        .sorted_by_key(|v| v.0)
        .collect_vec();

    let market = e2e.view_market()?;

    // Check that seats are ordered by pubkey (ascending) and compare the final state of each user's
    // seat to the expected state.
    for (seat, expected_seat) in market.seats.iter().zip_eq(expected_base) {
        let (expected_pk, expected_base_dep, expected_base_wd) = expected_seat;
        assert_eq!(seat.user, expected_pk);
        let amount_from_create_seat = 1;
        let base_remaining = (expected_base_dep + amount_from_create_seat) - expected_base_wd;
        assert_eq!(seat.base_available, base_remaining);
    }

    Ok(())
}
