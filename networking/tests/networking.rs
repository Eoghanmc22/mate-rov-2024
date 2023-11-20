use std::{
    net::ToSocketAddrs,
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Barrier,
    },
    thread,
    time::Duration,
};

use anyhow::Context;
use bincode::{DefaultOptions, Options};
use networking::{Event, Networking, Packet};
use serde::{Deserialize, Serialize};

#[test]
fn test_real_server_client() -> anyhow::Result<()> {
    let servers_a = (0u16..5).map(|it| it + 2048).collect::<Vec<_>>();
    let servers_b = (0u16..5).map(|it| it + 4096).collect::<Vec<_>>();

    let mut connected = AtomicU32::new(0);
    let mut accepted = AtomicU32::new(0);
    let mut pong = AtomicU64::new(0);

    let peer_a = Networking::<Protocol>::new()?;
    let messenger_a = peer_a.messenger();

    let peer_b = Networking::<Protocol>::new()?;
    let messenger_b = peer_b.messenger();

    let barrier = Barrier::new(2);

    thread::scope(|scope| -> Result<(), anyhow::Error> {
        scope.spawn(|| {
            peer_a.start(|event| match event {
                Event::Conected(_token, _socket) => {
                    connected.fetch_add(1, Ordering::Relaxed);
                }
                Event::Accepted(_token, _socket) => {
                    accepted.fetch_add(1, Ordering::Relaxed);
                }
                Event::Data(token, packet) => match packet {
                    Protocol::Ping(id) => {
                        messenger_a.send_packet(token, Protocol::Pong(id)).unwrap();
                    }
                    Protocol::Pong(id) => {
                        pong.fetch_add(id, Ordering::Relaxed);
                    }
                },
                Event::Disconnect(_token) => {
                    // Dont care
                }
                Event::Error(_token, error) => {
                    panic!("Error: {error}");
                }
            });
        });

        scope.spawn(|| {
            peer_b.start(|event| match event {
                Event::Conected(_token, _socket) => {
                    connected.fetch_add(1, Ordering::Relaxed);
                }
                Event::Accepted(_token, _socket) => {
                    accepted.fetch_add(1, Ordering::Relaxed);
                }
                Event::Data(token, packet) => match packet {
                    Protocol::Ping(id) => {
                        messenger_b.send_packet(token, Protocol::Pong(id)).unwrap();
                    }
                    Protocol::Pong(id) => {
                        pong.fetch_add(id, Ordering::Relaxed);
                    }
                },
                Event::Disconnect(_token) => {
                    // Dont care
                }
                Event::Error(_token, error) => {
                    panic!("Error: {error}");
                }
            });
        });

        scope.spawn(|| {
            for port in &servers_a {
                messenger_a
                    .bind_at(
                        ("127.0.0.1", *port)
                            .to_socket_addrs()
                            .expect("DNS")
                            .next()
                            .expect("Find SocketAddr"),
                    )
                    .unwrap()
            }

            barrier.wait();

            for port in &servers_b {
                messenger_a
                    .connect_to(
                        ("127.0.0.1", *port)
                            .to_socket_addrs()
                            .expect("DNS")
                            .next()
                            .expect("Find SocketAddr"),
                    )
                    .unwrap()
            }

            barrier.wait();

            for i in 0..100 {
                messenger_a.brodcast_packet(Protocol::Ping(i)).unwrap();
                thread::sleep(Duration::from_micros(100));
            }

            barrier.wait();

            messenger_a.shutdown()
        });

        scope.spawn(|| {
            for port in &servers_b {
                messenger_b
                    .bind_at(
                        ("127.0.0.1", *port)
                            .to_socket_addrs()
                            .expect("DNS")
                            .next()
                            .expect("Find SocketAddr"),
                    )
                    .unwrap()
            }

            barrier.wait();

            for port in &servers_a {
                messenger_b
                    .connect_to(
                        ("127.0.0.1", *port)
                            .to_socket_addrs()
                            .expect("DNS")
                            .next()
                            .expect("Find SocketAddr"),
                    )
                    .unwrap()
            }

            barrier.wait();

            for i in 0..100 {
                messenger_b.brodcast_packet(Protocol::Ping(i)).unwrap();
                thread::sleep(Duration::from_micros(100));
            }

            barrier.wait();

            messenger_b.shutdown()
        });

        Ok(())
    })?;

    assert_eq!(
        *connected.get_mut(),
        (servers_a.len() + servers_b.len()) as u32
    );
    assert_eq!(
        *accepted.get_mut(),
        (servers_a.len() + servers_b.len()) as u32
    );
    assert_eq!(
        *pong.get_mut(),
        2 * 4950 * (servers_a.len() + servers_b.len()) as u64
    );

    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum Protocol {
    Ping(u64),
    Pong(u64),
}

impl Packet for Protocol {
    fn expected_size(&self) -> anyhow::Result<u64> {
        options()
            .serialized_size(self)
            .context("Could not compute expected size")
    }

    fn write_buf(&self, buffer: &mut &mut [u8]) -> anyhow::Result<()> {
        options()
            .serialize_into(buffer, self)
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
