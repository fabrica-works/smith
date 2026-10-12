//! Local policy consumes each typed parent variant; process ownership is
//! exercised separately by the spawned lifecycle unit and composed worlds.
use super::{AgentLife, Domain, Phase, parent_request};
use crate::{Config, Contract, Event, Limits, Request};
use alloc::boxed::Box;
use skein_lib::{Duration, Env, Queue, Time, Token, Wall};
use smith_domain::{run, session};
use smith_host_domain::{self as host, parent, process};

fn running(config: Config) -> (Domain, Env<Limits>, Queue<Request>) {
    let mut limits = crate::tests::limits();
    limits.agent = crate::AgentLimits::Spawned(crate::tests::host_limits());
    let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits: limits.clone() };
    let mut domain = Domain::new(config, &limits, 71).expect("bounded local parent");
    domain.chat.phase = Phase::Running;
    domain.run = Some(Token::new(1));
    let out = Queue::with_capacity(crate::max_out(&limits));
    (domain, env, out)
}

#[test]
fn started_binds_the_handle_and_admission_keeps_an_empty_person_queue() {
    let (mut domain, env, mut out) = running(crate::tests::config());
    parent_request(
        &mut domain,
        &env,
        parent::Request::Started { client: Token::new(1), agent: Token::new(9) },
        &mut out,
    );
    parent_request(&mut domain, &env, parent::Request::Admitted { client: Token::new(1) }, &mut out);
    assert_eq!(domain.run, Some(Token::new(9)));
    assert!(out.is_empty());
}

#[test]
fn the_first_drop_cause_survives_store_failure_and_the_childs_later_fault() {
    for store_first in [false, true] {
        let (mut domain, env, mut out) = running(crate::tests::config());
        let failed_store = Event::StoreFailed { turn: Some(2), reason: crate::StoreFailure::Write };
        if store_first {
            crate::step(&mut domain, &env, failed_store, &mut out);
            crate::step(&mut domain, &env, Event::Drop, &mut out);
        } else {
            crate::step(&mut domain, &env, Event::Drop, &mut out);
            crate::step(&mut domain, &env, failed_store, &mut out);
        }
        let cause = if store_first {
            crate::EndCause::Host { turn: Some(2), reason: crate::StoreFailure::Write }
        } else {
            crate::EndCause::Cancelled
        };
        assert_eq!(domain.end_cause(), Some(cause));
        parent_request(
            &mut domain,
            &env,
            parent::Request::Faulted { client: Token::new(1), fault: host::Fault::Exited },
            &mut out,
        );
        parent_request(
            &mut domain,
            &env,
            parent::Request::Gone { client: Token::new(1), end: host::End::Stopped, detail: Box::default() },
            &mut out,
        );
        assert_eq!(domain.end_cause(), Some(cause), "later child terminals cannot replace the cause");
        let mut status = None;
        for _ in 0..crate::max_out(&env.limits) {
            let Some(request) = out.pop() else { break };
            if let Request::Exit { status: value } = request {
                status = Some(value);
            }
        }
        assert_eq!(status, Some(if store_first { crate::ExitStatus::Failed } else { crate::ExitStatus::Interrupted }));
    }
}

#[test]
fn waiting_and_both_long_notices_reach_the_person() {
    let notices = [
        (parent::Request::Waiting { client: Token::new(1), read: None }, b"Waiting for a message".as_slice()),
        (
            parent::Request::Long {
                client: Token::new(1),
                operation: host::Operation::Checks,
                span: Duration::from_secs(9),
            },
            b"Running checks".as_slice(),
        ),
        (parent::Request::LongDone { client: Token::new(1) }, b"Checks finished".as_slice()),
    ];
    let (mut domain, env, mut out) = running(crate::tests::config());
    for (notice, expected) in notices {
        parent_request(&mut domain, &env, notice, &mut out);
        let Some(Request::Show { text }) = out.pop() else { panic!("one person notice") };
        assert_eq!(text.as_ref(), expected);
        assert!(out.is_empty());
    }
}

#[test]
fn completion_and_command_spans_do_not_claim_checks_are_running() {
    let (mut domain, env, mut out) = running(crate::tests::config());
    for operation in [host::Operation::Completion, host::Operation::Command] {
        parent_request(
            &mut domain,
            &env,
            parent::Request::Long { client: Token::new(1), operation, span: Duration::from_secs(9) },
            &mut out,
        );
    }
    assert!(out.is_empty());
    for _ in 0_u32..2 {
        parent_request(&mut domain, &env, parent::Request::LongDone { client: Token::new(1) }, &mut out);
        assert!(out.is_empty());
    }
}

