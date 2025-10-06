use pinocchio::account_info::AccountInfo;

#[derive(Clone)]
pub struct SystemProgramInfo<'a> {
    pub info: &'a AccountInfo,
}

impl<'a> SystemProgramInfo<'a> {
    #[inline(always)]
    pub fn new_unchecked(info: &'a AccountInfo) -> SystemProgramInfo<'a> {
        SystemProgramInfo { info }
    }
}
