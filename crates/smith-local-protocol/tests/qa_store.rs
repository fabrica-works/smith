//! Independent, contract-derived real-directory QA of protocol/hosts.md, 5.3,
//! domain/host.md, 8, and shell.md, 5.3. Only the supplied public API and the
//! existing Disk adapter inform these cases; production and core tests are unread.
#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "this independent QA adapter is an ordinary Rust test harness (programming-model.md, section 10.2)"
)]
#[cfg(test)]
#[path = "support/disk.rs"]
pub mod disk;

#[cfg(test)]
mod tests {
    use super::disk;
    use disk::{Disk, Kind, snapshot};
    use skein_lib::{Token, Writer};
    use smith_local_domain::{ChatState, Event, Older, StoreFailure, StoreFamily};
    use smith_local_protocol::{History, StoreRequest, format, local_store_v1};
    use std::{
        fs,
        os::unix::{ffi::OsStrExt, fs::symlink},
        path::Path,
    };

    // The schema fixes both fingerprints at 32 bytes.
    const FINGERPRINT_BYTES: usize = 32;
    // Disk's public scan capacity is turns + deliveries + 4. The absence scan
    // reserves one of those eight cells for the marker (protocol/hosts.md, 5.3).
    const RESIDUE_AT_ENTRY_BOUND: usize = 7;
    const RESIDUE_PAST_ENTRY_BOUND: usize = RESIDUE_AT_ENTRY_BOUND + 1;
    // Two harmless residues surround the foreign-entry test's one old journal.
    const MIXED_SCAN_RESIDUE: usize = 2;
    const FORMAT_NAME: &str = "format";
    const RESIDUE_NAME: &str = "format.skein-0123456789abcdef";

    fn marker(parts: local_store_v1::LayoutsParts) -> Box<[u8]> {
        let record = local_store_v1::Layouts::new(&local_store_v1::CEILINGS, parts).unwrap();
        let mut writer = Writer::new(usize::try_from(record.measure()).expect("measured marker size fits usize"));
        record.encode(&mut writer).unwrap();
        writer.finish()
    }

    fn write_files(path: &Path, files: &[(&str, &[u8])]) {
        for (name, bytes) in files {
            fs::write(path.join(name), bytes).unwrap();
        }
    }

    fn compatible(path: &Path) {
        fs::write(path.join(FORMAT_NAME), format::file().bytes).unwrap();
    }

    fn state(activation: u64) -> ChatState {
        ChatState {
            identity: smith_domain::run::Identity::new(*b"qa-store-chat-id"),
            activation,
            next_message: 17,
            read: Some(Token::new(11)),
        }
    }

    fn write_state(path: &Path, value: ChatState) {
        let file = smith_local_protocol::save_state(value);
        fs::write(path.join(std::ffi::OsStr::from_bytes(&file.name)), file.bytes).unwrap();
    }

    // These are assertion/setup helpers for this binary, not a new IO surface.
    fn refused_unchanged(disk: &mut Disk, path: &Path, reason: StoreFailure) {
        let before = snapshot(path);
        let terminal = disk.drive(StoreRequest::Load);
        let requests = disk.requests().to_vec();
        disk.close();
        let after = snapshot(path);
        assert!(matches!(terminal, Event::StoreFailed { turn: None, reason: why } if why == reason), "{terminal:?}");
        assert_eq!(before, after);
        assert!(requests.iter().all(|kind| matches!(kind, Kind::Load | Kind::Scan)), "{requests:?}");
    }

    fn empty_loaded(terminal: Event) {
        assert!(
            matches!(terminal, Event::Loaded { state: None, transcript: None, deliveries } if deliveries.is_empty())
        );
    }

    fn interrupted_creates(path: &Path, count: usize) {
        for number in 0..count {
            let name = format!("format.skein-{number:016x}");
            fs::write(path.join(name), b"unfinished marker").unwrap();
        }
    }

