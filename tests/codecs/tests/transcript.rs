//! Turn wire boundaries (protocol/transcript.md, sections 2, 6 and 7).

use skein_codec::Reason;
use skein_lib::Reader;
use smith_transcript::v5::{CEILINGS, Path, Turn};

#[test]
fn turn_goldens_decode_at_their_measured_length() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-transcript/golden/v5");
    for name in ["record_turn_smallest.bin", "record_turn_full.bin"] {
        let bytes = std::fs::read(root.join(name)).expect("turn golden");
        let value = Turn::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("turn decodes");
        assert_eq!(u32::try_from(bytes.len()).expect("golden length fits"), value.measure());
    }
}

#[test]
fn turn_rejects_an_unsupported_version_before_fields() {
    let problem = Turn::decode(&CEILINGS, &mut Reader::new(&[0, 2])).expect_err("version two");
    assert_eq!((problem.path, problem.reason), (Path::TurnVersion, Reason::Version));
}

#[test]
fn nesting_problem_goldens_have_their_own_wire_tag() {
    use smith_transcript::v5::CallProblem;
    for bytes in [
        include_bytes!("../../../crates/smith-transcript/golden/v5/enum_call_problem_nesting_full.bin").as_slice(),
        include_bytes!("../../../crates/smith-transcript/golden/v5/enum_call_problem_nesting_smallest.bin").as_slice(),
    ] {
        let mut reader = Reader::new(bytes);
        assert_eq!(CallProblem::decode(&CEILINGS, &mut reader).expect("nesting problem"), CallProblem::Nesting);
        assert_eq!(bytes, [8], "the handwritten tag is distinct from every prior problem");
    }
}

#[test]
fn failure_goldens_keep_their_tags_and_now_require_a_resolved_path_payload() {
    use smith_transcript::{Outcome, Resolved, ResolvedFailure, ResolvedFailureParts, WorkingPath, WorkingPathParts};
    let failure = ResolvedFailure::new(
        &CEILINGS,
        ResolvedFailureParts {
            path: Resolved::Working(
                WorkingPath::new(&CEILINGS, WorkingPathParts { path: Box::new([]) }).expect("empty directory path"),
            ),
        },
    )
    .expect("bounded failure");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-transcript/golden/v5");
    for (name, tag, expected) in [
        ("outside", 7, Outcome::Outside(failure.clone())),
        ("read_only", 8, Outcome::ReadOnly(failure.clone())),
        ("not_found", 10, Outcome::NotFound(failure.clone())),
        ("not_directory", 14, Outcome::NotDirectory(failure)),
    ] {
        let bytes = std::fs::read(root.join(format!("enum_outcome_{name}_smallest.bin"))).expect("frozen golden");
        assert_eq!(bytes, [tag, 0, 0, 0, 0, 0], "tag, Working descriptor, zero path length");
        let decoded = Outcome::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("resolved golden");
        assert_eq!(decoded, expected);
        assert!(Outcome::decode(&CEILINGS, &mut Reader::new(&[tag])).is_err(), "bare failure lacks its path");
    }
}

#[test]
fn indexed_edit_failure_goldens_keep_the_first_failure_and_matching_reason() {
    use skein_lib::{List, Writer};
    use smith_transcript::{
        Ambiguous, AmbiguousParts, EditFailed, EditFailedParts, LineNumber, LineNumberParts, Mismatch, Outcome,
    };
    let mut lines = List::with_capacity(2);
    for number in [3, 7] {
        lines.push(LineNumber::new(&CEILINGS, LineNumberParts { number }).expect("line")).expect("room");
    }
    let value = Outcome::EditFailed(
        EditFailed::new(
            &CEILINGS,
            EditFailedParts {
                edit: 2,
                why: Mismatch::Ambiguous(
                    Ambiguous::new(&CEILINGS, AmbiguousParts { count: 5, lines }).expect("bounded reason"),
                ),
            },
        )
        .expect("indexed failure"),
    );
    let mut writer = Writer::new(usize::try_from(value.measure()).expect("bound"));
    value.encode(&mut writer).expect("sized encoding");
    let bytes = writer.finish();
    assert_eq!(bytes.as_ref(), &[18, 0, 0, 0, 2, 1, 0, 0, 0, 5, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 7]);
    assert_eq!(Outcome::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("decode"), value);
}

