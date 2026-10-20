use super::*;

#[test]
fn each_ending_answers_as_hosts_md_says() {
    let detail = b"stderr only";
    for ask in [SourceAsk::Check, SourceAsk::SignOut] {
        for (ended, reason) in [
            (Ended::Unspawned { detail }, FailureReason::Missing),
            (Ended::Deadline { detail }, FailureReason::Deadline),
            (
                Ended::Exited { exit: kernel::Exit::Code(2), stdout: Some(b"stdout secret"), detail },
                FailureReason::Exited,
            ),
            (
                Ended::Exited { exit: kernel::Exit::Signal(9), stdout: Some(b"stdout secret"), detail },
                FailureReason::Exited,
            ),
        ] {
            assert_eq!(translate(ask, ended), failure(ask, reason, detail));
        }
    }
    for (code, stdout, expected) in [
        (0, Some(br#"{"loggedIn":true,"authMethod":"claude.ai"}"#.as_slice()), ProgramCheck::Usable),
        (
            0,
            Some(br#"{"loggedIn":true,"authMethod":"api_key"}"#.as_slice()),
            ProgramCheck::SignedInOtherwise(ProgramMethod::new(b"api_key").expect("method")),
        ),
        (0, Some(br#"{"loggedIn":false,"authMethod":"none"}"#.as_slice()), ProgramCheck::NotSignedIn),
        (1, Some(b"whatever".as_slice()), ProgramCheck::NotSignedIn),
        (
            0,
            Some(b"not json".as_slice()),
            ProgramCheck::Failed(ProgramFailure {
                reason: FailureReason::Unreadable,
                detail: detail.as_slice().into(),
            }),
        ),
        (
            0,
            None,
            ProgramCheck::Failed(ProgramFailure {
                reason: FailureReason::Unreadable,
                detail: detail.as_slice().into(),
            }),
        ),
        (
            1,
            None,
            ProgramCheck::Failed(ProgramFailure {
                reason: FailureReason::Unreadable,
                detail: detail.as_slice().into(),
            }),
        ),
    ] {
        assert_eq!(
            translate(SourceAsk::Check, Ended::Exited { exit: kernel::Exit::Code(code), stdout, detail }),
            SourceAnswer::Check(expected)
        );
    }
    for stdout in [Some(b"ignored".as_slice()), None] {
        assert_eq!(
            translate(SourceAsk::SignOut, Ended::Exited { exit: kernel::Exit::Code(0), stdout, detail }),
            SourceAnswer::SignOut(ProgramSignOut::SignedOut)
        );
        assert_eq!(
            translate(SourceAsk::SignOut, Ended::Exited { exit: kernel::Exit::Code(1), stdout, detail }),
            failure(SourceAsk::SignOut, FailureReason::Exited, detail)
        );
    }
}

#[test]
fn cancellation_answers_only_after_resources_settle() {
    use skein_lib::Token;
    let owner = Token::new(1);
    let mut below = Queue::with_capacity(32);
    let mut source = ProgramSource::new(
        owner,
        SourceAsk::Check,
        &Programs { directory: b"/programs".as_slice().into(), claude: b"/claude".as_slice().into() },
        SourceName::new(b"work").unwrap(),
        &[],
        kernel::Fd::new(3),
        b".".as_slice().into(),
        Time::from_nanos(100),
        1024,
        &mut below,
    )
    .unwrap();
    below.pop().unwrap();
    let pipes = [Token::new(7), Token::new(8)];
    source.from_io(IoEvent::Spawned { owner, child: Token::new(9), pipes: Box::new(pipes) }, &mut below);
    while below.pop().is_some() {}
    source.cancel(&mut below);
    source.cancel(&mut below);
    assert_eq!(below.len(), 3);
    let Some(IoRequest::Signal { signal: kernel::Signal::Kill, .. }) = below.pop() else {
        panic!("cancel kills the child")
    };
    for pipe in pipes {
        let Some(IoRequest::Abort { entity }) = below.pop() else { panic!("cancel aborts each output") };
        assert_eq!(entity, pipe);
    }
    assert_eq!(source.answer(), None);
    source.from_io(IoEvent::Closed { owner }, &mut below);
    source.from_io(IoEvent::Closed { owner: pipes[0] }, &mut below);
    assert_eq!(source.answer(), None);
    source.from_io(IoEvent::Closed { owner: pipes[1] }, &mut below);
    assert_eq!(
        source.answer(),
        Some(SourceAnswer::Check(ProgramCheck::Failed(ProgramFailure {
            reason: FailureReason::Deadline,
            detail: Box::new([])
        })))
    );
    assert_eq!(source.answer(), None);
    source.cancel(&mut below);
    assert!(below.is_empty());
}
