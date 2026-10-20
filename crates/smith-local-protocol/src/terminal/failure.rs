//! Typed local failures rendered only for the person and operator
//! (domain/host.md, section 8; shell.md, section 4.3).
use super::{Shown, append, decimal};
use smith_events::{AnswerClass, AnswerFailure, CompletionFailure, Evidence, FailureClass, clip, words};
use smith_host_domain as host;
use smith_local_domain as local;

pub(super) fn run(
    text: &mut Shown,
    why: &host::RunFailure,
    diagnostic: &[u8],
    models: &[smith_domain::run::charter::Llm],
    account_name: Option<&[u8]>,
    endpoint_name: Option<&[u8]>,
    answered_at: Option<skein_lib::Wall>,
) {
    let mut value = failed(why);
    match why {
        host::RunFailure::Cancelled => append(text, b"interrupted: "),
        host::RunFailure::Model(host::ModelFault::Completion { model, .. }) => {
            if let Some(model) = models.get(usize::try_from(*model).expect("model index fits usize")) {
                value.endpoint = Some(match endpoint_name {
                    Some(name) => clip(name).into(),
                    None => skein_lib::Decimal::of(u64::from(model.endpoint.0)).as_bytes().into(),
                });
                value.model = Some(clip(&model.model).into());
                if let Some(account) = model.account {
                    value.account = Some(account);
                }
            }
            match value.completion.as_ref().expect("completion failure").class {
                FailureClass::Unauthorized if value.account.is_some() => {
                    value.state = Some(smith_events::AccountState::Unauthorized);
                }
                FailureClass::Unauthorized
                | FailureClass::Overloaded
                | FailureClass::RateLimited
                | FailureClass::Exhausted
                | FailureClass::Refused
                | FailureClass::Unsupported
                | FailureClass::Unavailable
                | FailureClass::TimedOut
                | FailureClass::ContextTooLong
                | FailureClass::Invalid
                | FailureClass::Limit
                | FailureClass::Protocol
                | FailureClass::Cancelled
                | FailureClass::Unknown(_) => append(text, b"run failed: "),
            }
        }
        host::RunFailure::Model(_)
        | host::RunFailure::Budget { exhausted: _, .. }
        | host::RunFailure::Policy(_)
        | host::RunFailure::Transcript(_)
        | host::RunFailure::Stale => append(text, b"run failed: "),
    }
    append(text, &words(&value, account_name, answered_at));
    if value.program.is_none() {
        detail(text, diagnostic);
    }
}

pub(super) fn refused(text: &mut Shown, why: host::Refusal) {
    append(text, b"run refused: ");
    append(text, &words(&refusal(why), None, None));
}

pub(super) fn detail(text: &mut Shown, diagnostic: &[u8]) {
    let kept = clip(diagnostic);
    if !kept.is_empty() {
        append(text, b": ");
        append(text, kept);
    }
}

pub(super) fn account(text: &mut Shown, why: &local::AccountFailure) {
    match why {
        local::AccountFailure::Unavailable => append(text, b"A model account is unavailable"),
        local::AccountFailure::NotRefreshed => append(text, b"A model account could not refresh"),
        local::AccountFailure::Limited { account, reset_at } => {
            append(text, b"run failed: account ");
            decimal(text, u64::from(*account));
            append(text, b" reached its usage limit");
            if let Some(at) = reset_at {
                append(text, b", usable again at ");
                append(text, &smith_events::clock(*at));
            }
        }
        local::AccountFailure::Refused { account, code, reason } => {
            append(text, b"run failed: account ");
            decimal(text, u64::from(*account));
            append(text, b": the provider refused it");
            detail(text, code);
            detail(text, reason);
        }
    }
}

