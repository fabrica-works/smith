//! Independent attacks on transcript.md, section 12, through the kept set alone.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use skein_codec::patch::{self, Capacity, Halt, PatchWrong, Patcher, Site, Step};
use skein_codec::schema::{self, FieldIx, Held, Schema, Shape, Type};
use skein_codec::value::{Part, Refusal, Value};
use skein_codegen::patch::{Finding, Landed};
use smith_transcript_patches::patches;

#[path = "support/patches.rs"]
mod host;

const CURRENT_TEXT: &str = include_str!("../../../crates/smith-transcript/schema/transcript-v6.schema");
const KEPT_V1: &str = include_str!("../../transcript-patches/schema/kept/fixture-turn-v1.schema");
const KEPT_V2: &str = include_str!("../../transcript-patches/schema/kept/fixture-turn-v2.schema");
const FIXTURE_PATCH: &str = include_str!("../../transcript-patches/patches/0001.patch");
const SMALLEST: &[u8] =
    include_bytes!("../../transcript-patches/patches/golden/0001/fixture-turn/record_turn_smallest.bin");
const FULL: &[u8] = include_bytes!("../../transcript-patches/patches/golden/0001/fixture-turn/record_turn_full.bin");
// transcript.md, section 12: Found.hits alone has the pieces word.
const HIT_BYTES: u32 = 2_097_152;
// Both fields of the fixture's kept version 1 have a ceiling of 64 bytes.
const FIELD_BYTES: usize = 64;
const PAST_FIELD_BYTES: usize = FIELD_BYTES + 1;
// codec.md, 11.6: two kept versions and one conversion plan for this fixture.
const VERSIONS: u32 = 2;
const PLANS: u32 = 1;
const TEXT_BYTES: u32 = 256;
const NO_PLANS: u32 = 0;
const SHORT_NODES: u32 = host::NODES - 1;
// Version first, then a u32 length before each of the two fields.
const VERSION_ONE: u16 = 1;
const VERSION_TWO: u16 = 2;
const CURRENT_VERSION: u16 = 6;
const FIRST_PATCH: u32 = 1;
const BYTE_PAST_CAPACITY: u8 = 0;
// No patch in either exported set has any written site, including patch 1.
const UNKNOWN_SITE_CASES: usize = 3;
const UNKNOWN_SITES: [Site; UNKNOWN_SITE_CASES] =
    [Site::new(0, 0), Site::new(FIRST_PATCH, 0), Site::new(u32::MAX, u32::MAX)];
const MARK_ONLY: &[u8] = b"patch 1\nowner fixture-smith 2\nmark fresh-windows\n";
const UNMARKED: &[u8] = b"patch 1\nowner fixture-smith 2\n";
// A holder's fields surround the held turn, so its resumed walk is visible.
const HOLDER_TEXT: &[u8] =
    b"family qa-holder 1\nversioned record Store { before: u8 turn: bytes 138 holds fixture-turn Turn after: u8 }\n";
const HOLDER_VERSIONS: u32 = 1;
const HOLDER_NODES: u32 = 4;
// Version, before, held length, largest turn, after: 2 + 1 + 4 + 138 + 1.
const HOLDER_BYTES: u32 = 146;
const HOLDER_BEFORE: u8 = 17;
const HOLDER_AFTER: u8 = 93;
const FROZEN_TEXT_PATH: &str = "schema/kept/fixture-turn-v1.schema";
const FROZEN_GOLDEN_PATH: &str = "patches/golden/0001/fixture-turn/record_turn_smallest.bin";
// Current schema, two kept texts, one patch and its two goldens.
const OWNER_DATA_FILES: usize = 6;
static NEXT_DIRECTORY: AtomicU32 = AtomicU32::new(0);

fn fixture_capacity() -> Capacity {
    Capacity { versions: VERSIONS, patches: PLANS, text: TEXT_BYTES, bytes: host::RECORD_BYTES, nodes: host::NODES }
}

