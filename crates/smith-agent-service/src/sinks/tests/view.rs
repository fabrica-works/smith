use super::{Capture, DEADLINE, Delivery, Destination, Sinks, drain, golden, limits};
use crate::sinks::View;
use skein_lib::{List, Token};
use smith_events::{Event, Record};

#[global_allocator]
static HEAP: skein_heap::Counting = skein_heap::Counting;

fn model() -> smith_events::Model {
    smith_events::Model {
        account: None,
        backend: smith_events::Backend::Direct,
        endpoint: vec![b'e'; 64].into(),
        model: vec![b'm'; 64].into(),
        effort: Some(vec![b'l'; 64].into()),
        window_tokens: 1024,
        output_tokens: 128,
    }
}

fn request() -> Record {
    Record {
        t_ms: 1,
        event: Event::ResponseStarted(smith_events::ResponseStarted {
            run: 1,
            conversation: 2,
            response: 3,
            turn: 1,
            attempt: 0,
            window: 1,
            model: model(),
            messages: 0,
            output_tokens: 128,
            tool_choice: smith_events::ToolChoice::None,
            prompt: Some(smith_events::Prompt {
                system: Some(vec![b'p'; 65_536].into()),
                tools: None,
                messages: List::with_capacity(0),
            }),
        }),
    }
}

fn completion_failure() -> smith_events::CompletionFailure {
    smith_events::CompletionFailure {
        field: None,
        code: None,
        request_id: None,
        class: smith_events::FailureClass::Unknown(vec![b'c'; 64].into()),
        evidence: smith_events::Evidence::Unknown(vec![b'e'; 64].into()),
        retry_after_ms: Some(20),
        phase: Some(smith_events::FailurePhase::Unknown(vec![b'p'; 64].into())),
        limit: Some(smith_events::FailureLimit { which: vec![b'l'; 64].into(), bound: 9 }),
        status: Some(400),
        detail: Some(vec![b'd'; 1024].into()),
    }
}

fn answer_failure() -> smith_events::AnswerFailure {
    smith_events::AnswerFailure {
        state: None,
        code: None,
        program: None,
        exit: None,
        detail: None,
        class: smith_events::AnswerClass::Unknown(vec![b'c'; 64].into()),
        endpoint: Some(vec![b'e'; 64].into()),
        model: Some(vec![b'm'; 64].into()),
        attempts: Some(1),
        completion: Some(completion_failure()),
        account: Some(1),
        reset_ms: Some(20),
        used: Some(2),
        bound: Some(1),
        which: Some(vec![b'w'; 64].into()),
        reason: Some(vec![b'r'; 64].into()),
        dropped: Some(false),
        ended: None,
        turn: Some(1),
    }
}

fn usage() -> smith_events::Usage {
    smith_events::Usage {
        input_tokens: Some(10),
        cache_read_tokens: None,
        cache_write_tokens: None,
        output_tokens: Some(4),
        reasoning_tokens: None,
    }
}

fn shown() -> Vec<Record> {
    let opened = crate::responses::tests::opened(2, 0);
    vec![
        golden(include_bytes!("../../../../smith-events/golden/v4/run_started_none.jsonl"), &limits(0)),
        request(),
        Record { t_ms: 0, event: Event::ConversationOpened(opened) },
        Record {
            t_ms: 0,
            event: Event::ConversationClosed(smith_events::ConversationClosed {
                run: 1,
                conversation: 2,
                end: smith_events::ConversationEnd::Failed,
                which: Some(vec![b'w'; 64].into()),
                failure: Some(completion_failure()),
                turns: 1,
                usage: usage(),
                spent: 2,
                duration_ms: 10,
            }),
        },
        Record {
            t_ms: 0,
            event: Event::TextDelta(smith_events::TextDelta {
                run: 1,
                conversation: 2,
                response: 3,
                block: 0,
                text: vec![b't'; 8192].into(),
            }),
        },
        tool_started(),
        Record {
            t_ms: 0,
            event: Event::ToolCompleted(smith_events::ToolCompleted {
                run: 1,
                conversation: 2,
                call: vec![b'c'; 64].into(),
                tool: vec![b't'; 64].into(),
                verdict: smith_events::Verdict::Unknown(vec![b'v'; 64].into()),
                exit: None,
                job: None,
                bytes: 1,
                omitted_bytes: 0,
                clamped: false,
                duration_ms: 10,
                child: None,
                delivery: None,
            }),
        },
        Record {
            t_ms: 0,
            event: Event::Notice(smith_events::Notice {
                code: None,
                level: smith_events::Level::Unknown(vec![b'l'; 64].into()),
                kind: smith_events::NoticeKind::Unknown(vec![b'k'; 64].into()),
                run: Some(1),
                account: None,
                reset_ms: None,
                bytes: Some(10),
                exit: None,
                forced: Some(smith_events::Forced::Unknown(vec![b'f'; 64].into())),
            }),
        },
        Record {
            t_ms: 0,
            event: Event::Budget(smith_events::BudgetStanding {
                run: 1,
                stage: smith_events::BudgetStage::Ordinary,
                turns: smith_events::Quantity { used: 1, limit: 10 },
                spend: smith_events::Quantity { used: 1, limit: 10 },
                time_ms: smith_events::Quantity { used: 1, limit: 10 },
                reserve: smith_events::Reserve { turns: 1, time_ms: 1, spend: 1 },
                call: Some(vec![b'c'; 65_536].into()),
            }),
        },
        response_completed(),
        Record {
            t_ms: 0,
            event: Event::RunCompleted(smith_events::RunCompleted {
                run: 1,
                wind_down: false,
                status: smith_events::Status::Failed,
                failure: Some(answer_failure()),
                result: None,
                turns: 1,
                spent: 2,
                usage: usage(),
                read: Some(1),
                unread: 0,
                duration_ms: 10,
            }),
        },
    ]
}

