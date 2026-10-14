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
