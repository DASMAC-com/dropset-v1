//! Defines an enum for all `dropset` events that facilitates unpacking them either as individual
//! events or contiguous instruction data.

use dropset_interface::events::{
    CancelOrderEventInstructionData,
    CloseSeatEventInstructionData,
    DepositEventInstructionData,
    DropsetEventTag,
    HeaderEventInstructionData,
    MarketOrderEventInstructionData,
    PostOrderEventInstructionData,
    RegisterMarketEventInstructionData,
    WithdrawEventInstructionData,
};
use instruction_macros_traits::Tagged;

use crate::events::display_types;

#[derive(Debug, strum_macros::VariantNames)]
pub enum DropsetEvent {
    Header(display_types::DisplayHeaderData),
    Deposit(DepositEventInstructionData),
    Withdraw(WithdrawEventInstructionData),
    RegisterMarket(display_types::DisplayRegisterMarketData),
    CloseSeat(CloseSeatEventInstructionData),
    PostOrder(PostOrderEventInstructionData),
    CancelOrder(CancelOrderEventInstructionData),
    MarketOrder(MarketOrderEventInstructionData),
}

impl DropsetEvent {
    fn len_with_tag(&self) -> usize {
        match self {
            Self::Header(_) => HeaderEventInstructionData::LEN_WITH_TAG,
            Self::Deposit(_) => DepositEventInstructionData::LEN_WITH_TAG,
            Self::Withdraw(_) => WithdrawEventInstructionData::LEN_WITH_TAG,
            Self::RegisterMarket(_) => RegisterMarketEventInstructionData::LEN_WITH_TAG,
            Self::CloseSeat(_) => CloseSeatEventInstructionData::LEN_WITH_TAG,
            Self::PostOrder(_) => PostOrderEventInstructionData::LEN_WITH_TAG,
            Self::CancelOrder(_) => CancelOrderEventInstructionData::LEN_WITH_TAG,
            Self::MarketOrder(_) => MarketOrderEventInstructionData::LEN_WITH_TAG,
        }
    }
}

#[derive(Debug)]
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
                HeaderEventInstructionData::unpack_untagged(data)
                    .map_err(|_| err())?
                    .into(),
            )),
            DropsetEventTag::DepositEvent => Ok(DropsetEvent::Deposit(
                DepositEventInstructionData::unpack_untagged(data).map_err(|_| err())?,
            )),
            DropsetEventTag::WithdrawEvent => Ok(DropsetEvent::Withdraw(
                WithdrawEventInstructionData::unpack_untagged(data).map_err(|_| err())?,
            )),
            DropsetEventTag::RegisterMarketEvent => Ok(DropsetEvent::RegisterMarket(
                RegisterMarketEventInstructionData::unpack_untagged(data)
                    .map_err(|_| err())?
                    .into(),
            )),
            DropsetEventTag::CloseSeatEvent => Ok(DropsetEvent::CloseSeat(
                CloseSeatEventInstructionData::unpack_untagged(data).map_err(|_| err())?,
            )),
            DropsetEventTag::PostOrderEvent => Ok(DropsetEvent::PostOrder(
                PostOrderEventInstructionData::unpack_untagged(data).map_err(|_| err())?,
            )),
            DropsetEventTag::CancelOrderEvent => Ok(DropsetEvent::CancelOrder(
                CancelOrderEventInstructionData::unpack_untagged(data).map_err(|_| err())?,
            )),
            DropsetEventTag::MarketOrderEvent => Ok(DropsetEvent::MarketOrder(
                MarketOrderEventInstructionData::unpack_untagged(data).map_err(|_| err())?,
            )),
        }
    }
}
