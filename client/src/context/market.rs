//! Market-level context for building `dropset` market instructions from contextual address data.

use dropset_interface::{
    instructions::{
        generated_client::*,
        BatchReplaceInstructionData,
        CancelOrderInstructionData,
        CloseSeatInstructionData,
        DepositInstructionData,
        MarketOrderInstructionData,
        PostOrderInstructionData,
        RegisterMarketInstructionData,
        WithdrawInstructionData,
    },
    seeds::event_authority,
    state::{
        sector::NIL,
        SYSTEM_PROGRAM_ID,
    },
};
use solana_address::Address;
use solana_instruction::Instruction;
use transaction_parser::views::MarketSeatView;

use crate::{
    context::token::TokenContext,
    pda::find_market_address,
    single_signer_instruction::SingleSignerInstruction,
    token_instructions::create_and_initialize_token_instructions,
};

/// A struct containing contextual fields for a market.
///
/// Implements helper methods for building all program instructions using those values.
pub struct MarketContext {
    pub market: Address,
    pub base: TokenContext,
    pub quote: TokenContext,
    pub base_market_ata: Address,
    pub quote_market_ata: Address,
}

#[derive(Clone, Copy)]
pub enum BookSide {
    Ask,
    Bid,
}

#[derive(Clone, Copy)]
pub enum Denomination {
    Base,
    Quote,
}

impl Denomination {
    pub fn is_base(&self) -> bool {
        matches!(&self, Denomination::Base)
    }
}

impl MarketContext {
    /// Creates a new [`MarketContext`] by deriving the market PDA and ATAs from the given token
    /// contexts.
    pub fn new(base: TokenContext, quote: TokenContext) -> Self {
        let (market, _bump) = find_market_address(&base.mint_address, &quote.mint_address);
        let base_market_ata = base.get_ata_for(&market);
        let quote_market_ata = quote.get_ata_for(&market);

        Self {
            market,
            base,
            quote,
            base_market_ata,
            quote_market_ata,
        }
    }

    pub fn get_base_ata(&self, owner: &Address) -> Address {
        self.base.get_ata_for(owner)
    }

    pub fn get_quote_ata(&self, owner: &Address) -> Address {
        self.quote.get_ata_for(owner)
    }

    /// Creates a seat for the user by depositing the minimum amount required to create a seat.
    ///
    /// This is because the amount cannot be zero:
    /// [`dropset_interface::error::DropsetError::AmountCannotBeZero`]
    pub fn create_seat(&self, user: Address) -> SingleSignerInstruction {
        self.deposit_base(user, 1, NIL)
    }

    /// Creates the instructions for initializing both the base and quote token mints.
    pub fn create_tokens(
        &self,
        mint_authority: Address,
        rent_lamports: u64,
    ) -> anyhow::Result<Vec<Instruction>> {
        let (create_base, initialize_base) = create_and_initialize_token_instructions(
            &mint_authority,
            &self.base.mint_address,
            rent_lamports,
            self.base.mint_decimals,
            &self.base.token_program,
        )?;

        let (create_quote, initialize_quote) = create_and_initialize_token_instructions(
            &mint_authority,
            &self.quote.mint_address,
            rent_lamports,
            self.quote.mint_decimals,
            &self.quote.token_program,
        )?;

        Ok(
            vec![create_base, initialize_base, create_quote, initialize_quote]
                .into_iter()
                .collect(),
        )
    }

    pub fn register_market(&self, payer: Address, num_sectors: u16) -> SingleSignerInstruction {
        RegisterMarket {
            event_authority: event_authority::ID,
            user: payer,
            market_account: self.market,
            base_market_ata: self.base_market_ata,
            quote_market_ata: self.quote_market_ata,
            base_mint: self.base.mint_address,
            quote_mint: self.quote.mint_address,
            base_token_program: self.base.token_program,
            quote_token_program: self.quote.token_program,
            ata_program: spl_associated_token_account_interface::program::ID,
            system_program: SYSTEM_PROGRAM_ID,
            dropset_program: dropset::ID,
        }
        .create_instruction(RegisterMarketInstructionData::new(num_sectors))
        .try_into()
        .expect("Should be a single signer instruction")
    }

    pub fn find_seat(&self, seats: &[MarketSeatView], user: &Address) -> Option<MarketSeatView> {
        seats.iter().find(|seat| &seat.user == user).cloned()
    }

    pub fn close_seat(&self, user: Address, sector_index_hint: u32) -> SingleSignerInstruction {
        CloseSeat {
            event_authority: event_authority::ID,
            user,
            market_account: self.market,
            base_user_ata: self.get_base_ata(&user),
            quote_user_ata: self.get_quote_ata(&user),
            base_market_ata: self.base_market_ata,
            quote_market_ata: self.quote_market_ata,
            base_mint: self.base.mint_address,
            quote_mint: self.quote.mint_address,
            base_token_program: self.base.token_program,
            quote_token_program: self.quote.token_program,
            dropset_program: dropset::ID,
        }
        .create_instruction(CloseSeatInstructionData::new(sector_index_hint))
        .try_into()
        .expect("Should be a single signer instruction")
    }

