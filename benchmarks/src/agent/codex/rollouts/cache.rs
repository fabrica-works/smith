//! Cache counts retain the rollout's response order, separately per named thread.
//! The fixed-model probe stays within its declared window; child scopes join
//! only by the native parent link, never by cumulative total subtraction.

use crate::{CacheObservation, Measure, Usage};

use super::{Link, ResponseUsage, Rollouts};

fn add(responses: &[ResponseUsage], value: &mut CacheObservation) -> Result<(), String> {
    let mut previous = None;
    for response in responses {
        let Usage::Codex { input, cached_input, .. } = response.usage else {
            return Err("named rollout has non-Codex usage".into());
        };
        if let Some(prompt) = previous
            && prompt >= 1152
        {
            value.cache_read = value.cache_read.checked_add(cached_input).ok_or("cache read sum overflow")?;
            value.predecessor_prompt = value.predecessor_prompt.checked_add(prompt).ok_or("prompt sum overflow")?;
            value.eligible = value.eligible.checked_add(1).ok_or("eligible count overflow")?;
            if u128::from(cached_input) * 2 < u128::from(prompt) {
                value.full_misses = value.full_misses.checked_add(1).ok_or("miss count overflow")?;
            }
        }
        previous = Some(input);
    }
    Ok(())
}

impl Rollouts {
    /// Native per-response cache counts, supplied to the paired triplet report.
    #[must_use]
    pub fn cache_reuse(
        &self,
        root: &str,
        median_at_least: f64,
        full_misses_per_ten_at_most: u32,
    ) -> Measure<CacheObservation> {
        let mut value = CacheObservation {
            cache_read: 0,
            predecessor_prompt: 0,
            eligible: 0,
            full_misses: 0,
            median_at_least,
            full_misses_per_ten_at_most,
        };
        let measured = (|| {
            add(&self.responses, &mut value)?;
            for child in &self.children {
                match &child.link {
                    Link::Linked { parent } if parent == root => {}
                    Link::Linked { .. } | Link::Unlinked => {
                        return Err("cache child lacks its actual direct parent link".into());
                    }
                }
                if child.responses.is_empty() {
                    return Err("cache child has no reported responses".into());
                }
                add(&child.responses, &mut value)?;
            }
            if value.eligible == 0 {
                return Err("no reported cacheable predecessor".into());
            }
            Ok::<(), String>(())
        })();
        match measured {
            Ok(()) => Measure::Observed { value },
            Err(reason) => Measure::unavailable(reason),
        }
    }
}
