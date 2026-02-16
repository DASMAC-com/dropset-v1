use std::fmt::Write;

use phoenix::{
    program::{
        deposit::DepositParams,
        instruction_builders::*,
        status::{
            MarketStatus,
            SeatApprovalStatus,
        },
        MarketSizeParams,
    },
    state::{
        OrderPacket,
        Side,
    },
};
use solana_program::{
    instruction::Instruction,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    system_instruction,
};
use solana_program_runtime::compute_budget::MAX_COMPUTE_UNIT_LIMIT;
use solana_program_test::{
    processor,
    ProgramTest,
    ProgramTestBanksClientExt,
    ProgramTestContext,
};
use solana_sdk::{
    commitment_config::CommitmentLevel,
    compute_budget::ComputeBudgetInstruction,
    program_pack::Pack as _,
    signature::{
        Keypair,
        Signer,
    },
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;

// ── Market parameters (matching Phoenix test defaults) ──────────────────────

pub const BASE_DECIMALS: u8 = 9;
pub const QUOTE_DECIMALS: u8 = 6;
pub const BASE_UNIT: u64 = 1_000_000_000; // 10^9
pub const QUOTE_UNIT: u64 = 1_000_000; // 10^6

pub const NUM_QUOTE_LOTS_PER_QUOTE_UNIT: u64 = 100_000;
pub const NUM_BASE_LOTS_PER_BASE_UNIT: u64 = 1_000;
pub const TICK_SIZE: u64 = 1_000;

// 1 base lot  = BASE_UNIT / NUM_BASE_LOTS_PER_BASE_UNIT = 1_000_000 atoms
// 1 quote lot = QUOTE_UNIT / NUM_QUOTE_LOTS_PER_QUOTE_UNIT = 10 atoms
// 1 tick      = TICK_SIZE * quote_lot_size / base_lot_size
//             = 1000 * 10 / 1_000_000 = 0.01 quote units per base unit
// So price_in_ticks = 1000 → 10 USDC/SOL

pub const BOOK_SIZE: u64 = 4096;
pub const NUM_SEATS: u64 = 128;
pub const N_WARMUP_ORDERS: usize = 10;

// ── TestFixture ─────────────────────────────────────────────────────────────

pub struct TestFixture {
    pub context: ProgramTestContext,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub mint_authority: Keypair,
    pub market: Pubkey,
    pub maker: Keypair,
    pub maker_base_ata: Pubkey,
    pub maker_quote_ata: Pubkey,
    pub logs: String,
}

// Print logs on TestFixture drop.
impl Drop for TestFixture {
    fn drop(&mut self) {
        if self.logs.is_empty() {
            return;
        }

        // Must run with `-- --nocapture` to see these.
        eprintln!("\n{}", self.logs);
    }
}

impl TestFixture {
    pub async fn new() -> Self {
        let program = ProgramTest::new(
            "phoenix",
            phoenix::id(),
            processor!(phoenix::process_instruction),
        );
        let mut context = program.start_with_context().await;
        solana_logger::setup();

        let mint_authority = Keypair::new();
        let payer = clone_keypair(&context.payer);

        // Fund the mint authority.
        airdrop(
            &mut context,
            &mint_authority.pubkey(),
            100 * LAMPORTS_PER_SOL,
        )
        .await;

        // Create base mint (9 decimals) and quote mint (6 decimals).
        let base_mint = Keypair::new();
        create_mint(
            &mut context,
            &base_mint,
            &mint_authority.pubkey(),
            BASE_DECIMALS,
        )
        .await;

        let quote_mint = Keypair::new();
        create_mint(
            &mut context,
            &quote_mint,
            &mint_authority.pubkey(),
            QUOTE_DECIMALS,
        )
        .await;

        // Payer needs a quote ATA for fee collection (even with 0 fees).
        let _payer_quote_ata =
            create_ata(&mut context, &payer.pubkey(), &quote_mint.pubkey()).await;

        // Create and activate the market.
        let market = Keypair::new();
        let params = MarketSizeParams {
            bids_size: BOOK_SIZE,
            asks_size: BOOK_SIZE,
            num_seats: NUM_SEATS,
        };

        let mut init_ixs = create_initialize_market_instructions_default(
            &market.pubkey(),
            &base_mint.pubkey(),
            &quote_mint.pubkey(),
            &payer.pubkey(),
            params,
            NUM_QUOTE_LOTS_PER_QUOTE_UNIT,
            NUM_BASE_LOTS_PER_BASE_UNIT,
            TICK_SIZE,
            0, // no taker fees for benchmarking
            None,
        )
        .unwrap();

        init_ixs.push(create_change_market_status_instruction(
            &payer.pubkey(),
            &market.pubkey(),
            MarketStatus::Active,
        ));

        send_txn(&mut context, &init_ixs, &[&payer, &market]).await;

        // Setup maker: fund, create ATAs, mint tokens.
        let maker = Keypair::new();
        airdrop(&mut context, &maker.pubkey(), 100 * LAMPORTS_PER_SOL).await;
        let maker_base_ata = create_ata(&mut context, &maker.pubkey(), &base_mint.pubkey()).await;
        let maker_quote_ata = create_ata(&mut context, &maker.pubkey(), &quote_mint.pubkey()).await;

        mint_to(
            &mut context,
            &mint_authority,
            &base_mint.pubkey(),
            &maker_base_ata,
            1_000_000 * BASE_UNIT,
        )
        .await;
        mint_to(
            &mut context,
            &mint_authority,
            &quote_mint.pubkey(),
            &maker_quote_ata,
            1_000_000 * QUOTE_UNIT,
        )
        .await;

        // Request seat (authorized by admin/payer) and approve.
        let request_ix = create_request_seat_authorized_instruction(
            &payer.pubkey(),
            &payer.pubkey(),
            &market.pubkey(),
            &maker.pubkey(),
        );
        send_txn(&mut context, &[request_ix], &[&payer]).await;

        let approve_ix = create_change_seat_status_instruction(
            &payer.pubkey(),
            &market.pubkey(),
            &maker.pubkey(),
            SeatApprovalStatus::Approved,
        );
        send_txn(&mut context, &[approve_ix], &[&payer]).await;

        let base_mint_key = base_mint.pubkey();
        let quote_mint_key = quote_mint.pubkey();
        let market_key = market.pubkey();

        TestFixture {
            context,
            base_mint: base_mint_key,
            quote_mint: quote_mint_key,
            mint_authority,
            market: market_key,
            maker,
            maker_base_ata,
            maker_quote_ata,
            logs: Default::default(),
        }
    }

    pub fn payer_keypair(&self) -> Keypair {
        clone_keypair(&self.context.payer)
    }

    pub fn maker_keypair(&self) -> Keypair {
        clone_keypair(&self.maker)
    }
}

pub const NUM_INITIAL_ORDERS_PER_SIDE: u64 = 5;

/// Deposit tokens for the maker and payer.
pub async fn initialize_fixture(f: &mut TestFixture) -> anyhow::Result<()> {
    let maker = f.maker_keypair();
    let payer = f.payer_keypair();

    // Deposit plenty of both tokens.
    let deposit_ix = create_deposit_funds_instruction(
        &f.market,
        &maker.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &DepositParams {
            quote_lots_to_deposit: 500_000 * NUM_QUOTE_LOTS_PER_QUOTE_UNIT,
            base_lots_to_deposit: 500 * NUM_BASE_LOTS_PER_BASE_UNIT,
        },
    );
    send_txn(&mut f.context, &[deposit_ix], &[&payer, &maker]).await;

    Ok(())
}

/// Create a fresh fixture then initialize and return it.
pub async fn new_initialized_fixture() -> anyhow::Result<TestFixture> {
    let mut f = TestFixture::new().await;
    initialize_fixture(&mut f).await?;
    Ok(f)
}

// ── CU measurement ─────────────────────────────────────────────────────────

/// Simulate a transaction to get CU consumed, then process it to apply state
/// changes. In Solana 1.14.x, `simulate_transaction` returns
/// `TransactionSimulationDetails` which includes `units_consumed`.
pub async fn send_txn_measure_cu(
    ctx: &mut ProgramTestContext,
    ixs: &[Instruction],
    extra_signers: &[&Keypair],
) -> u64 {
    let payer = clone_keypair(&ctx.payer);
    let (blockhash, _) = ctx
        .banks_client
        .get_latest_blockhash_with_commitment(CommitmentLevel::Confirmed)
        .await
        .unwrap()
        .unwrap();

    let mut signers: Vec<&Keypair> = vec![&payer];
    signers.extend(extra_signers);

    let tx = Transaction::new_signed_with_payer(
        &[
            vec![
                ComputeBudgetInstruction::set_compute_unit_limit(MAX_COMPUTE_UNIT_LIMIT),
                ComputeBudgetInstruction::set_compute_unit_price(1),
            ],
            ixs.to_vec(),
        ]
        .concat(),
        Some(&payer.pubkey()),
        &signers,
        blockhash,
    );

    // Simulate to get CU consumed.
    let sim = ctx
        .banks_client
        .simulate_transaction(tx.clone())
        .await
        .unwrap();
    match sim.result {
        Some(res) => match res {
            Ok(_) => {}
            Err(err) => panic!("Simulation failed: {:?}", err),
        },
        _ => {}
    }
    let cu = sim
        .simulation_details
        .expect("simulation_details should be present")
        .units_consumed;

    // Process to apply state changes.
    ctx.banks_client.process_transaction(tx).await.unwrap();
    cu
}

/// Measure CU for a single instruction.
pub async fn measure_ixn(
    fixture: &mut TestFixture,
    ix: Instruction,
    n_items: u64,
    signer: Keypair,
) -> u64 {
    let cu = send_txn_measure_cu(&mut fixture.context, &[ix], &[&signer]).await;
    writeln!(&mut fixture.logs, "Total{} {:>6} CU", " ".repeat(15), cu).unwrap();
    writeln!(
        &mut fixture.logs,
        "Average{:<13} {:>6} CU",
        " ",
        cu / n_items
    )
    .unwrap();
    cu
}

// ── Instruction helpers ─────────────────────────────────────────────────────

/// Build a PostOnly ask for use in warmup/benchmarks.
pub fn simple_post_only_ask(price_in_ticks: u64, num_base_lots: u64) -> OrderPacket {
    OrderPacket::new_post_only_default(Side::Ask, price_in_ticks, num_base_lots)
}

/// Build a PostOnly bid for use in warmup/benchmarks.
pub fn simple_post_only_bid(price_in_ticks: u64, num_base_lots: u64) -> OrderPacket {
    OrderPacket::new_post_only_default(Side::Bid, price_in_ticks, num_base_lots)
}

/// Build an IOC buy order (taker swap).
pub fn ioc_buy(price_in_ticks: u64, num_base_lots: u64) -> OrderPacket {
    OrderPacket::new_ioc_by_lots(
        Side::Bid,
        price_in_ticks,
        num_base_lots,
        phoenix::state::SelfTradeBehavior::CancelProvide,
        None,
        0,
        false,
    )
}

// ── Low-level helpers (public for use in tests) ────────────────────────────

pub fn clone_keypair(kp: &Keypair) -> Keypair {
    Keypair::from_bytes(&kp.to_bytes()).unwrap()
}

pub async fn create_ata_pub(
    ctx: &mut ProgramTestContext,
    wallet: &Pubkey,
    mint: &Pubkey,
) -> Pubkey {
    create_ata(ctx, wallet, mint).await
}

pub async fn mint_to_pub(
    ctx: &mut ProgramTestContext,
    authority: &Keypair,
    mint: &Pubkey,
    to: &Pubkey,
    amount: u64,
) {
    mint_to(ctx, authority, mint, to, amount).await;
}

async fn airdrop(ctx: &mut ProgramTestContext, to: &Pubkey, amount: u64) {
    let payer = clone_keypair(&ctx.payer);
    let ix = system_instruction::transfer(&payer.pubkey(), to, amount);
    send_txn(ctx, &[ix], &[&payer]).await;
}

async fn create_mint(
    ctx: &mut ProgramTestContext,
    mint: &Keypair,
    authority: &Pubkey,
    decimals: u8,
) {
    let payer = clone_keypair(&ctx.payer);
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let ixs = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            rent.minimum_balance(spl_token::state::Mint::LEN),
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &mint.pubkey(),
            authority,
            Some(authority),
            decimals,
        )
        .unwrap(),
    ];
    send_txn(ctx, &ixs, &[&payer, mint]).await;
}

async fn create_ata(ctx: &mut ProgramTestContext, wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let payer = clone_keypair(&ctx.payer);
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        wallet,
        mint,
        &spl_token::ID,
    );
    send_txn(ctx, &[ix], &[&payer]).await;
    get_associated_token_address(wallet, mint)
}

async fn mint_to(
    ctx: &mut ProgramTestContext,
    authority: &Keypair,
    mint: &Pubkey,
    to: &Pubkey,
    amount: u64,
) {
    let payer = clone_keypair(&ctx.payer);
    let ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint,
        to,
        &authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    send_txn(ctx, &[ix], &[&payer, authority]).await;
}

pub async fn send_txn(ctx: &mut ProgramTestContext, ixs: &[Instruction], signers: &[&Keypair]) {
    let blockhash = ctx
        .banks_client
        .get_new_latest_blockhash(&ctx.last_blockhash)
        .await
        .unwrap();
    let tx = Transaction::new_signed_with_payer(
        ixs,
        Some(&ctx.payer.pubkey()),
        &[vec![&ctx.payer], signers.to_vec()].concat(),
        blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();
}