pub(super) fn credential(text: &mut Shown, account: u32, why: local::CredentialFailure) {
    append(text, b"account ");
    decimal(text, u64::from(account));
    let suffix: &[u8] = match why {
        local::CredentialFailure::Missing => b" has no credential: run `smith login ",
        local::CredentialFailure::BorrowedExpiring { .. } => {
            b" has a borrowed login approaching expiry: run `smith login "
        }
        local::CredentialFailure::Refresh => b" could not renew its credential: run `smith login ",
        local::CredentialFailure::ProgramNotSignedIn => b": claude_code is not signed in: run `smith login ",
        local::CredentialFailure::ProgramSignedInOtherwise { method } => {
            append(text, b": claude_code is signed in with ");
            escaped(text, method.as_bytes());
            append(text, b", not a subscription: run `smith login ");
            decimal(text, u64::from(account));
            append(text, b"`");
            return;
        }
        local::CredentialFailure::ProgramFailed(failure) => {
            append(text, b": claude_code could not be checked: ");
            escaped(text, &failure.detail);
            return;
        }
        local::CredentialFailure::ProgramInterrupted => {
            append(text, b": claude_code could not be checked: the check was interrupted");
            return;
        }
    };
    append(text, suffix);
    decimal(text, u64::from(account));
    append(text, b"`");
}

fn escaped(text: &mut Shown, bytes: &[u8]) {
    for byte in bytes {
        for escaped in byte.escape_ascii() {
            append(text, &[escaped]);
        }
    }
}

pub(super) fn end(text: &mut Shown, ended: host::End) {
    match ended {
        host::End::Busy => append(text, b"agent start refused: no agent slot"),
        host::End::Invalid(why) => {
            append(text, b"agent start refused: invalid ");
            append(
                text,
                match why {
                    host::Invalid::Messages => b"messages",
                    host::Invalid::Activation => b"activation",
                    host::Invalid::Charter => b"charter",
                    host::Invalid::Transcript => b"transcript",
                    host::Invalid::Answered => b"saved answers",
                    host::Invalid::Directories => b"directories",
                    host::Invalid::Grants => b"grants",
                    host::Invalid::Limits => b"limits",
                },
            );
        }
        host::End::Unspawned => append(text, b" (not spawned)"),
        host::End::Settled => append(text, b" (inline run settled)"),
        host::End::Exited { exit, forced } => {
            match exit {
                Some(host::Exit::Code(code)) => {
                    append(text, b" (exit ");
                    decimal(text, u64::from(code));
                    append(text, b")");
                }
                Some(host::Exit::Signal(signal)) => {
                    append(text, b" (signal ");
                    decimal(text, u64::from(signal));
                    append(text, b")");
                }
                None => append(text, b" (exit not observed)"),
            }
            append(
                text,
                match forced {
                    host::Forced::No => b"",
                    host::Forced::Terminated => b"; terminated",
                    host::Forced::Killed => b"; killed",
                },
            );
        }
    }
}

fn failure(class: AnswerClass, which: Option<&[u8]>) -> AnswerFailure {
    let mut result = AnswerFailure {
        state: None,
        code: None,
        program: None,
        exit: None,
        detail: None,
        reset_ms: None,
        used: None,
        bound: None,
        dropped: None,
        ended: None,
        turn: None,
        endpoint: None,
        model: None,
        attempts: None,
        class,
        completion: None,
        account: None,
        which: None,
        reason: None,
    };
    if let Some(which) = which {
        result.which = Some(which.into());
    }
    result
}

fn refusal(refusal: host::Refusal) -> AnswerFailure {
    match refusal {
        host::Refusal::Busy => failure(AnswerClass::Busy, None),
        host::Refusal::Invalid(invalid) => failure(AnswerClass::Invalid, Some(invalid_name(invalid))),
    }
}

