//! Source ownership checks through the root boundary; provider terminals are
//! whole completions, so a failed retry supplies no captured partial text.

use super::*;
use crate::llm::{CompletionName, Evidence, Failure};

pub(super) fn named(emitted: Box<[Request]>) -> (Token, Prompt, CompletionName) {
    let [Request::Complete { owner, name, .. }] = &*emitted else {
        panic!("expected one completion, got {emitted:?}");
    };
    let name = *name;
    let expected_owner = *owner;
    let (owner, prompt) = completing(emitted);
    assert_eq!(owner, expected_owner);
    (owner, prompt, name)
}

fn retry(harness: &mut Harness, owner: Token) -> CompletionName {
    assert!(
        harness
            .step(Event::Failed {
                owner,
                failure: Failure::Unavailable,
                evidence: Evidence::Unknown,
                detail: bytes(b"failed provider"),
            })
            .is_empty()
    );
    harness.env.now = harness.domain.next_deadline().expect("actual transient backoff");
    let (reissued, _, name) = named(harness.fire());
    assert_eq!(reissued, owner, "retries retain their session owner");
    name
}

#[test]
fn no_text_retry_starts_keep_the_owner_sequence_and_number_with_queued_native_facts() {
    let mut harness = Harness::new();
    let (_, owner, _, first) = harness.admit_named(7, charter());
    assert_eq!(first, CompletionName { sequence: 1, retry: 0 });
    assert!(harness.domain.pop_content().is_none(), "a start need not produce text");
    let later = retry(&mut harness, owner);
    assert_eq!(later, CompletionName { sequence: 1, retry: 1 });
    assert!(harness.domain.pop_content().is_none(), "retry start is explicit without text");
    assert!(harness.domain.has_facts(), "native facts remain queued independently of the retry name");
    drop(harness.says(owner, b"retry answer"));
    assert_eq!(
        harness.domain.pop_content(),
        Some(crate::Content::Text { owner, completion: later, text: bytes(b"retry answer") })
    );
    assert!(harness.domain.has_facts());
}

#[test]
fn queued_text_calls_and_usage_keep_the_ended_completion_after_the_next_retry_starts() {
    let mut harness = Harness::new();
    let (_, owner, _, first) = harness.admit_named(7, charter());
    let invalid = Said::ToolCall {
        id: bytes(b"provider-id"),
        name: bytes(b"unknown"),
        input: bytes(b"{}"),
        call: Decoded::Invalid { problem: Problem::UnknownTool },
        replay: None,
    };
    assert!(
        harness
            .answer(owner, Box::new([Said::Text { text: bytes(b"earlier prefix"), replay: None }, invalid,]))
            .is_empty()
    );
    let (next_owner, _, next) = named(harness.next());
    assert_eq!(next_owner, owner);
    assert_eq!(next, CompletionName { sequence: 2, retry: 0 });
    let retried = retry(&mut harness, owner);
    assert_eq!(retried, CompletionName { sequence: 2, retry: 1 });
    assert_eq!(
        harness.domain.pop_content(),
        Some(crate::Content::Text { owner, completion: first, text: bytes(b"earlier prefix") })
    );
    assert_eq!(
        harness.domain.pop_content(),
        Some(crate::Content::Call {
            owner,
            completion: first,
            id: bytes(b"provider-id"),
            name: bytes(b"unknown"),
            input: bytes(b"{}"),
        })
    );
    assert_eq!(harness.domain.pop_content(), Some(crate::Content::Usage { owner, completion: first, usage: USAGE }));
    drop(harness.says(owner, b"different next turn"));
    assert_eq!(
        harness.domain.pop_content(),
        Some(crate::Content::Text { owner, completion: retried, text: bytes(b"different next turn") })
    );
    assert_eq!(harness.domain.pop_content(), Some(crate::Content::Usage { owner, completion: retried, usage: USAGE }));
    assert!(harness.domain.pop_content().is_none());
    assert_eq!(
        harness.turns.iter().next().expect("the first concrete turn").sequence,
        first.sequence,
        "concrete turn identity is independent of retry"
    );
}

#[test]
fn empty_text_and_a_completion_without_text_retain_their_retry_identity() {
    let completions: [Box<[Said]>; 2] = [Box::new([]), Box::new([Said::Text { text: bytes(b""), replay: None }])];
    for content in completions {
        let has_text = !content.is_empty();
        let mut harness = Harness::new();
        let (_, owner, _, _) = harness.admit_named(7, charter());
        let name = retry(&mut harness, owner);
        drop(harness.step(Event::Completed {
            owner,
            completion: Completion { content, stop: Stop::EndTurn, usage: USAGE, reasoning_dropped: Box::default() },
        }));
        if has_text {
            assert_eq!(
                harness.domain.pop_content(),
                Some(crate::Content::Text { owner, completion: name, text: bytes(b"") })
            );
        }
        assert_eq!(harness.domain.pop_content(), Some(crate::Content::Usage { owner, completion: name, usage: USAGE }));
        assert!(harness.domain.pop_content().is_none());
    }
}

