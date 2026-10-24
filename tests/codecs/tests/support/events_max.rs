//! Largest typed event payloads under configured limits.

use skein_lib::List;
use smith_events::{
    Agent, AnswerClass, AnswerFailure, Block, Budget, BudgetStage, BudgetStanding, Capture, CheckCompleted, CheckExit,
    CheckStarted, Clamp, CommandExit, Completion, CompletionFailure, ConversationClosed, ConversationEnd,
    ConversationKind, ConversationOpened, Count, Cpu, Delivered, Delivery, Effect, Event, Evidence, FailureClass,
    Family, Field, Form, Level, Loss, Message, Mode, Model, Notice, NoticeKind, Outcome, PeakRss, Prompt, Quantity,
    Record, Requested, Reserve, ResponseCompleted, ResponseStarted, Role, RunCompleted, RunResult, RunStarted,
    SessionEnded, SessionStarted, Share, ShareBound, SharePart, Source, Status, Stop, TextDelta, ToolChoice,
    ToolCompleted, ToolStarted, Tools, Usage, Verdict, Versions,
};

fn payload(length: u32) -> Box<[u8]> {
    vec![1; usize::try_from(length).expect("length")].into_boxed_slice()
}

fn max_agent(limits: &smith_events::Limits) -> Agent {
    Agent { name: payload(limits.string), version: payload(limits.string) }
}

fn max_versions(_limits: &smith_events::Limits) -> Versions {
    Versions {
        events: u64::MAX,
        channel: u64::MAX,
        charter: u64::MAX,
        transcript: u64::MAX,
        chat_store: Some(u64::MAX),
    }
}

fn max_loss(_limits: &smith_events::Limits) -> Loss {
    Loss { events: u64::MAX, channel: Some(u64::MAX) }
}

fn max_cpu(_limits: &smith_events::Limits) -> Cpu {
    Cpu { user: u64::MAX, system: u64::MAX, children_user: u64::MAX, children_system: u64::MAX }
}

fn max_peak_rss(_limits: &smith_events::Limits) -> PeakRss {
    PeakRss { self_bytes: u64::MAX, children: u64::MAX }
}

fn max_model(limits: &smith_events::Limits) -> Model {
    Model {
        account: Some(u32::MAX),
        backend: smith_events::Backend::Unknown(payload(limits.string)),
        endpoint: payload(limits.string),
        model: payload(limits.string),
        window_tokens: u64::MAX,
        output_tokens: u64::MAX,
        effort: Some(payload(limits.string)),
    }
}

fn max_usage(_limits: &smith_events::Limits) -> Usage {
    Usage {
        input_tokens: Some(u64::MAX),
        cache_read_tokens: Some(u64::MAX),
        cache_write_tokens: Some(u64::MAX),
        output_tokens: Some(u64::MAX),
        reasoning_tokens: Some(u64::MAX),
    }
}

fn max_budget(_limits: &smith_events::Limits) -> Budget {
    Budget {
        turns: u64::MAX,
        spend: u64::MAX,
        time_ms: u64::MAX,
        reserve: Reserve { turns: u64::MAX, time_ms: u64::MAX, spend: u64::MAX },
    }
}

fn max_tools(limits: &smith_events::Limits) -> Tools {
    Tools {
        withheld: max_list_family(limits),
        families: max_list_family(limits),
        wait: false,
        deliver: false,
        host: max_list_text(limits),
    }
}

fn max_completion_failure(limits: &smith_events::Limits) -> CompletionFailure {
    CompletionFailure {
        field: Some(payload(limits.string)),
        code: Some(payload(limits.string)),
        request_id: Some(payload(limits.string)),
        phase: Some(smith_events::FailurePhase::Unknown(payload(limits.string))),
        limit: Some(smith_events::FailureLimit { which: payload(limits.string), bound: u64::MAX }),
        status: Some(u64::MAX),
        detail: Some(payload(limits.content)),
        class: FailureClass::Unknown(payload(limits.string)),
        evidence: Evidence::Unknown(payload(limits.string)),
        retry_after_ms: Some(u64::MAX),
    }
}

fn max_answer_failure(limits: &smith_events::Limits) -> AnswerFailure {
    AnswerFailure {
        state: Some(smith_events::AccountState::Unknown(payload(limits.string))),
        code: Some(payload(limits.string)),
        program: Some(payload(limits.string)),
        exit: Some(smith_events::AgentExit::Signal(u64::MAX)),
        detail: Some(payload(limits.content)),
        reset_ms: Some(u64::MAX),
        used: Some(u64::MAX),
        bound: Some(u64::MAX),
        endpoint: Some(payload(limits.string)),
        model: Some(payload(limits.string)),
        attempts: Some(u64::MAX),
        dropped: Some(false),
        ended: Some(smith_events::AgentExit::Code(i64::MIN)),
        turn: Some(u64::MAX),
        class: AnswerClass::Unknown(payload(limits.string)),
        completion: Some(max_completion_failure(limits)),
        account: Some(u32::MAX),
        which: Some(payload(limits.string)),
        reason: Some(payload(limits.string)),
    }
}

