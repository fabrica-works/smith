//! Plain byte costs supplied at startup by the protocol's schema renderer.
//! This module holds no JSON grammar or provider policy. `finish` and `deliver`
//! sum element costs and supplied text widths before entrance effects.
//! Contract: protocol/limits.md, section 3.9, "How the run counts what the protocol renders".

use crate::outcome::{ChangeSpec, FieldRule, OutcomeSpec};

/// Measured schema element costs sent by startup to every run entrance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SchemaCosts {
    pub finish_text: u64,
    pub deliver_text: u64,
    pub finish: u64,
    pub deliver: u64,
    pub change: u64,
    pub report: u64,
    pub failure: u64,
    /// The first form's cost excludes the inter-form separator.
    pub form_join: u64,
    pub verdict_first: u64,
    pub verdict_next: u64,
    pub kind_first: u64,
    pub kind_next: u64,
    pub field_first: u64,
    pub field_next: u64,
    /// Rendered widths of text bytes; quotes are already in the element costs.
    pub escaped: [u8; 256],
}

impl SchemaCosts {
    /// Synthetic schema element costs for a typed harness without provider encoding.
    pub const NONE: Self = Self {
        finish_text: 0,
        deliver_text: 0,
        finish: 0,
        deliver: 0,
        change: 0,
        report: 0,
        failure: 0,
        form_join: 0,
        verdict_first: 0,
        verdict_next: 0,
        kind_first: 0,
        kind_next: 0,
        field_first: 0,
        field_next: 0,
        escaped: [0; 256],
    };

    /// Count the exact finish schema, including every host-controlled form.
    #[must_use]
    pub fn finish(&self, outcome: &OutcomeSpec) -> Option<u64> {
        let mut bytes = self.finish;
        let mut groups = 0_u64;
        if let Some(change) = &outcome.change {
            bytes = bytes.checked_add(self.change)?.checked_add(self.fields(&change.fields)?)?;
            groups = groups.checked_add(1)?;
        }
        let mut first = true;
        for verdict in &outcome.verdicts {
            let base = if first { self.verdict_first } else { self.verdict_next };
            bytes = bytes
                .checked_add(base)?
                .checked_add(self.text(&verdict.name)?)?
                .checked_add(number(verdict.text_max))?
                .checked_add(number(verdict.items.min))?
                .checked_add(number(verdict.items.max))?
                .checked_add(self.fields(&verdict.fields)?)?;
            let mut first_kind = true;
            for kind in &verdict.items.kinds {
                let base = if first_kind { self.kind_first } else { self.kind_next };
                bytes = bytes
                    .checked_add(base)?
                    .checked_add(self.text(&kind.kind)?)?
                    .checked_add(self.fields(&kind.fields)?)?;
                first_kind = false;
            }
            first = false;
        }
        if !first {
            groups = groups.checked_add(1)?;
        }
        if let Some(report) = &outcome.report {
            bytes = bytes
                .checked_add(self.report)?
                .checked_add(number(report.max))?
                .checked_add(self.fields(&report.fields)?)?;
            groups = groups.checked_add(1)?;
        }
        if let Some(failure) = &outcome.failure {
            bytes = bytes
                .checked_add(self.failure)?
                .checked_add(number(failure.max))?
                .checked_add(self.fields(&failure.fields)?)?;
            groups = groups.checked_add(1)?;
        }
        match groups.checked_sub(1) {
            Some(joins) => bytes.checked_add(joins.checked_mul(self.form_join)?),
            None => Some(bytes),
        }
    }

    /// Count the exact schema of a separately granted delivery.
    #[must_use]
    pub fn deliver(&self, change: &ChangeSpec) -> Option<u64> {
        self.deliver.checked_add(self.fields(&change.fields)?)
    }

    fn fields(&self, fields: &[FieldRule]) -> Option<u64> {
        let mut bytes = 0_u64;
        let mut first = true;
        for field in fields {
            let base = if first { self.field_first } else { self.field_next };
            bytes = bytes
                .checked_add(base)?
                .checked_add(self.text(&field.name)?.checked_mul(2)?)?
                .checked_add(number(field.max))?;
            first = false;
        }
        Some(bytes)
    }

    fn text(&self, text: &[u8]) -> Option<u64> {
        let mut bytes = 0_u64;
        for byte in text {
            bytes = bytes.checked_add(u64::from(
                *self.escaped.get(usize::from(*byte)).expect("byte indexes the 256-entry width table"),
            ))?;
        }
        Some(bytes)
    }
}

fn number(mut value: u32) -> u64 {
    let mut digits = 1_u64;
    for _ in 0_u32..9 {
        value = value.checked_div(10).expect("nonzero fixed divisor");
        if value > 0 {
            digits = digits.checked_add(1).expect("a u32 has at most ten digits");
        }
    }
    digits
}
