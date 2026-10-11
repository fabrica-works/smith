//! Neutral settled evidence supplies no inferred agent diagnostics or answers.

use std::path::Path;

use smith_bench::grade::{Ended, Snapshot};
use smith_bench::{End, Exit, Measure, OutcomeCheck};

fn checks(ended: &Ended, checks: &[OutcomeCheck]) -> Vec<smith_bench::CheckVerdict> {
    smith_bench::grade::checks(Path::new("."), &Snapshot::new(), &Snapshot::new(), checks, ended)
}

fn neutral_outcomes_read_only_the_settled_end_exit_and_retained_text() {
    let mut ended = Ended {
        end: End::Completed,
        exit: Measure::Observed { value: Exit::Code(0) },
        last_line: Some("smith: completion failed: protocol".into()),
        answer: Some("survey complete: marker-42".into()),
        login_unchanged: Measure::Observed { value: true },
    };
    let expected = [
        OutcomeCheck::End { end: "completed".into() },
        OutcomeCheck::ExitCode { code: 0 },
        OutcomeCheck::LastLine { pattern: "^smith: completion failed: protocol$".into() },
        OutcomeCheck::Answer { pattern: "marker-42$".into() },
        OutcomeCheck::LoginUnchanged,
    ];
    assert!(checks(&ended, &expected).iter().all(|check| check.passed == Measure::Observed { value: true }));
    ended.end = End::Timeout;
    ended.exit = Measure::Observed { value: Exit::Signal(9) };
    ended.login_unchanged = Measure::Observed { value: false };
    let judged = checks(&ended, &expected);
    for index in [0, 1, 4] {
        assert_eq!(judged[index].passed, Measure::Observed { value: false });
    }
    assert_eq!(judged[2].passed, Measure::Observed { value: true });
    assert_eq!(judged[3].passed, Measure::Observed { value: true });
    ended.exit = Measure::unavailable("setup never spawned");
    ended.last_line = None;
    ended.answer = None;
    ended.login_unchanged = Measure::unavailable("no source login was borrowed");
    let judged = checks(&ended, &expected);
    for index in [1, 2, 3, 4] {
        assert!(matches!(judged[index].passed, Measure::Unavailable { .. }));
    }
}

fn every_terminal_name_is_distinct_from_answer_content_and_exit_code() {
    let cases = [
        (End::Completed, "completed"),
        (End::Timeout, "timeout"),
        (End::Budget { which: smith_bench::BudgetLimit::Turns }, "budget"),
        (End::Refused { setup: "no login".into() }, "refused"),
        (End::Failed { reason: smith_bench::FailureReason::Model, detail: "wire".into() }, "failed"),
        (End::HarnessError { what: "known shape changed".into() }, "harness-error"),
    ];
    for (end, name) in cases {
        let ended = Ended {
            end,
            exit: Measure::unavailable("no exit"),
            last_line: None,
            answer: None,
            login_unchanged: Measure::unavailable("no source"),
        };
        let judged = checks(&ended, &[OutcomeCheck::End { end: name.into() }]);
        assert_eq!(judged[0].passed, Measure::Observed { value: true });
    }
}

#[test]
fn settled_agent_neutral_outcome_contracts() {
    neutral_outcomes_read_only_the_settled_end_exit_and_retained_text();
    every_terminal_name_is_distinct_from_answer_content_and_exit_code();
}
