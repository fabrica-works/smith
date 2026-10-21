//! Scalar activation bills through root/run/session/tools and a typed fake provider.
//! The outside oracle prices the actual typed fake-provider callbacks with its own literal
//! model table, and sums each callback once. Child-inclusive Turn bills and
//! historical bills never supply that global expected total.
//! Contract: domain/run.md, sections 9, 10, 13 and 14;
//! domain/session.md, section 6; testing-strategy.md, sections 2.3, 6 and 7.

use skein_fake_checkout::Checkout;
use skein_fake_llm_domain::api::{Finish, Line, Part, Query, Role, Script, Turn};
use skein_lib::{Duration, Token};
use skein_world::domain::Span;
use smith_agent_world::{CompletionObservation, CompletionTerminal, Job, Settings, World};
use smith_domain::{run, session::llm};

const CHILD: &[u8] = b"BUDGET-CHILD: actual child result.";
const CHILD_INPUT: &[u8] = br#"{"brief":"@budget-child own task","tools":["inspect"],"llm":"fake-2"}"#;
const GRAND_INPUT: &[u8] = br#"{"brief":"@budget-grand own task","tools":["inspect"],"llm":"fake-3"}"#;
const REPORT: &[u8] = br#"{"report":"Budget work finished.","source":"actual-checkout"}"#;
const READ: &[u8] = br#"{"path":"data.txt"}"#;

#[derive(Clone, Copy)]
struct Rate {
    model: &'static [u8],
    input: u64,
    cached: u64,
    cache_write: Option<u64>,
    output: u64,
    unit: u32,
}

const MODEL_RATES: [Rate; 3] = [
    Rate { model: b"fake-1", input: 2, cached: 3, cache_write: Some(29), output: 5, unit: 7 },
    Rate { model: b"fake-2", input: 7, cached: 11, cache_write: Some(31), output: 13, unit: 10 },
    Rate { model: b"fake-3", input: 17, cached: 19, cache_write: None, output: 23, unit: 29 },
];
fn charge(usage: llm::Usage, rate: Rate) -> u64 {
    // One combined numerator is rounded once. This oracle uses no domain
    // price method, accumulated spend, child bill or saved Turn value.
    let numerator = u128::from(usage.input_tokens.expect("scripted peer supplies count")) * u128::from(rate.input)
        + u128::from(usage.cache_write_tokens.expect("scripted peer supplies count"))
            * u128::from(rate.cache_write.unwrap_or(rate.input))
        + u128::from(usage.cache_read_tokens.expect("scripted peer supplies count")) * u128::from(rate.cached)
        + u128::from(usage.output_tokens.expect("scripted peer supplies count")) * u128::from(rate.output);
    let denominator = u128::from(rate.unit);
    u64::try_from(numerator.div_ceil(denominator)).expect("finite story prices fit u64")
}

fn usage(observation: &CompletionObservation) -> llm::Usage {
    let (at, terminal) = observation.terminal.expect("every actual completion settled");
    assert!(at >= observation.started);
    match terminal {
        CompletionTerminal::Completed(usage) => usage,
        CompletionTerminal::Failed | CompletionTerminal::Cancelled => panic!("this story requires an actual winner"),
    }
}

fn total(observations: &[CompletionObservation], rates: &[Rate]) -> run::Spend {
    let observations: Vec<_> = observations
        .iter()
        .filter(|observation| matches!(observation.terminal, Some((_, CompletionTerminal::Completed(_)))))
        .collect();
    let mut expected = run::Spend {
        input: observations
            .iter()
            .filter_map(|observation| usage(observation).input_tokens)
            .reduce(|total, next| total + next),
        output: observations
            .iter()
            .filter_map(|observation| usage(observation).output_tokens)
            .reduce(|total, next| total + next),
        cache_read: observations
            .iter()
            .filter_map(|observation| usage(observation).cache_read_tokens)
            .reduce(|total, next| total + next),
        cache_write: observations
            .iter()
            .filter_map(|observation| usage(observation).cache_write_tokens)
            .reduce(|total, next| total + next),
        reasoning: observations
            .iter()
            .filter_map(|observation| usage(observation).reasoning_tokens)
            .reduce(|total, next| total + next),
        ..run::Spend::ZERO
    };
    for observation in observations {
        let actual = usage(observation);
        let rate = rates.iter().find(|rate| rate.model == observation.model.as_ref()).expect("outside model table");
        expected.turns += 1;
        expected.units += charge(actual, *rate);
    }
    expected
}

