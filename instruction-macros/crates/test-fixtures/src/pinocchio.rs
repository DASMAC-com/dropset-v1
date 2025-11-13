#[cfg(feature = "pinocchio")]
pub mod test {
    use instruction_macros::ProgramInstruction;

    pub mod program {
        use pinocchio::pubkey::Pubkey;
        use pinocchio_pubkey::pubkey;

        pub const ID: Pubkey = pubkey!("TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q");
    }

    const PROGRAM_ID: [u8; 32] = program::ID;

    #[repr(u8)]
    #[derive(ProgramInstruction)]
    // Also works:
    // #[program_id(crate::pinocchio_test::program::ID)]
    // #[program_id(PROGRAM_ID)]
    // #[program_id(crate::ID)]
    #[program_id(crate::pinocchio::test::PROGRAM_ID)]
    #[rustfmt::skip]
    pub enum PinocchioDropsetInstruction {
        #[account(0, signer,   name = "user",                desc = "The user closing their seat.")]
        #[account(1, writable, name = "market_account",      desc = "The market account PDA.")]
        #[account(2, writable, name = "base_user_ata",       desc = "The user's associated base mint token account.")]
        #[account(3, writable, name = "quote_user_ata",      desc = "The user's associated quote mint token account.")]
        #[account(4, writable, name = "base_market_ata",     desc = "The market's associated base mint token account.")]
        #[account(5, writable, name = "quote_market_ata",    desc = "The market's associated quote mint token account.")]
        #[account(6,           name = "base_mint",           desc = "The base token mint account.")]
        #[account(7,           name = "quote_mint",          desc = "The quote token mint account.")]
        #[account(8,           name = "base_token_program",  desc = "The base mint's token program.")]
        #[account(9,           name = "quote_token_program", desc = "The quote mint's token program.")]
        #[args(sector_index_hint: u32, "A hint indicating which sector the user's seat resides in.")]
        CloseSeat,

        #[account(0, signer,   name = "user",           desc = "The user depositing or registering their seat.")]
        #[account(1, writable, name = "market_account", desc = "The market account PDA.")]
        #[account(2, writable, name = "user_ata",       desc = "The user's associated token account.")]
        #[account(3, writable, name = "market_ata",     desc = "The market's associated token account.")]
        #[account(4,           name = "mint",           desc = "The token mint account.")]
        #[account(5,           name = "token_program",  desc = "The mint's token program.")]
        #[args(amount: u64, "The amount to deposit.")]
        #[args(sector_index_hint: u32, "A hint indicating which sector the user's seat resides in (pass `NIL` when registering a new seat).")]
        Deposit,

        #[account(0, signer, name = "event_authority", desc = "Flush events.")]
        FlushEvents,

        Batch,
    }
}
