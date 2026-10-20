//! Transfer program bindings after memory admission (shell.md, section 7.1).
use crate::credentials::{Location, Source, program_source};
use skein_lib::List;
use smith_host_domain::SourceName;
use smith_protocol_program::Programs;
use std::os::unix::ffi::OsStrExt;

pub(crate) fn hand(
    service: &mut smith_local_service::Service,
    programs: Programs,
    locations: &List<Location>,
    base: Box<[Box<[u8]>]>,
) -> Result<(), crate::startup::Failure> {
    let mut sources = List::with_capacity(locations.len());
    for location in locations {
        if matches!(location.source, Source::Program { .. }) {
            let source =
                SourceName::new(location.name.as_bytes()).ok_or_else(|| failure("invalid program source name"))?;
            sources.push((location.account, source)).expect("selected program accounts");
        }
    }
    let path = std::path::Path::new(std::ffi::OsStr::from_bytes(&programs.directory));
    let root = program_source::open_root(path).map_err(|why| failure(&why))?;
    service
        .handed_programs(
            sources,
            programs,
            base,
            root,
            program_source::DETAIL_BYTES,
            smith_agent_service::profile::standard().policy.check_timeout,
        )
        .map_err(|why| failure(&format!("program check binding: {why:?}")))
}

fn failure(detail: &str) -> crate::startup::Failure {
    crate::startup::Failure::Host { detail: detail.into() }
}
