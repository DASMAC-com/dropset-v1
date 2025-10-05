use pinocchio::{
    account_info::AccountInfo,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};

/// Transfers `lamports_diff` lamports from `payer` to `account`, where `lamports_diff` is the
/// calculated difference in lamports required for the account given the requested additional space.
///
/// Typically this call should be followed by an account resize. It isn't provided here so the
/// caller can decide on a case by case basis which version of the resize invocation to call.
///
/// - If the lamport diff is zero, the transfer CPI isn't invoked.
/// - Otherwise, the `payer` transfers the necessary lamports.
///
/// # Safety:
/// Caller must guarantee there are no active borrows of `account`'s account data.
pub unsafe fn fund_then_resize_unchecked(
    payer: &AccountInfo,
    account: &AccountInfo,
    additional_space: usize,
) -> ProgramResult {
    let current_size = account.data_len();
    let current_lamports = account.lamports();
    let new_size = current_size + additional_space;
    let new_lamports_required = Rent::get()?.minimum_balance(new_size);
    let lamports_diff = new_lamports_required.saturating_sub(current_lamports);

    if lamports_diff == 0 {
        return Ok(());
    }

    pinocchio_system::instructions::Transfer {
        from: payer,
        to: account,
        lamports: lamports_diff,
    }
    .invoke()?;

    unsafe { account.resize_unchecked(new_size) }
}
