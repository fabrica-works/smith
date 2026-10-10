//! Resolved failure feedback through the real session and its provider query.
//! Contract: domain/tools.md, section 2; protocol/llm.md, section 5.

use skein_fake_llm_domain::api::Part as ProviderPart;
use skein_lib::Token;
use smith_domain_session::{Event, llm};
use smith_domain_tools::{Authority, Call, Done, Grants, Name, Part, Path, Repo};
use smith_session_world::{recorded, tickets::Tickets, translate};

#[test]
fn a_resolved_missing_path_reaches_the_llm_in_the_fixed_conversation_namespace() {
    for several in [false, true] {
        let mut opening = recorded::opening(None, u64::MAX);
        opening.spec.delegated = Box::new([]);
        let name = Name::new(b"work".as_slice().into()).expect("working name");
        let mut repos = vec![Repo {
            host: Box::new([smith_domain_tools::Name::new(Box::from(b"host".as_slice())).expect("fixture host name")]),
            mount: Box::new([name.clone()]),
            root: Token::new(10),
            writable: true,
        }];
        if several {
            repos.push(Repo {
                host: Box::new([
                    smith_domain_tools::Name::new(Box::from(b"host".as_slice())).expect("fixture host name")
                ]),
                mount: Box::new([Name::new(b"docs".as_slice().into()).expect("other name")]),
                root: Token::new(11),
                writable: false,
            });
        }
        opening.spec.authority = Authority {
            cwd: Box::new([name]),
            repos: repos.into(),
            grants: Grants { inspect: true, modify: false, shell: false },
        };
        let mut world = recorded::World::new(9105, 256);
        // A failure path exceeds this read window, so its own reservation
        // must keep the whole descriptor rather than price it as one byte.
        world.env.limits.tools.read_bytes = 1;
        world.env.limits.tools.repos = 2;
        world.domain = smith_domain_session::Domain::new(&world.env.limits, 9105);
        world.open(opening);
        assert!(world.completing.is_some(), "provider query admitted: {:?}", world.end);
        world.complete(
            Box::new([llm::Block::ToolCall {
                id: b"missing".as_slice().into(),
                name: b"read".as_slice().into(),
                input: br#"{"path":"work/src/missing.rs"}"#.as_slice().into(),
                call: llm::Decoded::Owned {
                    call: Call::Read {
                        path: Path {
                            absolute: false,
                            parts: [b"work".as_slice(), b"src", b"missing.rs"]
                                .into_iter()
                                .map(|bytes| Part::Name { name: Name::new(bytes.into()).expect("name") })
                                .collect(),
                        },
                        offset: 1,
                        window: smith_domain_tools::Budget::Default,
                    },
                },
                replay: None,
            }]),
            llm::Stop::ToolUse,
            llm::Usage::ZERO,
        );
        let owner = world.operations[0].0;
        world.step(Event::Done { owner, done: Done::Missing });
        assert!(world.completing.is_some(), "failure starts the next provider query: {:?}", world.end);
        let prompt = world.prompts.last().expect("the tools' terminal starts the next query").clone();
        let query = translate::query(prompt, &Tickets::default());
        let outputs: Vec<_> = query
            .messages
            .iter()
            .flat_map(|message| &message.parts)
            .filter_map(|part| match part {
                ProviderPart::ToolOutput { output, is_error: true, .. } => Some(output.as_ref()),
                ProviderPart::Opaque { .. }
                | ProviderPart::Text { .. }
                | ProviderPart::ToolCall { .. }
                | ProviderPart::ToolOutput { .. } => None,
            })
            .collect();
        let expected: &[u8] =
            if several { b"path not found: /work/work/src/missing.rs" } else { b"path not found: work/src/missing.rs" };
        assert_eq!(outputs, [expected]);
        world.complete(Box::new([]), llm::Stop::EndTurn, llm::Usage::ZERO);
        world.close();
    }
}
