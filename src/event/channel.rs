use serde::{Deserialize, Serialize};

/// Discriminator for which channel a message belongs to. Doubles as the
/// `channel` field every JSONL line carries on stdin / stdout.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Channel {
    Diagnostic,
    Input,
    Command,
    Domain,
    Marker,
    Presentation,
    CommandAck,
    Snapshot,
}