#[test]
fn a_turn_round_trips_its_highest_job_number() {
    use skein_lib::Writer;
    let smallest = include_bytes!("../../../crates/smith-transcript/golden/v5/record_turn_smallest.bin");
    let mut parts = Turn::decode(&CEILINGS, &mut Reader::new(smallest)).expect("smallest turn").into_parts();
    parts.jobs = 37;
    let turn = Turn::new(&CEILINGS, parts).expect("turn with jobs");
    let mut writer = Writer::new(usize::try_from(turn.measure()).expect("turn length"));
    turn.encode(&mut writer).expect("sized encoding");
    let bytes = writer.finish();
    let decoded = Turn::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("turn decodes");
    assert_eq!(decoded.jobs(), 37);
    assert_eq!(decoded, turn);
}

#[test]
fn a_running_result_round_trips() {
    use smith_transcript::{BytesClamp, CommandRunning, CommandRunningParts, Outcome, TimeoutClamp};
    let running = CommandRunning::new(
        &CEILINGS,
        CommandRunningParts {
            job: 12,
            head: b"before yield".as_slice().into(),
            last: b"latest".as_slice().into(),
            dropped: 19,
            timeout: TimeoutClamp::Within,
            output: BytesClamp::Within,
            after: TimeoutClamp::Within,
        },
    )
    .expect("bounded running result");
    round_trip_outcome(&Outcome::Running(running));
}

#[test]
fn a_job_result_round_trips_each_end() {
    use smith_transcript::{
        BytesClamp, Exit, JobEnd, JobExit, JobExitParts, JobRead, JobReadParts, Outcome, TimeoutClamp,
    };
    let exited = JobExit::new(&CEILINGS, JobExitParts { exit: Exit::TimedOut }).expect("ended job");
    for end in [JobEnd::Running, JobEnd::Ended(exited), JobEnd::Stopped, JobEnd::Cancelled] {
        let read = JobRead::new(
            &CEILINGS,
            JobReadParts {
                job: 12,
                head: b"after yield".as_slice().into(),
                last: b"last".as_slice().into(),
                dropped: 7,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end,
            },
        )
        .expect("bounded job result");
        round_trip_outcome(&Outcome::Job(read));
    }
}

fn round_trip_outcome(value: &smith_transcript::Outcome) {
    use skein_lib::Writer;
    let mut writer = Writer::new(usize::try_from(value.measure()).expect("result length"));
    value.encode(&mut writer).expect("sized encoding");
    let bytes = writer.finish();
    let mut reader = Reader::new(&bytes);
    let decoded = smith_transcript::Outcome::decode(&CEILINGS, &mut reader).expect("result decodes");
    assert_eq!(&decoded, value, "result {value:?}");
    assert!(reader.is_empty(), "the result consumed all bytes");
}

#[test]
fn the_transcripts_effort_bound_equals_the_charters() {
    assert_eq!(smith_transcript::CEILINGS.turn_effort_some, smith_charter::CEILINGS.llm_effort_some);
}

#[test]
fn a_not_run_why_survives_a_round_trip() {
    use skein_lib::{List, Writer};
    use smith_transcript::{
        AfterFailedWrite, AfterFailedWriteParts, AllowedCalls, AllowedCallsParts, CallName, CallNameParts, NotRun,
        NotRunParts, NotRunWhy, OutsideChoice, OutsideChoiceParts, Returned,
    };
    const ALLOWED_CALLS: u32 = 2;
    let mut names = List::with_capacity(ALLOWED_CALLS);
    for name in [b"finish".as_slice(), b"deliver".as_slice()] {
        names.push(CallName::new(&CEILINGS, CallNameParts { name: name.into() }).expect("bounded name")).expect("room");
    }
    let allowed = AllowedCalls::new(&CEILINGS, AllowedCallsParts { names }).expect("allowed calls");
    for why in [
        NotRunWhy::Stopped,
        NotRunWhy::OutsideChoice(OutsideChoice::new(&CEILINGS, OutsideChoiceParts { only: None }).expect("no calls")),
        NotRunWhy::OutsideChoice(
            OutsideChoice::new(&CEILINGS, OutsideChoiceParts { only: Some(allowed) }).expect("choice"),
        ),
        NotRunWhy::AfterFailedWrite(
            AfterFailedWrite::new(&CEILINGS, AfterFailedWriteParts { call: 12 }).expect("position"),
        ),
    ] {
        let value = Returned::NotRun(NotRun::new(&CEILINGS, NotRunParts { why }).expect("not run"));
        let mut writer = Writer::new(usize::try_from(value.measure()).expect("size"));
        value.encode(&mut writer).expect("encoding");
        assert_eq!(Returned::decode(&CEILINGS, &mut Reader::new(&writer.finish())).expect("decoding"), value);
    }
}

#[test]
fn a_version_four_transcript_is_refused() {
    let problem = Turn::decode(&CEILINGS, &mut Reader::new(&[0, 4])).expect_err("version four");
    assert_eq!((problem.path, problem.reason), (Path::TurnVersion, Reason::Version));
}
