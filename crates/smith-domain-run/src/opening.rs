//! Main's immutable conversation opening (domain/run.md, sections 3.3 and
//! 3.4; programming-model.md, section 6.2). This module keeps no state and
//! knows no provider dialect, host policy or live clock. `main` copies the
//! admitted charter and preparation findings into the opening value; the
//! run retains its phase, message offer and transcript ownership.

use skein_lib::Duration;
use skein_lib::bytes::copy_of;

use crate::boundary::Opening;
use crate::budget::Spend;
use crate::charter::{Charter, Families};
use crate::prepare::Found;
use crate::prompt;
use crate::workspace::{self, Workspace};

/// The opening of a run's main conversation, given what the run found in its
/// checkout, what it has spent and the time it has left. What it holds is
/// rendered from the charter or copied: the run keeps the charter (copy at
/// emission).
pub(crate) fn main(
    charter: &Charter,
    activation: u64,
    identity: crate::Identity,
    workspace: Option<&Workspace>,
    found: &Found,
    spent: Spend,
    left: Duration,
) -> Opening {
    Opening {
        final_turn: None,
        identity,
        ordinal: 0,
        activation,
        transcript: None,
        wait: charter.grants.wait,
        host_tools: charter.grants.host_tools.clone(),
        deliver: charter.grants.deliver.is_some(),
        llm: charter.llm.clone(),
        system: prompt::system(charter, workspace, found, charter.budget.remainder(spent, left)),
        prompt: copy_of(prompt::BEGIN),
        tools: workspace::families(workspace, Families::of(&charter.grants)).tools,
        workspace: workspace.cloned(),
        budget: charter.budget.remainder(spent, left),
        finish: true,
        families: workspace::families(workspace, Families::of(&charter.grants)),
    }
}