#[test]
fn rejection_keeps_the_account_and_exhaustion_keeps_its_notice() {
    let (mut domain, env, mut out) = running(crate::tests::config());
    parent_request(
        &mut domain,
        &env,
        parent::Request::Rejected { client: Token::new(1), account: 7, generation: 4 },
        &mut out,
    );
    let Some(Request::Credential { account: 7 }) = out.pop() else { panic!("refresh the named account") };
    parent_request(
        &mut domain,
        &env,
        parent::Request::Exhausted { client: Token::new(1), account: 7, retry_after: Duration::from_secs(2) },
        &mut out,
    );
    let Some(Request::Show { text }) = out.pop() else { panic!("quota notice") };
    assert_eq!(text.as_ref(), b"A model account is exhausted");
}

#[test]
fn told_and_withdrawn_do_not_abandon_a_durable_delivery() {
    let (mut domain, env, mut out) = running(crate::tests::config());
    domain.delivery_owner = Some(Token::new(55));
    parent_request(
        &mut domain,
        &env,
        parent::Request::Told { client: Token::new(1), body: b"fact".as_slice().into() },
        &mut out,
    );
    parent_request(
        &mut domain,
        &env,
        parent::Request::Withdrawn { client: Token::new(1), call: Token::new(55) },
        &mut out,
    );
    assert_eq!(domain.delivery_owner, Some(Token::new(55)));
    assert!(out.is_empty());
}

#[test]
fn a_parent_turn_moves_its_concrete_value_to_the_durable_store() {
    let (mut domain, env, mut out) = running(crate::tests::config());
    let turn = smith_domain::Turn {
        kind: session::record::Kind::First { affinity: smith_domain::llm::Affinity { key: [0; 16], thread: 0 } },
        version: session::record::VERSION,
        endpoint: session::llm::Endpoint(4),
        dialect: 1,
        sequence: 1,
        usage: session::llm::Usage::NONE,
        spent: 29,
        messages: Box::default(),
    };
    let expected = turn.clone();
    parent_request(
        &mut domain,
        &env,
        parent::Request::Turn {
            client: Token::new(1),
            turn: host::Turn {
                opens_window: true,
                number: 1,
                read: None,
                spent: 29,
                body: host::TurnValue::new(turn, crate::tests::host_limits().turn_bytes).expect("bounded native turn"),
            },
        },
        &mut out,
    );
    let Some(Request::SaveTurn { turn, number: 1, .. }) = out.pop() else { panic!("durable native turn") };
    assert_eq!(turn, expected);
}

#[test]
fn the_parent_answer_keeps_its_host_spend_until_gone() {
    let (mut domain, env, mut out) = running(crate::tests::config());
    parent_request(
        &mut domain,
        &env,
        parent::Request::Answered {
            client: Token::new(1),
            answer: host::Answer { read: None, turns: 3, spent: 29, result: host::RunResult::Parked },
        },
        &mut out,
    );
    assert!(out.is_empty());
    let Some(answer) = &domain.turns.answer else { panic!("typed host answer retained") };
    assert_eq!(answer.spent, 29);
    assert_eq!(answer.turns, 3);
    parent_request(
        &mut domain,
        &env,
        parent::Request::Gone { client: Token::new(1), end: host::End::Stopped, detail: Box::default() },
        &mut out,
    );
    let Some(Request::Show { text }) = out.pop() else { panic!("answer shown after Gone") };
    assert_eq!(text.as_ref(), b"Chat parked");
    assert_eq!(domain.agent_life, AgentLife::Gone);
}

#[test]
fn an_agent_fault_keeps_the_current_failure_class() {
    let (mut domain, env, mut out) = running(crate::tests::config());
    parent_request(
        &mut domain,
        &env,
        parent::Request::Faulted { client: Token::new(1), fault: host::Fault::Rules },
        &mut out,
    );
    assert!(domain.agent_failed);
    let Some(Request::Show { text }) = out.pop() else { panic!("failure notice") };
    assert_eq!(text.as_ref(), b"Agent process failed");
}

#[test]
fn every_message_refusal_ends_the_message_without_failing_the_agent() {
    for reason in [
        host::MessageRefusal::Full,
        host::MessageRefusal::TooLarge,
        host::MessageRefusal::NameInUse,
        host::MessageRefusal::Ending,
    ] {
        let (mut domain, env, mut out) = running(crate::tests::config());
        parent_request(
            &mut domain,
            &env,
            parent::Request::MessageRefused { client: Token::new(1), name: Token::new(3), reason },
            &mut out,
        );
        assert!(!domain.agent_failed);
        assert_eq!(domain.chat.phase, Phase::Running);
        let Some(Request::Show { text }) = out.pop() else { panic!("message terminal notice") };
        assert_eq!(text.as_ref(), b"Message not delivered");
        assert!(out.is_empty());
    }
}

#[test]
fn unread_names_reach_the_parent_without_an_agent_failure_or_another_activation() {
    let (mut domain, env, mut out) = running(crate::tests::config());
    parent_request(
        &mut domain,
        &env,
        parent::Request::Unread { client: Token::new(1), names: Box::from([Token::new(3)]) },
        &mut out,
    );
    assert!(!domain.agent_failed);
    assert_eq!(domain.chat.phase, Phase::Running);
    assert!(out.is_empty());
}

