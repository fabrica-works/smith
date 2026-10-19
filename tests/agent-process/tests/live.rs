//! Live channel text through the hosted agent and independent byte provider.
use skein_fake_llm_domain::api;
use skein_lib::Duration;
use skein_world::domain::heap::Counting;
use smith_agent_process_world::{TraceCase, World, charter};
use smith_agent_service::sinks::Capture;

#[global_allocator]
static HEAP: Counting = Counting;

const SEED: u64 = 37;
// The native trace fixture reserves four host output frames.
const FRAMES: u32 = 4;
const REPLY: &[u8] = b"Hello";

fn world(case: TraceCase) -> World {
    let mut world = World::new(SEED, &charter());
    world.anthropic();
    world.observe_facts(case, FRAMES);
    world.check_memory();
    world
}

#[test]
fn a_start_asking_for_text_gets_the_reply_as_pieces_before_its_turn() {
    let mut world = world(TraceCase::Failed);
    world.capture(Capture::None);
    world.ask(true, false, false);
    assert!(world.settle());
    let pieces = world.pieces();
    let mut bytes = Vec::new();
    let positions: Vec<_> = pieces.iter().map(|piece| (piece.turn(), piece.retry(), piece.offset())).collect();
    assert_eq!(positions, [(1, 0, 0)]);
    for piece in &pieces {
        bytes.extend_from_slice(piece.bytes());
    }
    assert_eq!(bytes, REPLY);
    let mut turn_told = false;
    for frame in world.observed() {
        if frame.kind == 0x0109 {
            turn_told = true;
        }
        if frame.kind == 0x0112 {
            assert!(!turn_told, "pieces precede their turn's record");
        }
    }
    assert!(
        world.records().iter().all(|record| !matches!(record.event, smith_events::Event::TextDelta(_))),
        "capture none receives no event deltas for channel-only requests"
    );
}

#[test]
fn no_ask_and_an_ask_without_text_give_no_piece() {
    for ask in [None, Some((false, false, false)), Some((false, true, true))] {
        let mut world = world(TraceCase::Failed);
        if let Some((text, reasoning, input)) = ask {
            world.ask(text, reasoning, input);
        }
        assert!(world.settle());
        assert_eq!(world.pieces().len(), 0, "ask={ask:?}");
    }
}

#[test]
fn a_sub_agents_completion_gives_no_piece() {
    let mut world = world(TraceCase::ChildAnswered);
    world.ask(true, false, false);
    assert!(world.settle());
    assert_eq!(world.pieces().len(), 0, "the main replies call tools; the child's text stays off the channel");
    let mut deltas = Vec::new();
    for record in world.records() {
        if let smith_events::Event::TextDelta(delta) = &record.event {
            deltas.extend_from_slice(&delta.text);
        }
    }
    assert_eq!(deltas, b"actual child answer", "the event owner's child deltas remain observable");
}

const HOLD: &[u8] = b"h";
const FIRST: &[u8] = b"Hel";
const LAST: &[u8] = b"lo";
// The fixture releases one second after its first query, beyond the 100 ms interval.
const RELEASE_AFTER: Duration = Duration::from_secs(1);
const FIRST_QUERY: usize = 1;
// Byte offsets of the literal prefix "Hel".
const FIRST_BYTES: u64 = 3;

fn held_after(first: &[u8], hold: &[u8], last: &[u8]) -> Box<[api::Turn]> {
    Box::new([api::Turn {
        lines: Box::new([api::Line::Pieces {
            pieces: Box::new([
                api::Piece { text: first.into(), hold: None },
                api::Piece { text: last.into(), hold: Some(hold.into()) },
            ]),
        }]),
        finish: api::Finish::Stop,
        tokens: 1,
    }])
}

fn held_world(queries: usize) -> World {
    let mut world = world(TraceCase::Failed);
    world.ask(true, false, false);
    world.script(held_after(FIRST, HOLD, LAST));
    world.release_after(HOLD, queries, RELEASE_AFTER);
    world
}

fn positions(pieces: &[smith_channel::Live]) -> Vec<(u32, u64, &[u8])> {
    pieces.iter().map(|piece| (piece.retry(), piece.offset(), piece.bytes())).collect()
}

#[test]
fn a_held_reply_shows_its_first_piece_at_the_intervals_end() {
    let mut world = held_world(FIRST_QUERY);
    assert!(world.settle());
    let before = world.pieces_at_release();
    assert_eq!(positions(&before), [(0, 0, FIRST)]);
    let pieces = world.pieces();
    assert_eq!(positions(&pieces), [(0, 0, FIRST), (0, FIRST_BYTES, LAST)]);
}
