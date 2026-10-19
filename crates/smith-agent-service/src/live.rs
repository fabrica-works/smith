//! Best effort main-conversation text (protocol/channel.md 7; limits.md 3.6).
use skein_channel::Lower;
use skein_lib::{Duration, Queue, Time, Token};
use smith_domain::llm;
use smith_protocol_channel::{self as channel, Piece};

/// Held payload bound, independent of provider delta size (limits.md 3.6).
const HELD: u32 = 16_384;
const PIECE: u32 = 4096;

pub(crate) trait Room {
    fn takes(&self, bytes: u32) -> bool;
    fn send(&mut self, piece: Piece<'_>);
    fn lose(&mut self, bytes: u64);
}

pub(crate) struct Connection<'a> {
    pub channel: &'a mut channel::Component,
    pub above: &'a mut Queue<channel::OpenEvent>,
    pub below: &'a mut Queue<Lower>,
    pub next: &'a mut u64,
    pub failed: bool,
    pub maximum: channel::MaxOut,
    pub ending: bool,
}

impl Room for Connection<'_> {
    fn takes(&self, bytes: u32) -> bool {
        let maximum = self.maximum;
        let entrances = if self.ending { 2 } else { 1 };
        self.above.room() >= maximum.to_domain.saturating_mul(entrances)
            && self.below.room() >= maximum.below.saturating_mul(entrances)
            && self.channel.takes_live(bytes)
    }
    fn send(&mut self, piece: Piece<'_>) {
        let token = Token::new(*self.next);
        *self.next = self.next.checked_add(1).expect("one run does not exhaust send names");
        if self.channel.send_live(piece, token, self.above, self.below).is_err() {
            self.failed = true;
        }
    }
    fn lose(&mut self, bytes: u64) {
        self.channel.lose_live(bytes);
    }
}

pub(crate) enum Action<'a> {
    Open { owner: Token, retry: u32 },
    Delta { owner: Token, bytes: &'a [u8] },
    End { turn: u32 },
    Pump,
}

#[derive(Debug)]
struct Stream {
    owner: Token,
    turn: u32,
    retry: u32,
    ended: bool,
}

// Each gap precedes at least one retained byte. Thus HELD cells bound even
// alternating one-byte accepted runs and arbitrary dropped provider deltas.
#[derive(Debug)]
struct Gap {
    at: u64,
    offset: u64,
}

#[derive(Debug)]
struct HeldByte {
    byte: u8,
    arrived: Time,
}

#[derive(Debug)]
pub(crate) struct Live {
    ask: Option<smith_channel::LiveAsk>,
    last_turn: u32,
    stream: Option<Stream>,
    held: Queue<HeldByte>,
    interval: Duration,
    gaps: Queue<Gap>,
    gap: Option<Gap>,
    read: u64,
    written: u64,
    offset: u64,
    next: u64,
    tail: u64,
}

pub(crate) fn worst_case() -> Option<u64> {
    Queue::<HeldByte>::worst_case(HELD)?.checked_add(Queue::<Gap>::worst_case(HELD)?)
}

pub(crate) fn wanted(ask: Option<&smith_channel::LiveAsk>, prompt: &llm::Prompt) -> bool {
    let text = match ask {
        Some(ask) => ask.text(),
        None => false,
    };
    let ordinary = match prompt.purpose {
        llm::Purpose::Ordinary { .. } => true,
        llm::Purpose::Compaction { .. } => false,
    };
    text && prompt.asker == llm::Asker::Main && ordinary
}

