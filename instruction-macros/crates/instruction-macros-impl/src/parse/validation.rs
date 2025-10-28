use crate::{
    parse::{
        instruction_account::InstructionAccount,
        instruction_argument::InstructionArgument,
    },
    ParsingError,
};

/// Validate the vector of instruction accounts to ensure no duplicate names, indices, etc.
pub fn validate_accounts(accs: &[InstructionAccount], span: proc_macro2::Span) -> syn::Result<()> {
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
    dupe_type: &str,
) -> syn::Result<()> {
    names.sort();
    names
        .windows(2)
        .map(|window| <&[String; 2]>::try_from(window).expect("Should have 2"))
        .try_for_each(|[prev_name, curr_name]| {
            if prev_name == curr_name {
                let duplicate_error =
                    ParsingError::DuplicateName(dupe_type.to_string(), curr_name.clone());
                Err(duplicate_error.new_err(span))
            } else {
                Ok(())
            }
        })
}