fn invalid_name(invalid: host::RunInvalid) -> &'static [u8] {
    match invalid {
        host::RunInvalid::HostPart { .. } => b"host_part",
        host::RunInvalid::Room { .. } => b"room",
        host::RunInvalid::Messages => b"messages",
        host::RunInvalid::CharterVersion => b"charter_version",
        host::RunInvalid::MalformedCharter => b"malformed_charter",
        host::RunInvalid::Endpoint => b"endpoint",
        host::RunInvalid::Activation => b"activation",
        host::RunInvalid::Window => b"window",
        host::RunInvalid::Conventions => b"conventions",
        host::RunInvalid::TooLarge => b"too_large",
        host::RunInvalid::Workspace => b"workspace",
        host::RunInvalid::Grants => b"grants",
        host::RunInvalid::Outcome => b"outcome",
        host::RunInvalid::Budget(why) => match why {
            host::BudgetRefusal::Reserve(part) => match part {
                host::Part::Turns => b"budget_reserve_turns",
                host::Part::Spend => b"budget_reserve_spend",
                host::Part::Time => b"budget_reserve_time",
            },
            host::BudgetRefusal::Ceiling(_)
            | host::BudgetRefusal::Empty(_)
            | host::BudgetRefusal::Checks { .. }
            | host::BudgetRefusal::Completion { .. } => b"budget",
        },
        host::RunInvalid::Price(_) => b"price",
        host::RunInvalid::Llm => b"llm",
        host::RunInvalid::Effort => b"effort",
        host::RunInvalid::Conversation => b"conversation",
    }
}

fn failed(why: &host::RunFailure) -> AnswerFailure {
    match why {
        host::RunFailure::Model(fault) => match fault {
            host::ModelFault::Completion { failure: why, evidence, model, attempts, report: _ } => {
                let mut result = failure(AnswerClass::Model, None);
                result.completion = Some(completion(*why, *evidence));
                result.attempts = Some(u64::from(*attempts));
                result.model = Some(skein_lib::Decimal::of(u64::from(*model)).as_bytes().into());
                result
            }
            host::ModelFault::Limited { account, reset } => {
                let mut result = failure(AnswerClass::Model, None);
                result.account = Some(*account);
                result.state = Some(smith_events::AccountState::Limited);
                result.reset_ms = match reset {
                    Some(reset) => reset.as_nanos().checked_div(1_000_000),
                    None => None,
                };
                result
            }
            host::ModelFault::Provider => failure(AnswerClass::Model, Some(b"provider")),
            host::ModelFault::Program { name, exit, detail } => {
                let mut result = failure(AnswerClass::Model, None);
                result.program = Some(name.clone());
                if let Some(code) = exit {
                    result.exit = Some(smith_events::AgentExit::Code(i64::from(*code)));
                }
                result.detail = Some(detail.clone());
                result
            }
            host::ModelFault::Refused { account, code } => {
                let mut result = failure(AnswerClass::Model, None);
                result.account = Some(*account);
                result.state = Some(smith_events::AccountState::Refused);
                result.code = Some(code.clone());
                result
            }
            host::ModelFault::Declined => failure(AnswerClass::Model, Some(b"refused")),
            host::ModelFault::Unauthorized { account } => {
                let mut result = failure(AnswerClass::Model, None);
                result.account = Some(*account);
                result.state = Some(smith_events::AccountState::Unauthorized);
                result
            }
            host::ModelFault::Truncated => failure(AnswerClass::Model, Some(b"truncated")),
            host::ModelFault::Malformed => failure(AnswerClass::Model, Some(b"malformed")),
        },
        host::RunFailure::Budget { exhausted, used, bound } => {
            let which: &[u8] = match exhausted {
                host::Exhausted::Turns => b"turns",
                host::Exhausted::Spend => b"spend",
                host::Exhausted::Time => b"time",
                host::Exhausted::Tokens(receiving) => match receiving {
                    host::ReceivingLimit::Input => b"input_tokens",
                    host::ReceivingLimit::Output => b"output_tokens",
                    host::ReceivingLimit::CacheRead => b"cache_read_tokens",
                    host::ReceivingLimit::CacheWrite => b"cache_write_tokens",
                },
                host::Exhausted::Overflow(overflow) => match overflow {
                    host::Overflow::Spend => b"spend_overflow",
                    host::Overflow::Usage => b"usage_overflow",
                },
            };
            let mut result = failure(AnswerClass::Budget, Some(which));
            result.used = *used;
            result.bound = *bound;
            result
        }
        host::RunFailure::Policy(host::Policy::Unfinished { .. }) => failure(AnswerClass::Policy, None),
        host::RunFailure::Cancelled => {
            let mut answer = failure(AnswerClass::Cancelled, None);
            answer.dropped = Some(false);
            answer
        }
        host::RunFailure::Stale => failure(AnswerClass::Stale, None),
        host::RunFailure::Transcript(refusal) => {
            let which: &[u8] = match refusal {
                host::TranscriptRefusal::Version => b"version",
                host::TranscriptRefusal::Malformed => b"malformed",
                host::TranscriptRefusal::Unresolved => b"unresolved",
                host::TranscriptRefusal::TooLarge => b"too_large",
            };
            let mut result = failure(AnswerClass::Transcript, None);
            result.reason = Some(which.into());
            result
        }
    }
}

