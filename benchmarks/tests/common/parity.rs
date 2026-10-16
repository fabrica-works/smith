use sha2::{Digest, Sha256};
use smith_bench::{Measure, RunSummary};

/// Recorded arm walls and smith priced spend, supplied to the fixture.
#[derive(Clone, Copy)]
pub struct Rows {
    pub smith_wall_ms: u64,
    pub codex_wall_ms: u64,
    pub smith_usd: f64,
}

impl Rows {
    pub const fn equal() -> Self {
        Self { smith_wall_ms: 100_000, codex_wall_ms: 100_000, smith_usd: 1.0 }
    }
}

pub fn parity_summary(rows: Rows) -> RunSummary {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/parity/on-par.json");
    let mut summary = smith_bench::read_summary(&file).expect("recorded parity");
    for row in &mut summary.attempts {
        row.task_wall_ms =
            Measure::Observed { value: if row.agent == "smith" { rows.smith_wall_ms } else { rows.codex_wall_ms } };
        row.priced_usd = Measure::Observed { value: if row.agent == "smith" { rows.smith_usd } else { 1.0 } };
    }
    summary
}

pub const CHANGE_COMMIT: &str = "0000000000000000000000000000000000000000";
pub const PARENT_COMMIT: &str = "1111111111111111111111111111111111111111";

pub fn change_summary() -> RunSummary {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/parity/change.json");
    smith_bench::read_summary(&file).expect("recorded change")
}

pub fn change_baseline() -> RunSummary {
    let mut summary = parity_summary(Rows::equal());
    summary.attempts.clear();
    summary.groups.clear();
    for criterion in &mut summary.parity.as_mut().expect("recorded parity").criteria {
        if criterion.name != "O" {
            criterion.interval = (0.9, 1.1);
        }
    }
    summary
}

pub fn named_change(name: &str) -> (RunSummary, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!("smith-change-{name}-{}", std::process::id()));
    std::fs::create_dir(&directory).expect("temporary input directory");
    let baseline = directory.join("baseline.json");
    smith_bench::write_summary(&baseline, &change_baseline()).expect("baseline input");
    let mut summary = change_summary();
    let report = summary.change.as_mut().expect("recorded section");
    report.baseline_run = Some(smith_bench::parity::change::Input {
        sha256: Some(format!("{:x}", Sha256::digest(std::fs::read(&baseline).expect("baseline bytes")))),
        path: baseline,
    });
    for projection in &mut report.projection {
        projection.baseline_interval = (0.9, 1.1);
    }
    (summary, directory)
}
