//! Native account driver outputs (protocol/hosts.md, section 5.4).
use alloc::boxed::Box;
use smith_local_domain::{self as local, CredentialFailure};

/// One request to the local service or its person and network neighbours.
#[expect(missing_debug_implementations, reason = "token requests and grants contain secret values")]
pub enum CredentialRequest {
    /// The handed-in driver and all its borrowed directories have closed.
    Closed,
    /// A handed-in login approaches expiry without ending its held grant.
    Notice { account: u32, reason: CredentialFailure },
    /// Show this authorization page's address to the person.
    Visit { url: Box<[u8]> },
    /// The native driver's explicit sign-in completed after its record was kept.
    SignedIn { account: u32 },
    /// Lend a fresh grant to the domain and the access value to the agent's protocol layer.
    Ready { event: local::Event, value: Box<[u8]> },
    /// Tell the domain why the account has no usable credential.
    Failed { event: local::Event },
}