fn completion(why: host::CompletionFailure, evidence: host::CompletionEvidence) -> CompletionFailure {
    let (class, retry_after_ms) = match why {
        host::CompletionFailure::Limit { .. } | host::CompletionFailure::ApplicationLimit => {
            (FailureClass::Limit, None)
        }
        host::CompletionFailure::Protocol => (FailureClass::Protocol, None),
        host::CompletionFailure::Cancelled => (FailureClass::Cancelled, None),
        host::CompletionFailure::Overloaded => (FailureClass::Overloaded, None),
        host::CompletionFailure::Unavailable => (FailureClass::Unavailable, None),
        host::CompletionFailure::TimedOut { .. } => (FailureClass::TimedOut, None),
        host::CompletionFailure::ContextTooLong => (FailureClass::ContextTooLong, None),
        // smith 05.2 gives the new classes their event names.
        host::CompletionFailure::Refused => (FailureClass::Unknown(b"refused".as_slice().into()), None),
        host::CompletionFailure::Unsupported | host::CompletionFailure::Invalid => (FailureClass::Invalid, None),
        host::CompletionFailure::Unauthorized => (FailureClass::Unauthorized, None),
        host::CompletionFailure::RateLimited { retry_after } => {
            (FailureClass::RateLimited, Some(retry_after.as_nanos() / 1_000_000))
        }
        host::CompletionFailure::Exhausted { retry_after } => (
            FailureClass::Exhausted,
            match retry_after {
                Some(reset) => reset.as_nanos().checked_div(1_000_000),
                None => None,
            },
        ),
    };
    let mut status = None;
    match evidence {
        host::CompletionEvidence::Unsent | host::CompletionEvidence::Unknown => {}
        host::CompletionEvidence::Response { status: observed } => {
            if let Some(observed) = observed {
                status = Some(u64::from(observed));
            }
        }
    }
    let phase = match why {
        host::CompletionFailure::TimedOut { phase } => Some(match phase {
            host::Phase::Connect => smith_events::FailurePhase::Connect,
            host::Phase::Handshake => smith_events::FailurePhase::Handshake,
            host::Phase::Head => smith_events::FailurePhase::Head,
            host::Phase::Idle => smith_events::FailurePhase::Idle,
            host::Phase::Whole => smith_events::FailurePhase::Whole,
        }),
        host::CompletionFailure::Limit { .. }
        | host::CompletionFailure::ApplicationLimit
        | host::CompletionFailure::Protocol
        | host::CompletionFailure::Cancelled
        | host::CompletionFailure::Overloaded
        | host::CompletionFailure::Unavailable
        | host::CompletionFailure::ContextTooLong
        | host::CompletionFailure::Invalid
        | host::CompletionFailure::Refused
        | host::CompletionFailure::Unsupported
        | host::CompletionFailure::Unauthorized
        | host::CompletionFailure::RateLimited { .. }
        | host::CompletionFailure::Exhausted { .. } => None,
    };
    let limit = match why {
        host::CompletionFailure::Limit { which, bound } => {
            Some(smith_events::FailureLimit { which: limit_name(which).into(), bound })
        }
        host::CompletionFailure::ApplicationLimit
        | host::CompletionFailure::Protocol
        | host::CompletionFailure::Cancelled
        | host::CompletionFailure::Overloaded
        | host::CompletionFailure::Unavailable
        | host::CompletionFailure::TimedOut { .. }
        | host::CompletionFailure::ContextTooLong
        | host::CompletionFailure::Invalid
        | host::CompletionFailure::Refused
        | host::CompletionFailure::Unsupported
        | host::CompletionFailure::Unauthorized
        | host::CompletionFailure::RateLimited { .. }
        | host::CompletionFailure::Exhausted { .. } => None,
    };
    let evidence = match evidence {
        host::CompletionEvidence::Unsent => Evidence::Unsent,
        host::CompletionEvidence::Unknown => Evidence::MaybeSent,
        host::CompletionEvidence::Response { .. } => Evidence::Response,
    };
    CompletionFailure {
        field: None,
        code: None,
        request_id: None,
        class,
        evidence,
        retry_after_ms,
        phase,
        limit,
        status,
        detail: None,
    }
}

