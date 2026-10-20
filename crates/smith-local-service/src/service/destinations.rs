//! Product destinations belong to the chosen process's producer, never its child.
//! Adoption retains only IO tokens; startup follows adoption (protocol/events.md, sections 2 and 5.2).
use super::{Effects, Service};
use skein_io::kernel;
use skein_lib::Time;
use smith_agent_service::sinks::{Delivery, Destination};

impl Service {
    /// Adopt JSON output as an append file or a write stream for either placement.
    pub fn adopt_output(&mut self, descriptor: kernel::Fd, append: bool, delivery: Delivery) -> Result<(), kernel::Fd> {
        match &mut self.below {
            Effects::Inline { agent, local } => {
                agent.adopt_destination(descriptor, Destination::Output, delivery, append, local.io())
            }
            Effects::Spawned(process) => {
                let Some(stream) = &mut self.stream else { return Err(descriptor) };
                if !stream.producer.sinks().room_for_destination(Destination::Output) {
                    return Err(descriptor);
                }
                let token = process.adopt_output(descriptor, append, stream.write_deadline)?;
                stream
                    .producer
                    .sinks()
                    .add(Destination::Output, token, delivery)
                    .expect("destination room checked before adoption");
                Ok(())
            }
        }
    }

    /// Start the chosen product stream after its destinations have been adopted.
    pub fn start_session(&mut self, now: Time, header: smith_events::SessionStarted) {
        match &mut self.below {
            Effects::Inline { agent, .. } => agent.start_session(now, header),
            Effects::Spawned(_) => {
                if let Some(stream) = &mut self.stream {
                    stream.producer.started(now, header);
                }
            }
        }
    }
}
