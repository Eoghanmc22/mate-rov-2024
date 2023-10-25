//! Repersents the protocol used for two way communication

use anyhow::Context;
use bincode::{DefaultOptions, Options};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Representation of all messages that can be communicated between peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Protocol {
    /// Asks the peer to reply with a Pong, used to measure communication latency
    Ping { ping: SystemTime },
    /// Response to a Ping, used to measure communication latency
    Pong { ping: SystemTime, pong: SystemTime },
}

impl networking::Packet for Protocol {
    fn expected_size(&self) -> anyhow::Result<u64> {
        options()
            .serialized_size(self)
            .context("Could not compute expected size")
    }

    fn write_buf(self, buffer: &mut &mut [u8]) -> anyhow::Result<()> {
        options()
            .serialize_into(buffer, &self)
            .context("Could not serialize packet")
    }

    fn read_buf(buffer: &mut &[u8]) -> anyhow::Result<Self> {
        options()
            .deserialize_from(buffer)
            .context("Could not deserialize packet")
    }
}

fn options() -> impl Options {
    DefaultOptions::new()
}