    pub fn deposit_base(
        &self,
        user: Address,
        amount: u64,
        sector_index_hint: u32,
    ) -> SingleSignerInstruction {
        let data = DepositInstructionData::new(amount, sector_index_hint);
        self.deposit(user, data, true)
    }

    pub fn deposit_quote(
        &self,
        user: Address,
        amount: u64,
        sector_index_hint: u32,
    ) -> SingleSignerInstruction {
        let data = DepositInstructionData::new(amount, sector_index_hint);
        self.deposit(user, data, false)
    }

    pub fn withdraw_base(
        &self,
        user: Address,
        amount: u64,
        sector_index_hint: u32,
    ) -> SingleSignerInstruction {
        let data = WithdrawInstructionData::new(amount, sector_index_hint);
        self.withdraw(user, data, true)
    }

    pub fn withdraw_quote(
        &self,
        user: Address,
        amount: u64,
        sector_index_hint: u32,
    ) -> SingleSignerInstruction {
        let data = WithdrawInstructionData::new(amount, sector_index_hint);
        self.withdraw(user, data, false)
    }

    pub fn post_order(
        &self,
        user: Address,
        data: PostOrderInstructionData,
    ) -> SingleSignerInstruction {
        PostOrder {
            event_authority: event_authority::ID,
            user,
            market_account: self.market,
            dropset_program: dropset::ID,
        }
        .create_instruction(data)
        .try_into()
        .expect("Should be a single signer instruction")
    }

    pub fn cancel_order(
        &self,
        user: Address,
        data: CancelOrderInstructionData,
    ) -> SingleSignerInstruction {
        CancelOrder {
            event_authority: event_authority::ID,
            user,
            market_account: self.market,
            dropset_program: dropset::ID,
        }
        .create_instruction(data)
        .try_into()
        .expect("Should be a single signer instruction")
    }

    pub fn market_order(
        &self,
        user: Address,
        data: MarketOrderInstructionData,
    ) -> SingleSignerInstruction {
        MarketOrder {
            event_authority: event_authority::ID,
            user,
            market_account: self.market,
            base_user_ata: self.get_base_ata(&user),
            quote_user_ata: self.get_quote_ata(&user),
            base_market_ata: self.base_market_ata,
            quote_market_ata: self.quote_market_ata,
            base_mint: self.base.mint_address,
            quote_mint: self.quote.mint_address,
            base_token_program: self.base.token_program,
            quote_token_program: self.quote.token_program,
            dropset_program: dropset::ID,
        }
        .create_instruction(data)
        .try_into()
        .expect("Should be a single signer instruction")
    }

    pub fn batch_replace(
        &self,
        user: Address,
        data: BatchReplaceInstructionData,
    ) -> SingleSignerInstruction {
        BatchReplace {
            event_authority: event_authority::ID,
            user,
            market_account: self.market,
            dropset_program: dropset::ID,
        }
        .create_instruction(data)
        .try_into()
        .expect("Should be a single signer instruction")
    }

    fn deposit(
        &self,
        user: Address,
        data: DepositInstructionData,
        is_base: bool,
    ) -> SingleSignerInstruction {
        match is_base {
            true => Deposit {
                event_authority: event_authority::ID,
                user,
                market_account: self.market,
                user_ata: self.get_base_ata(&user),
                market_ata: self.base_market_ata,
                mint: self.base.mint_address,
                token_program: self.base.token_program,
                dropset_program: dropset::ID,
            },
            false => Deposit {
                event_authority: event_authority::ID,
                user,
                market_account: self.market,
                user_ata: self.get_quote_ata(&user),
                market_ata: self.quote_market_ata,
                mint: self.quote.mint_address,
                token_program: self.quote.token_program,
                dropset_program: dropset::ID,
            },
        }
        .create_instruction(data)
        .try_into()
        .expect("Should be a single signer instruction")
    }

    fn withdraw(
        &self,
        user: Address,
        data: WithdrawInstructionData,
        is_base: bool,
    ) -> SingleSignerInstruction {
        match is_base {
            true => Withdraw {
                event_authority: event_authority::ID,
                user,
                market_account: self.market,
                user_ata: self.get_base_ata(&user),
                market_ata: self.base_market_ata,
                mint: self.base.mint_address,
                token_program: self.base.token_program,
                dropset_program: dropset::ID,
            },
            false => Withdraw {
                event_authority: event_authority::ID,
                user,
                market_account: self.market,
                user_ata: self.get_quote_ata(&user),
                market_ata: self.quote_market_ata,
                mint: self.quote.mint_address,
                token_program: self.quote.token_program,
                dropset_program: dropset::ID,
            },
        }
        .create_instruction(data)
        .try_into()
        .expect("Should be a single signer instruction")
    }
}
