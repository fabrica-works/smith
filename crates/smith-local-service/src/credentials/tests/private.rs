use super::*;
use skein_fake_machine::{How, Item, Machine, Opened};
use skein_fake_oauth as fake;
use skein_world::Host;
use smith_local_process_world::oauth::{Browser, Peer};

const ACCESS: &[u8] = b"eyJhbGciOiJub25lIn0.eyJleHAiOjEwMDAwLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiYWNjb3VudC1zZXZlbiJ9fQ.c2ln";
const AUTHORIZE: &[u8] = b"http://127.0.0.1:444/authorize";
const ENDPOINT: &[u8] = b"http://127.0.0.1:444/token";
const REDIRECT: &[u8] = b"http://127.0.0.1:2345/callback";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Story {
    SignIn,
    Load,
    Refresh,
    Cancel,
    KeepFailed,
    Unsafe,
    NotSignedIn,
    Interactive,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Until {
    Visit,
    Ready,
    Failed,
    Closed,
    SignedIn,
}
struct Private {
    owner: Accounts,
    sim: skein_sim::Sim,
    pid: skein_sim::Pid,
    peer_pid: skein_sim::Pid,
    browser: Option<(skein_sim::Pid, Browser)>,
    peer: Peer,
    files: Machine,
    root: Opened,
    story: Story,
    outcomes: List<CredentialRequest>,
    syncs: u32,
    renames: u32,
    kept_before_grant: bool,
}
fn native_limits() -> oauth::Limits {
    let mut native = limits();
    native.listeners = 1;
    native.io.sockets = 3;
    native.client.scope_bytes = 128;
    native
}
fn record(expired: bool) -> skein_oauth::SavedToken {
    skein_oauth::SavedToken {
        issued_client_id: None,
        granted_scopes: None,
        subject: None,
        id_token: None,
        earliest_refresh_at: None,
        key: 0,
        generation: 1,
        access_token: ACCESS.into(),
        refresh_token: Some(b"refresh-old".as_slice().into()),
        metadata: Some(b"account-seven".as_slice().into()),
        expires_at: if expired { wall() } else { Wall::from_nanos(300_000_000_000) },
    }
}
fn issuer() -> Peer {
    let mut peer = Peer::new(
        ([127, 0, 0, 1], 444).into(),
        skein_fake_peers::Transport::Plaintext,
        smith_agent_process_world::fake::limits(),
        fake::Config {
            authorization_url: AUTHORIZE.into(),
            token_endpoint: ENDPOINT.into(),
            client_id: b"client".as_slice().into(),
            client_secret: None,
            terms: None,
            redirect_uri: REDIRECT.into(),
            refresh_token: b"refresh-old".as_slice().into(),
        },
        fake::Limits {
            document: native_limits().client.document,
            uri_bytes: 4096,
            request_bytes: 4096,
            codes: 2,
            carried: 0,
            rotations: 2,
            plans: 2,
        },
        skein_http::server::Limits { head: 4096, headers: 16, body: 4096, read: 1024, response: 8192, send: 1024 },
    )
    .expect("fake issuer");
    peer.queue(fake::Plan {
        status: 200,
        body: fake::Body::Token(skein_oauth::TokenResponse {
            scope: None,
            id_token: None,
            earliest_refresh_at: None,
            access_token: ACCESS.into(),
            refresh_token: Some(b"refresh-new".as_slice().into()),
            expires_in: 3600,
        }),
        delay: Duration::ZERO,
        retry_after: Duration::ZERO,
    })
    .expect("one response");
    peer
}

impl Private {
    fn new(story: Story) -> Self {
        Self::configured(story, false)
    }

    fn configured(story: Story, shared: bool) -> Self {
        let native = native_limits();
        let mut files = Machine::for_user(7);
        let mut items = List::with_capacity(2);
        items.push(Item::directory(b"tokens").mode(0o700)).expect("directory fixture");
        if story == Story::Load || story == Story::Refresh || story == Story::Unsafe {
            let encoded = skein_oauth::encode_record(&record(story == Story::Refresh), &native.client.document)
                .expect("synthetic private record");
            items
                .push(Item::file(b"tokens/record", &encoded).mode(if story == Story::Unsafe { 0o644 } else { 0o600 }))
                .expect("record fixture");
        }
        let root = files.lay(items.as_slice());
        let mut sim_config = skein_sim::Config::calm();
        sim_config.wall = wall();
        let mut sim = skein_sim::Sim::new(61, sim_config);
        let pid = sim.spawn_process();
        let peer_pid = sim.spawn_process();
        let descriptor = sim.root(pid, skein_sim::Handle::new(root.raw()));
        let mut sources = List::with_capacity(1);
        assert!(
            sources
                .push(Source::SignIn {
                    account: 7,
                    root: descriptor,
                    user: 7,
                    directory: b"tokens".as_slice().into(),
                    file: b"record".as_slice().into(),
                    registration: skein_oauth::Registration {
                        terms: None,
                        authorization_url: AUTHORIZE.into(),
                        token_endpoint: ENDPOINT.into(),
                        client_id: b"client".as_slice().into(),
                        redirect_uri: REDIRECT.into(),
                        scope: b"read".as_slice().into(),
                        wire: skein_oauth::WireFormat::Form,
                        client_secret: None,
                        pkce_for_confidential: false,
                        metadata_claim: Some(skein_oauth::ClaimSelector {
                            object: b"https://api.openai.com/auth".as_slice().into(),
                            field: b"chatgpt_account_id".as_slice().into(),
                        }),
                    },
                    endpoint: oauth::Endpoint {
                        address: kernel::Addr::from(([127, 0, 0, 1], 444)),
                        transport: oauth::Transport::Plaintext
                    },
                    account_id: Box::new([]),
                    interactive: story == Story::Interactive,
                })
                .is_ok()
        );
        let mut owner =
            if shared { Accounts::shared(sources, native, [61; 32]) } else { Accounts::new(sources, native, [61; 32]) }
                .expect("private driver");
        if story == Story::Load || story == Story::Refresh || story == Story::NotSignedIn || story == Story::Interactive
        {
            assert!(owner.request(7));
        } else {
            assert!(owner.sign_in(7));
        }

        Self {
            owner,
            sim,
            pid,
            peer_pid,
            browser: None,
            peer: issuer(),
            files,
            root,
            story,
            outcomes: List::with_capacity(16),
            syncs: 0,
            renames: 0,
            kept_before_grant: false,
        }
    }
    fn saved(&mut self) -> skein_oauth::SavedToken {
        let file = self.files.open(self.root, b"tokens/record", How::Read).expect("kept private record");
        let bytes = self.files.read(file, 0, 1024).expect("private encoding");
        self.files.close(file);
        skein_oauth::decode_record(&bytes, &native_limits().client.document).expect("native private encoding")
    }
    fn step(&mut self) {
        self.step_before(None);
    }
    fn step_before(&mut self, ceiling: Option<Time>) {
        let now = self.sim.now();
        let wall = self.sim.wall();
        self.sim.reap(self.peer_pid, self.peer.completions());
        self.peer.iterate(now, wall);
        self.sim.submit(self.peer_pid, self.peer.submissions());
        if let Some((pid, browser)) = &mut self.browser {
            self.sim.reap(*pid, browser.completions());
            browser.iterate(now, wall);
            self.sim.submit(*pid, browser.submissions());
        }
        let mut completions = Queue::with_capacity(64);
        self.sim.reap(self.pid, &mut completions);
        while let Some(complete) = completions.pop() {
            self.owner.complete(complete);
        }
        let mut out = Queue::with_capacity(ACCOUNTS_MAX_OUT);
        self.owner.advance(now, wall, &mut out);
        while let Some(outcome) = out.pop() {
            match &outcome {
                CredentialRequest::Visit { url } if self.story != Story::Cancel && self.story != Story::Unsafe => {
                    self.browser = Some((self.sim.spawn_process(), Browser::new(url)));
                }
                CredentialRequest::SignedIn { .. } => {
                    if !self.owner.entries.get(0).expect("account").wanted {
                        assert!(self.owner.request(7));
                    }
                }
                CredentialRequest::Ready { value, .. } => {
                    self.kept_before_grant = self.saved().access_token == *value
                        && (self.story == Story::Load || (self.syncs >= 2 && self.renames == 1));
                }
                CredentialRequest::Closed
                | CredentialRequest::Notice { .. }
                | CredentialRequest::Visit { .. }
                | CredentialRequest::Failed { .. } => {}
            }
            assert!(self.outcomes.push(outcome).is_ok(), "bounded outcomes");
        }
        self.sim.submit(self.pid, self.owner.submissions());
        let mut calls = Queue::with_capacity(64);
        let mut answers = Queue::with_capacity(64);
        self.sim.calls(&mut calls);
        while let Some(call) = calls.pop() {
            let failed = match &call.ask {
                skein_sim::Ask::Sync { .. } => {
                    self.syncs = self.syncs.checked_add(1).expect("sync count");
                    false
                }
                skein_sim::Ask::Rename { .. } => {
                    self.renames = self.renames.checked_add(1).expect("rename count");
                    false
                }
                skein_sim::Ask::Write { .. } => self.story == Story::KeepFailed,
                skein_sim::Ask::Spawn { .. }
                | skein_sim::Ask::Lock { .. }
                | skein_sim::Ask::Open { .. }
                | skein_sim::Ask::Read { .. }
                | skein_sim::Ask::Append { .. }
                | skein_sim::Ask::Truncate { .. }
                | skein_sim::Ask::Stat { .. }
                | skein_sim::Ask::Remove { .. }
                | skein_sim::Ask::MakeDirectory { .. }
                | skein_sim::Ask::List { .. }
                | skein_sim::Ask::Close { .. } => false,
            };
            if failed {
                answers.push(skein_sim::Answer { ticket: call.ticket, result: Err(kernel::Error::NoSpace) });
            } else {
                skein_fake_machine::step(&mut self.files, call, &mut answers);
            }
        }
        self.sim.answer(&mut answers);
        let browser_pending = match &self.browser {
            Some((pid, browser)) => browser.work_pending(now) || self.sim.ready(*pid) > 0,
            None => false,
        };
        let pending = self.owner.work_pending(now)
            || self.peer.work_pending(now)
            || self.sim.ready(self.pid) > 0
            || self.sim.ready(self.peer_pid) > 0
            || browser_pending;
        if !pending {
            let browser_deadline = match &self.browser {
                Some((_, browser)) => browser.next_deadline(),
                None => None,
            };
            if let Some(next) =
                [self.sim.next_due(), self.owner.next_deadline(), self.peer.next_deadline(), browser_deadline]
                    .into_iter()
                    .flatten()
                    .min()
            {
                let next = match ceiling {
                    Some(ceiling) => next.min(ceiling),
                    None => next,
                };
                self.sim.advance_to(next);
            }
        }
    }
    fn count(&self, until: Until) -> u32 {
        let mut count = 0_u32;
        for outcome in &self.outcomes {
            let kind = match outcome {
                CredentialRequest::Visit { .. } => Some(Until::Visit),
                CredentialRequest::Ready { .. } => Some(Until::Ready),
                CredentialRequest::Failed { .. } => Some(Until::Failed),
                CredentialRequest::Closed => Some(Until::Closed),
                CredentialRequest::SignedIn { .. } => Some(Until::SignedIn),
                CredentialRequest::Notice { .. } => None,
            };
            if kind == Some(until) {
                count = count.checked_add(1).expect("bounded outcomes");
            }
        }
        count
    }
    fn until(&mut self, until: Until) {
        for _ in 0..4000_u32 {
            self.step();
            if self.count(until) > 0 {
                return;
            }
        }
        panic!("native routing did not reach its terminal");
    }
    fn close(&mut self) {
        self.owner.close(smith_agent_service::Close::Graceful, self.sim.now(), self.sim.wall());
        self.until(Until::Closed);
        assert!(self.owner.closed());
        assert!(self.owner.next_deadline().is_none());
    }
}

#[test]
fn a_private_sign_in_lends_only_after_the_native_record_is_durable() {
    let mut world = Private::new(Story::SignIn);
    world.until(Until::Ready);
    assert!(world.kept_before_grant);
    assert_eq!(world.saved().refresh_token.as_deref(), Some(b"refresh-new".as_slice()));
    world.close();
    assert!(world.browser.as_ref().expect("visited browser").1.saw_reply());
}

#[test]
fn a_private_refresh_keeps_its_rotation_before_lending_the_new_grant() {
    let mut world = Private::new(Story::Refresh);
    world.until(Until::Ready);
    assert!(world.kept_before_grant);
    assert_eq!(world.saved().generation, 2);
    assert_eq!(world.count(Until::Visit), 0);
    world.close();
}

#[test]
fn a_loaded_private_grant_supplies_its_account_id_from_the_claims() {
    let mut world = Private::new(Story::Load);
    world.until(Until::Ready);
    assert_eq!(world.owner.account_id(7), Some(b"account-seven".as_slice()));
    assert_eq!(world.renames, 0);
    world.close();
}

#[test]
fn closing_a_private_sign_in_cancels_the_waiting_browser_before_close() {
    let mut world = Private::new(Story::Cancel);
    world.until(Until::Visit);
    let before = world.sim.now();
    world.close();
    assert!(world.sim.now().as_nanos().saturating_sub(before.as_nanos()) < Duration::from_secs(1).as_nanos());
    assert_eq!(world.count(Until::Failed), 1);
    assert_eq!(world.count(Until::SignedIn), 0);
    assert_eq!(world.count(Until::Ready), 0);
}

#[test]
fn a_private_keep_failure_lends_no_token() {
    let mut world = Private::new(Story::KeepFailed);
    world.until(Until::Failed);
    assert_eq!(world.count(Until::Ready), 0);
    assert_eq!(world.count(Until::SignedIn), 0);
    world.close();
}

#[test]
fn an_unsafe_private_record_is_refused_before_a_visit() {
    let mut world = Private::new(Story::Unsafe);
    world.until(Until::Failed);
    assert_eq!(world.count(Until::Visit), 0);
    world.close();
}

#[test]
fn cancelling_a_queued_private_file_request_emits_its_terminal_without_starting_it() {
    let mut world = Private::new(Story::Cancel);
    let mut out = Queue::with_capacity(ACCOUNTS_MAX_OUT);
    world.owner.advance(world.sim.now(), world.sim.wall(), &mut out);
    assert!(!world.owner.files.takes());
    let owner = Token::new(99);
    world.owner.file_requests.push(oauth::FileLower::Request {
        request: file::Request::Load {
            owner,
            root: Token::new(1),
            path: b"never-opened".as_slice().into(),
            max: 1024,
            no_follow: true,
        },
        deadline: world.sim.now().saturating_add(Duration::from_secs(30)),
    });
    world.owner.file_requests.push(oauth::FileLower::Cancel { owner });
    world.owner.private_files(world.sim.now(), world.sim.wall());
    assert_eq!(world.owner.events.pop(), Some(file::Event::Cancelled { owner }));
    assert!(world.owner.file_requests.is_empty());
    world.close();
}

#[test]
fn an_abort_after_close_settles_the_private_sign_in_once() {
    let mut world = Private::new(Story::Cancel);
    world.until(Until::Visit);
    world.owner.close(smith_agent_service::Close::Graceful, world.sim.now(), world.sim.wall());
    world.step();
    world.owner.close(smith_agent_service::Close::Aborted, world.sim.now(), world.sim.wall());
    world.until(Until::Closed);
    assert!(world.owner.closed());
    assert_eq!(world.count(Until::Failed), 1);
    assert_eq!(world.count(Until::Closed), 1);
    assert!(world.owner.next_deadline().is_none());
}

#[test]
fn a_missing_private_record_starts_no_sign_in_without_the_owners_permission() {
    let mut world = Private::new(Story::NotSignedIn);
    world.until(Until::Failed);
    assert_eq!(world.count(Until::Visit), 0);
    assert_eq!(world.count(Until::Ready), 0);
    world.close();
}

#[test]
fn an_interactive_private_grant_waits_for_sign_in_and_keeping() {
    let mut world = Private::new(Story::Interactive);
    world.until(Until::Ready);
    assert_eq!(world.count(Until::Visit), 1);
    assert!(world.kept_before_grant);
    world.close();
}

#[path = "lock.rs"]
mod shared_tests;
