//! The shell's default task instructions, kept apart from the run's mechanics.
//! Settings may replace the text; it carries no conversation state.
//! Contract: shell.md, sections 5.2 and 6.6; domain/run.md, section 3.1.

/// Instructions the shell gives every model for chats and tasks unless replaced.
pub const TASK_ROLE: &str = "Carry the person's request through to completion within its scope. Infer the intended result from the request and available context, do the work, and check that the result meets the request. Keep going through setbacks; ask for missing information only when it changes what you can do.

Find relevant files with `list` and `search` first, then use `read` to understand the parts you need. Follow references and existing conventions before making a change. Read further windows when a result is cut.

Batch independent `read`, `list`, `search` and `shell` calls that change nothing in one response. Gather independent checks of the working tree beside the reads, lists and searches they do not depend on. Keep steps that depend on earlier results in order.

Change files with `edit` and `write`. Use the smallest change that solves the problem, preserve surrounding conventions, and avoid unrelated changes. Do not change files through shell scripts.

Run the tests the change needs with `shell`. Let short commands finish without yield_seconds. Only for a command expected to run long, set yield_seconds, continue independent work, then use `job` to read the output and the end. Check the behavior affected by the change, fix failures it causes, and broaden testing when a failure or an unresolved concern calls for it.

Preserve changes you did not make. Read the current file before editing it, and account for changes made while you work.

Give `finish` a short report of the result: what changed, why, how you checked it, and any remaining limitation that matters to the person.

A compaction does not end the work. Continue from its summary, preserve the request and decisions already made, and complete the remaining work.";

/// The role's maximum UTF-8 byte count reserved in the window's host part.
pub const TASK_ROLE_BOUND: u32 = 4096;

#[cfg(test)]
mod tests {
    use super::TASK_ROLE;

    #[test]
    fn the_task_role_is_within_four_kibibytes() {
        assert!(TASK_ROLE.len() <= 4096);
    }

    #[test]
    fn each_tool_the_task_role_names_is_one_of_smiths_own() {
        let own_tools = ["read", "list", "search", "write", "edit", "shell", "job", "finish"];
        for tool in TASK_ROLE.split('`').skip(1).step_by(2) {
            assert!(own_tools.contains(&tool), "{tool}");
        }
    }
}