fn spent(answer: &run::Answer) -> run::Spend {
    match answer {
        run::Answer::Accepted { spent, .. } | run::Answer::Parked { spent, .. } | run::Answer::Failed { spent, .. } => {
            *spent
        }
        run::Answer::Refused(reason) => panic!("actual story was refused: {reason:?}"),
    }
}

fn conservation(world: &World, rates: &[Rate]) -> run::Spend {
    let expected = total(world.completions(), rates);
    assert_eq!(spent(world.answer()), expected, "one own-completion charge per outside callback");
    assert_eq!(world.prompts().len(), world.completions().len());
    assert_eq!(world.turn_metadata().last().expect("actual main Turn").2, expected);
    assert!(
        expected.input.unwrap_or(0) + expected.cache_read.unwrap_or(0) + expected.cache_write.unwrap_or(0) > 0
            && expected.output.unwrap_or(0) > 0
    );
    assert!(world.checked().is_empty() && world.pushes().is_empty() && world.host_submissions().is_empty());
    assert!(world.judged().0 > 0 && world.judged().1 == 1);
    expected
}

fn settings(seed: u64, writable: bool, resume: bool) -> Settings {
    let mut settings = Settings {
        job: Job::Reporting,
        writable,
        resume,
        waiting: Duration::from_secs(1),
        network: Span::millis(1, 1),
        ..Settings::calm(seed)
    };
    settings.budget.spend = 1 << 20;
    settings.limits.run.budget.spend = 1 << 20;
    settings.limits.session.spend = 1 << 20;
    settings
}

fn charter(settings: &Settings, cue: &[u8], rates: &[Rate]) -> run::Charter {
    use run::charter::{Brief, Endpoint, Grants, Llm, Section, Tools};
    use run::outcome::{FieldRule, OutcomeSpec, TextSpec};

    let model = |rate: Rate| Llm {
        effort: None,
        prices: run::Prices {
            input: rate.input,
            cached: rate.cached,
            cache_write: rate.cache_write,
            output: rate.output,
            unit: rate.unit,
        },
        account: Some(0),
        endpoint: Endpoint(0),
        model: rate.model.into(),
        window: 65_536,
        output: 4096,
        dialect: 1,
    };
    run::Charter {
        mode: run::Mode::Finite,
        reserve: run::charter::Reserve { turns: 0, time: skein_lib::Duration::ZERO },
        instructions: cue.into(),
        brief: Brief {
            sections: Box::new([Section {
                title: b"Budget task".as_slice().into(),
                text: b"Use the actual workspace.".as_slice().into(),
            }]),
        },
        grants: Grants {
            wait: true,
            deliver: None,
            tools: Tools { inspect: true, modify: settings.writable, shell: false },
            agents: true,
            host_tools: Box::new([]),
        },
        outcome: OutcomeSpec {
            change: None,
            verdicts: Box::new([]),
            report: Some(TextSpec {
                max: 1024,
                fields: Box::new([FieldRule { name: b"source".as_slice().into(), max: 128 }]),
            }),
            failure: None,
        },
        budget: settings.budget,
        llm: model(rates[0]),
        models: rates[1..].iter().copied().map(model).collect(),
        conventions: Some(run::Conventions {
            guide: b"AGENTS.md".as_slice().into(),
            checks: b".temper/pre-pr".as_slice().into(),
        }),
        resume: settings.resume,
        waiting: settings.waiting,
    }
}

fn workspace(writable: bool) -> (Checkout, run::Workspace) {
    let mut disk = Checkout::new();
    disk.mkdir(b"work");
    disk.write(b"work/AGENTS.md", b"BUDGET-GUIDE: preserve exact task results.\n");
    disk.write(b"work/.git/HEAD", b"HOST-GIT-METADATA");
    disk.write(b"work/data.txt", b"before\n");
    let root = Token::new(disk.root(b"work"));
    (
        disk,
        run::Workspace {
            directories: Box::new([run::Directory {
                host: Box::from(b"/host".as_slice()),
                name: b"work".as_slice().into(),
                root,
                writable,
                git: true,
                conflicts: Box::new([]),
            }]),
        },
    )
}

