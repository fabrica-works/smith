//! Adopt source descriptors without changing their public shape (protocol/hosts.md, 5.4).
use super::{Account, Origin, Source, lock, oauth};
use alloc::boxed::Box;
use skein_io::file_layer;
use skein_lib::{List, Wall};

pub(super) struct Binding {
    pub account: Account,
    pub configured: oauth::Account,
}

pub(super) fn source(
    source: Source,
    files: &mut file_layer::FileIo,
    locks: &mut List<lock::Lock>,
    shared: bool,
) -> Result<Binding, crate::Error> {
    let mut identifier: Box<[u8]> = Box::new([]);
    let (number, origin, configured) = match source {
        Source::Env { account, borrowed } => {
            (account, Origin::Env { initial: Some(borrowed) }, oauth::Account::HandedIn)
        }
        Source::SignIn { account, root, directory, file, registration, endpoint, account_id, interactive, .. } => {
            if account_id.len() > Source::account_id_bytes() {
                return Err(crate::Error::Memory);
            }
            identifier = account_id;
            let root = files.adopt_root(root).expect("validated private source count fits");
            let claim = registration.metadata_claim.clone();
            locks.push(lock::Lock::new(account, root, &directory, &file)).ok().expect("account lock capacity");
            (
                account,
                Origin::SignIn { root: Some(root), claim, interactive, sign_in: false, signing: false },
                oauth::Account::SignIn {
                    registration,
                    endpoint,
                    origins: List::with_capacity(0),
                    shared,
                    keeper: oauth::Keeper::Private { root, directory, file },
                },
            )
        }
    };
    Ok(Binding {
        account: Account {
            number,
            source: origin,
            identifier,
            generation: 0,
            expires: Wall::EPOCH,
            startup: true,
            loading: false,
            wanted: false,
            subject: None,
            subject_failure: None,
            pending: None,
        },
        configured,
    })
}
