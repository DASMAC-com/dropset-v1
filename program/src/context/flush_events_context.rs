//! See [`FlushEventsContext`].

use dropset_interface::instructions::generated_program::FlushEvents;
use pinocchio::{
    account::AccountView,
    error::ProgramError,
};

use crate::validation::event_authority::EventAuthorityView;

/// The account context for the [`FlushEvents`] instruction.
#[derive(Clone)]
pub struct FlushEventsContext<'a> {
    pub _event_authority: EventAuthorityView<'a>,
}

impl<'a> FlushEventsContext<'a> {
    #[inline(always)]
    pub fn load(accounts: &'a [AccountView]) -> Result<FlushEventsContext<'a>, ProgramError> {
        let FlushEvents { event_authority } = FlushEvents::load_accounts(accounts)?;

        Ok(Self {
            _event_authority: EventAuthorityView::new(event_authority)?,
        })
    }
}
