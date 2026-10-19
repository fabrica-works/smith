//! Bounded account notices (protocol/channel.md, section 4).
use super::{Component, Phase};
use crate::{Error, OpenEvent};
use alloc::boxed::Box;
use skein_channel::{Lower, Request, frame_writer};
use skein_lib::{Duration, Queue, Token, Writer};

impl Component {
    /// Tell the host a provider rejected one named credential generation.
    pub fn send_unauthorized(
        &mut self,
        name: smith_domain::GrantName,
        token: Token,
        to_service: &mut Queue<OpenEvent>,
        below: &mut Queue<Lower>,
    ) -> Result<(), Error> {
        if self.phase != Phase::Admitted {
            return Err(Error::Order);
        }
        let record = smith_channel::Unauthorized::new(
            &self.bodies,
            smith_channel::UnauthorizedParts { account: name.account, generation: name.generation },
        )?;
        let Ok(length) = usize::try_from(record.measure()) else {
            return Err(Error::ResultCapacity);
        };
        let mut body = Writer::new(length);
        record.encode(&mut body)?;
        let mut frame = frame_writer(0x010d, record.measure())?;
        frame.put(&body.finish())?;
        self.machine.down(Request::Send { token, frame: frame.finish()? }, &mut self.events, below);
        self.drain(to_service, below);
        self.fire(to_service, below);
        Ok(())
    }

    /// Tell the host an account reached its usage limit, with an optional reset.
    pub fn send_limited(
        &mut self,
        account: u32,
        reset: Option<Duration>,
        token: Token,
        to_service: &mut Queue<OpenEvent>,
        below: &mut Queue<Lower>,
    ) -> Result<(), Error> {
        if self.phase != Phase::Admitted {
            return Err(Error::Order);
        }
        let record = smith_channel::Limited::new(&self.bodies, smith_channel::LimitedParts { account, reset })?;
        let Ok(length) = usize::try_from(record.measure()) else {
            return Err(Error::ResultCapacity);
        };
        let mut body = Writer::new(length);
        record.encode(&mut body)?;
        let mut frame = frame_writer(0x010e, record.measure())?;
        frame.put(&body.finish())?;
        self.machine.down(Request::Send { token, frame: frame.finish()? }, &mut self.events, below);
        self.drain(to_service, below);
        self.fire(to_service, below);
        Ok(())
    }

    /// Tell the host why the provider refused an account's request.
    pub fn send_refused(
        &mut self,
        account: u32,
        code: &[u8],
        reason: &[u8],
        token: Token,
        to_service: &mut Queue<OpenEvent>,
        below: &mut Queue<Lower>,
    ) -> Result<(), Error> {
        if self.phase != Phase::Admitted {
            return Err(Error::Order);
        }
        let record = refused_notice(&self.bodies, account, code, reason)?;
        let length = usize::try_from(record.measure()).or(Err(Error::ResultCapacity))?;
        let mut body = Writer::new(length);
        record.encode(&mut body)?;
        let mut frame = frame_writer(0x0113, record.measure())?;
        frame.put(&body.finish())?;
        self.machine.down(Request::Send { token, frame: frame.finish()? }, &mut self.events, below);
        self.drain(to_service, below);
        self.fire(to_service, below);
        Ok(())
    }
}

fn refused_notice(
    limits: &smith_channel::Limits,
    account: u32,
    code: &[u8],
    reason: &[u8],
) -> Result<smith_channel::AccountRefused, Error> {
    let code =
        if code.len() <= usize::try_from(limits.account_refused_code.min(128)).expect("u32 fits") { code } else { b"" };
    let mut end = reason.len().min(usize::try_from(limits.account_refused_reason.min(512)).expect("u32 fits"));
    for _ in 0_u8..3_u8 {
        match reason.get(end) {
            Some(byte) if byte & 0xc0 == 0x80 => end = end.saturating_sub(1),
            Some(_) | None => break,
        }
    }
    Ok(smith_channel::AccountRefused::new(
        limits,
        smith_channel::AccountRefusedParts {
            account,
            code: Box::from(code),
            reason: Box::from(reason.get(..end).ok_or(Error::Grants)?),
        },
    )?)
}

#[cfg(test)]
mod tests {
    use super::refused_notice;

    #[test]
    fn a_refused_notice_keeps_its_bounds() {
        // protocol/limits.md section 2.4 fixes these text bounds.
        const CODE_BOUND: usize = 128;
        const REASON_BOUND: usize = 512;
        let limits = smith_channel::CEILINGS;
        let long_code = [b'x'; CODE_BOUND + 1];
        let prefix = [b'a'; REASON_BOUND - 1];
        let mut cut_reason = [b'a'; REASON_BOUND + 1];
        cut_reason[REASON_BOUND - 1] = 0xc3;
        cut_reason[REASON_BOUND] = 0xa9;
        for (code, reason, expected_code, expected_reason) in [
            (&long_code[..], b"reason".as_slice(), b"".as_slice(), b"reason".as_slice()),
            (b"code".as_slice(), &cut_reason[..], b"code".as_slice(), &prefix[..]),
        ] {
            let notice = refused_notice(&limits, 7, code, reason).expect("bounded notice");
            assert_eq!((notice.code(), notice.reason()), (expected_code, expected_reason));
        }
    }
}
