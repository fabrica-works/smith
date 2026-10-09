//! Four inspect-only children settle their real delegated turns before main finishes.
//! Contract: domain/run.md, sections 4, 5.3 and 13; protocol/limits.md, section 6.

use skein_fake_checkout::Checkout;
use skein_fake_llm_domain::api::{Finish, Line, Part, Script, Turn};
use skein_lib::Token;
use skein_world::domain::Span;
use smith_agent_world::{Boundary, CompletionTerminal, Job, Settings, World, scripted_charter};
use smith_domain::{run, tools};

fn calls(lines: Vec<Line>) -> Turn {
    Turn { lines: lines.into(), finish: Finish::ToolCalls, tokens: 1 }
}

#[test]
fn four_inspect_only_sub_agents_in_parallel_are_each_answered() {
    let settings =
        Settings { job: Job::Reporting, writable: false, network: Span::millis(1, 1), ..Settings::calm(981) };
    let mut charter = scripted_charter(&settings);
    charter.instructions = b"@pool-parent".as_slice().into();
    let mut scripts = Vec::new();
    let mut children = Vec::new();
    for index in 0..4 {
        let cue = format!("@pool-child-{index}");
        children.push(Line::Call {
            name: b"sub_agent".as_slice().into(),
            arguments: format!(r#"{{"brief":"{cue}","tools":["inspect"]}}"#).into_bytes().into(),
        });
        scripts.push(Script {
            cue: cue.into_bytes().into(),
            turns: Box::new([
                calls(vec![Line::Call {
                    name: b"read".as_slice().into(),
                    arguments: br#"{"path":"README.md"}"#.as_slice().into(),
                }]),
                Turn {
                    lines: Box::new([Line::Text { text: format!("child {index} inspected").into_bytes().into() }]),
                    finish: Finish::Stop,
                    tokens: 1,
                },
            ]),
        });
    }
    scripts.push(Script {
        cue: b"@pool-parent".as_slice().into(),
        turns: Box::new([
            calls(children),
            calls(vec![Line::Call {
                name: b"finish".as_slice().into(),
                arguments: br#"{"report":"Four inspections returned.","source":"workspace"}"#.as_slice().into(),
            }]),
        ]),
    });
    let mut disk = Checkout::new();
    disk.mkdir(b"work");
    disk.write(b"work/README.md", b"Inspect this exact file.");
    let workspace = run::Workspace {
        directories: Box::new([run::Directory {
            host: Box::from(b"/host".as_slice()),
            name: b"work".as_slice().into(),
            root: Token::new(disk.root(b"work")),
            writable: false,
            git: false,
            conflicts: Box::new([]),
        }]),
    };
    let mut world =
        World::with_workspace_scripts_charter(settings, None, Some(workspace), disk, scripts.into(), charter);
    world.run(10_000);
    assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
    let parent = world.prompts().last().expect("actual final main prompt");
    for index in 0..4 {
        let answer = format!("child {index} inspected");
        assert!(parent.messages.iter().flat_map(|message| &message.parts).any(|part| {
            matches!(part, Part::ToolOutput { output, is_error: false, .. } if output.starts_with(answer.as_bytes()) && output.windows(b"\n\nChild usage: used 2 turns".len()).any(|bytes| bytes == b"\n\nChild usage: used 2 turns"))
        }));
    }
    assert_eq!(
        world
            .boundaries()
            .iter()
            .filter(|(_, event)| matches!(event, Boundary::Io { op: tools::Op::Load { .. } }))
            .count(),
        4
    );
    assert_eq!(world.completions().len(), 10, "two main turns and two actual turns per child");
    assert!(
        world
            .completions()
            .iter()
            .all(|completion| matches!(completion.terminal, Some((_, CompletionTerminal::Completed(_)))))
    );
    let first_children = &world.completions()[1..5];
    let earliest_end = first_children
        .iter()
        .map(|completion| completion.terminal.expect("settled child").0)
        .min()
        .expect("four actual children");
    assert!(
        first_children.iter().all(|completion| completion.started < earliest_end),
        "all four child calls started before any of those calls completed"
    );
}
