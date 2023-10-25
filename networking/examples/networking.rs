use std::{
    net::SocketAddr,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use anyhow::Context;
use bincode::{DefaultOptions, Options};
use clap::{Parser, ValueEnum};
use networking::{Event, Networking, Packet};
use serde::{Deserialize, Serialize};

static STOP_THE_WORLD: AtomicBool = AtomicBool::new(false);

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let net = Networking::<Protocol>::new()?;
    let messenger = net.messenger();

    let handle = {
        let messenger = net.messenger();
        thread::spawn(move || {
            net.start(|event| {
                println!("{event:?}");

                if let Event::Data(_token, packet) = event {
                    messenger
                        .brodcast_packet(packet)
                        .context("explosion")
                        .unwrap();
                }
            });
        })
    };

    {
        ctrlc::set_handler(|| {
            println!("Got ctrlc");
            STOP_THE_WORLD.store(true, Ordering::Relaxed);
        })
        .context("ctrlc")?;

        match args.mode {
            Modes::Server => {
                messenger.bind_at(args.ip).context("bind")?;
            }
            Modes::Client => {
                messenger.connect_to(args.ip).context("connect")?;
            }
        };

        while !STOP_THE_WORLD.load(Ordering::Relaxed) {
            messenger
                .brodcast_packet(Protocol::Packet("FANCY PAYLOAD EXPLOSION!!!!".to_owned()))
                .context("brodcast")?;
            thread::sleep(Duration::from_millis(500));
        }
    }

    println!("Shutting down");

    messenger.shutdown().context("shutdown")?;
    handle.join().expect("join");

    Ok(())
}

#[derive(Parser)]
struct Args {
    mode: Modes,
    ip: SocketAddr,
}

#[derive(ValueEnum, Clone, Debug)]
enum Modes {
    Server,
    Client,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum Protocol {
    Packet(String),
}

impl Packet for Protocol {
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