fn call(name: &[u8], arguments: &[u8]) -> Line {
    Line::Call { name: name.into(), arguments: arguments.into() }
}

fn calls(lines: Vec<Line>, tokens: u64) -> Turn {
    Turn { lines: lines.into(), finish: Finish::ToolCalls, tokens }
}

fn says(text: &[u8], tokens: u64) -> Turn {
    Turn { lines: Box::new([Line::Text { text: text.into() }]), finish: Finish::Stop, tokens }
}

fn script(cue: &[u8], turns: Vec<Turn>) -> Script {
    Script { cue: cue.into(), turns: turns.into() }
}

fn typed_world(settings: &Settings, cue: &[u8], rates: &[Rate], scripts: Box<[Script]>) -> World {
    let (disk, workspace) = workspace(settings.writable);
    World::with_workspace_scripts_charter(
        *settings,
        None,
        Some(workspace),
        disk,
        scripts,
        charter(settings, cue, rates),
    )
}

fn query_result(query: &Query, name: &[u8], input: &[u8], output: &[u8]) -> bool {
    query.messages.windows(2).any(|pair| {
        pair[0].role == Role::Assistant
            && pair[1].role == Role::User
            && pair[0].parts.iter().any(|part| {
                let Part::ToolCall { id, name: actual, arguments } = part else { return false };
                actual.as_ref() == name
                    && arguments.as_ref() == input
                    && pair[1].parts.iter().any(|part| {
                        matches!(part, Part::ToolOutput { id: returned, output: text, is_error: false }
                    if id == returned && text.as_ref() == output)
                    })
            })
    })
}

fn model_calls<'a>(world: &'a World, model: &[u8]) -> Vec<&'a CompletionObservation> {
    world.completions().iter().filter(|observation| observation.model.as_ref() == model).collect()
}

fn own_units(calls: &[&CompletionObservation], rate: Rate) -> u64 {
    calls.iter().map(|actual| charge(usage(actual), rate)).sum()
}

fn turn_usage(actual: llm::Usage) -> llm::Usage {
    actual
}