    #[test]
    fn unrecorded_records_refuse_before_journal_or_state_reads() {
        for record in ["state", "fresh", "turn-00000001", "delivery", "unrelated"] {
            let mut disk = Disk::new();
            write_files(disk.path(), &[(record, b"opaque old record")]);
            let path = disk.path().to_path_buf();
            let before = snapshot(&path);
            let terminal = disk.drive(StoreRequest::Load);
            let requests = disk.requests().to_vec();
            disk.close();
            assert!(
                matches!(
                    terminal,
                    Event::StoreFailed { turn: None, reason: StoreFailure::OlderBuild { why: Older::Unrecorded } }
                ),
                "{record}: {terminal:?}"
            );
            assert_eq!(requests, [Kind::Load, Kind::Scan], "{record}");
            assert_eq!(snapshot(&path), before, "{record}");
        }
    }

    #[test]
    fn near_miss_temporary_names_are_not_adopted() {
        for name in [
            "format.skein-0123456789abcde",
            "format.skein-0123456789abcdef0",
            "format.skein-0123456789abcdeF",
            "format.skein-0123456789abcdeg",
            "format.skein-0123456789abcdef.extra",
            "state.skein-0123456789abcdef",
            "xformat.skein-0123456789abcdef",
            ".format.skein-0123456789abcdef",
        ] {
            let mut disk = Disk::new();
            write_files(disk.path(), &[(name, b"residue")]);
            let path = disk.path().to_path_buf();
            refused_unchanged(&mut disk, &path, StoreFailure::OlderBuild { why: Older::Unrecorded });
        }
    }

    #[test]
    fn temporary_shaped_directory_is_not_interrupted_create() {
        let mut disk = Disk::new();
        fs::create_dir(disk.path().join(RESIDUE_NAME)).unwrap();
        let path = disk.path().to_path_buf();
        refused_unchanged(&mut disk, &path, StoreFailure::OlderBuild { why: Older::Unrecorded });
    }

    #[test]
    fn temporary_shaped_symlink_is_not_interrupted_create() {
        let mut disk = Disk::new();
        symlink("absent-target", disk.path().join(RESIDUE_NAME)).unwrap();
        let path = disk.path().to_path_buf();
        refused_unchanged(&mut disk, &path, StoreFailure::OlderBuild { why: Older::Unrecorded });
    }

    #[test]
    fn empty_directory_gets_marker_and_survives_reopen() {
        let mut disk = Disk::new();
        let first = disk.drive(StoreRequest::Load);
        let marker_bytes = fs::read(disk.path().join(FORMAT_NAME)).unwrap();
        let after_create = snapshot(disk.path());
        disk.reopen();
        let second = disk.drive(StoreRequest::Load);
        let after_reopen = snapshot(disk.path());
        disk.close();
        empty_loaded(first);
        empty_loaded(second);
        format::check(&marker_bytes).expect("the created marker records current layouts");
        assert_eq!(after_create, after_reopen);
    }

    #[test]
    fn interrupted_create_residue_is_kept_through_reopen() {
        let mut disk = Disk::new();
        interrupted_creates(disk.path(), RESIDUE_AT_ENTRY_BOUND);
        let residue = snapshot(disk.path());
        let first = disk.drive(StoreRequest::Load);
        let after_create = snapshot(disk.path());
        disk.reopen();
        let second = disk.drive(StoreRequest::Load);
        let after_reopen = snapshot(disk.path());
        disk.close();
        empty_loaded(first);
        empty_loaded(second);
        assert_eq!(after_create, after_reopen);
        let retained: Vec<_> = after_create.into_iter().filter(|entry| entry.name != FORMAT_NAME).collect();
        assert_eq!(retained, residue);
        assert_eq!(after_reopen.len(), RESIDUE_AT_ENTRY_BOUND + 1);
        assert_eq!(snapshot(disk.path()), after_reopen);
    }

    #[test]
    fn absence_scan_reserves_a_cell_for_future_marker() {
        let mut disk = Disk::new();
        interrupted_creates(disk.path(), RESIDUE_PAST_ENTRY_BOUND);
        let path = disk.path().to_path_buf();
        refused_unchanged(&mut disk, &path, StoreFailure::OlderBuild { why: Older::Unrecorded });
    }

