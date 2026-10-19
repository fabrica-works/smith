//! Decode bounded account notices for the host (protocol/channel.md, section 4).
use crate::{Error, OpenEvent};
use alloc::boxed::Box;
use skein_lib::Reader;

pub(super) fn decode(limits: &smith_channel::Limits, kind: u16, body: &[u8]) -> Result<OpenEvent, Error> {
    match kind {
        0x010d => {
            let notice = smith_channel::Unauthorized::decode(limits, &mut Reader::new(body))?;
            Ok(OpenEvent::Unauthorized { account: notice.account(), generation: notice.generation() })
        }
        0x010e => {
            let notice = smith_channel::Limited::decode(limits, &mut Reader::new(body))?;
            Ok(OpenEvent::Limited { account: notice.account(), reset: *notice.reset() })
        }
        0x0113 => {
            let notice = smith_channel::AccountRefused::decode(limits, &mut Reader::new(body))?;
            Ok(OpenEvent::Refused {
                account: notice.account(),
                code: Box::from(notice.code()),
                reason: Box::from(notice.reason()),
            })
        }
        _ => Err(Error::InvalidAnswer),
    }
}
