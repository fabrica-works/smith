//! The configured command environment, encoded once before machine startup.
//! It retains only bounded `name=value` bytes, never domain authority, and
//! does not interpret their values. `encode` validates the configuration; processes copy the owned
//! entries according to their purpose (domain/tools.md, section 5;
//! protocol/agent.md, section 3).

use alloc::boxed::Box;

use skein_lib::{List, Set, Writer};

use crate::limits::Limits;
use crate::process::{Purpose, Sees, sees};

/// One variable supplied by the agent configuration to the machine encoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variable {
    pub name: Box<[u8]>,
    pub value: Box<[u8]>,
}

/// The machine's complete configured environment, encoded once at startup.
#[derive(Clone, Debug)]
pub struct Environment {
    entries: Box<[Box<[u8]>]>,
    bytes: u32,
}

/// Why a configured environment cannot be encoded for startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unencodable {
    /// The indexed variable has an empty name, `=` or NUL in its name, or NUL in its value.
    Variable { index: u32 },
    /// The indexed variable repeats an earlier name.
    Repeated { index: u32 },
    /// The encoded entries exceed the machine's byte bound.
    TooLarge { bytes: u64, bound: u32 },
}

impl Environment {
    /// Copy the already encoded environment for a host's delivery command.
    #[must_use]
    pub fn command_entries(&self) -> Box<[Box<[u8]>]> {
        self.for_process(Purpose::Shell)
    }

    /// Validate the complete configuration and encode its entries within `env_bytes`.
    pub fn encode(variables: &[Variable], limits: &Limits) -> Result<Environment, Unencodable> {
        let mut bytes = 0_u64;
        for (index, variable) in variables.iter().enumerate() {
            let index = u32::try_from(index).expect("bounded variables have u32 indices");
            if variable.name.is_empty()
                || variable.name.contains(&b'=')
                || variable.name.contains(&0)
                || variable.value.contains(&0)
            {
                return Err(Unencodable::Variable { index });
            }
            let entry = u64::try_from(variable.name.len())
                .expect("name length fits u64")
                .checked_add(u64::try_from(variable.value.len()).expect("value length fits u64"));
            let entry = entry.ok_or(Unencodable::TooLarge { bytes: u64::MAX, bound: limits.env_bytes })?;
            let entry =
                entry.checked_add(1).ok_or(Unencodable::TooLarge { bytes: u64::MAX, bound: limits.env_bytes })?;
            bytes = match bytes.checked_add(entry) {
                Some(total) => total,
                None => return Err(Unencodable::TooLarge { bytes: u64::MAX, bound: limits.env_bytes }),
            };
            if bytes > u64::from(limits.env_bytes) {
                return Err(Unencodable::TooLarge { bytes, bound: limits.env_bytes });
            }
        }
        let count = u32::try_from(variables.len()).expect("each variable costs at least two bounded bytes");
        {
            let mut names = Set::with_capacity(count);
            for (index, variable) in variables.iter().enumerate() {
                let fresh = names.insert(variable.name.as_ref()).expect("room for every configured name");
                if !fresh {
                    return Err(Unencodable::Repeated { index: u32::try_from(index).expect("index below count") });
                }
            }
        }
        let mut entries = List::with_capacity(count);
        for variable in variables {
            let length = variable
                .name
                .len()
                .checked_add(variable.value.len())
                .expect("validated name and value fit")
                .checked_add(1)
                .expect("validated entry length fits the byte bound");
            let mut entry = Writer::new(length);
            entry.put(&variable.name).expect("entry length includes name");
            entry.put(b"=").expect("entry length includes equals");
            entry.put(&variable.value).expect("entry length includes value");
            entries.push(entry.finish()).expect("room for every configured entry");
        }
        Ok(Environment {
            entries: entries.into_boxed(),
            bytes: u32::try_from(bytes).expect("encoded bytes within u32 bound"),
        })
    }

    /// Whether this encoded configuration fits the machine receiving it.
    pub(crate) fn fits(&self, limits: &Limits) -> bool {
        self.bytes <= limits.env_bytes
    }

    /// Copy the already encoded configuration only for commands and checks.
    pub(crate) fn for_process(&self, purpose: Purpose) -> Box<[Box<[u8]>]> {
        match sees(purpose) {
            Sees::Configured => self.entries.clone(),
            Sees::Nothing => Box::new([]),
        }
    }
}
