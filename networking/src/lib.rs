#![feature(split_array)]

pub mod error;

pub(crate) mod acceptor;
pub(crate) mod buf;
pub(crate) mod header;
pub(crate) mod peer;
pub(crate) mod raw;
pub(crate) mod worker;

use crossbeam::channel::{self, Receiver, Sender};
pub use mio::Token;
use mio::{Poll, Waker};

use std::{net::SocketAddr, sync::Arc};

const WAKER_TOKEN: Token = Token(0);

const PROBE_LENGTH: usize = 4096;

#[derive(Debug)]
pub struct Networking<P> {
    poll: Poll,
    waker: Arc<Waker>,
    queue: (Sender<Message<P>>, Receiver<Message<P>>),
}

impl<P: Packet> Networking<P> {
    pub fn new() -> error::NetResult<Self> {
        let poll = Poll::new()?;

        let waker = Waker::new(poll.registry(), WAKER_TOKEN)?;
        let waker = Arc::new(waker);

        let queue = channel::bounded(50);

        Ok(Networking { poll, waker, queue })
    }

    pub fn messenger(&self) -> Messenger<P> {
        Messenger {
            waker: self.waker.clone(),
            sender: self.queue.0.clone(),
        }
    }

    pub fn start(self, handler: impl FnMut(Event<P>)) {
        let Networking { poll, waker, queue } = self;
        let _ = waker;

        worker::start_worker(poll, queue.1, handler);
    }
}

pub trait Packet: Clone {
    fn expected_size(&self) -> anyhow::Result<u64>;
    fn write_buf(self, buffer: &mut &mut [u8]) -> anyhow::Result<()>;
    fn read_buf(buffer: &mut &[u8]) -> anyhow::Result<Self>;
}

#[derive(Debug)]
pub enum Event<P> {
    Conected(Token, SocketAddr),
    Accepted(Token, SocketAddr),

    Data(Token, P),

    Error(Option<Token>, error::NetError),
}

#[derive(Debug)]
pub enum Message<P> {
    Connect(SocketAddr),
    Bind(SocketAddr),
    Disconect(Token),
    Packet(Token, P),
    PacketBrodcast(P),
    Shutdown,
}

#[derive(Debug)]
pub struct Messenger<P> {
    waker: Arc<Waker>,
    sender: Sender<Message<P>>,
}

impl<P> Messenger<P> {
    pub fn send_packet(&self, peer: Token, packet: P) -> Result<(), error::MessageError> {
        let message = Message::Packet(peer, packet);

        self.send_message(message)
    }

    pub fn brodcast_packet(&self, packet: P) -> Result<(), error::MessageError> {
        let message = Message::PacketBrodcast(packet);

        self.send_message(message)
    }

    pub fn connect_to(&self, peer: SocketAddr) -> Result<(), error::MessageError> {
        let message = Message::Connect(peer);

        self.send_message(message)
    }

    pub fn disconnect(&self, peer: Token) -> Result<(), error::MessageError> {
        let message = Message::Disconect(peer);

        self.send_message(message)
    }

    pub fn bind_at(&self, addr: SocketAddr) -> Result<(), error::MessageError> {
        let message = Message::Bind(addr);

        self.send_message(message)
    }

    pub fn shutdown(&self) -> Result<(), error::MessageError> {
        let message = Message::Shutdown;

        self.send_message(message)
    }

    pub fn send_message(&self, message: Message<P>) -> Result<(), error::MessageError> {
        self.sender
            .try_send(message)
            .map_err(|_| error::MessageError)?;
        self.waker.wake().map_err(|_| error::MessageError)
    }
}
