#[global_allocator]
static HEAP: skein_heap::Counting = skein_heap::Counting;

#[cfg(test)]
mod tests {
    use smith_fake_claude::{Status, Story};
    use smith_host_domain::{FailureReason, ProgramCheck, ProgramMethod, ProgramSignOut};
    use smith_host_protocol::{SourceAnswer, SourceAsk};
    use smith_hosts_world::program_world::{Run, run};

    fn check(login: Option<&[u8]>, story: Option<Story>) -> Run {
        run(7, story, login, &[SourceAsk::Check], false, &[])
    }

    fn failed(run: &Run, expected: FailureReason) {
        let SourceAnswer::Check(ProgramCheck::Failed(failure)) = run.answers.first().expect("one answer") else {
            unreachable!("failed check")
        };
        assert_eq!(failure.reason, expected);
        assert!(failure.detail.is_empty(), "no stdout in failure detail");
    }

    #[test]
    fn a_source_never_signed_in_is_not_signed_in() {
        assert_eq!(check(None, None).answers, [SourceAnswer::Check(ProgramCheck::NotSignedIn)]);
    }

    #[test]
    fn a_source_signed_in_is_usable() {
        let first = check(Some(b"claude.ai"), None);
        assert_eq!(first.answers, [SourceAnswer::Check(ProgramCheck::Usable)]);
        assert_eq!(first.trace, check(Some(b"claude.ai"), None).trace, "same seed replays the source world");
    }

    #[test]
    fn a_source_signed_in_otherwise_names_how() {
        assert_eq!(
            check(Some(b"api_key"), None).answers,
            [SourceAnswer::Check(ProgramCheck::SignedInOtherwise(ProgramMethod::new(b"api_key").expect("method")))]
        );
    }

    #[test]
    fn a_silent_status_fails_at_the_deadline() {
        failed(&check(None, Some(Story { status: Status::Silent, ..Story::default() })), FailureReason::Deadline);
    }

    #[test]
    fn an_unreadable_status_fails_as_unreadable() {
        failed(&check(None, Some(Story { status: Status::Unreadable, ..Story::default() })), FailureReason::Unreadable);
    }

    #[test]
    fn a_missing_program_fails_as_missing() {
        failed(&run(7, None, None, &[SourceAsk::Check], true, &[]), FailureReason::Missing);
    }

    #[test]
    fn signing_out_removes_the_login() {
        let run = run(7, None, Some(b"claude.ai"), &[SourceAsk::SignOut, SourceAsk::Check], false, &[]);
        assert_eq!(
            run.answers,
            [SourceAnswer::SignOut(ProgramSignOut::SignedOut), SourceAnswer::Check(ProgramCheck::NotSignedIn)]
        );
        assert_eq!(run.login, None);
    }

    #[test]
    fn the_status_never_sees_another_login_s_variables() {
        let run =
            run(7, None, Some(b"claude.ai"), &[SourceAsk::Check], false, &[b"ANTHROPIC_API_KEY=x".as_slice().into()]);
        assert_eq!(run.answers, [SourceAnswer::Check(ProgramCheck::Usable)]);
        assert_eq!(run.observations.len(), 1);
        assert_eq!(run.observations.first().expect("one observation").arguments, ["auth", "status"]);
        assert_eq!(run.observations.first().expect("one observation").environment, ["HOME", "CLAUDE_CONFIG_DIR"]);
    }
}
