//! Read-only login stories and observations outside every hosted process.
use super::{Action, Ending, Files, How, Placement, World};
use skein_lib::{Duration, Queue, Writer, base64};

/// The borrowed login fixture supplied to the same local service composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Borrowing {
    Valid,
    Expired,
    Renewed,
    Env,
    Key,
}

pub(crate) fn login_token(expired: bool) -> Box<[u8]> {
    login_at(skein_tls_world::pki::VALID, expired)
}

/// Access-only synthetic login for a real-loop story's clock.
#[must_use]
pub fn login_at(wall: skein_lib::Wall, expired: bool) -> Box<[u8]> {
    let wall = wall.as_nanos();
    let nanos =
        if expired { wall - Duration::from_secs(1).as_nanos() } else { wall + Duration::from_secs(7200).as_nanos() };
    let claims = serde_json::to_vec(&serde_json::json!({
        "exp": nanos / 1_000_000_000,
        "https://api.openai.com/auth": {"chatgpt_account_id": "acc"}
    }))
    .expect("synthetic claims");
    let length =
        base64::encoded_len(base64::Alphabet::Url, u32::try_from(claims.len()).expect("fixture claims length"))
            .expect("fixture encoded length");
    let mut encoded =
        Writer::new(usize::try_from(length).expect("fixture length").checked_add(9).expect("fixture token length"));
    encoded.put(b"e30.").expect("fixture header");
    base64::encode(base64::Alphabet::Url, &claims, &mut encoded).expect("fixture claims");
    encoded.put(b".c2ln").expect("fixture signature");
    encoded.finish()
}

fn login_document(expired: bool) -> Box<[u8]> {
    serde_json::to_vec(&serde_json::json!({"tokens": {
        "access_token": String::from_utf8(login_token(expired).into_vec()).expect("fixture token"),
        "account_id": "acc", "refresh_token": "UNISSUED_NEVER_COPY"
    }}))
    .expect("synthetic login")
    .into()
}

impl World {
    #[must_use]
    pub fn borrowed(seed: u64, files: &Files, borrowing: Borrowing) -> Self {
        let mut world = Self::new(seed, files, Placement::InProcess, b"First answer");
        world.scenario.launch.borrowing = Some(borrowing);
        if borrowing == Borrowing::Expired {
            let minutes = (skein_tls_world::pki::VALID.as_nanos() - Duration::from_secs(1).as_nanos()) / 60_000_000_000;
            world.checkout.1 = Some(format!("smith: the codex login borrowed from {} expired at {:02}:{:02}. Run `codex` once to refresh it, then try again.\n", files.path().join("auth.json").display(), minutes / 60 % 24, minutes % 60).into_bytes());
            world.scenario.commands = vec![
                Action::Send(b"hello\n".as_slice().into()),
                Action::Wait(b"Run `codex` once".as_slice().into()),
                Action::Eof,
            ];
            world.scenario.ending = Ending::Unavailable;
        }
        let machine = world.machine.as_mut().expect("unsettled fixture");
        if borrowing == Borrowing::Key {
            use std::os::unix::fs::PermissionsExt;
            let key = files.path().join("key");
            std::fs::write(&key, b"key-for-work").expect("scratch key");
            std::fs::set_permissions(key, std::fs::Permissions::from_mode(0o600)).expect("private key");
            world.check_memory();
            return world;
        }
        machine.login_checks = true;
        machine.renew_login = borrowing == Borrowing::Renewed;
        let file =
            machine.files.open(machine.master, b"auth.json", How::Create { mode: 0o600 }).expect("scratch login");
        machine
            .files
            .write(file, 0, &login_document(matches!(borrowing, Borrowing::Expired | Borrowing::Renewed)))
            .expect("fixture login");
        machine.files.close(file);
        world.check_memory();
        world
    }

    #[must_use]
    pub fn login_reads(&self) -> u32 {
        self.outcome.as_ref().expect("settled fixture").machine.login_reads
    }
    #[must_use]
    pub fn login_writes(&self) -> u32 {
        self.outcome.as_ref().expect("settled fixture").machine.login_writes
    }
    #[must_use]
    pub fn borrowed_login(&mut self) -> Box<[u8]> {
        let machine = &mut self.outcome.as_mut().expect("settled fixture").machine;
        let file = machine.files.open(machine.master, b"auth.json", How::Read).expect("scratch login");
        let bytes = machine.files.read(file, 0, 16_384).expect("bounded login");
        machine.files.close(file);
        bytes.into()
    }
}

impl super::Machine {
    pub(super) fn login_step(&mut self, call: skein_sim::Call, answers: &mut Queue<skein_sim::Answer>) {
        let opening = matches!(&call.ask, skein_sim::Ask::Open { path, .. } if path.as_ref() == b"auth.json");
        if let skein_sim::Ask::Open { how, .. } = &call.ask
            && opening
        {
            assert_eq!(*how, skein_io::kernel::OpenHow::ReadNoFollow, "the service opens its borrowed login read-only");
        }
        let closing = matches!(&call.ask, skein_sim::Ask::Close { file } if self.login_handles.contains(file));
        self.login_writes += u32::from(match &call.ask {
            skein_sim::Ask::Write { file, .. }
            | skein_sim::Ask::Append { file, .. }
            | skein_sim::Ask::Truncate { file, .. } => self.login_handles.contains(file),
            skein_sim::Ask::Rename { from, to, .. } => from.as_ref() == b"auth.json" || to.as_ref() == b"auth.json",
            skein_sim::Ask::Remove { name, .. } => name.as_ref() == b"auth.json",
            _ => false,
        });
        let mut replied = Queue::with_capacity(1);
        skein_fake_machine::step(&mut self.files, call, &mut replied);
        let answer = replied.pop().expect("one machine reply");
        if opening && let Ok(skein_sim::Reply::Opened(handle)) = &answer.result {
            self.login_handles.push(*handle);
            self.login_reads += 1;
        }
        answers.push(answer);
        if closing && self.renew_login {
            self.renew_login = false;
            let file =
                self.files.open(self.master, b"auth.json", How::Write).expect("the login's program owns its file");
            self.files.truncate(file, 0).expect("replace synthetic login");
            self.files.write(file, 0, &login_document(false)).expect("renewed synthetic login");
            self.files.close(file);
        }
    }
}