fn limit_name(which: host::Limit) -> &'static [u8] {
    match which {
        host::Limit::Pool { .. } => b"pool",
        host::Limit::History => b"history",
        host::Limit::Window => b"window",
        host::Limit::Call => b"call",
        host::Limit::Message => b"message",
        host::Limit::Path(which) => match which {
            host::Cap::Request => b"request",
            host::Cap::RequestHead => b"request_head",
            host::Cap::ResponseHead => b"response_head",
            host::Cap::ResponseFields => b"response_fields",
            host::Cap::Skip => b"skip",
            host::Cap::Field => b"field",
            host::Cap::Retained => b"retained",
            host::Cap::Tokens => b"tokens",
            host::Cap::Depth => b"depth",
            host::Cap::Strings => b"strings",
            host::Cap::Number => b"number",
            host::Cap::Reasoning => b"reasoning",
            host::Cap::OutputItems => b"output_items",
            host::Cap::Metadata => b"metadata",
            host::Cap::Tools => b"tools",
            host::Cap::HistoryItems => b"history_items",
            host::Cap::Output => b"output",
            host::Cap::Answer => b"answer",
            host::Cap::ErrorBody => b"error_body",
            host::Cap::AccessToken => b"access_token",
            host::Cap::AccountId => b"account_id",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_credentials_use_the_numbered_fallback() {
        use smith_host_domain::{FailureReason, ProgramFailure, ProgramMethod};
        for (reason, expected) in [
            (
                local::CredentialFailure::ProgramNotSignedIn,
                "account 7: claude_code is not signed in: run `smith login 7`",
            ),
            (
                local::CredentialFailure::ProgramSignedInOtherwise { method: ProgramMethod::new(b"api").unwrap() },
                "account 7: claude_code is signed in with api, not a subscription: run `smith login 7`",
            ),
            (
                local::CredentialFailure::ProgramFailed(ProgramFailure {
                    reason: FailureReason::Unreadable,
                    detail: b"bad\x1b\n".as_slice().into(),
                }),
                r"account 7: claude_code could not be checked: bad\x1b\n",
            ),
            (
                local::CredentialFailure::ProgramInterrupted,
                "account 7: claude_code could not be checked: the check was interrupted",
            ),
        ] {
            let mut text = Shown { bytes: skein_lib::List::with_capacity(1024), cut: false };
            credential(&mut text, 7, reason);
            assert_eq!(text.bytes.as_slice(), expected.as_bytes());
        }
    }

    #[test]
    fn program_failures_keep_their_terminal_metadata() {
        for (exit, expected) in [(None, None), (Some(7), Some(smith_events::AgentExit::Code(7)))] {
            let fault = host::RunFailure::Model(host::ModelFault::Program {
                name: b"claude_code".as_slice().into(),
                exit,
                detail: b"program detail".as_slice().into(),
            });
            let value = failed(&fault);
            assert_eq!(value.class, AnswerClass::Model);
            assert_eq!(value.program.as_deref(), Some(b"claude_code".as_slice()));
            assert_eq!(value.exit, expected);
            assert_eq!(value.detail.as_deref(), Some(b"program detail".as_slice()));
        }
    }
}