fn kept_patcher(capacity: Capacity) -> Patcher {
    let mut patcher = Patcher::new(capacity);
    for text in patches::kept::fixture_turn::TEXTS {
        patcher.keep(text.as_bytes()).expect("keep the fixture owner's texts");
    }
    patcher
}

fn planned_patcher(capacity: Capacity) -> Patcher {
    let mut patcher = kept_patcher(capacity);
    for text in patches::PATCHES {
        patcher.plan(text.as_bytes()).expect("plan the fixture owner's set");
    }
    patcher
}

fn kept_turn(said: &[u8], replay: &[u8]) -> Box<[u8]> {
    let mut record = VERSION_ONE.to_be_bytes().to_vec();
    record.extend_from_slice(&u32::try_from(said.len()).expect("said length").to_be_bytes());
    record.extend_from_slice(said);
    record.extend_from_slice(&u32::try_from(replay.len()).expect("replay length").to_be_bytes());
    record.extend_from_slice(replay);
    record.into_boxed_slice()
}

fn current_turn(said: &[u8]) -> Box<[u8]> {
    let mut record = VERSION_TWO.to_be_bytes().to_vec();
    record.extend_from_slice(&u32::try_from(said.len()).expect("said length").to_be_bytes());
    record.extend_from_slice(said);
    record.extend_from_slice(&0_u32.to_be_bytes());
    record.into_boxed_slice()
}

fn field_named(schema: &Schema, declaration: &[u8], name: &[u8]) -> FieldIx {
    let root = schema.find(declaration).expect("declaration in kept text");
    let Shape::Record { first, count, .. } = schema.declaration(root).shape else {
        panic!("the kept declaration is a record");
    };
    (first.index()..first.index() + count)
        .map(FieldIx::new)
        .find(|index| schema.spelled(schema.field(*index).name) == name)
        .expect("field in kept text")
}

fn assert_decode_field(record: Box<[u8]>, field: FieldIx) {
    let mut patcher = host::fixture_patcher();
    let result = host::patch_to_current(&mut patcher, record);
    assert!(
        matches!(result, Err(Halt::Decode(Refusal::Problem(problem))) if problem.path.part == Part::Field(field)),
        "the old kept text must refuse the field before its patch can discard it"
    );
}

fn holder_record(turn: &[u8]) -> Box<[u8]> {
    let mut record = VERSION_ONE.to_be_bytes().to_vec();
    record.push(HOLDER_BEFORE);
    record.extend_from_slice(&u32::try_from(turn.len()).expect("held turn length").to_be_bytes());
    record.extend_from_slice(turn);
    record.push(HOLDER_AFTER);
    record.into_boxed_slice()
}

fn waiting_holder(turn: &[u8]) -> (Patcher, skein_codec::value::Node, schema::Span, schema::Span) {
    let mut holder = Patcher::new(Capacity {
        versions: HOLDER_VERSIONS,
        patches: PLANS,
        text: TEXT_BYTES,
        bytes: HOLDER_BYTES,
        nodes: HOLDER_NODES,
    });
    holder.keep(HOLDER_TEXT).expect("keep the holder's own text");
    let plan = holder.identity(VERSION_ONE).expect("holder identity reaches its held turn");
    let root = holder.schema(VERSION_ONE).expect("holder schema").find(b"Store").expect("holder root");
    holder.start(plan, root, holder_record(turn)).expect("start the holder");
    let Step::Held { at, family, name } = holder.next().expect("reach the held turn") else {
        panic!("the holder must stop before its held turn");
    };
    (holder, at, family, name)
}

/// A fixture owner made only from allowed data; all mutations die with it.
struct FixtureOwner(PathBuf);

