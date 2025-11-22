//! Higher-level validation over the parsed instruction model, enforcing structural and semantic
//! invariants.

use crate::{
    parse::{
        instruction_account::InstructionAccount,
        instruction_argument::InstructionArgument,
    },
    ParsingError,
};

/// Validate the vector of instruction accounts.
pub fn validate_accounts(
    as_instruction_events: bool,
    accs: &[InstructionAccount],
    span: proc_macro2::Span,
) -> syn::Result<()> {
    // Validate the accounts for an instruction that's parsed as an instruction event.
    // It should not have any accounts, since it's entirely just instruction data.
    if as_instruction_events {
        let events_res = if accs.is_empty() {
            Ok(())
        } else {
            Err(ParsingError::InstructionEventHasAccounts.new_err(span))
        };

        return events_res;
    }

    // Otherwise, validate as a typical instruction, where there is at least one signing account
    // and each account has a unique name.
    if accs.is_empty() {
        return Err(ParsingError::ZeroAccounts.new_err(span));
    };

    if !accs.iter().any(|acc| acc.is_signer) {
        return Err(ParsingError::MissingSigner.new_err(span));
    };

    let names: Vec<String> = accs.iter().map(|acc| acc.name.clone()).collect();
    check_duplicate_names(names, span, "account")?;

    Ok(())
}

/// Validate the vector of instruction arguments to ensure no duplicate names.
pub fn validate_args(args: &[InstructionArgument], span: proc_macro2::Span) -> syn::Result<()> {
    let names: Vec<String> = args
        .iter()
        .map(|arg| arg.name.clone().to_string())
        .collect();
    check_duplicate_names(names, span, "argument")
}

fn check_duplicate_names(
    mut names: Vec<String>,
    span: proc_macro2::Span,
    duplicate_message_type: &str,
) -> syn::Result<()> {
    names.sort();
    names
        .windows(2)
        .map(|window| <&[String; 2]>::try_from(window).expect("Should have 2"))
        .try_for_each(|[prev_name, curr_name]| {
            if prev_name == curr_name {
                let duplicate_error = ParsingError::DuplicateName(
                    duplicate_message_type.to_string(),
                    curr_name.clone(),
                );
                Err(duplicate_error.new_err(span))
            } else {
                Ok(())
            }
        })
}
