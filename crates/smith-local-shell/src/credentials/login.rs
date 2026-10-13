//! One login drives the existing account owner through its terminal and close.
//! Its words retain account names and credential state, never token values.
use super::{Account, Credential, Source};
use crate::{command::Command, settings};
use skein_io::kernel;
use skein_lib::{List, Queue, Time, Wall};
use skein_shell::{Clock, Host};
use smith_local_service::{ACCOUNTS_MAX_OUT, Accounts, CredentialRequest};

/// The named account's usable or unavailable terminal after one login.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginOutcome {
    /// The driver kept a usable private record, or the handed-in source is usable.
    Usable,
    /// The selected source could supply no usable credential.
    Unavailable,
}

impl LoginOutcome {
    /// The product's credential exit code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Usable => 0,
            Self::Unavailable => 3,
        }
    }
}

/// Render the account's next action without its access value.
#[must_use]
pub fn credential_words(account: &str, credential: &Credential) -> String {
    credential.line(account)
}

/// Handle only login arguments; the other commands retain their owners.
#[must_use]
pub fn command(arguments: &[std::ffi::OsString], clock: &Clock) -> Option<u8> {
    let Ok(Command::Login { shared, account }) = crate::command::parse(arguments.iter().skip(1).cloned().collect())
    else {
        return None;
    };
    Some(match run(&shared, &account, clock) {
        Ok(outcome) => outcome.code(),
        Err(why) => {
            eprintln!("smith: {why}");
            2
        }
    })
}

fn run(shared: &crate::command::Shared, name: &str, clock: &Clock) -> Result<LoginOutcome, String> {
    let settings = settings::read(shared).map_err(|why| why.line())?;
    let prefix = format!("accounts.{}.", settings::name(name));
    let fields = settings
        .values
        .iter()
        .filter_map(|(key, value)| key.strip_prefix(&prefix).map(|field| (field.into(), value.clone())))
        .collect();
    let account = Account { name: name.into(), number: 0, fields };
    if !account.fields.contains_key("source") {
        return Err(format!("unknown account {name}"));
    }
    let codex = settings.values.iter().any(|(key, value)| {
        key.starts_with("endpoints.")
            && key.ends_with(".account")
            && value.value.as_str() == Some(name)
            && key.strip_suffix("account").is_some_and(|prefix| {
                ["dialect", "preset"].iter().any(|field| {
                    settings.values.get(&format!("{prefix}{field}")).and_then(|value| value.value.as_str())
                        == Some("codex")
                })
            })
    });
    let source = super::source(&account.fields, &settings.workspace, codex, name);
    match source {
        Source::Borrow { .. } | Source::Env { .. } => {
            let credential = super::inspect(&source, clock.now().wall);
            if let Source::Borrow { file, .. } = &source {
                eprintln!("account {name}: reads the login from {}", file.display());
            }
            eprintln!("{}", credential_words(name, &credential));
            Ok(if credential.usable() { LoginOutcome::Usable } else { LoginOutcome::Unavailable })
        }
        Source::SignIn { file } => {
            let registered = super::registration(&account, codex, &settings.workspace).map_err(|why| why.line())?;
            let source = super::sign_in::private_source(0, &file, registered, false)?;
            let io = skein_io::Limits {
                sockets: 3,
                names: 0,
                refusals: 4,
                intake: 32_768,
                receive: 4096,
                output: 32_768,
                sends: 8,
                accepts: 1,
                backlog: 2,
                close_timeout: skein_lib::Duration::from_secs(1),
                retry: skein_lib::Duration::from_millis(10),
            };
            let limits = super::limits(io, 1);
            let mut sources = List::with_capacity(1);
            assert!(sources.push(source).is_ok(), "one login source");
            let entropy = crate::startup::oauth_entropy()?;
            let mut driver =
                Accounts::new(sources, limits, entropy).map_err(|why| format!("account driver: {why:?}"))?;
            assert!(driver.sign_in(0), "selected sign-in account");
            let operations = skein_io::operations(&limits.io)
                .and_then(|operations| operations.checked_add(64))
                .ok_or("login operation bound overflows")?;
            let mut kernel = skein_shell::Kernel::open(skein_shell::Config { operations })
                .map_err(|why| format!("login kernel: {why}"))?;
            let worst = smith_local_service::accounts_worst_case(&limits)
                .and_then(|bytes| bytes.checked_add(Queue::<CredentialRequest>::worst_case(ACCOUNTS_MAX_OUT)?))
                .and_then(|bytes| bytes.checked_add(Queue::<kernel::Complete>::worst_case(8)?))
                .and_then(|bytes| bytes.checked_add(u64::from(limits.client.url_bytes).checked_mul(2)?))
                .and_then(|bytes| bytes.checked_add(u64::try_from(size_of::<Login>()).ok()?))
                .and_then(|bytes| bytes.checked_add(u64::try_from(name.len()).ok()?))
                .ok_or("login memory bound overflows")?;
            let mut host = Login {
                driver,
                outcome: None,
                name: name.into(),
                out: Queue::with_capacity(ACCOUNTS_MAX_OUT),
                completions: Queue::with_capacity(8),
                operations,
                worst,
            };
            drop(account);
            drop(settings);
            drop(file);
            let _exit = skein_shell::drive(&mut kernel, clock, &mut host);
            Ok(host.outcome.expect("login closes after its terminal"))
        }
    }
}