fn max_field(limits: &smith_events::Limits) -> Field {
    Field { name: payload(limits.string), text: payload(limits.content) }
}

fn max_run_result(limits: &smith_events::Limits) -> RunResult {
    RunResult {
        form: Form::Unknown(payload(limits.string)),
        label: Some(payload(limits.string)),
        text: Some(payload(limits.content)),
        fields: Some(max_list_field(limits)),
    }
}

fn max_message(limits: &smith_events::Limits) -> Message {
    Message { role: Role::Unknown(payload(limits.string)), blocks: max_list_block(limits) }
}

fn max_prompt(limits: &smith_events::Limits) -> Prompt {
    Prompt {
        system: Some(payload(limits.content)),
        tools: Some(max_list_text(limits)),
        messages: max_list_message(limits),
    }
}

fn max_completion(limits: &smith_events::Limits) -> Completion {
    Completion { blocks: max_list_block(limits) }
}

fn max_command_exit(_limits: &smith_events::Limits) -> CommandExit {
    CommandExit::Signal { signal: u64::MAX }
}

fn max_session_started(limits: &smith_events::Limits) -> SessionStarted {
    SessionStarted {
        chat: Some(payload(limits.string)),
        limits: {
            let mut values = List::with_capacity(limits.items);
            for index in 0..limits.items {
                let mut name = payload(limits.string);
                *name.last_mut().expect("max metadata has a name") = u8::try_from(index).expect("small fixture");
                values
                    .push(smith_events::EffectiveLimit {
                        name,
                        value: smith_events::LimitValue::Fraction { numerator: u64::MAX, denominator: u64::MAX },
                    })
                    .expect("configured count");
            }
            values
        },
        wall_ms: u64::MAX,
        agent: max_agent(limits),
        mode: Mode::Unknown(payload(limits.string)),
        containment: smith_events::Containment::Unknown(payload(limits.string)),
        pid: u64::MAX,
        profile: payload(limits.string),
        capture: Capture::Unknown(payload(limits.string)),
        delivery: Delivery::Unknown(payload(limits.string)),
        versions: max_versions(limits),
    }
}

fn max_session_ended(limits: &smith_events::Limits) -> SessionEnded {
    SessionEnded {
        exit: i64::MIN,
        answered: false,
        aborted: Some(false),
        teardown_ms: Some(u64::MAX),
        emitted: max_list_count(limits),
        loss: max_loss(limits),
        cpu_ms: Some(max_cpu(limits)),
        peak_rss_bytes: Some(max_peak_rss(limits)),
    }
}

fn max_run_started(limits: &smith_events::Limits) -> RunStarted {
    RunStarted {
        run: u64::MAX,
        resumed: false,
        messages: u64::MAX,
        main: max_model(limits),
        budget: max_budget(limits),
        tools: max_tools(limits),
        contract: max_list_form(limits),
    }
}

fn max_run_completed(limits: &smith_events::Limits) -> RunCompleted {
    RunCompleted {
        run: u64::MAX,
        wind_down: false,
        status: Status::Unknown(payload(limits.string)),
        failure: Some(max_answer_failure(limits)),
        result: Some(max_run_result(limits)),
        turns: u64::MAX,
        spent: u64::MAX,
        usage: max_usage(limits),
        read: Some(u64::MAX),
        unread: u64::MAX,
        duration_ms: u64::MAX,
    }
}

fn max_conversation_opened(limits: &smith_events::Limits) -> ConversationOpened {
    ConversationOpened {
        run: u64::MAX,
        conversation: u64::MAX,
        kind: ConversationKind::Unknown(payload(limits.string)),
        parent: Some(u64::MAX),
        call: Some(payload(limits.string)),
        thread: u64::MAX,
        model: max_model(limits),
        requested: Some(Requested { turns: Some(u64::MAX), seconds: Some(u64::MAX) }),
        share: Some(Share { turns: u64::MAX, time_ms: u64::MAX, spend: u64::MAX }),
        clamped: {
            let mut values = List::with_capacity(limits.items);
            for _ in 0..limits.items {
                values
                    .push(Clamp {
                        part: SharePart::Unknown(payload(limits.string)),
                        by: ShareBound::Unknown(payload(limits.string)),
                    })
                    .expect("configured clamp item room");
            }
            values
        },
    }
}

fn max_conversation_closed(limits: &smith_events::Limits) -> ConversationClosed {
    ConversationClosed {
        run: u64::MAX,
        conversation: u64::MAX,
        end: ConversationEnd::Unknown(payload(limits.string)),
        which: Some(payload(limits.string)),
        failure: Some(max_completion_failure(limits)),
        turns: u64::MAX,
        usage: max_usage(limits),
        spent: u64::MAX,
        duration_ms: u64::MAX,
    }
}

