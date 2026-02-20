use dropset_interface::error::DropsetError;
use mollusk_svm::result::Check;

/// Extension trait for converting a [`DropsetError`] directly into a [`Check`] that asserts
/// the instruction failed with that error.
pub trait IntoCheckFailure {
    fn into_check_failure(self) -> Check<'static>;
}

impl IntoCheckFailure for DropsetError {
    fn into_check_failure(self) -> Check<'static> {
        Check::err(self.into())
    }
}