    #[test]
    fn foreign_entry_among_valid_residue_still_refuses() {
        let mut disk = Disk::new();
        interrupted_creates(disk.path(), MIXED_SCAN_RESIDUE);
        write_files(disk.path(), &[("fresh", b"old pending recovery")]);
        let path = disk.path().to_path_buf();
        refused_unchanged(&mut disk, &path, StoreFailure::OlderBuild { why: Older::Unrecorded });
    }

    #[test]
    fn every_truncated_marker_refuses_without_record_reads() {
        let full = format::file().bytes;
        for prefix_bytes in 0..full.len() {
            let mut disk = Disk::new();
            write_files(disk.path(), &[(FORMAT_NAME, &full[..prefix_bytes]), ("fresh", b"do not recover")]);
            let before = snapshot(disk.path());
            let terminal = disk.drive(StoreRequest::Load);
            let requests = disk.requests().to_vec();
            disk.close();
            assert!(
                matches!(
                    terminal,
                    Event::StoreFailed {
                        turn: None,
                        reason: StoreFailure::OlderBuild { why: Older::Layout(StoreFamily::Store) }
                    }
                ),
                "prefix {prefix_bytes}: {terminal:?}"
            );
            assert_eq!(requests, [Kind::Load], "prefix {prefix_bytes}");
            assert_eq!(snapshot(disk.path()), before, "prefix {prefix_bytes}");
        }
    }

    #[test]
    fn trailing_bytes_do_not_make_a_valid_marker() {
        let mut disk = Disk::new();
        let mut bytes = format::file().bytes.into_vec();
        bytes.extend_from_slice(b"trailing record");
        write_files(disk.path(), &[(FORMAT_NAME, &bytes), ("fresh", b"do not recover")]);
        let path = disk.path().to_path_buf();
        // The fixed-size marker read rejects this oversized file before decoding.
        refused_unchanged(&mut disk, &path, StoreFailure::Read);
    }

    #[test]
    fn every_store_fingerprint_byte_is_compared_first() {
        for position in 0..FINGERPRINT_BYTES {
            let mut parts = format::layouts().into_parts();
            parts.store[position] ^= 1;
            parts.transcript[position] ^= 1;
            let bytes = marker(parts);
            let mut disk = Disk::new();
            write_files(disk.path(), &[(FORMAT_NAME, &bytes), ("fresh", b"do not recover")]);
            let path = disk.path().to_path_buf();
            refused_unchanged(&mut disk, &path, StoreFailure::OlderBuild { why: Older::Layout(StoreFamily::Store) });
        }
    }

    #[test]
    fn every_transcript_fingerprint_byte_is_compared() {
        for position in 0..FINGERPRINT_BYTES {
            let mut parts = format::layouts().into_parts();
            parts.transcript[position] ^= 1;
            let bytes = marker(parts);
            let mut disk = Disk::new();
            write_files(disk.path(), &[(FORMAT_NAME, &bytes), ("state", b"not a state")]);
            let path = disk.path().to_path_buf();
            refused_unchanged(
                &mut disk,
                &path,
                StoreFailure::OlderBuild { why: Older::Layout(StoreFamily::Transcript) },
            );
        }
    }

    #[test]
    fn symlink_marker_never_adopts_its_valid_target() {
        let mut disk = Disk::new();
        fs::write(disk.path().join("valid-target"), format::file().bytes).unwrap();
        symlink("valid-target", disk.path().join(FORMAT_NAME)).unwrap();
        let path = disk.path().to_path_buf();
        refused_unchanged(&mut disk, &path, StoreFailure::Read);
    }

    #[test]
    fn directory_marker_refuses_without_adoption() {
        let mut disk = Disk::new();
        fs::create_dir(disk.path().join(FORMAT_NAME)).unwrap();
        let path = disk.path().to_path_buf();
        refused_unchanged(&mut disk, &path, StoreFailure::Read);
    }

    #[test]
    fn oversized_marker_refuses_without_adoption() {
        let mut bounds = disk::limits();
        bounds.file_bytes = format::max_bytes();
        let past_file_bound = usize::try_from(bounds.file_bytes)
            .expect("tiny file bound fits usize")
            .checked_add(1)
            .expect("one byte past the tiny bound fits usize");
        let mut disk = Disk::bounded(bounds);
        fs::write(disk.path().join(FORMAT_NAME), vec![0; past_file_bound]).unwrap();
        let path = disk.path().to_path_buf();
        refused_unchanged(&mut disk, &path, StoreFailure::Read);
    }

