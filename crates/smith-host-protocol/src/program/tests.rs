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
