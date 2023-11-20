use crate::{
    acceptor::Acceptor, buf::Buffer, error::NetError, peer::Peer, Event, Message, Packet,
    PROBE_LENGTH, WAKER_TOKEN,
};
use ahash::HashMap;
use crossbeam::channel::Receiver;
use mio::{
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Token,
};
use std::{
    io::ErrorKind,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
};
use tracing::{error, span, warn, Level};

static NEXT_TOKEN: AtomicUsize = AtomicUsize::new(1);

pub fn start_worker<P: Packet>(
    mut poll: Poll,
    receiver: Receiver<Message<P>>,
    mut handler: impl FnMut(Event<P>),
) {
    span!(Level::INFO, "Network Worker Thread");

    let mut peers = HashMap::default();
    let mut accptors = HashMap::default();
    let mut temp_buf = Buffer::with_capacity(PROBE_LENGTH * 2);

    let mut events = Events::with_capacity(100);

    'outer: loop {
        let res = poll.poll(&mut events, None);

        if let Err(err) = res {
            error!("Could not poll, sleeping 300ms");
            (handler)(Event::Error(None, err.into()));

            // Slight cool down to avoid a possible error spam
            thread::sleep(Duration::from_millis(300));
            continue 'outer;
        }

        'event: for event in &events {
            if event.token() == WAKER_TOKEN {
                // Handle incomming Message events
                'message: for message in receiver.try_iter() {
                    match message {
                        Message::Connect(peer) => {
                            // Create socket
                            let res = TcpStream::connect(peer);
                            let mut socket = match res {
                                Ok(socket) => socket,
                                Err(err) => {
                                    (handler)(Event::Error(
                                        None,
                                        NetError::from(err).chain("Connect to peer".to_owned()),
                                    ));
                                    continue 'message;
                                }
                            };

                            // Assign Token
                            let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
                            let token = Token(token);

                            // Register event intreast
                            let res = poll.registry().register(
                                &mut socket,
                                token,
                                Interest::READABLE | Interest::WRITABLE,
                            );
                            if let Err(err) = res {
                                (handler)(Event::Error(
                                    Some(token),
                                    NetError::from(err).chain("Register socket".to_owned()),
                                ));
                                (handler)(Event::Disconnect(token));
                                continue 'message;
                            }

                            let peer = Peer::new(socket);

                            // Register peer
                            peers.insert(token, peer);
                        }
                        Message::Bind(addr) => {
                            // Create listner
                            let listener = TcpListener::bind(addr);
                            let mut listener = match listener {
                                Ok(socket) => socket,
                                Err(err) => {
                                    (handler)(Event::Error(
                                        None,
                                        NetError::from(err).chain("Bind listner".to_owned()),
                                    ));
                                    continue 'message;
                                }
                            };

                            // Assign token
                            let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
                            let token = Token(token);

                            // Register event intreast
                            let res =
                                poll.registry()
                                    .register(&mut listener, token, Interest::READABLE);
                            if let Err(err) = res {
                                (handler)(Event::Error(
                                    Some(token),
                                    NetError::from(err).chain("Register listner".to_owned()),
                                ));
                                (handler)(Event::Disconnect(token));
                                continue 'message;
                            }

                            // Register acceptor
                            accptors.insert(token, Acceptor { listener });
                        }
                        Message::Disconect(token) => {
                            (handler)(Event::Disconnect(token));
                            peers.remove(&token);
                            accptors.remove(&token);
                        }
                        Message::Packet(peer_token, packet) => {
                            // Lookup peer and send packet
                            if let Some(peer) = peers.get_mut(&peer_token) {
                                let res = peer.write_packet(&packet, &mut temp_buf);
                                if let Err(err) = res {
                                    (handler)(Event::Error(
                                        Some(peer_token),
                                        err.chain("Write packet".to_owned()),
                                    ));
                                    (handler)(Event::Disconnect(peer_token));
                                    peers.remove(&peer_token);
                                    continue 'message;
                                }
                            } else {
                                // Handle peer not found
                                (handler)(Event::Error(
                                    None,
                                    NetError::UnknownPeer(peer_token)
                                        .chain("Write packet".to_owned()),
                                ));
                                continue 'message;
                            }
                        }
                        Message::PacketBrodcast(packet) => {
                            let mut to_remove = Vec::new();

                            // Send packet to every peer
                            'peer: for (token, peer) in &mut peers {
                                let res = peer.write_packet(&packet, &mut temp_buf);
                                if let Err(err) = res {
                                    (handler)(Event::Error(
                                        Some(*token),
                                        err.chain("Brodcast packet".to_owned()),
                                    ));
                                    (handler)(Event::Disconnect(*token));
                                    to_remove.push(*token);
                                    continue 'peer;
                                }
                            }

                            // Remove peers that errored
                            // Needed to bypass lifetime issues
                            for token in to_remove {
                                peers.remove(&token);
                            }
                        }
                        Message::Shutdown => {
                            break 'outer;
                        }
                    }
                }
            } else if let Some(peer) = peers.get_mut(&event.token()) {
                // Peers don't connect isntantly
                // Set up the socket if the peer just connected
                // else ignore events for unconected peers
                if !peer.conected && !event.is_error() {
                    if event.is_writable() {
                        match peer.socket.peer_addr() {
                            Ok(addr) => {
                                let res = peer.connect();
                                match res {
                                    Ok(()) => {
                                        (handler)(Event::Conected(event.token(), addr));
                                        // Happy path
                                    }
                                    Err(err) => {
                                        // Couldnt setup the peer's socket
                                        (handler)(Event::Error(
                                            Some(event.token()),
                                            err.chain("Setup peer socket".to_owned()),
                                        ));
                                        (handler)(Event::Disconnect(event.token()));
                                        peers.remove(&event.token());
                                        continue 'event;
                                    }
                                }
                            }
                            Err(err) if err.kind() == ErrorKind::NotConnected => {
                                // Try again on the next event
                                continue 'event;
                            }
                            Err(err) => {
                                // Couldnt connect for whatever reason
                                (handler)(Event::Error(
                                    Some(event.token()),
                                    NetError::from(err).chain("Connect to peer".to_owned()),
                                ));
                                (handler)(Event::Disconnect(event.token()));
                                peers.remove(&event.token());
                                continue 'event;
                            }
                        }
                    } else {
                        // Shouldn't be hit but this is not guranetted
                        // Ignore false event
                        continue 'event;
                    }
                }

                // Handle the socket being newly writeable
                if event.is_writable() {
                    // Write any buffered packets
                    // Also marks peer as writeable if it preaviously wasnt
                    let res = peer.write_remaining();
                    if let Err(err) = res {
                        (handler)(Event::Error(
                            Some(event.token()),
                            err.chain("Write packets".to_owned()),
                        ));
                        (handler)(Event::Disconnect(event.token()));
                        peers.remove(&event.token());
                        continue 'event;
                    }
                }

                // Handle the socket being newly readable
                if event.is_readable() {
                    // Read all incomming packets from peer
                    'packets: loop {
                        let res = peer.read_packet(&mut temp_buf);
                        match res {
                            Ok(Some(packet)) => {
                                (handler)(Event::Data(event.token(), packet));
                            }
                            Ok(None) => {
                                break 'packets;
                            }
                            Err(err) => {
                                (handler)(Event::Error(
                                    Some(event.token()),
                                    err.chain("Read packets".to_owned()),
                                ));
                                (handler)(Event::Disconnect(event.token()));
                                peers.remove(&event.token());
                                continue 'event;
                            }
                        }
                    }
                }
            } else if let Some(acceptor) = accptors.get_mut(&event.token()) {
                if event.is_readable() {
                    // Accept all new connections
                    'accept: loop {
                        // Create socket
                        let res = acceptor.listener.accept();
                        let (mut socket, addr) = match res {
                            Ok(socket) => socket,
                            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                                break 'accept;
                            }
                            Err(err) => {
                                (handler)(Event::Error(
                                    None,
                                    NetError::from(err).chain("Accept to peer".to_owned()),
                                ));
                                continue 'accept;
                            }
                        };

                        // Assign token
                        let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
                        let token = Token(token);

                        // Register event intreast
                        let res = poll.registry().register(
                            &mut socket,
                            token,
                            Interest::READABLE | Interest::WRITABLE,
                        );
                        if let Err(err) = res {
                            (handler)(Event::Error(
                                Some(token),
                                NetError::from(err).chain("Register accepted".to_owned()),
                            ));
                            (handler)(Event::Disconnect(token));
                            continue 'accept;
                        }

                        let mut peer = Peer::new(socket);

                        // Should already be connected
                        // Setup the socket
                        let res = peer.connect();
                        if let Err(err) = res {
                            (handler)(Event::Error(
                                Some(token),
                                err.chain("Setup accepted socket".to_owned()),
                            ));
                            (handler)(Event::Disconnect(token));
                            continue 'accept;
                        }

                        (handler)(Event::Accepted(event.token(), addr));

                        // Register peer
                        peers.insert(token, peer);
                    }
                }
            } else {
                warn!("Got event for unknown token");
            }
        }
    }
}
