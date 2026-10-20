use super::*;

pub(super) const CHILD: Token = Token::new(7);
const OUT: Token = Token::new(8);
const ERR: Token = Token::new(9);

pub(super) struct Fixture {
    pub(super) checks: Checks,
    pub(super) below: Queue<io::Request>,
    pub(super) events: Queue<local::Event>,
    pub(super) shell: Queue<local::Request>,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let mut accounts = List::with_capacity(3);
        for number in 1..=3 {
            accounts.push(Account { number, source: SourceName::new(b"work").unwrap(), covered: true }).unwrap();
        }
        Self {
            checks: Checks {
                accounts,
                waiting: Queue::with_capacity(3),
                programs: Programs { directory: b"/programs".as_slice().into(), claude: b"/claude".as_slice().into() },
                base: Box::new([]),
                root: Some(kernel::Fd::new(3)),
                root_closed: false,
                detail_bytes: 1024,
                duration: Duration::from_secs(300),
                next_owner: 1 << 60,
                active: None,
                interrupted: 0,
            },
            below: Queue::with_capacity(64),
            events: Queue::with_capacity(64),
            shell: Queue::with_capacity(64),
        }
    }
    pub(super) fn request(&mut self, account: u32) {
        assert!(self.checks.request(account, Time::ZERO, &mut self.below, &mut self.shell));
    }
    pub(super) fn advance(&mut self) {
        self.checks.advance(Time::ZERO, &mut self.below, &mut self.events, &mut self.shell, true);
    }
    pub(super) fn spawn(&mut self) {
        let Some(io::Request::Spawn { owner, .. }) = self.below.pop() else { panic!("check spawn") };
        self.checks.receive(io::Event::Spawned { owner, child: CHILD, pipes: Box::new([OUT, ERR]) }, &mut self.below);
        while self.below.pop().is_some() {}
    }
    pub(super) fn shell_account(&mut self, expected: u32) {
        let Some(local::Request::Credential { account }) = self.shell.pop() else { panic!("shell credential request") };
        assert_eq!(account, expected);
    }
    pub(super) fn refused(&mut self, expected: u32, expected_reason: local::CredentialFailure) {
        let Some(local::Event::NoCredential { account, reason }) = self.events.pop() else {
            panic!("credential failure")
        };
        assert_eq!(account, expected);
        assert_eq!(reason, expected_reason);
    }
    pub(super) fn finish(&mut self, output: &[u8], code: u8) {
        let owner = Token::new(self.checks.next_owner.checked_sub(1).unwrap());
        self.checks.receive(io::Event::Stream { owner: OUT, up: stream::Up::Bytes(output.into()) }, &mut self.below);
        self.checks.receive(io::Event::Exited { owner, exit: kernel::Exit::Code(code) }, &mut self.below);
        self.checks.receive(io::Event::Closed { owner }, &mut self.below);
        self.checks.receive(io::Event::Closed { owner: OUT }, &mut self.below);
        self.advance();
        assert!(self.checks.active.is_some(), "stderr must settle too");
        self.checks.receive(io::Event::Closed { owner: ERR }, &mut self.below);
        while self.below.pop().is_some() {}
        self.advance();
    }
}

pub(super) fn service() -> crate::Service {
    let mut config = crate::tests::config();
    config.limits.local.smith.accounts = 1;
    let mut service = crate::Service::new(config, 7).unwrap();
    let mut sources = List::with_capacity(1);
    sources.push((7, SourceName::new(b"work").unwrap())).unwrap();
    service
        .handed_programs(
            sources,
            Programs { directory: b"/programs".as_slice().into(), claude: b"/claude".as_slice().into() },
            Box::new([]),
            kernel::Fd::new(3),
            1024,
            Duration::from_secs(300),
        )
        .unwrap();
    service.account_request(7);
    service.shell.pop().unwrap();
    service
}
