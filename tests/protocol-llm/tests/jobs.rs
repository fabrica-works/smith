//! Offered jobs cross each native dialect and the real tools domain.
use skein_fake_checkout::{Checkout, Exit, Program};
use skein_fake_llm_domain::api::{Finish, Line, Part, Script, Turn};
use skein_lib::{Duration, Token};
use skein_world::domain::Span;
use smith_domain::{llm, run, tools};
use smith_protocol_llm_world::{Job, Settings, World, adapter, wire};

const COMMAND_MS: u64 = 3_000;
const ITERATIONS: u32 = 10_000;
const STOP: &[u8] = br#"{"job":1,"stop":true}"#;
const READ: &[u8] = br#"{"job":1,"wait_seconds":5}"#;

fn call(name: &[u8], arguments: &[u8]) -> Turn {
    Turn {
        lines: Box::new([Line::Call { name: name.into(), arguments: arguments.into() }]),
        finish: Finish::ToolCalls,
        tokens: 1,
    }
}

fn world(configuration: wire::Configuration, job_input: &[u8]) -> World {
    let (disk, workspace) = checkout();
    let mut settings = Settings { job: Job::Reporting, network: Span::millis(0, 0), ..Settings::calm(1102) };
    let mut client = skein_llm_world::limits();
    client.request = 32768;
    client.http.request = 32768;
    settings.limits.session.completion_bytes =
        adapter::completion_worst_case(&client, settings.limits.decoded_call_bytes).expect("native receiving");
    settings.limits.session.completion_blocks = client.output_items;
    settings.limits.session.failure_bytes = client.detail_bytes;
    let bounds = adapter::Limits {
        client,
        tool_bytes: 32768,
        history_bytes: u64::from(client.request),
        rendered_result: client.strings,
        stated: adapter::Stated::from_tools(&settings.limits.session.tools, client.input).expect("effective bounds"),
    };
    let scripts = Box::new([Script {
        cue: Box::new([]),
        turns: Box::new([
            call(b"shell", br#"{"command":"build","yield_seconds":1}"#),
            call(b"job", job_input),
            call(b"finish", br#"{"report":"Read the job's end.","source":"job"}"#),
        ]),
    }]);
    World::with_workspace_wire(settings, None, Some(workspace), disk, configuration, bounds, scripts)
}

fn checkout() -> (Checkout, run::Workspace) {
    let mut disk = Checkout::new();
    disk.mkdir(b"work");
    let root = Token::new(disk.root(b"work"));
    disk.program(
        b"build",
        Program {
            duration: std::time::Duration::from_millis(COMMAND_MS),
            output: b"abcdef".to_vec(),
            exit: Exit::Code(0),
            changes: Vec::new(),
        },
    );
    let workspace = run::Workspace {
        directories: Box::new([run::Directory {
            host: b"/host".as_slice().into(),
            name: b"work".as_slice().into(),
            root,
            writable: true,
            git: false,
            conflicts: Box::new([]),
        }]),
    };
    (disk, workspace)
}

fn outputs(world: &World, prompt: usize) -> Vec<Vec<u8>> {
    world.prompts()[prompt]
        .messages
        .iter()
        .flat_map(|message| &message.parts)
        .filter_map(|part| {
            if let Part::ToolOutput { output, .. } = part {
                Some(output.strip_prefix(b"Error: ").unwrap_or(output).to_vec())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn a_shell_call_with_a_yield_is_answered_running_then_read_with_job_in_each_dialect() {
    for configuration in wire::configurations() {
        let mut world = world(configuration, READ);
        world.run(ITERATIONS);
        assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
        assert_eq!(outputs(&world, 1), [b"job 1 running; use job to read the rest\nab".to_vec()]);
        assert_eq!(
            outputs(&world, 2),
            [
                b"job 1 running; use job to read the rest\nab".to_vec(),
                b"job 1 output since its last answer:\ncdef\nexit code 0".to_vec(),
            ]
        );
    }
}

#[test]
fn a_job_call_with_stop_is_rendered_stopped() {
    for configuration in wire::configurations() {
        let mut world = world(configuration, STOP);
        world.run(ITERATIONS);
        assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
        assert_eq!(
            outputs(&world, 2),
            [
                b"job 1 running; use job to read the rest\nab".to_vec(),
                b"job 1 output since its last answer:\nstopped".to_vec(),
            ]
        );
    }
}

fn decoded(name: &[u8], input: &[u8]) -> llm::Decoded {
    let client = skein_llm_world::limits();
    adapter::decode(
        name,
        input,
        tools::Grants { inspect: false, modify: false, shell: true },
        &[],
        llm::Asker::Main,
        &adapter::Limits {
            client,
            tool_bytes: 32768,
            history_bytes: u64::from(client.request),
            rendered_result: client.strings,
            stated: adapter::stated(),
        },
    )
}

#[test]
fn a_job_without_optional_fields_uses_the_effective_defaults() {
    assert_eq!(
        decoded(b"job", br#"{"job":7}"#),
        llm::Decoded::Owned {
            call: tools::Call::Job {
                job: tools::JobNumber(7),
                wait: tools::Wait::Default,
                output: tools::Budget::Default,
                stop: false,
            }
        }
    );
}

#[test]
fn a_job_decodes_asked_wait_output_and_stop() {
    assert_eq!(
        decoded(b"job", br#"{"job":7,"wait_seconds":9,"output_bytes":23,"stop":true}"#),
        llm::Decoded::Owned {
            call: tools::Call::Job {
                job: tools::JobNumber(7),
                wait: tools::Wait::Asked(Duration::from_secs(9)),
                output: tools::Budget::Asked(23),
                stop: true,
            }
        }
    );
}

#[test]
fn a_job_can_be_polled_without_waiting() {
    assert_eq!(
        decoded(b"job", br#"{"job":7,"wait_seconds":0}"#),
        llm::Decoded::Owned {
            call: tools::Call::Job {
                job: tools::JobNumber(7),
                wait: tools::Wait::Asked(Duration::ZERO),
                output: tools::Budget::Default,
                stop: false,
            }
        }
    );
}

#[test]
fn a_shell_yield_can_answer_without_waiting() {
    assert_eq!(
        decoded(b"shell", br#"{"command":"build","yield_seconds":0}"#),
        llm::Decoded::Owned {
            call: tools::Call::Shell {
                after: tools::Yield::Asked(Duration::ZERO),
                command: b"build".as_slice().into(),
                timeout: tools::Timeout::Default,
                output: tools::Budget::Default,
            }
        }
    );
}