fn budget_prose(world: &World, settings: &Settings, child_bill: u64) {
    let main_queries = world.prompts().iter().filter(|query| query.model.as_ref() == b"fake-1").collect::<Vec<_>>();
    let child_queries = world.prompts().iter().filter(|query| query.model.as_ref() == b"fake-2").collect::<Vec<_>>();
    let mut admitted_at = None;
    let mut main_open = None;
    let mut child_open = None;
    let mut child_return = None;
    for fact in world.facts() {
        let smith_domain::Fact::Run { fact, opening: _ } = fact else { continue };
        match fact.kind {
            run::facts::FactKind::Admitted { .. } => admitted_at = Some(fact.at),
            run::facts::FactKind::Opened { kind: run::facts::Kind::Main, .. } => main_open = Some(fact.at),
            run::facts::FactKind::Opened { kind: run::facts::Kind::Child { granted, .. }, .. } => {
                child_open = Some((fact.at, granted));
            }
            run::facts::FactKind::Returned { result: run::facts::Return::Answered { used, .. }, .. } => {
                child_return = Some((fact.at, used));
            }
            run::facts::FactKind::MessageRead { .. }
            | run::facts::FactKind::MessageFence { .. }
            | run::facts::FactKind::MessageUnread { .. }
            | run::facts::FactKind::MessageReceived { .. }
            | run::facts::FactKind::MessageRefused { .. }
            | run::facts::FactKind::Prepared { .. }
            | run::facts::FactKind::Budget { .. }
            | run::facts::FactKind::Ended { .. }
            | run::facts::FactKind::Called { .. }
            | run::facts::FactKind::Returned { .. }
            | run::facts::FactKind::CheckStarted { .. }
            | run::facts::FactKind::CheckFinished { .. }
            | run::facts::FactKind::Delivered { .. }
            | run::facts::FactKind::Answered { .. } => {}
        }
    }
    let admitted_at = admitted_at.expect("actual admission clock");
    let main_at = main_open.expect("actual main opening clock");
    let (child_at, grant) = child_open.expect("actual effective child opening");
    let (returned_at, child_used) = child_return.expect("actual child terminal");
    assert_eq!(child_used.turns, 3);
    assert_eq!(child_used.spend, child_bill);
    assert_eq!(child_used.time, returned_at.saturating_since(child_at));
    let main_seconds =
        (settings.budget.time.as_nanos() - main_at.saturating_since(admitted_at).as_nanos()) / 1_000_000_000;
    let main_text = format!(
        "## Budget\n\nAt open, you have 64 turns and {main_seconds} seconds of wall time. Spend is bounded too.\nKeep 0 turns and 0 seconds to finish."
    );
    assert!(main_queries.iter().all(|query| query.system == main_queries[0].system), "Budget stays fixed at open");
    assert!(String::from_utf8_lossy(&main_queries[0].system).contains(&main_text));
    assert_eq!(grant.turns, 63, "one actual main turn preceded delegation");
    let child_left = (settings.budget.time.as_nanos() - child_at.saturating_since(admitted_at).as_nanos())
        .min(settings.limits.session.tools.tool_deadline.as_nanos());
    assert_eq!(grant.time.as_nanos(), child_left, "unasked share is the ordinary pool within the common tool deadline");
    assert_eq!(grant.time_set, run::Set::Deadline);
    let child_seconds = child_left / 1_000_000_000;
    let child_text = format!(
        "## Your share\n\nAt open, your share is 63 turns and {child_seconds} seconds. Your answer may hold at most {} bytes",
        settings.limits.run.answer_bytes
    );
    assert!(child_queries.iter().all(|query| String::from_utf8_lossy(&query.system).contains(&child_text)));
    let used_seconds = child_used.time.as_nanos() / 1_000_000_000;
    assert!(used_seconds > 1, "actual child work advances the injected clock");
    let left_seconds =
        (settings.budget.time.as_nanos() - returned_at.saturating_since(admitted_at).as_nanos()) / 1_000_000_000;
    let suffix = format!(
        "\n\nChild usage: used 3 turns and {used_seconds} seconds; stopped for budget: no; ordinary left 60 turns and {left_seconds} seconds."
    );
    let result = [CHILD, suffix.as_bytes()].concat();
    assert!(query_result(main_queries[1], b"sub_agent", CHILD_INPUT, &result));
}

#[test]
fn two_priced_models_and_a_refused_child_delegate_conserve_global_own_charges() {
    let scripts = Box::new([
        script(
            b"@budget-main",
            vec![
                calls(vec![call(b"sub_agent", CHILD_INPUT)], 3),
                calls(vec![call(b"read", READ)], 5),
                calls(vec![call(b"finish", REPORT)], 7),
            ],
        ),
        script(
            b"@budget-child",
            vec![
                calls(vec![call(b"sub_agent", GRAND_INPUT)], 11),
                calls(vec![call(b"read", READ)], 13),
                says(CHILD, 17),
            ],
        ),
    ]);
    let settings = settings(701, false, false);
    let mut world = typed_world(&settings, b"@budget-main MAIN-INSTRUCTIONS", &MODEL_RATES, scripts);
    world.run(20_000);
    let expected = conservation(&world, &MODEL_RATES);
    assert_eq!(expected.turns, 6);
    assert_eq!(expected.output, Some(56));
    assert!(expected.cache_read.unwrap_or(0) > 0 && expected.cache_write.unwrap_or(0) > 0);
    assert_eq!(world.turns().len(), 3, "only main's settled Turns cross the host boundary");
    let main = model_calls(&world, b"fake-1");
    let child = model_calls(&world, b"fake-2");
    assert!(model_calls(&world, b"fake-3").is_empty());
    assert_eq!((main.len(), child.len()), (3, 3));
    let child_bill = own_units(&child, MODEL_RATES[1]);
    let mut inclusive = child_bill;
    for (index, actual) in main.iter().enumerate() {
        inclusive += charge(usage(actual), MODEL_RATES[0]);
        assert_eq!(world.turns()[index].spent, inclusive, "main includes the child's whole subtree bill once");
        assert_eq!(world.turns()[index].usage, turn_usage(usage(actual)));
    }
    assert_eq!(inclusive, expected.units);
    let child_queries = world.prompts().iter().filter(|query| query.model.as_ref() == b"fake-2").collect::<Vec<_>>();
    budget_prose(&world, &settings, child_bill);
    assert!(
        child_queries[1]
            .messages
            .iter()
            .flat_map(|message| &message.parts)
            .any(|part| { matches!(part, Part::ToolOutput { is_error: true, .. }) })
    );
    assert!(child_queries.iter().all(|query| query.system.starts_with(b"@budget-child own task\n\n")));
    assert!(
        matches!(world.answer(), run::Answer::Accepted { outcome: run::outcome::Declared::Report(report), turns: 3, .. }
        if report.text.as_ref() == b"Budget work finished.")
    );
    let mut duplicated = world.completions().to_vec();
    duplicated.push(duplicated[0].clone());
    assert_ne!(
        total(&duplicated, &MODEL_RATES),
        expected,
        "the whole positive conservation oracle detects double charging"
    );
}

