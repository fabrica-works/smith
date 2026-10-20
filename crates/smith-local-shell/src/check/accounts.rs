//! Check's account projection (shell.md, 4.4; protocol/hosts.md, 5.2).
//! Retains redacted states only. `report` checks every declared program source
//! before inspecting the catalog; it starts no run or sign-in.
use crate::{credentials, settings::resolve::Resolved};
use skein_lib::Wall;
use skein_shell::Clock;
use std::collections::BTreeMap;

pub(super) fn report(resolved: &Resolved, wall: Wall, clock: &Clock) -> BTreeMap<String, credentials::AccountReport> {
    let catalog = credentials::account_report::accounts(resolved);
    let states = credentials::program_check::check(
        resolved,
        catalog.values().filter(|account| account.source() == "program").map(|account| account.name.as_str()),
        clock,
    );
    credentials::report(resolved, wall, &states)
}
