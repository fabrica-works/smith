//! The legacy reader names missing contracts rather than proving their absence.
//! Main acceptance and the settled host exit are its only exit agreement proof;
//! the runner adds its own teardown, tree and filesystem measurements.
//! Contract: benchmarks.md section11.2; reliability-plan00 Interface sketches.

use std::collections::BTreeMap;

use crate::{CheckVerdict, End, Exit, Forced, Health, Measure, Passive};

use super::super::legacy::{Observer, Value};

pub(super) fn legacy(observer: &Observer, exit: Exit, forced: Forced) -> Health {
    let mut health = Health {
        loss_per_sink: BTreeMap::from([("trace".into(), observer.loss())]),
        headroom: BTreeMap::new(),
        passive_checks: Vec::new(),
    };
    for check in [
        Passive::StreamWhole,
        Passive::NothingLost,
        Passive::Paired,
        Passive::MessageTerminals,
        Passive::SpendSums,
        Passive::NoLimitFailure,
    ] {
        health.passive_checks.push(CheckVerdict {
            check: check.name().into(),
            passed: Measure::unavailable("legacy trace cannot prove the complete contract or its absence"),
            evidence: Vec::new(),
        });
    }
    let accepted = observer.accepted().is_some() || observer.answer() == Some(&Value::Name("Accepted".into()));
    let classification = observer.classify(exit, forced);
    let passed = if accepted {
        Measure::Observed { value: classification.end == End::Completed }
    } else {
        Measure::unavailable("legacy trace has no main acceptance with an observable matching exit")
    };
    health.passive_checks.push(CheckVerdict {
        check: Passive::ExitAgrees.name().into(),
        passed,
        evidence: classification.warnings,
    });
    crate::passive::unavailable_headroom(&mut health, "legacy trace has no effective startup limits");
    health
}
