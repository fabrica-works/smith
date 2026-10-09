//! Selected-file streams against an independent whole-tree set and checked heap bound.

use skein_lib::{Duration, Rng};
use skein_world::domain::heap::{Counting, Meter};
use smith_domain::tools;
use smith_protocol_machine::{Limits, Listing};
use std::collections::{BTreeMap, BTreeSet};

#[global_allocator]
static HEAP: Counting = Counting;

#[test]
fn seeded_selected_paths_keep_the_exact_tree_prefix_and_omitted_count_within_memory() {
    for seed in 0..128 {
        let mut rng = Rng::new(seed);
        let depth = match rng.below(4) {
            0 => tools::Depth::Whole,
            value => tools::Depth::Levels(u32::try_from(value).expect("small depth")),
        };
        let maximum = u32::try_from(rng.below(17)).expect("small count");
        let bytes = rng.below(2049);
        let mut selected = BTreeSet::new();
        selected.insert(b".Git/config".to_vec());
        for index in 0..24 {
            selected.insert(format!("a{}/deep{}/file{index:02}", rng.below(4), rng.below(3)).into_bytes());
        }
        let mut all = BTreeMap::new();
        let levels = match depth {
            tools::Depth::Whole => u32::MAX,
            tools::Depth::Levels(levels) => levels,
        };
        let mut output = Vec::new();
        for file in &selected {
            output.extend_from_slice(file);
            output.push(0);
            if file.starts_with(b".Git/") {
                continue;
            }
            let components: Vec<_> = file.split(|byte| *byte == b'/').collect();
            let mut name = Vec::new();
            for (index, component) in components.iter().enumerate() {
                if u32::try_from(index + 1).expect("finite depth") > levels {
                    break;
                }
                if index > 0 {
                    name.push(b'/');
                }
                name.extend_from_slice(component);
                let kind = if index + 1 == components.len() { tools::Kind::File } else { tools::Kind::Directory };
                all.insert(name.clone(), kind);
            }
        }
        let mut expected = Vec::new();
        let mut used = 0_u64;
        for (name, kind) in &all {
            let next = used + u64::try_from(core::mem::size_of::<tools::Entry>() + name.len()).expect("finite entry");
            if expected.len() == usize::try_from(maximum).expect("finite count") || next > bytes {
                break;
            }
            expected.push(tools::Entry { name: name.clone().into(), kind: *kind });
            used = next;
        }
        let more = u64::try_from(all.len() - expected.len()).expect("finite remainder");
        let limits = Limits {
            operations: 0,
            roots: 0,
            path_bytes: 32,
            file_bytes: 0,
            entries: maximum,
            entry_bytes: bytes,
            processes: 1,
            output_bytes: 0,
            search_hits: 0,
            search_bytes: 0,
            search_line_bytes: 256,
            env_bytes: 0,
            group_stop: Duration::ZERO,
        };
        let bound = smith_protocol_machine::worst_case(&limits).expect("checked memory bound");
        let meter = Meter::new();
        meter.start();
        let mut listing = Listing::new(maximum, bytes, depth, 256);
        for byte in output {
            listing.push(byte);
        }
        let actual = listing.finish(Some(skein_io::kernel::Exit::Code(0)), false);
        let peak = meter.check(meter.end(), bound, &seed);
        assert!(peak > 0);
        assert_eq!(actual, tools::Done::Listed { entries: expected.into(), more }, "seed {seed}");
    }
}