impl FixtureOwner {
    fn new() -> Self {
        let serial = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let owner = Self(std::env::temp_dir().join(format!("smith-qa-patches-{}-{serial}", std::process::id())));
        fs::create_dir(&owner.0).expect("make a fresh temporary fixture owner");
        let files: [(&str, &[u8]); OWNER_DATA_FILES] = [
            ("schema/fixture-turn-v2.schema", KEPT_V2.as_bytes()),
            (FROZEN_TEXT_PATH, KEPT_V1.as_bytes()),
            ("schema/kept/fixture-turn-v2.schema", KEPT_V2.as_bytes()),
            ("patches/0001.patch", FIXTURE_PATCH.as_bytes()),
            (FROZEN_GOLDEN_PATH, SMALLEST),
            ("patches/golden/0001/fixture-turn/record_turn_full.bin", FULL),
        ];
        for (relative, bytes) in files {
            let destination = owner.0.join(relative);
            fs::create_dir_all(destination.parent().expect("fixture data has a parent")).expect("fixture directories");
            fs::write(destination, bytes).expect("copy only the fixture owner's data");
        }
        owner
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for FixtureOwner {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove the temporary fixture owner");
    }
}

#[test]
fn smith_set_is_named_data_empty_before_the_baseline() {
    assert_eq!(smith_transcript::OWNER, "smith");
    assert_eq!(smith_transcript::FAMILY, "smith-transcript");
    assert_eq!(smith_transcript::ROOT, "Turn");
    assert_eq!(smith_transcript::FRESH_WINDOWS, "fresh-windows");
    assert!(smith_transcript::patches::PATCHES.is_empty(), "no patch before the baseline");
    assert!(smith_transcript::patches::kept::smith_transcript::TEXTS.is_empty(), "no kept text before the baseline");
}

#[test]
fn found_hits_names_hit_as_held_pieces_at_its_bound() {
    let schema = schema::parse(CURRENT_TEXT.as_bytes()).expect("the current turn schema");
    let hits = field_named(&schema, b"Found", b"hits");
    assert_eq!(
        schema.ty(schema.field(hits).ty),
        Type::Bytes { bound: HIT_BYTES, held: Held::Pieces(schema.find(b"Hit").expect("Hit declaration")) },
        "transcript.md 12 owes the pieces word without moving version 6"
    );
    assert_eq!(schema.version(), CURRENT_VERSION);
}

#[test]
fn every_other_turn_bytes_field_remains_opaque() {
    let schema = schema::parse(CURRENT_TEXT.as_bytes()).expect("the current turn schema");
    let hits = field_named(&schema, b"Found", b"hits");
    for declaration in 0..schema.declarations() {
        if let Shape::Record { first, count, .. } = schema.declaration(schema::DeclIx::new(declaration)).shape {
            for index in first.index()..first.index() + count {
                let field = FieldIx::new(index);
                if let Type::Bytes { held, .. } = schema.ty(schema.field(field).ty)
                    && field != hits
                {
                    assert_eq!(held, Held::Opaque, "only Found.hits is pieces; replay, paths and text stay opaque");
                }
            }
        }
    }
}

#[test]
fn mark_without_any_family_still_requires_fresh_windows() {
    let header = patch::header(MARK_ONLY).expect("a marked header needs no declaration change");
    assert!(header.marked(MARK_ONLY, smith_transcript::FRESH_WINDOWS.as_bytes()));
    assert!(!header.marked(MARK_ONLY, b"fresh-window"), "mark spelling is exact");
}

#[test]
fn unmarked_header_allows_resume_across_the_patch() {
    let header = patch::header(UNMARKED).expect("an unmarked patch header");
    assert!(!header.marked(UNMARKED, smith_transcript::FRESH_WINDOWS.as_bytes()));
}

#[test]
fn fixture_owner_header_carries_the_fresh_windows_mark() {
    let text = FIXTURE_PATCH.as_bytes();
    let header = patch::header(text).expect("the fixture owner's exported patch header");
    let owner = header.owner();
    let owner_range =
        usize::try_from(owner.start).expect("owner start")..usize::try_from(owner.end).expect("owner end");
    assert_eq!(&text[owner_range], b"fixture-smith");
    assert_eq!(header.level(), FIRST_PATCH);
    assert!(header.marked(text, smith_transcript::FRESH_WINDOWS.as_bytes()));
}

#[test]
fn mark_only_patch_consumes_no_family_plan_room() {
    let mut patcher = host::fixture_patcher();
    assert_eq!(patcher.plan(MARK_ONLY).expect("read the mark even with plan room full"), None);
    assert!(patcher.plan_from(VERSION_ONE).is_some(), "the fixture owner's conversion remains planned");
}

#[test]
fn fixture_set_keeps_versions_and_field_names_in_order() {
    assert_eq!(patches::kept::fixture_turn::TEXTS, &[KEPT_V1, KEPT_V2]);
    assert_eq!(patches::PATCHES, &[FIXTURE_PATCH]);
    let expected = [
        (VERSION_ONE, b"said".as_slice(), patches::kept::fixture_turn::v1::turn::SAID),
        (VERSION_ONE, b"replay".as_slice(), patches::kept::fixture_turn::v1::turn::REPLAY),
        (VERSION_TWO, b"said".as_slice(), patches::kept::fixture_turn::v2::turn::SAID),
        (VERSION_TWO, b"tokens".as_slice(), patches::kept::fixture_turn::v2::turn::TOKENS),
    ];
    let patcher = host::fixture_patcher();
    for (version, name, field) in expected {
        let schema = patcher.schema(version).expect("exported kept version");
        assert_eq!(schema.spelled(schema.field(field).name), name, "a field constant names its own kept version");
    }
}

#[test]
fn fixture_goldens_patch_to_the_reviewed_current_bytes() {
    let cases = [(kept_turn(b"", b""), SMALLEST), (kept_turn(b"abc", b"abc"), FULL)];
    let mut patcher = host::fixture_patcher();
    for (old, expected) in cases {
        let converted = host::patch_to_current(&mut patcher, old).expect("patch a fixture golden");
        assert_eq!(converted.as_ref(), expected, "the set must reproduce its reviewed golden");
    }
}

#[test]
fn repeated_reads_in_either_order_keep_identical_turn_bytes() {
    let orders = [[b"first".as_slice(), b"last".as_slice()], [b"last".as_slice(), b"first".as_slice()]];
    let mut patcher = host::fixture_patcher();
    for order in orders {
        for said in order {
            let kept = kept_turn(said, b"provider replay");
            let converted =
                host::patch_to_current(&mut patcher, kept.clone()).expect("patch independently on every read");
            assert_eq!(converted, current_turn(said), "a prior read must not affect this turn");
            assert_eq!(kept, kept_turn(said, b"provider replay"), "the host keeps the bytes as originally written");
        }
    }
}

#[test]
fn kept_text_accepts_its_exact_utf8_byte_bound() {
    // Each e-acute takes two UTF-8 bytes; bounds count bytes, not characters.
    let said = "é".repeat(FIELD_BYTES / "é".len());
    let converted = host::patch_to_current(&mut host::fixture_patcher(), kept_turn(said.as_bytes(), b""))
        .expect("a kept text at its byte bound");
    assert_eq!(converted, current_turn(said.as_bytes()));
}

#[test]
fn kept_text_refuses_one_byte_past_its_bound() {
    assert_decode_field(kept_turn(&[b'a'; PAST_FIELD_BYTES], b""), patches::kept::fixture_turn::v1::turn::SAID);
}

#[test]
fn kept_replay_is_checked_before_the_patch_drops_it() {
    assert_decode_field(kept_turn(b"", &[0xff; PAST_FIELD_BYTES]), patches::kept::fixture_turn::v1::turn::REPLAY);
}

#[test]
fn truncated_kept_replay_is_refused_before_it_is_dropped() {
    let mut record = kept_turn(b"", b"replay").into_vec();
    record.pop().expect("truncate a nonempty replay");
    assert_decode_field(record.into_boxed_slice(), patches::kept::fixture_turn::v1::turn::REPLAY);
}

#[test]
fn old_kept_turn_refuses_a_trailing_byte() {
    let mut record = kept_turn(b"", b"").into_vec();
    record.push(BYTE_PAST_CAPACITY);
    let result = host::patch_to_current(&mut host::fixture_patcher(), record.into_boxed_slice());
    assert!(matches!(result, Err(Halt::Decode(Refusal::Problem(problem))) if problem.path.part == Part::Tail));
}

#[test]
fn kept_text_refuses_invalid_utf8_while_replay_stays_opaque() {
    assert_decode_field(kept_turn(&[0xff], b""), patches::kept::fixture_turn::v1::turn::SAID);
}

#[test]
fn largest_kept_turn_fits_exact_byte_and_node_room() {
    let said = [b'a'; FIELD_BYTES];
    let replay = [0xff; FIELD_BYTES];
    let converted = host::patch_to_current(&mut host::fixture_patcher(), kept_turn(&said, &replay))
        .expect("138 bytes and three nodes fit exactly, including opaque non-UTF8 replay");
    assert_eq!(converted, current_turn(&said));
}

#[test]
fn kept_turn_one_byte_past_capacity_halts_with_room() {
    let mut record = kept_turn(&[b'a'; FIELD_BYTES], &[0xff; FIELD_BYTES]).into_vec();
    record.push(BYTE_PAST_CAPACITY);
    let result = host::patch_to_current(&mut host::fixture_patcher(), record.into_boxed_slice());
    assert_eq!(result, Err(Halt::Room), "the old record cannot exceed the host's byte room");
}

#[test]
fn kept_turn_one_node_past_capacity_halts_with_decode_room() {
    let mut capacity = fixture_capacity();
    capacity.nodes = SHORT_NODES;
    let mut patcher = planned_patcher(capacity);
    let result = host::patch_to_current(&mut patcher, kept_turn(b"", b""));
    assert_eq!(result, Err(Halt::Decode(Refusal::Room)), "the root and both kept fields need three nodes");
}

#[test]
fn fixture_plan_refuses_one_plan_past_capacity() {
    let mut capacity = fixture_capacity();
    capacity.patches = NO_PLANS;
    let mut patcher = kept_patcher(capacity);
    let error = patcher.plan(FIXTURE_PATCH.as_bytes()).expect_err("one fixture plan cannot fit zero plan room");
    assert_eq!(error.wrong, PatchWrong::Room);
}

#[test]
fn next_start_after_decode_halt_begins_afresh() {
    let mut patcher = host::fixture_patcher();
    let _halt = host::patch_to_current(&mut patcher, kept_turn(&[0xff], b""));
    let converted = host::patch_to_current(&mut patcher, kept_turn(b"after halt", b"opaque"))
        .expect("a new start ends the prior halt");
    assert_eq!(converted, current_turn(b"after halt"));
}

#[test]
fn identity_is_never_returned_by_plan_from() {
    // Keep the identity at the current version, beside the one historical plan.
    let mut capacity = fixture_capacity();
    capacity.patches = PLANS + PLANS;
    let mut patcher = planned_patcher(capacity);
    patcher.identity(VERSION_TWO).expect("a current identity plan");
    assert_eq!(patcher.plan_from(VERSION_TWO), None, "a host's patch loop must terminate at the current version");
}

#[test]
fn written_dispatches_answer_every_unknown_site_with_no_site() {
    let mut patcher = host::fixture_patcher();
    let plan = patcher.plan_from(VERSION_ONE).expect("fixture plan");
    let root = patcher.schema(VERSION_ONE).expect("kept text").find(b"Turn").expect("Turn root");
    patcher.start(plan, root, kept_turn(b"", b"")).expect("old turn");
    let (old, new) = patcher.parts();
    let at = old.root();
    let mut smith_context = smith_transcript::patches::Context::default();
    let mut fixture_context = patches::Context::default();
    for site in UNKNOWN_SITES {
        assert_eq!(
            smith_transcript::patches::written(site, old, new, at, &mut smith_context),
            Err(Halt::NoSite(site)),
            "the pre-baseline set has no written site"
        );
        assert_eq!(patches::written(site, old, new, at, &mut fixture_context), Err(Halt::NoSite(site)));
    }
}

#[test]
fn held_stop_names_the_fixture_family_and_turn_root() {
    let (holder, _at, family, name) = waiting_holder(&kept_turn(b"held", b""));
    assert_eq!(holder.spelled(family), b"fixture-turn", "the held family belongs to the fixture owner");
    assert_eq!(holder.spelled(name), smith_transcript::ROOT.as_bytes(), "the held root is Turn");
}

#[test]
fn holder_identity_can_carry_the_kept_turn_unchanged() {
    let kept = kept_turn(b"held", b"opaque replay");
    let (mut holder, at, _family, _name) = waiting_holder(&kept);
    let (old, new) = holder.parts();
    new.carry(old, at).expect("carry the held turn as the host keeps it");
    assert_eq!(holder.next(), Ok(Step::Done));
    assert_eq!(holder.finish().expect("finish the holder"), holder_record(&kept).as_ref());
}

#[test]
fn holder_identity_accepts_the_fixture_owners_patched_turn() {
    let kept = kept_turn(b"held", b"opaque replay");
    let (mut holder, _at, _family, _name) = waiting_holder(&kept);
    let converted = host::patch_to_current(&mut host::fixture_patcher(), kept).expect("the fixture owner patches Turn");
    holder.parts().1.put_content(&converted).expect("answer the held stop with the patched turn");
    assert_eq!(holder.next(), Ok(Step::Done));
    assert_eq!(holder.finish().expect("finish the holder"), holder_record(&current_turn(b"held")).as_ref());
}

#[test]
fn unanswered_held_turn_ends_the_record_with_order() {
    let (mut holder, _at, _family, _name) = waiting_holder(&kept_turn(b"held", b""));
    assert_eq!(holder.next(), Err(Halt::Order), "the host owes exactly one value at a held stop");
}

#[test]
fn fixture_owner_data_passes_the_landed_check() {
    let owner = FixtureOwner::new();
    let landed = Landed::read(owner.path()).expect("the fixture owner's landed data");
    assert_eq!(skein_codegen::patch::check(owner.path(), &landed).expect("check the fixture owner"), []);
}

#[test]
fn landed_kept_text_is_frozen_even_for_a_comment() {
    let owner = FixtureOwner::new();
    let landed = Landed::read(owner.path()).expect("the fixture owner's landed data");
    fs::write(owner.path().join(FROZEN_TEXT_PATH), format!("{KEPT_V1}# changed after landing\n"))
        .expect("change only the temporary owner's frozen text");
    let findings = skein_codegen::patch::check(owner.path(), &landed).expect("check frozen kept text");
    assert!(findings.contains(&Finding::Changed { path: PathBuf::from(FROZEN_TEXT_PATH) }));
}

#[test]
fn landed_patch_golden_cannot_disappear() {
    let owner = FixtureOwner::new();
    let landed = Landed::read(owner.path()).expect("the fixture owner's landed data");
    fs::remove_file(owner.path().join(FROZEN_GOLDEN_PATH)).expect("remove only the temporary owner's golden");
    let findings = skein_codegen::patch::check(owner.path(), &landed).expect("check frozen golden");
    assert!(findings.contains(&Finding::Changed { path: PathBuf::from(FROZEN_GOLDEN_PATH) }));
}

#[test]
fn patched_turn_is_a_generic_value_of_the_current_kept_text() {
    let mut patcher = host::fixture_patcher();
    let converted = host::patch_to_current(&mut patcher, kept_turn(b"meaning", b"opaque replay"))
        .expect("patch the fixture owner's kept turn");
    let schema = patcher.schema(VERSION_TWO).expect("the current kept text");
    let declaration = schema.find(b"Turn").expect("the current Turn");
    let mut value = Value::with_room(host::NODES);
    let root = value.decode(schema, declaration, converted).expect("the patched turn must decode as current");
    let said = value.field(root, patches::kept::fixture_turn::v2::turn::SAID).expect("current said");
    let tokens = value.field(root, patches::kept::fixture_turn::v2::turn::TOKENS).expect("current tokens");
    assert_eq!(value.content(said).expect("said content"), b"meaning");
    assert_eq!(value.uint(tokens).expect("tokens count"), 0);
}