    #[test]
    fn marker_at_file_bound_is_adopted() {
        let mut bounds = disk::limits();
        bounds.file_bytes = format::max_bytes();
        let mut disk = Disk::bounded(bounds);
        compatible(disk.path());
        let before = snapshot(disk.path());
        let terminal = disk.drive(StoreRequest::Load);
        disk.close();
        empty_loaded(terminal);
        assert_eq!(snapshot(disk.path()), before);
    }

    #[test]
    fn compatible_marker_loads_the_exact_public_state() {
        let mut disk = Disk::new();
        compatible(disk.path());
        let expected = state(23);
        write_state(disk.path(), expected);
        let before = snapshot(disk.path());
        let terminal = disk.drive(StoreRequest::Load);
        disk.close();
        assert!(
            matches!(&terminal, Event::Loaded { state: Some(value), transcript: None, deliveries } if *value == expected && deliveries.is_empty()),
            "{terminal:?}"
        );
        assert_eq!(snapshot(disk.path()), before);
    }

    #[test]
    fn repeated_load_refreshes_state_on_the_checked_root() {
        let mut disk = Disk::new();
        compatible(disk.path());
        let _loaded = disk.drive(StoreRequest::Load);
        let expected = state(29);
        write_state(disk.path(), expected);
        let terminal = disk.drive(StoreRequest::Load);
        disk.close();
        assert!(
            matches!(&terminal, Event::Loaded { state: Some(value), transcript: None, deliveries } if *value == expected && deliveries.is_empty()),
            "{terminal:?}"
        );
    }

    #[test]
    fn selecting_another_chat_resets_the_checked_root() {
        let mut disk = Disk::new();
        compatible(disk.path());
        let other = disk.path().join("other-chat");
        fs::create_dir(&other).unwrap();
        let mut parts = format::layouts().into_parts();
        parts.transcript.fill(0);
        write_files(&other, &[(FORMAT_NAME, &marker(parts)), ("fresh", b"do not recover")]);
        let _loaded = disk.drive(StoreRequest::Load);
        let _selected = disk.drive(StoreRequest::SelectChat { name: b"other-chat".as_slice().into(), create: false });
        disk.clear_requests();
        refused_unchanged(&mut disk, &other, StoreFailure::OlderBuild { why: Older::Layout(StoreFamily::Transcript) });
    }

    #[test]
    fn reopening_a_checked_root_rechecks_its_marker() {
        let mut disk = Disk::new();
        compatible(disk.path());
        let _loaded = disk.drive(StoreRequest::Load);
        fs::remove_file(disk.path().join(FORMAT_NAME)).unwrap();
        write_state(disk.path(), state(31));
        disk.reopen();
        let path = disk.path().to_path_buf();
        refused_unchanged(&mut disk, &path, StoreFailure::OlderBuild { why: Older::Unrecorded });
    }

    #[test]
    fn fresh_history_keeps_the_original_layout_marker() {
        let mut disk = Disk::new();
        compatible(disk.path());
        write_state(disk.path(), state(37));
        let marker_before: Vec<_> =
            snapshot(disk.path()).into_iter().filter(|entry| entry.name == FORMAT_NAME).collect();
        let _loaded = disk.drive(StoreRequest::Load);
        let expected = state(41);
        let saved = disk.drive(StoreRequest::SaveState { state: expected, history: History::Fresh });
        disk.reopen();
        let terminal = disk.drive(StoreRequest::Load);
        let marker_after: Vec<_> =
            snapshot(disk.path()).into_iter().filter(|entry| entry.name == FORMAT_NAME).collect();
        disk.close();
        assert!(matches!(saved, Event::StateSaved), "{saved:?}");
        assert!(
            matches!(&terminal, Event::Loaded { state: Some(value), transcript: None, deliveries } if *value == expected && deliveries.is_empty()),
            "{terminal:?}"
        );
        assert_eq!(marker_before, marker_after);
    }
}
