use mio::Token;
use thiserror::Error;

use std::io;

pub type NetResult<T> = Result<T, NetError>;

#[derive(Error, Debug)]
pub enum NetError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    #[error("Peer closed socket")]
    PeerClosed,
    #[error("Tried to write packet with len {0} which does not fit in header")]
    OversizedPacket(usize),
    #[error("Messenging Error: {0}")]
    Message(#[from] MessageError),
    #[error("Tried to send packet to unknown peer: {0:?}")]
    UnknownPeer(Token),
    #[error("Could not write packet: {0}")]
    WritingError(anyhow::Error),
    #[error("Could not parse packet: {0}")]
    ParsingError(anyhow::Error),
    #[error("Error {0}: Caused by: ({1})")]
    Chain(String, #[source] Box<NetError>),
}

impl NetError {
    pub fn chain(self, message: String) -> Self {
        NetError::Chain(message, Box::new(self))
    }
}

#[derive(Error, Debug, Default)]
#[error("Failed to send message to worker")]
pub struct MessageError;
