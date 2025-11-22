use dropset_interface::events::{
    CloseSeatInstructionData,
    DepositInstructionData,
    DropsetEventTag,
    HeaderInstructionData,
    RegisterMarketInstructionData,
    WithdrawInstructionData,
};

#[derive(strum_macros::VariantNames)]
pub enum DropsetEvent {
    Header(HeaderInstructionData),
    Deposit(DepositInstructionData),
    Withdraw(WithdrawInstructionData),
    RegisterMarket(RegisterMarketInstructionData),
    CloseSeat(CloseSeatInstructionData),
}

impl DropsetEvent {
    fn len_with_tag(&self) -> usize {
        match self {
            Self::Header(_) => HeaderInstructionData::LEN_WITH_TAG,
            Self::Deposit(_) => DepositInstructionData::LEN_WITH_TAG,
            Self::Withdraw(_) => WithdrawInstructionData::LEN_WITH_TAG,
            Self::RegisterMarket(_) => RegisterMarketInstructionData::LEN_WITH_TAG,
            Self::CloseSeat(_) => CloseSeatInstructionData::LEN_WITH_TAG,
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
            DropsetEventTag::Header => Ok(DropsetEvent::Header(
                HeaderInstructionData::unpack(data).map_err(|_| err())?,
            )),
            DropsetEventTag::Deposit => Ok(DropsetEvent::Deposit(
                DepositInstructionData::unpack(data).map_err(|_| err())?,
            )),
            DropsetEventTag::Withdraw => Ok(DropsetEvent::Withdraw(
                WithdrawInstructionData::unpack(data).map_err(|_| err())?,
            )),
            DropsetEventTag::RegisterMarket => Ok(DropsetEvent::RegisterMarket(
                RegisterMarketInstructionData::unpack(data).map_err(|_| err())?,
            )),
            DropsetEventTag::CloseSeat => Ok(DropsetEvent::CloseSeat(
                CloseSeatInstructionData::unpack(data).map_err(|_| err())?,
            )),
        }
    }
}