#[test]
fn parallel_children_cannot_spend_the_last_ordinary_turn_twice_and_main_finishes() {
    let settings = settings(707, false, false);
    let (disk, workspace) = workspace(false);
    let mut charter = charter(&settings, b"@reserve-main", &MODEL_RATES);
    charter.budget.turns = 4;
    charter.reserve = run::charter::Reserve { turns: 2, time: Duration::from_secs(300) };
    let scripts = Box::new([
        script(
            b"@reserve-main",
            vec![
                calls(vec![call(b"sub_agent", CHILD_INPUT), call(b"sub_agent", CHILD_INPUT)], 3),
                calls(vec![call(b"finish", REPORT)], 7),
            ],
        ),
        script(b"@budget-child", vec![says(CHILD, 5)]),
    ]);
    let mut world = World::with_workspace_scripts_charter(settings, None, Some(workspace), disk, scripts, charter);
    world.run(20_000);
    let expected = conservation(&world, &MODEL_RATES);
    assert_eq!(expected.turns, 2);
    assert_eq!(model_calls(&world, b"fake-2").len(), 1, "the admitted child attempt is closed before main finishes");
    assert!(matches!(model_calls(&world, b"fake-2")[0].terminal, Some((_, CompletionTerminal::Cancelled))));
    assert_eq!(model_calls(&world, b"fake-1").len(), 2, "main takes a reserved turn to finish");
    assert!(matches!(world.answer(), run::Answer::Accepted { outcome: run::outcome::Declared::Report(_), .. }));
}

#[test]
fn a_real_child_session_answers_its_last_turn_without_running_written_calls() {
    const QUOTA_INPUT: &[u8] =
        br#"{"brief":"@budget-child own task","tools":["inspect"],"llm":"fake-2","max_turns":1,"max_seconds":30}"#;
    let settings = settings(708, false, false);
    let scripts = Box::new([
        script(
            b"@budget-main",
            vec![calls(vec![call(b"sub_agent", QUOTA_INPUT)], 3), calls(vec![call(b"finish", REPORT)], 7)],
        ),
        script(
            b"@budget-child",
            vec![
                calls(vec![Line::Text { text: CHILD.into() }, call(b"read", READ)], 5),
                says(b"This second completion must not start.", 11),
            ],
        ),
    ]);
    let mut world = typed_world(&settings, b"@budget-main MAIN-INSTRUCTIONS", &MODEL_RATES, scripts);
    world.run(20_000);
    let expected = conservation(&world, &MODEL_RATES);
    assert_eq!(expected.turns, 3);
    assert_eq!(model_calls(&world, b"fake-2").len(), 1, "actual child Session stops at its own turn quota");
    assert_eq!(model_calls(&world, b"fake-1").len(), 2, "actual main Session receives the child terminal and finishes");
    let resumed =
        world.prompts().iter().filter(|query| query.model.as_ref() == b"fake-1").nth(1).expect("main resumes");
    assert!(resumed.messages.iter().flat_map(|message| &message.parts).any(|part| {
        matches!(part, Part::ToolOutput { is_error: false, output, .. }
            if output.starts_with(CHILD) && output.windows(b"stopped for budget: yes".len()).any(|text| text == b"stopped for budget: yes"))
    }), "the actual asker receives the answer and budget stop");
    let child_query =
        world.prompts().iter().find(|query| query.model.as_ref() == b"fake-2").expect("actual child request");
    assert_eq!(child_query.choice, skein_fake_llm_domain::api::ToolChoice::None, "actual native child choice");
    assert!(child_query.messages.last().expect("one final note").parts.iter().any(|part| {
        matches!(part, Part::Text { text } if text.as_ref() == b"Your share is ending. This is your final turn. Answer now with what you have. No tool calls will run.")
    }));
    assert!(world.facts().iter().any(|fact| {
        matches!(fact, smith_domain::Fact::Session { fact }
            if matches!(fact.kind, smith_domain::session::FactKind::CompletionStarted { choice: smith_domain::session::Choice::None, .. }))
    }));
    assert!(matches!(world.answer(), run::Answer::Accepted { outcome: run::outcome::Declared::Report(_), .. }));
}

