//! Access-only records handed in by the env source (shell.md, section 7.1).
//! The shell reads the variable once; no login file or refresh value is retained.
use alloc::boxed::Box;

/// An access-only record handed to the account driver, never kept or refreshed.
#[expect(missing_debug_implementations, reason = "the record contains a credential")]
pub struct Borrowed {
    pub record: skein_oauth::SavedToken,
    pub account_id: Box<[u8]>,
}