#[test]
fn a_message_refused_after_the_last_word_keeps_the_single_transcript_reload() {
    let (mut domain, env, mut out) = running(crate::tests::config());
    parent_request(
        &mut domain,
        &env,
        parent::Request::Answered {
            client: Token::new(1),
            answer: host::Answer {
                read: None,
                turns: 0,
                spent: 0,
                result: host::RunResult::Failed {
                    failure: host::RunFailure::Transcript(host::TranscriptRefusal::Version),
                },
            },
        },
        &mut out,
    );
    parent_request(
        &mut domain,
        &env,
        parent::Request::Gone { client: Token::new(1), end: host::End::Stopped, detail: Box::default() },
        &mut out,
    );
    parent_request(
        &mut domain,
        &env,
        parent::Request::MessageRefused {
            client: Token::new(1),
            name: Token::new(3),
            reason: host::MessageRefusal::Ending,
        },
        &mut out,
    );
    let Some(Request::Show { text }) = out.pop() else { panic!("actual transcript refusal") };
    assert_eq!(text.as_ref(), b"Saved transcript was refused");
    let Some(Request::Load) = out.pop() else { panic!("one transcript reload") };
    let Some(Request::Show { text }) = out.pop() else { panic!("late message terminal") };
    assert_eq!(text.as_ref(), b"Message not delivered");
    assert!(out.is_empty());
    assert_eq!(domain.chat.phase, Phase::Loading);
    assert!(!domain.agent_failed);
}

#[test]
fn gone_without_an_answer_fails_after_retained_turns_are_saved() {
    let (mut domain, env, mut out) = running(crate::tests::config());
    parent_request(
        &mut domain,
        &env,
        parent::Request::Gone {
            client: Token::new(1),
            end: host::End::Unspawned,
            detail: b"unavailable".as_slice().into(),
        },
        &mut out,
    );
    let Some(Request::Show { text }) = out.pop() else { panic!("missing answer notice") };
    assert_eq!(text.as_ref(), b"Agent process ended without an answer");
    let Some(Request::Exit { status: crate::ExitStatus::Failed }) = out.pop() else { panic!("failed invocation") };
}

#[test]
fn a_delivery_call_keeps_its_fields_owner_and_deadline_in_local_policy() {
    let mut config = crate::tests::config();
    config.reserve.time = crate::tests::limits().smith.run.check_timeout;
    config.budget.time = Duration::from_secs(120);
    config.contract = Contract::Change(run::outcome::ChangeSpec {
        checks_must_pass: false,
        fields: Box::from([run::outcome::FieldRule { name: b"title".as_slice().into(), max: 64 }]),
    });
    config.workspace = Some(run::Workspace {
        directories: Box::from([run::Directory {
            host: Box::from(b"/host".as_slice()),
            name: b"work".as_slice().into(),
            root: Token::new(9),
            writable: true,
            git: false,
            conflicts: Box::default(),
        }]),
    });
    let (mut domain, env, mut out) = running(config);
    let change = run::outcome::Change {
        fields: Box::from([run::outcome::Field {
            name: b"title".as_slice().into(),
            value: b"keep the change".as_slice().into(),
        }]),
    };
    let deadline = Time::from_nanos(99);
    parent_request(
        &mut domain,
        &env,
        parent::Request::Called {
            client: Token::new(1),
            logical_run: Token::new(1),
            call: Token::new(55),
            name: host::CallName { activation: 1, completion: 2, position: 1 },
            deadline,
            ask: host::Ask::Deliver { fields: host::Fields::new(change, 1024).expect("bounded fields") },
        },
        &mut out,
    );
    let Some(Request::PlainStatus { owner, directory, deadline: received }) = out.pop() else {
        panic!("survey typed delivery")
    };
    assert_eq!(owner, Token::new(55));
    assert_eq!(directory, 0);
    assert_eq!(received, deadline);
}

#[test]
fn an_unconfigured_host_call_receives_one_unavailable_parent_answer() {
    let (mut domain, env, mut out, owner) = crate::tests::spawned_chat_with_saved_hello();
    crate::step(
        &mut domain,
        &env,
        Event::Process(process::Event::Received {
            owner,
            message: host::Up::Call {
                call: Token::new(55),
                name: host::CallName { activation: 1, completion: 2, position: 1 },
                deadline: Time::from_nanos(99),
                ask: host::Ask::Host {
                    tool: b"missing".as_slice().into(),
                    effect: host::Effect::Read,
                    body: Box::default(),
                },
            },
        }),
        &mut out,
    );
    let mut replies = 0_u32;
    while let Some(request) = out.pop() {
        if let Request::Process(process::Request::Send {
            message: host::Down::Answer { call, reply: host::Reply::Unavailable },
            ..
        }) = request
        {
            assert_eq!(call, Token::new(55));
            replies += 1;
        }
    }
    assert_eq!(replies, 1);
}
