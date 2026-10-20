//! Translation of one typed domain request to its component or host channel.
//! The service retains delivery and observation state; translation never chooses
//! a domain result. Contract: protocol/agent.md, sections 4–6.

use super::up::live_step;
use super::{Failure, Service, program, teardown};
use alloc::boxed::Box;
use skein_lib::{Duration, Time, Token};
use smith_domain as domain;
use smith_protocol_llm as llm;
use smith_protocol_machine as machine;

#[expect(clippy::too_many_lines, reason = "one exhaustive root-domain request translation")]
pub(super) fn domain_request(service: &mut Service, request: domain::Request) {
    match request {
        domain::Request::Waiting { host_run: _, read } => {
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_waiting(read, token, &mut service.channel_events, &mut service.channel_below)
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::Turn { host_run: _, number, position: _, read, spent, opens_window, turn } => {
            program::settled(service, &turn);
            live_step(service, crate::live::Action::End { turn: number });
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_turn(
                    number,
                    read,
                    spent,
                    opens_window,
                    &turn,
                    token,
                    &mut service.channel_events,
                    &mut service.channel_below,
                )
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::HostCall { host_run: _, relay, name, tool, effect, input, deadline } => {
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_host_call(
                    service.domain_env.now,
                    name,
                    relay,
                    tool,
                    effect,
                    input,
                    deadline,
                    token,
                    &mut service.channel_events,
                    &mut service.channel_below,
                )
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::WithdrawHost { relay } => {
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_withdraw(relay, token, &mut service.channel_events, &mut service.channel_below)
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::MessageRefused { host_run: _, name, reason } => {
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_message_refused(name, reason, token, &mut service.channel_events, &mut service.channel_below)
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::Admitted { host_run: _, run } => {
            service.admitted = Some(run);
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_admitted(token, &mut service.channel_events, &mut service.channel_below)
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::Answer { to, answer, read, detail } => {
            crate::answers::remember(service, &answer, detail, service.domain_env.now);
            assert!(to.into_token() == Token::new(1), "one Start reply right per process");
            if let Some(events) = &mut service.events {
                events.answer(service.domain_env.now, service.run_started, &answer, read);
            }
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_answer(
                    service.domain_env.now,
                    answer,
                    read,
                    token,
                    &mut service.channel_events,
                    &mut service.channel_below,
                )
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
            teardown::answered(service);
        }
        domain::Request::Long { host_run: _, operation, until } => {
            let token = service.next_send();
            let span = until.saturating_since(service.domain_env.now);
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_long(operation, span, token, &mut service.channel_events, &mut service.channel_below)
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::LongDone { host_run: _ } => {
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_long_done(token, &mut service.channel_events, &mut service.channel_below)
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::Deliver { name, deadline, host_run: _, owner, change } => {
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_delivery(
                    service.domain_env.now,
                    name,
                    owner,
                    change,
                    deadline,
                    token,
                    &mut service.channel_events,
                    &mut service.channel_below,
                )
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::Complete {
            owner,
            name,
            grant,
            prompt,
            deadlines,
            max_completion_bytes,
            max_completion_blocks,
            max_failure_bytes,
            decoded_call_bytes,
        } => {
            let observation = match &mut service.events {
                Some(events) => events.request(service.domain_env.now, owner, &prompt),
                None => None,
            };
            let wanted = service.live.wants(&prompt);
            if wanted {
                live_step(service, crate::live::Action::Open { owner, retry: name.retry });
            }
            let deltas = observation.is_some() || wanted;
            let outcome = service.outcome.clone().expect("an admitted Start retains its result contract");
            let request = llm::FromDomain::Complete {
                owner,
                grant,
                prompt,
                deadlines,
                bounds: llm::Receiving {
                    max_completion_bytes,
                    max_completion_blocks,
                    max_failure_bytes,
                    decoded_call_bytes,
                },
                outcome,
                deliver: service.delivery.clone(),
                observation,
                deltas,
            };
            program::complete(service, request);
        }
        domain::Request::Unauthorized { grant } => {
            account_notice(
                service,
                service.domain_env.now,
                smith_events::NoticeKind::CredentialUnauthorized,
                grant.account,
                None,
                None,
            );
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_unauthorized(grant, token, &mut service.channel_events, &mut service.channel_below)
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::Limited { account, reset } => {
            account_notice(
                service,
                service.domain_env.now,
                smith_events::NoticeKind::AccountLimited,
                account,
                reset,
                None,
            );
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_limited(account, reset, token, &mut service.channel_events, &mut service.channel_below)
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::Refused { account, code, reason } => {
            account_notice(
                service,
                service.domain_env.now,
                smith_events::NoticeKind::AccountRefused,
                account,
                None,
                Some(code.clone()),
            );
            let token = service.next_send();
            if service
                .channel
                .as_mut()
                .expect("framed agent owns its channel")
                .send_refused(account, &code, &reason, token, &mut service.channel_events, &mut service.channel_below)
                .is_err()
            {
                service.mark_failed(Failure::Send);
            }
        }
        domain::Request::Cancel { owner } => {
            program::complete(service, llm::FromDomain::Cancel { owner });
        }
        domain::Request::Io { owner, op, deadline } => {
            service.machine.from_domain(
                &service.machine_env,
                machine::FromDomain::Op { owner, op, deadline },
                &mut service.machine_events,
                &mut service.machine_below,
            );
        }
        domain::Request::Job { owner, act } => {
            service.machine.from_domain(
                &service.machine_env,
                machine::FromDomain::Job { owner, act },
                &mut service.machine_events,
                &mut service.machine_below,
            );
        }
        domain::Request::CancelIo { owner } => {
            service.machine.from_domain(
                &service.machine_env,
                machine::FromDomain::Cancel { owner },
                &mut service.machine_events,
                &mut service.machine_below,
            );
        }
        domain::Request::Read { owner, at, max, deadline } => {
            service.machine.from_domain(
                &service.machine_env,
                machine::FromDomain::Read { owner, at, max, deadline },
                &mut service.machine_events,
                &mut service.machine_below,
            );
        }
        domain::Request::Probe { owner, at, deadline } => {
            service.machine.from_domain(
                &service.machine_env,
                machine::FromDomain::Probe { owner, at, deadline },
                &mut service.machine_events,
                &mut service.machine_below,
            );
        }
        domain::Request::Check { owner, program, deadline, tail } => {
            service.machine.from_domain(
                &service.machine_env,
                machine::FromDomain::Check { owner, program, deadline, tail },
                &mut service.machine_events,
                &mut service.machine_below,
            );
        }
        domain::Request::Abort { owner } => {
            service.machine.from_domain(
                &service.machine_env,
                machine::FromDomain::Abort { owner },
                &mut service.machine_events,
                &mut service.machine_below,
            );
        }
    }
}

#[expect(clippy::manual_map, reason = "the strict subset requires explicit matches instead of closures")]
fn account_notice(
    service: &mut Service,
    now: Time,
    kind: smith_events::NoticeKind,
    account: u32,
    reset: Option<Duration>,
    code: Option<Box<[u8]>>,
) {
    if let Some(events) = &mut service.events {
        events.notice(
            now,
            smith_events::Notice {
                level: smith_events::Level::Warning,
                kind,
                run: None,
                account: Some(account),
                reset_ms: match reset {
                    Some(reset) => Some(reset.as_nanos() / 1_000_000),
                    None => None,
                },
                code,
                bytes: None,
                exit: None,
                forced: None,
            },
        );
    }
}