#[test]
fn a_request_prompt_never_enters_the_live_view() {
    let record = request();
    let span = skein_heap::Span::start();
    let projection = View::project(&record).expect("displayed request metadata");
    let measured = span.end();
    let View::Model { conversation, model: observed } = projection else { panic!("model projection") };
    assert_eq!(conversation, 2);
    assert_eq!(observed, model());
    assert_eq!(measured.peak, 192, "only the three model names were copied");
}

#[test]
fn unshown_records_do_not_use_a_view_slot_or_count_as_lost() {
    let bounds = limits(0);
    let mut sinks = Sinks::with_front(&bounds, Capture::None, 7, DEADLINE, true);
    for _ in 0_u32..5 {
        sinks.record(&Record {
            t_ms: 0,
            event: Event::MessageReceived(smith_events::MessageReceived {
                run: 1,
                name: 1,
                bytes: 10,
                refused: None,
                label: None,
                text: Some(b"not shown".as_slice().into()),
            }),
        });
    }
    assert!(sinks.pop_front().is_none());
    assert_eq!(sinks.front_lost(), 0);
}

#[test]
fn each_shown_projection_stays_within_its_owned_payload_bound() {
    let bound = View::worst_case(8192).expect("projection bound");
    for record in shown() {
        let span = skein_heap::Span::start();
        let projection = View::project(&record).expect("shown record");
        let measured = span.end();
        assert!(u64::try_from(measured.peak).unwrap() <= bound);
        drop(projection);
    }
}

#[test]
fn a_projected_request_keeps_its_original_full_capture_trace() {
    let mut bounds = limits(1);
    bounds.records.content = 4096;
    bounds.observation_line = 32_768;
    bounds.output = 196_608;
    let token = Token::new(400);
    let mut record = request();
    let Event::ResponseStarted(started) = &mut record.event else { panic!("request fixture") };
    started.prompt.as_mut().unwrap().system = Some(vec![b'p'; 4096].into());
    let mut sinks = Sinks::with_front(&bounds, Capture::Everything, 4096, DEADLINE, true);
    sinks.add(Destination::Trace, token, Delivery::Complete).unwrap();
    sinks.record(&record);
    let Some(View::Model { conversation, .. }) = sinks.pop_front() else { panic!("model projection") };
    assert_eq!(conversation, 2);
    sinks.finish();
    let trace = drain(&mut sinks, token);
    let Event::ResponseStarted(observed) = &trace[0].event else { panic!("original trace request") };
    let Event::ResponseStarted(original) = &record.event else { panic!("source request") };
    assert_eq!(observed.prompt, original.prompt);
}

fn tool_started() -> Record {
    Record {
        t_ms: 0,
        event: Event::ToolStarted(smith_events::ToolStarted {
            run: 1,
            conversation: 2,
            call: vec![b'c'; 64].into(),
            tool: vec![b't'; 64].into(),
            source: smith_events::Source::Workspace,
            effect: smith_events::Effect::Read,
            deadline_ms: Some(100),
            input_bytes: 65_536,
            input: Some(vec![b'i'; 65_536].into()),
        }),
    }
}

fn response_completed() -> Record {
    Record {
        t_ms: 0,
        event: Event::ResponseCompleted(smith_events::ResponseCompleted {
            run: 1,
            conversation: 2,
            response: 3,
            outcome: smith_events::Outcome::Failed,
            stop: None,
            blocks: 1,
            calls: 0,
            invalid: 0,
            oversized: 0,
            cut: 0,
            usage: usage(),
            spent: 2,
            request_bytes: Some(65_536),
            first_byte_ms: Some(1),
            largest_gap_ms: Some(1),
            duration_ms: 10,
            failure: Some(completion_failure()),
            retry_ms: None,
            completion: None,
        }),
    }
}