#[test]
fn nested_sessions_name_their_own_retries_without_renaming_the_parent_content() {
    let mut harness = Harness::new();
    let (_, main, _, parent) = harness.admit_named(7, charter());
    let (child, _, child_first) = named(harness.answer(main, Box::new([served(b"child", sub_agent(b"Find it."))])));
    assert_ne!(child, main);
    assert_eq!(child_first, CompletionName { sequence: 1, retry: 0 });
    let child_retry = retry(&mut harness, child);
    assert_eq!(child_retry, CompletionName { sequence: 1, retry: 1 });
    drop(harness.says(child, b"child answer"));
    let Some(crate::Content::Call { owner, completion, .. }) = harness.domain.pop_content() else {
        panic!("parent provider call");
    };
    assert_eq!((owner, completion), (main, parent));
    assert_eq!(
        harness.domain.pop_content(),
        Some(crate::Content::Usage { owner: main, completion: parent, usage: USAGE })
    );
    assert_eq!(
        harness.domain.pop_content(),
        Some(crate::Content::Text { owner: child, completion: child_retry, text: bytes(b"child answer") })
    );
    assert_eq!(
        harness.domain.pop_content(),
        Some(crate::Content::Usage { owner: child, completion: child_retry, usage: USAGE })
    );
    assert!(harness.next().is_empty());
    let (owner, _, next_parent) = named(harness.next());
    assert_eq!(owner, main);
    assert_eq!(next_parent, CompletionName { sequence: 2, retry: 0 });
}

#[test]
fn retry_exhaustion_ends_the_run_without_another_start_or_content() {
    let mut harness = Harness::new();
    let (_, owner, _, _) = harness.admit_named(7, charter());
    assert_eq!(retry(&mut harness, owner).retry, LIMITS.session.retries);
    let emitted = harness.step(Event::Failed {
        owner,
        failure: Failure::Unavailable,
        evidence: Evidence::Unknown,
        detail: bytes(b"failed again"),
    });
    let [Request::Answer { answer: run::Answer::Failed { .. }, .. }] = &*emitted else {
        panic!("retry exhaustion ends the run, got {emitted:?}");
    };
    assert!(harness.next().is_empty());
    assert!(harness.domain.pop_content().is_none());
}

#[test]
fn dropping_all_content_does_not_drop_or_default_the_completion_start_name() {
    let mut harness = Harness::new();
    harness.domain.content = Queue::with_capacity(0);
    let (_, owner, _, first) = harness.admit_named(7, charter());
    assert_eq!(first, CompletionName { sequence: 1, retry: 0 });
    let retry_name = retry(&mut harness, owner);
    assert_eq!(retry_name, CompletionName { sequence: 1, retry: 1 });
    let (next_owner, _, next) = named(harness.says(owner, b"dropped content"));
    assert_eq!(next_owner, owner);
    assert_eq!(next, CompletionName { sequence: 2, retry: 0 });
    assert!(harness.domain.has_facts(), "native facts survive omitted content observations");
    assert!(harness.domain.pop_content().is_none());
    assert!(harness.domain.content_lost() > 0);
}

#[test]
fn a_completion_winning_its_cancel_keeps_the_issued_retry_name() {
    let mut harness = Harness::new();
    let (run, owner, _, _) = harness.admit_named(7, charter());
    let name = retry(&mut harness, owner);
    assert!(harness.step(Event::Cancel { run }).is_empty());
    assert_eq!(&*harness.next(), &[Request::Cancel { owner }]);
    let ended = harness.says(owner, b"received after cancel");
    let [Request::Answer { .. }] = &*ended else {
        panic!("the cancelled run ends after the provider terminal, got {ended:?}");
    };
    assert_eq!(
        harness.domain.pop_content(),
        Some(crate::Content::Text { owner, completion: name, text: bytes(b"received after cancel") })
    );
    assert_eq!(harness.domain.pop_content(), Some(crate::Content::Usage { owner, completion: name, usage: USAGE }));
}

#[test]
fn a_restored_prefix_names_the_next_source_sequence_without_changing_the_retry() {
    let mut original = Harness::new();
    let (_, owner, _, _) = original.admit_named(7, charter());
    drop(original.says(owner, b"saved answer"));
    let saved = original.turns.iter().next().expect("actual emitted concrete turn").clone();
    let sequence = saved.sequence.checked_add(1).expect("next source sequence");
    let transcript = crate::Transcript {
        version: saved.version,
        endpoint: saved.endpoint,
        dialect: saved.dialect,
        turns: Box::new([saved]),
    };
    let mut restored = Harness::new();
    let emitted = restored.step(Event::Start {
        messages: Box::default(),
        window: crate::Window { turns: u32::MAX, bytes: u64::MAX },
        answered: Box::default(),
        workspace: None,
        grants: Box::new([crate::Grant {
            name: crate::GrantName { account: 0, generation: 0 },
            valid: Duration::from_secs(100_000),
        }]),
        reply_to: ReplyTo::new(Token::new(8)),
        host_run: Token::new(8),
        activation: 2,
        charter: Charter { resume: true, ..charter() },
        transcript: Some(transcript),
    });
    let [Request::Admitted { .. }, Request::Complete { owner, name, .. }] = &*emitted else {
        panic!("restored main completion, got {emitted:?}");
    };
    assert_eq!(*name, CompletionName { sequence, retry: 0 });
    let retried = retry(&mut restored, *owner);
    assert_eq!(retried, CompletionName { sequence, retry: 1 });
}