#[test]
fn main_final_turns_keep_definitions_append_one_note_and_skip_outside_workspace_calls() {
    let mut settings = settings(711, true, false);
    settings.budget.turns = 4;
    settings.limits.run.budget.turns = 4;
    let (disk, workspace) = workspace(true);
    let root = workspace.directories[0].root;
    let mut charter = charter(&settings, b"@final-main", &MODEL_RATES);
    charter.reserve = run::charter::Reserve { turns: 2, time: Duration::from_secs(300) };
    let scripts = Box::new([script(
        b"@final-main",
        vec![
            calls(vec![call(b"read", READ)], 3),
            calls(vec![call(b"read", READ)], 3),
            calls(
                vec![call(b"edit", br#"{"path":"data.txt","old":"before","new":"AFTER"}"#), call(b"finish", b"{}")],
                3,
            ),
            calls(vec![call(b"finish", REPORT)], 7),
        ],
    )]);
    let mut world = World::with_workspace_scripts_charter(settings, None, Some(workspace), disk, scripts, charter);
    world.run(20_000);
    assert!(matches!(world.answer(), run::Answer::Accepted { outcome: run::outcome::Declared::Report(_), .. }));
    assert_eq!(world.completions().len(), 4, "two ordinary and two actual final completions");
    assert_eq!(world.disk().load(root.raw(), b"data.txt", u64::MAX).expect("actual disk").0, b"before\n");
    let queries = world.prompts();
    for query in queries {
        assert_eq!(query.tools, queries[0].tools, "offered definitions stay fixed");
    }
    let final_query = queries.last().expect("finish request");
    assert!(final_query.messages.iter().flat_map(|message| &message.parts).any(|part| {
        matches!(part, Part::ToolOutput { output, .. } if output.windows(b"OutsideChoice".len()).any(|part| part == b"OutsideChoice"))
    }));
    assert!(final_query.messages.iter().flat_map(|message| &message.parts).any(|part| {
        matches!(part, Part::ToolOutput { output, .. } if output.windows(b"UnknownTool".len()).any(|part| part == b"UnknownTool"))
    }), "an allowed malformed finish preserves its actual argument problem");
    let note = |query: &Query| {
        query.messages.iter().flat_map(|message| &message.parts).filter(|part| {
        matches!(part, Part::Text { text } if text.windows(b"budget is spent".len()).any(|part| part == b"budget is spent"))
    }).count()
    };
    assert_eq!(note(&queries[0]), 0);
    assert_eq!(note(&queries[1]), 0);
    assert_eq!(note(&queries[2]), 1);
    assert_eq!(note(&queries[3]), 1, "the note is appended once and retained after the concrete not-run result");
    let choices: Vec<_> = world
        .facts()
        .iter()
        .filter_map(|fact| {
            let smith_domain::Fact::Session { fact } = fact else { return None };
            let smith_domain::session::FactKind::CompletionStarted { choice, .. } = fact.kind else {
                return None;
            };
            Some(choice)
        })
        .collect();
    assert!(choices.contains(&smith_domain::session::Choice::Only));
}