fn max_response_started(limits: &smith_events::Limits) -> ResponseStarted {
    ResponseStarted {
        run: u64::MAX,
        conversation: u64::MAX,
        response: u64::MAX,
        turn: u64::MAX,
        attempt: u64::MAX,
        window: u64::MAX,
        model: max_model(limits),
        messages: u64::MAX,
        output_tokens: u64::MAX,
        tool_choice: ToolChoice::Unknown(payload(limits.string)),
        prompt: Some(max_prompt(limits)),
    }
}

fn max_response_completed(limits: &smith_events::Limits) -> ResponseCompleted {
    ResponseCompleted {
        run: u64::MAX,
        conversation: u64::MAX,
        response: u64::MAX,
        outcome: Outcome::Unknown(payload(limits.string)),
        stop: Some(Stop::Unknown(payload(limits.string))),
        blocks: u64::MAX,
        calls: u64::MAX,
        invalid: u64::MAX,
        oversized: u64::MAX,
        cut: u64::MAX,
        usage: max_usage(limits),
        spent: u64::MAX,
        request_bytes: Some(u64::MAX),
        first_byte_ms: Some(u64::MAX),
        largest_gap_ms: Some(u64::MAX),
        duration_ms: u64::MAX,
        failure: Some(max_completion_failure(limits)),
        retry_ms: Some(u64::MAX),
        completion: Some(max_completion(limits)),
    }
}

fn max_text_delta(limits: &smith_events::Limits) -> TextDelta {
    TextDelta {
        run: u64::MAX,
        conversation: u64::MAX,
        response: u64::MAX,
        block: u64::MAX,
        text: payload(limits.content),
    }
}

fn max_tool_started(limits: &smith_events::Limits) -> ToolStarted {
    ToolStarted {
        run: u64::MAX,
        conversation: u64::MAX,
        call: payload(limits.string),
        tool: payload(limits.string),
        source: Source::Unknown(payload(limits.string)),
        effect: Effect::Unknown(payload(limits.string)),
        deadline_ms: Some(u64::MAX),
        input_bytes: u64::MAX,
        input: Some(payload(limits.content)),
    }
}

fn max_tool_completed(limits: &smith_events::Limits) -> ToolCompleted {
    ToolCompleted {
        run: u64::MAX,
        conversation: u64::MAX,
        call: payload(limits.string),
        tool: payload(limits.string),
        verdict: Verdict::Unknown(payload(limits.string)),
        exit: Some(max_command_exit(limits)),
        job: Some(u64::MAX),
        bytes: u64::MAX,
        omitted_bytes: u64::MAX,
        clamped: true,
        duration_ms: u64::MAX,
        child: Some(smith_events::ChildCompleted {
            conversation: u64::MAX,
            stop: smith_events::ChildStop::Unknown(payload(limits.string)),
            turns: u64::MAX,
            time_ms: u64::MAX,
            spent: u64::MAX,
            usage: max_usage(limits),
        }),
        delivery: Some(Delivered::Unknown(payload(limits.string))),
    }
}

fn max_check_started(_limits: &smith_events::Limits) -> CheckStarted {
    CheckStarted { run: u64::MAX, deadline_ms: Some(u64::MAX) }
}

fn max_check_completed(limits: &smith_events::Limits) -> CheckCompleted {
    CheckCompleted {
        run: u64::MAX,
        exit: CheckExit::Unknown(payload(limits.string)),
        passed: false,
        duration_ms: u64::MAX,
    }
}

fn max_notice(limits: &smith_events::Limits) -> Notice {
    Notice {
        code: Some(payload(limits.string)),
        level: Level::Unknown(payload(limits.string)),
        kind: NoticeKind::Unknown(payload(limits.string)),
        run: Some(u64::MAX),
        account: Some(u32::MAX),
        bytes: Some(u64::MAX),
        reset_ms: Some(u64::MAX),
        exit: Some(smith_events::AgentExit::Signal(u64::MAX)),
        forced: Some(smith_events::Forced::Unknown(payload(limits.string))),
    }
}

fn max_block(limits: &smith_events::Limits) -> Block {
    Block::Call { call: payload(limits.string), tool: payload(limits.string), input: payload(limits.content) }
}

fn max_list_block(limits: &smith_events::Limits) -> List<Block> {
    let mut values = List::with_capacity(limits.items);
    for _ in 0..limits.items {
        values.push(max_block(limits)).expect("item room");
    }
    values
}

