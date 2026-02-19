use solana_address::Address;
use solana_instruction::Instruction;
use solana_sdk::program_pack::Pack;
use spl_token_interface::state::Mint;

pub fn create_and_initialize_token_instructions(
    mint_authority_and_payer: &Address,
    mint: &Address,
    rent_lamports: u64,
    mint_decimals: u8,
    token_program: &Address,
) -> anyhow::Result<(Instruction, Instruction)> {
    let create_mint_account = solana_system_interface::instruction::create_account(
        mint_authority_and_payer,
        mint,
        rent_lamports,
        Mint::LEN as u64,
        token_program,
    );

    let initialize_mint = spl_token_2022_interface::instruction::initialize_mint2(
        token_program,
        mint,
        mint_authority_and_payer,
        None,
        mint_decimals,
    )?;

    Ok((create_mint_account, initialize_mint))
}
