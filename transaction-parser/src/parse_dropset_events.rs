//! See [`ParseDropsetEvents`].

use dropset_interface::instructions::DropsetInstruction;

use crate::events::dropset_event::{
    unpack_instruction_events,
    DropsetEvent,
    EventError,
};

/// A trait for parsing `dropset` events that provides a default implementation for parsing events
/// using the program ID and instruction data accessors.
pub trait ParseDropsetEvents {
    fn program_id(&self) -> &[u8; 32];

    fn instruction_data(&self) -> &[u8];

    fn parse_events(&self) -> Result<Vec<DropsetEvent>, EventError> {
        let (tag_byte, instruction_event_data) = match self.instruction_data().split_at_checked(1) {
            Some(v) => v,
            None => return Ok(vec![]),
        };

        let tag = tag_byte
            .first()
            .and_then(|byte| DropsetInstruction::try_from(*byte).ok());

        match (self.program_id(), tag) {
            (&dropset::ID, Some(DropsetInstruction::FlushEvents)) => {
                unpack_instruction_events(instruction_event_data)
            }
            _ => Ok(vec![]),
        }
    }
}
