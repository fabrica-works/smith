//! Actual provider calls refuse nesting through main and child sessions.
//! Contract: protocol/llm.md, section 4; domain/run.md, sections 5.3 and 13.

use skein_fake_llm_domain::api::{Finish, Line, Part, Turn};
use smith_domain::{run, session::llm};
use smith_protocol_llm_world::{Job, Settings, World, adapter, script, wire};

#[test]
fn nesting_asked_for_is_answered_as_its_problem_on_actual_provider_requests() {
    for configuration in wire::configurations() {
        let error_flag = configuration.error_flag;
        let mut scripts = script::all().into_vec();
        let main = scripts.iter_mut().find(|script| script.cue.as_ref() == b"@delegate").expect("main script");
        let mut turns = vec![Turn {
            lines: Box::new([Line::Call {
                name: b"sub_agent".as_slice().into(),
                arguments:
                    br#"{"brief":"@explore Never opened.","tools":["inspect"],"agents":false}"#.as_slice().into(),
            }]),
            finish: Finish::ToolCalls,
            tokens: 4,
        }];
        turns.extend(std::mem::take(&mut main.turns).into_vec());
        main.turns = turns.into_boxed_slice();
        let mut bounds = adapter::Limits {
            client: skein_llm_world::limits(),
            tool_bytes: 32768,
            rendered_result: skein_llm_world::limits().dialect.string_bytes,
            shell_default: skein_lib::Duration::from_secs(120),
            shell_maximum: skein_lib::Duration::from_secs(1200),
        };
        bounds.client.http.request = 16384;
        bounds.client.dialect.request_bytes = 16384;
        let mut settings = Settings { job: Job::Delegating, ..Settings::calm(704) };
        settings.limits.session.completion_bytes =
            adapter::completion_worst_case(&bounds.client, settings.limits.decoded_call_bytes)
                .expect("receiving bound");
        settings.limits.session.completion_blocks = bounds.client.dialect.parts;
        let mut world = World::with_wire(settings, None, configuration, bounds, scripts.into_boxed_slice());
        world.run(100_000);
        assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
        let note = b"a sub-agent does its work itself";
        let feedback = world
            .prompts()
            .iter()
            .flat_map(|query| &query.messages)
            .flat_map(|message| &message.parts)
            .filter(|part| {
                matches!(part, Part::ToolOutput { output, is_error, .. }
                if *is_error == error_flag && output.windows(note.len()).any(|bytes| bytes == note))
            })
            .count();
        assert!(feedback >= 2, "main's agents field and child's unoffered call each receive the nesting explanation");
        assert_eq!(
            world.facts().iter().filter(|fact| matches!(fact, smith_domain::Fact::Run { fact } if matches!(fact.kind, run::facts::FactKind::Opened { .. }))).count(),
            4,
            "only main and the three legitimate children open"
        );
        assert!(
            world.turns().iter().flat_map(|turn| &turn.messages).flat_map(|message| &message.content).any(
                |block| matches!(
                    block,
                    llm::Block::ToolResult { result: llm::Returned::Invalid { problem: llm::Problem::Nesting }, .. }
                )
            ),
            "actual persisted main result retains the typed problem"
        );
        let child =
            world.prompts().iter().filter(|query| query.system.windows(b"@fix".len()).any(|bytes| bytes == b"@fix"));
        assert!(child.clone().count() > 0);
        assert!(child.clone().all(|query| !query.tools.iter().any(|tool| tool.name.as_ref() == b"sub_agent")));
        assert!(
            child
                .flat_map(|query| &query.messages)
                .flat_map(|message| &message.parts)
                .any(|part| matches!(part, Part::ToolOutput { output, is_error, .. }
                if *is_error == error_flag && output.windows(note.len()).any(|bytes| bytes == note)))
        );
    }
}