fn max_list_count(limits: &smith_events::Limits) -> List<Count> {
    let mut values = List::with_capacity(limits.items);
    for item in 0..limits.items {
        let mut record = payload(limits.string).into_vec();
        let first = record.first_mut().expect("nonzero metadata limit");
        *first = u8::try_from(item).expect("test item count").saturating_add(32);
        values.push(Count { record: record.into_boxed_slice(), count: u64::MAX }).expect("item room");
    }
    values
}

fn max_list_family(limits: &smith_events::Limits) -> List<Family> {
    let mut values = List::with_capacity(limits.items);
    for _ in 0..limits.items {
        values.push(Family::Unknown(payload(limits.string))).expect("item room");
    }
    values
}

fn max_list_field(limits: &smith_events::Limits) -> List<Field> {
    let mut values = List::with_capacity(limits.items);
    for _ in 0..limits.items {
        values.push(max_field(limits)).expect("item room");
    }
    values
}

fn max_list_form(limits: &smith_events::Limits) -> List<Form> {
    let mut values = List::with_capacity(limits.items);
    for _ in 0..limits.items {
        values.push(Form::Unknown(payload(limits.string))).expect("item room");
    }
    values
}

fn max_list_message(limits: &smith_events::Limits) -> List<Message> {
    let mut values = List::with_capacity(limits.items);
    for _ in 0..limits.items {
        values.push(max_message(limits)).expect("item room");
    }
    values
}

fn max_list_text(limits: &smith_events::Limits) -> List<Box<[u8]>> {
    let mut values = List::with_capacity(limits.items);
    for _ in 0..limits.items {
        values.push(payload(limits.string)).expect("item room");
    }
    values
}

pub fn largest_samples(limits: &smith_events::Limits) -> Vec<Record> {
    vec![
        Record {
            t_ms: u64::MAX,
            event: Event::Budget(BudgetStanding {
                call: Some(payload(limits.string)),
                run: u64::MAX,
                stage: BudgetStage::Unknown(payload(limits.string)),
                turns: Quantity { used: u64::MAX, limit: u64::MAX },
                spend: Quantity { used: u64::MAX, limit: u64::MAX },
                time_ms: Quantity { used: u64::MAX, limit: u64::MAX },
                reserve: Reserve { turns: u64::MAX, time_ms: u64::MAX, spend: u64::MAX },
            }),
        },
        Record { t_ms: u64::MAX, event: Event::SessionStarted(max_session_started(limits)) },
        Record { t_ms: u64::MAX, event: Event::SessionEnded(max_session_ended(limits)) },
        Record { t_ms: u64::MAX, event: Event::RunStarted(max_run_started(limits)) },
        Record { t_ms: u64::MAX, event: Event::RunCompleted(max_run_completed(limits)) },
        Record { t_ms: u64::MAX, event: Event::ConversationOpened(max_conversation_opened(limits)) },
        Record { t_ms: u64::MAX, event: Event::ConversationClosed(max_conversation_closed(limits)) },
        Record { t_ms: u64::MAX, event: Event::ResponseStarted(max_response_started(limits)) },
        Record { t_ms: u64::MAX, event: Event::ResponseCompleted(max_response_completed(limits)) },
        Record { t_ms: u64::MAX, event: Event::TextDelta(max_text_delta(limits)) },
        Record { t_ms: u64::MAX, event: Event::ToolStarted(max_tool_started(limits)) },
        Record { t_ms: u64::MAX, event: Event::ToolCompleted(max_tool_completed(limits)) },
        Record {
            t_ms: u64::MAX,
            event: Event::JobEnded(smith_events::JobEnded {
                run: u64::MAX,
                conversation: u64::MAX,
                job: u64::MAX,
                call: payload(limits.string),
                verdict: Verdict::Unknown(payload(limits.string)),
                exit: Some(max_command_exit(limits)),
                unread_bytes: u64::MAX,
                duration_ms: u64::MAX,
            }),
        },
        Record { t_ms: u64::MAX, event: Event::CheckStarted(max_check_started(limits)) },
        Record { t_ms: u64::MAX, event: Event::CheckCompleted(max_check_completed(limits)) },
        Record { t_ms: u64::MAX, event: Event::Notice(max_notice(limits)) },
        Record {
            t_ms: u64::MAX,
            event: Event::MessageReceived(smith_events::MessageReceived {
                run: u64::MAX,
                name: u64::MAX,
                bytes: u64::MAX,
                refused: Some(smith_events::MessageRefusal::Unknown(payload(limits.string))),
                label: Some(payload(limits.string)),
                text: Some(payload(limits.content)),
            }),
        },
        Record {
            t_ms: u64::MAX,
            event: Event::MessageRead(smith_events::MessageRead { run: u64::MAX, name: u64::MAX, turn: u64::MAX }),
        },
        Record {
            t_ms: u64::MAX,
            event: Event::MessageUnread(smith_events::MessageUnread { run: u64::MAX, name: u64::MAX }),
        },
    ]
}