struct Login {
    driver: Accounts,
    outcome: Option<LoginOutcome>,
    name: String,
    out: Queue<CredentialRequest>,
    completions: Queue<kernel::Complete>,
    operations: u32,
    worst: u64,
}

impl Host for Login {
    fn iterate(&mut self, now: Time, wall: Wall) {
        for _ in 0..self.completions.len() {
            self.driver.complete(self.completions.pop().expect("queued login completion"));
        }
        self.driver.advance(now, wall, &mut self.out);
        for _ in 0..ACCOUNTS_MAX_OUT {
            match self.out.pop() {
                Some(CredentialRequest::Visit { url }) => eprintln!("Sign in: {}", String::from_utf8_lossy(&url)),
                Some(CredentialRequest::SignedIn { .. }) => self.outcome = Some(LoginOutcome::Usable),
                Some(CredentialRequest::Failed { .. }) => self.outcome = Some(LoginOutcome::Unavailable),
                Some(
                    CredentialRequest::Closed | CredentialRequest::Notice { .. } | CredentialRequest::Ready { .. },
                )
                | None => {}
            }
        }
        if let Some(outcome) = self.outcome {
            if !self.driver.closed() {
                self.driver.close(smith_agent_service::Close::Graceful, now, wall);
            }
            if self.driver.closed() {
                eprintln!(
                    "account {}: {}",
                    self.name,
                    if outcome == LoginOutcome::Usable { "signed in" } else { "not signed in" }
                );
            }
        }
    }
    fn completions(&mut self) -> &mut Queue<kernel::Complete> {
        &mut self.completions
    }
    fn submissions(&mut self) -> &mut Queue<kernel::Submit> {
        self.driver.submissions()
    }
    fn work_pending(&self, now: Time) -> bool {
        self.driver.work_pending(now)
    }
    fn next_deadline(&self) -> Option<Time> {
        self.driver.next_deadline()
    }
    fn is_empty(&self) -> bool {
        self.outcome.is_some() && self.driver.closed()
    }
    fn next_policy_deadline(&self) -> Option<Time> {
        self.driver.next_deadline()
    }
    fn worst_case(&self) -> u64 {
        self.worst
    }
    fn operations(&self) -> u32 {
        self.operations
    }
    fn exit(&self) -> Option<kernel::Exit> {
        self.outcome.map(|outcome| kernel::Exit::Code(outcome.code()))
    }
}
