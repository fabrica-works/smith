//! Text pieces at the channel boundary (protocol/channel.md, sections 4 and 7).
use crate::Error;
use alloc::boxed::Box;
use skein_channel::{Frame, frame_writer};
use skein_lib::Writer;

/// A piece cut by the service, before its turn's record.
#[derive(Debug)]
pub struct Piece<'a> {
    pub turn: u32,
    pub retry: u32,
    pub offset: u64,
    pub bytes: &'a [u8],
}

pub(crate) fn frame(piece: Piece<'_>, limits: &smith_channel::Limits) -> Result<Frame, Error> {
    let record = smith_channel::Live::new(
        limits,
        smith_channel::LiveParts {
            turn: piece.turn,
            retry: piece.retry,
            kind: smith_channel::LiveKind::Text,
            offset: piece.offset,
            bytes: Box::from(piece.bytes),
            call: None,
        },
    )?;
    let Ok(length) = usize::try_from(record.measure()) else {
        return Err(Error::ResultCapacity);
    };
    let mut body = Writer::new(length);
    record.encode(&mut body)?;
    let mut frame = frame_writer(0x0112, record.measure())?;
    frame.put(&body.finish())?;
    Ok(frame.finish()?)
}

pub(crate) fn frame_bytes(limits: &smith_channel::Limits, bytes: u32) -> Option<u32> {
    if bytes > limits.live_bytes || bytes > smith_channel::CEILINGS.live_bytes {
        return None;
    }
    let limits = smith_channel::Limits { live_bytes: bytes, ..*limits };
    u32::try_from(smith_channel::Live::worst_case_bytes(&limits)?).ok()?.checked_add(8)
}