impl Live {
    pub(crate) fn new(ask: Option<smith_channel::LiveAsk>, live_interval: Duration) -> Self {
        Self {
            ask,
            last_turn: 0,
            stream: None,
            held: Queue::with_capacity(HELD),
            interval: live_interval,
            gaps: Queue::with_capacity(HELD),
            gap: None,
            read: 0,
            written: 0,
            offset: 0,
            next: 0,
            tail: 0,
        }
    }
    pub(crate) fn ask(&mut self, ask: Option<smith_channel::LiveAsk>) {
        self.ask = ask;
    }
    pub(crate) fn wants(&self, prompt: &llm::Prompt) -> bool {
        wanted(self.ask.as_ref(), prompt)
    }
    pub(crate) fn act<R: Room>(&mut self, action: Action<'_>, now: Time, room: &mut R) {
        match action {
            Action::Open { owner, retry } => {
                self.open(owner, self.last_turn.checked_add(1).expect("bounded turns"), retry, room);
            }
            Action::Delta { owner, bytes } => self.delta(owner, bytes, now, room),
            Action::End { turn } => {
                self.end(room);
                self.last_turn = turn;
            }
            Action::Pump => self.pump(now, room),
        }
    }
    fn open<R: Room>(&mut self, owner: Token, turn: u32, retry: u32, room: &mut R) {
        self.end(room);
        self.stream = Some(Stream { owner, turn, retry, ended: false });
        self.read = 0;
        self.written = 0;
        self.offset = 0;
        self.next = 0;
        self.tail = 0;
    }
    fn delta<R: Room>(&mut self, owner: Token, bytes: &[u8], now: Time, room: &mut R) {
        let Some(stream) = &self.stream else {
            return;
        };
        if stream.owner != owner {
            return;
        }
        let length = u64::try_from(bytes.len()).expect("provider delta length fits u64");
        if stream.ended {
            room.lose(length);
            return;
        }
        let count = bytes.len().min(usize::try_from(self.held.room()).expect("held bound fits usize"));
        if count > 0 {
            if self.held.is_empty() {
                self.offset = self.next;
            } else if self.tail != self.next {
                self.gaps.push(Gap { at: self.written, offset: self.next });
            }
            for byte in bytes.get(..count).expect("admitted prefix") {
                self.held.push(HeldByte { byte: *byte, arrived: now });
            }
            let count = u64::try_from(count).expect("held bytes fit u64");
            self.written = self.written.checked_add(count).expect("bounded stream bytes");
            self.tail = self.next.checked_add(count).expect("bounded stream bytes");
        }
        self.next = self.next.checked_add(length).expect("bounded stream bytes");
        room.lose(u64::try_from(bytes.len().saturating_sub(count)).expect("delta length fits u64"));
        self.pump(now, room);
    }
    pub(crate) fn deadline(&self) -> Option<Time> {
        let byte = self.held.iter().next()?;
        Some(byte.arrived.saturating_add(self.interval))
    }
    fn pump<R: Room>(&mut self, now: Time, room: &mut R) {
        let Some(stream) = &self.stream else {
            return;
        };
        if stream.ended {
            return;
        }
        for _ in 0..HELD {
            let due = match self.deadline() {
                Some(deadline) => now >= deadline,
                None => false,
            };
            if self.held.is_empty() || (self.held.len() < PIECE && !due) {
                break;
            }
            if self.gap.is_none() {
                self.gap = self.gaps.pop();
            }
            if let Some(gap) = &self.gap
                && gap.at == self.read
            {
                self.offset = self.gap.take().expect("gap at read cursor").offset;
                self.gap = self.gaps.pop();
            }
            let mut count = self.held.len().min(PIECE);
            if let Some(gap) = &self.gap {
                count = count.min(
                    u32::try_from(gap.at.checked_sub(self.read).expect("gap follows read cursor"))
                        .expect("gap within held bytes"),
                );
            }
            if !room.takes(count) {
                break;
            }
            let mut bytes = [0; 4096];
            for byte in bytes.get_mut(..usize::try_from(count).expect("piece fits usize")).expect("piece bound") {
                *byte = self.held.pop().expect("held piece byte").byte;
            }
            room.send(Piece {
                turn: stream.turn,
                retry: stream.retry,
                offset: self.offset,
                bytes: bytes.get(..usize::try_from(count).expect("piece fits usize")).expect("piece bound"),
            });
            self.read = self.read.checked_add(u64::from(count)).expect("bounded stream bytes");
            self.offset = self.offset.checked_add(u64::from(count)).expect("bounded stream bytes");
        }
    }
    fn end<R: Room>(&mut self, room: &mut R) {
        self.pump(Time::from_nanos(u64::MAX), room);
        room.lose(u64::from(self.held.len()));
        for _ in 0..HELD {
            if self.held.pop().is_none() {
                break;
            }
        }
        for _ in 0..HELD {
            if self.gaps.pop().is_none() {
                break;
            }
        }
        self.gap = None;
        if let Some(stream) = &mut self.stream {
            stream.ended = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Live, Room as ChannelRoom, wanted};
    use crate::profile::standard;
    use alloc::boxed::Box;
    use skein_lib::{List, Time, Token};
    use smith_domain::llm;
    use smith_protocol_channel::Piece;

    const AT_ZERO: Time = Time::ZERO;
    const PIECE_BOUND: usize = 4096;
    const OWNER: Token = Token::new(7);
    const RETRY_OWNER: Token = Token::new(8);
    // limits.md 3.6: 16 KiB held, cut into four 4096-byte pieces.
    const HELD: usize = 16_384;
    const LARGE_DELTA: usize = 10_000;
    static AT_BOUND: [u8; HELD] = [b'a'; HELD];
    static PAST_BOUND: [u8; HELD + 1] = [b'a'; HELD + 1];
    #[derive(Debug)]
    struct Sent {
        turn: u32,
        retry: u32,
        offset: u64,
        bytes: Box<[u8]>,
    }
    struct Room {
        takes: bool,
        remaining: u32,
        lost: u64,
        sent: List<Sent>,
    }
    impl Room {
        fn new(takes: bool) -> Self {
            Self { takes, remaining: u32::MAX, lost: 0, sent: List::with_capacity(super::HELD) }
        }
        fn payloads(&self) -> List<&[u8]> {
            let mut pieces = List::with_capacity(self.sent.len());
            for piece in &self.sent {
                pieces.push(piece.bytes.as_ref()).expect("sent piece count");
            }
            pieces
        }
        fn offsets(&self) -> List<(u64, usize)> {
            let mut offsets = List::with_capacity(self.sent.len());
            for piece in &self.sent {
                offsets.push((piece.offset, piece.bytes.len())).expect("sent piece count");
            }
            offsets
        }
    }
    impl ChannelRoom for Room {
        fn takes(&self, _: u32) -> bool {
            self.takes && self.remaining > 0
        }
        fn send(&mut self, piece: Piece<'_>) {
            self.sent
                .push(Sent {
                    turn: piece.turn,
                    retry: piece.retry,
                    offset: piece.offset,
                    bytes: Box::from(piece.bytes),
                })
                .expect("piece bound");
            self.remaining = self.remaining.saturating_sub(1);
        }
        fn lose(&mut self, bytes: u64) {
            self.lost = self.lost.saturating_add(bytes);
        }
    }
    fn ask(text: bool, reasoning: bool, input: bool) -> smith_channel::LiveAsk {
        smith_channel::LiveAsk::new(&smith_channel::CEILINGS, smith_channel::LiveAskParts { text, reasoning, input })
            .expect("ask")
    }
    fn stream(takes: bool) -> (Live, Room) {
        let mut live = Live::new(Some(ask(true, false, false)), standard().policy.live_interval);
        let mut room = Room::new(takes);
        live.open(OWNER, 1, 0, &mut room);
        (live, room)
    }
    #[test]
    fn a_stream_past_the_held_bytes_drops_the_newest_and_counts_them() {
        let (mut live, mut room) = stream(false);
        live.delta(OWNER, &[b'a'; LARGE_DELTA], AT_ZERO, &mut room);
        live.delta(OWNER, &[b'b'; LARGE_DELTA], AT_ZERO, &mut room);
        assert_eq!(room.lost, 3616, "20,000 provider bytes minus the held bound");
        room.takes = true;
        live.pump(AT_ZERO.saturating_add(standard().policy.live_interval), &mut room);
        assert_eq!(room.offsets().as_slice(), [(0, 4096), (4096, 4096), (8192, 4096), (12_288, 4096)]);
        live.delta(OWNER, b"c", AT_ZERO, &mut room);
        live.pump(AT_ZERO.saturating_add(standard().policy.live_interval), &mut room);
        assert_eq!(room.sent.last().expect("later piece").offset, 20_000, "the omitted suffix leaves a gap");
    }
    #[test]
    fn a_delta_past_the_piece_bound_goes_in_several_pieces() {
        let (mut live, mut room) = stream(true);
        live.delta(OWNER, &[b'a'; LARGE_DELTA], AT_ZERO, &mut room);
        live.pump(AT_ZERO.saturating_add(standard().policy.live_interval), &mut room);
        assert_eq!(room.offsets().as_slice(), [(0, 4096), (4096, 4096), (8192, 1808)]);
        let mut joined = List::with_capacity(u32::try_from(LARGE_DELTA).expect("fixture bound"));
        for piece in &room.sent {
            for byte in &piece.bytes {
                joined.push(*byte).expect("fixture payload");
            }
        }
        assert_eq!(joined.as_slice(), &[b'a'; LARGE_DELTA]);
    }
    #[test]
    fn ending_a_turn_settles_the_held_bytes_first() {
        for (takes, pieces, lost) in [(true, 1, 0), (false, 0, 4)] {
            let (mut live, mut room) = stream(false);
            live.delta(OWNER, b"held", AT_ZERO, &mut room);
            room.takes = takes;
            live.end(&mut room);
            assert_eq!((room.sent.len(), room.lost), (pieces, lost), "room={takes}");
            room.takes = true;
            live.pump(AT_ZERO.saturating_add(standard().policy.live_interval), &mut room);
            assert_eq!(room.sent.len(), pieces, "no piece follows the ended turn");
        }
    }
    #[test]
    fn a_delta_after_its_turn_is_dropped_and_counted() {
        let (mut live, mut room) = stream(true);
        live.end(&mut room);
        live.delta(OWNER, b"late", AT_ZERO, &mut room);
        assert_eq!((room.sent.len(), room.lost), (0, 4));
    }
    #[test]
    fn a_new_retry_starts_again_at_offset_zero() {
        let (mut live, mut room) = stream(true);
        live.delta(OWNER, b"first", AT_ZERO, &mut room);
        live.pump(AT_ZERO.saturating_add(standard().policy.live_interval), &mut room);
        room.takes = false;
        live.delta(OWNER, b"held", AT_ZERO, &mut room);
        live.open(RETRY_OWNER, 1, 1, &mut room);
        room.takes = true;
        live.delta(RETRY_OWNER, b"again", AT_ZERO, &mut room);
        live.pump(AT_ZERO.saturating_add(standard().policy.live_interval), &mut room);
        let mut identities = List::with_capacity(room.sent.len());
        for piece in &room.sent {
            identities.push((piece.turn, piece.retry, piece.offset)).expect("sent identities");
        }
        assert_eq!(identities.as_slice(), [(1, 0, 0), (1, 1, 0)]);
        assert_eq!(room.lost, 4);
    }
    #[test]
    fn only_the_main_ordinary_completion_is_wanted() {
        let mut prompt = crate::responses::tests::prompt(0);
        for (text, reasoning, input, main_ordinary) in [
            (false, false, false, false),
            (true, false, false, true),
            (false, true, false, false),
            (false, false, true, false),
            (true, true, true, true),
        ] {
            let ask = ask(text, reasoning, input);
            for (asker, purpose, eligible) in [
                (llm::Asker::Main, llm::Purpose::Ordinary { guard: None }, true),
                (llm::Asker::Child, llm::Purpose::Ordinary { guard: None }, false),
                (llm::Asker::Main, llm::Purpose::Compaction { room: 1 }, false),
                (llm::Asker::Child, llm::Purpose::Compaction { room: 1 }, false),
            ] {
                prompt.asker = asker;
                prompt.purpose = purpose;
                assert_eq!(
                    wanted(Some(&ask), &prompt),
                    main_ordinary && eligible,
                    "{asker:?}, {purpose:?}, text={text}, reasoning={reasoning}, input={input}"
                );
                assert!(!wanted(None, &prompt), "an absent ask wants no completion");
            }
        }
    }
    #[test]
    fn the_held_bound_keeps_the_prefix_and_drops_only_the_excess() {
        for (input, lost) in [(AT_BOUND.as_slice(), 0), (PAST_BOUND.as_slice(), 1)] {
            let (mut live, mut room) = stream(false);
            live.delta(OWNER, input, AT_ZERO, &mut room);
            assert_eq!(room.lost, lost);
            room.takes = true;
            live.pump(AT_ZERO.saturating_add(standard().policy.live_interval), &mut room);
            let mut joined = List::with_capacity(super::HELD);
            for piece in &room.sent {
                for byte in &piece.bytes {
                    joined.push(*byte).expect("held prefix");
                }
            }
            assert_eq!(joined.as_slice(), &[b'a'; HELD]);
        }
    }
    #[test]
    fn a_gap_survives_partial_draining_and_later_held_bytes() {
        let (mut live, mut room) = stream(false);
        live.delta(OWNER, &[b'a'; LARGE_DELTA], AT_ZERO, &mut room);
        live.delta(OWNER, &[b'b'; LARGE_DELTA], AT_ZERO, &mut room);
        room.takes = true;
        room.remaining = 1;
        live.pump(AT_ZERO.saturating_add(standard().policy.live_interval), &mut room);
        live.delta(OWNER, b"c", AT_ZERO, &mut room);
        room.remaining = u32::MAX;
        live.pump(AT_ZERO.saturating_add(standard().policy.live_interval), &mut room);
        assert_eq!(room.offsets().as_slice(), [(0, 4096), (4096, 4096), (8192, 4096), (12_288, 4096), (20_000, 1)]);
        assert_eq!(room.lost, 3616);
    }
    #[test]
    fn a_short_piece_goes_at_the_intervals_end() {
        let interval = standard().policy.live_interval;
        let (mut live, mut room) = stream(true);
        live.delta(OWNER, b"Hel", AT_ZERO, &mut room);
        let end = AT_ZERO.saturating_add(interval);
        live.pump(Time::from_nanos(end.as_nanos() - 1), &mut room);
        assert_eq!(room.sent.len(), 0, "held before the interval ends");
        assert_eq!(live.deadline(), Some(end));
        live.pump(end, &mut room);
        assert_eq!(room.payloads().as_slice(), [b"Hel"]);
        assert_eq!(live.deadline(), None);
    }

    #[test]
    fn a_piece_reaching_the_bound_goes_before_the_interval() {
        let (mut live, mut room) = stream(true);
        live.delta(OWNER, &[b'a'; PIECE_BOUND], AT_ZERO, &mut room);
        assert_eq!(room.payloads().as_slice(), [[b'a'; PIECE_BOUND].as_slice()]);
        assert_eq!(live.deadline(), None);
    }

    #[test]
    fn draining_a_piece_retains_the_next_bytes_arrival_time() {
        let interval = standard().policy.live_interval;
        let (mut live, mut room) = stream(true);
        live.delta(OWNER, &[b'a'; PIECE_BOUND - 1], AT_ZERO, &mut room);
        let end = AT_ZERO.saturating_add(interval);
        let later = Time::from_nanos(end.as_nanos() - 1);
        live.delta(OWNER, b"bc", later, &mut room);
        assert_eq!(live.deadline(), Some(later.saturating_add(interval)));
    }
}
