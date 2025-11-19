use dropset_interface::events::{
    CloseSeatEventInstructionData,
    DepositEventInstructionData,
    DropsetEventTag,
    HeaderEventInstructionData,
    RegisterMarketEventInstructionData,
    WithdrawEventInstructionData,
};

#[derive(strum_macros::VariantNames)]
pub enum DropsetEvent {
    Header(HeaderEventInstructionData),
    Deposit(DepositEventInstructionData),
    Withdraw(WithdrawEventInstructionData),
    RegisterMarket(RegisterMarketEventInstructionData),
    CloseSeat(CloseSeatEventInstructionData),
}

impl DropsetEvent {
    fn len_with_tag(&self) -> usize {
        match self {
            Self::Header(_) => HeaderEventInstructionData::LEN_WITH_TAG,
            Self::Deposit(_) => DepositEventInstructionData::LEN_WITH_TAG,
            Self::Withdraw(_) => WithdrawEventInstructionData::LEN_WITH_TAG,
            Self::RegisterMarket(_) => RegisterMarketEventInstructionData::LEN_WITH_TAG,
            Self::CloseSeat(_) => CloseSeatEventInstructionData::LEN_WITH_TAG,
        }
    }
}

pub enum EventError {
    HeaderNotFirstEvent,
    InstructionDataTooShort,
    UnpackError(DropsetEventTag),
    InvalidTag,
    EventBufferHasRemainingBytes,
}

/// Unpack instruction events from instruction data that starts *after* the instruction tag is
/// peeled off of the front of the slice.
///
/// That is, `instruction_data` here starts after the instruction tag.
pub fn unpack_instruction_events(instruction_data: &[u8]) -> Result<Vec<DropsetEvent>, EventError> {
    let original_len = instruction_data.len();

    // The first event should be the event header.
    let header = match DropsetEvent::unpack(instruction_data) {
        Ok(DropsetEvent::Header(data)) => data,
        _ => return Err(EventError::HeaderNotFirstEvent),
    };

    let num_events = header.emitted_count as usize;
    let header_event = DropsetEvent::Header(header);
    let mut cursor = header_event.len_with_tag();
    let mut res = vec![];

    for _ in 0..num_events {
        let instruction_data = &instruction_data[cursor..];
        let event = DropsetEvent::unpack(instruction_data)?;

        cursor += event.len_with_tag();
        res.push(event);
    }

    if cursor != original_len {
        return Err(EventError::EventBufferHasRemainingBytes);
    }

    Ok(res)
}

impl DropsetEvent {
    pub fn unpack(instruction_data: &[u8]) -> Result<DropsetEvent, EventError> {
        let [tag, data @ ..] = instruction_data else {
            return Err(EventError::InstructionDataTooShort);
        };

        let tag = DropsetEventTag::try_from(*tag).map_err(|_| EventError::InvalidTag)?;
        let err = || EventError::UnpackError(tag);
        match tag {
            DropsetEventTag::HeaderEvent => Ok(DropsetEvent::Header(
                HeaderEventInstructionData::unpack(data).map_err(|_| err())?,
            )),
            DropsetEventTag::DepositEvent => Ok(DropsetEvent::Deposit(
                DepositEventInstructionData::unpack(data).map_err(|_| err())?,
            )),
            DropsetEventTag::WithdrawEvent => Ok(DropsetEvent::Withdraw(
                WithdrawEventInstructionData::unpack(data).map_err(|_| err())?,
            )),
            DropsetEventTag::RegisterMarketEvent => Ok(DropsetEvent::RegisterMarket(
                RegisterMarketEventInstructionData::unpack(data).map_err(|_| err())?,
            )),
            DropsetEventTag::CloseSeatEvent => Ok(DropsetEvent::CloseSeat(
                CloseSeatEventInstructionData::unpack(data).map_err(|_| err())?,
            )),
        }
    }
}
