//! Bounded credential generations below the agent domain
//! (protocol/channel.md, section 6; domain/host.md, section 7).
//!
//! This table retains at most two generations per account. The domain sees
//! only names and validity; `value` is for the service's LLM preparation.

use alloc::boxed::Box;
use core::fmt;
use skein_lib::Map;

use crate::Error;

/// Secret bytes whose debug view cannot print the value.
pub(crate) struct Secret(Box<[u8]>);

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[redacted credential]")
    }
}

/// The last two credential values per configured account.
#[derive(Debug)]
pub(crate) struct Credentials {
    values: Map<(u32, u64), Secret>,
}

impl Credentials {
    pub(crate) fn new(capacity: u32) -> Credentials {
        Credentials { values: Map::with_capacity(capacity) }
    }

    pub(crate) fn value(&self, account: u32, generation: u64) -> Option<&[u8]> {
        match self.values.get(&(account, generation)) {
            Some(secret) => Some(&secret.0),
            None => None,
        }
    }

    // smith 04.1 replaces the withdrawn envelope with tagged values.
    pub(crate) fn keep_wire(
        &mut self,
        account: u32,
        generation: u64,
        value: &smith_channel::GrantValue,
    ) -> Result<(), Error> {
        let value = match value {
            smith_channel::GrantValue::Withdrawn(value) => value,
            smith_channel::GrantValue::Credential(_) | smith_channel::GrantValue::Program(_) => {
                return Err(Error::Grants);
            }
        };
        let payload = value.id().len().checked_add(value.bearer().expose().len()).ok_or(Error::Grants)?;
        let size = payload.checked_add(2).ok_or(Error::Grants)?;
        let mut bytes = skein_lib::Writer::new(size);
        let id_len = u16::try_from(value.id().len()).or(Err(Error::Grants))?;
        bytes.put(&id_len.to_be_bytes())?;
        bytes.put(value.id())?;
        bytes.put(value.bearer().expose())?;
        self.keep(account, generation, bytes.finish())
    }

    pub(crate) fn keep(&mut self, account: u32, generation: u64, value: Box<[u8]>) -> Result<(), Error> {
        if generation == 0 {
            return Err(Error::Grants);
        }
        let mut oldest = None;
        let mut newest = 0_u64;
        let mut count = 0_u32;
        for ((held_account, held_generation), _) in &self.values {
            if *held_account == account {
                count = count.checked_add(1).ok_or(Error::Grants)?;
                if oldest.is_none() {
                    oldest = Some(*held_generation);
                }
                newest = newest.max(*held_generation);
            }
        }
        if generation <= newest && count > 0 {
            return Err(Error::Grants);
        }
        if count >= 2 {
            let oldest = oldest.ok_or(Error::Grants)?;
            self.values.remove(&(account, oldest));
        }
        if self.values.insert((account, generation), Secret(value)).is_err() {
            return Err(Error::Grants);
        }
        Ok(())
    }
}
