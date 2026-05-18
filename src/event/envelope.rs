use serde::Serialize;

use crate::event::channel::Channel;

/// Wraps a payload with its channel discriminator. The runtime stdout
/// serializer writes one envelope per JSONL line; the `channel` field
/// becomes the routing key on the wire.
///
/// Borrowing form so payloads don't have to clone when serializing.
#[derive(Debug, Serialize)]
pub struct BusEnvelope<'a, T: Serialize> {
    pub channel: Channel,
    #[serde(flatten)]
    pub payload: &'a T,
}

impl<'a, T: Serialize> BusEnvelope<'a, T> {
    #[must_use]
    pub fn new(channel: Channel, payload: &'a T) -> Self {
        Self { channel, payload }
    }
}
