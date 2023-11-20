use mio::net::TcpStream;
use tracing::warn;

use std::io::{Read, Write};

use crate::{
    buf::Buffer,
    error::{NetError, NetResult},
    header, raw, Packet,
};

pub struct Peer<S> {
    pub conected: bool,

    pub writeable: bool,

    pub write_buffer: Buffer,
    pub read_buffer: Buffer,

    pub socket: S,
}

impl<S> Peer<S> {
    pub fn new(socket: S) -> Self {
        Peer {
            conected: false,
            writeable: false,
            write_buffer: Buffer::new(),
            read_buffer: Buffer::new(),
            socket,
        }
    }
}

impl Peer<TcpStream> {
    pub fn connect(&mut self) -> NetResult<()> {
        self.conected = true;
        self.socket.set_nodelay(true)?;

        Ok(())
    }
}

impl<S> Peer<S>
where
    for<'a> &'a mut S: Write,
{
    pub fn write_packet<P: Packet>(&mut self, packet: &P, temp: &mut Buffer) -> NetResult<()> {
        // Clear junk from buffer
        temp.reset();

        // Write the packet to the buffer
        write_packet_to_buffer(packet, temp)?;

        // Write the buffer to the socket
        {
            if self.conected && self.writeable {
                let writeable = raw::raw_write(&mut self.socket, temp)?;
                self.writeable = writeable;
            }

            // Store any data not written to the socket untill the next writeable event
            self.write_buffer.copy_from(temp.get_written());
        }

        Ok(())
    }

    pub fn write_remaining(&mut self) -> NetResult<()> {
        let writeable = raw::raw_write(&mut self.socket, &mut self.write_buffer)?;
        self.writeable = writeable;

        // Move any remaining data to the front of the buffer
        self.write_buffer.consume(0);

        Ok(())
    }
}

impl<S: Read> Peer<S> {
    pub fn read_packet<P: Packet>(&mut self, temp: &mut Buffer) -> NetResult<Option<P>> {
        temp.reset();

        // Copy any unprocessed data from last read
        temp.copy_from(self.read_buffer.get_written());
        self.read_buffer.reset();

        // A packet may be split across multiple read calls
        // And a single read call may return multiple packets
        let packet = loop {
            // Attempt to parse a packet
            if let Some(packet) = try_read_one_packet_from_buffer(temp)? {
                break Some(packet);
            }

            // Not enough data was available
            // Try to read some more for the next irreration
            let readable = raw::raw_read_once(&mut self.socket, temp)?;

            if !readable {
                // There was no more data to read
                break None;
            }
        };

        // Keep unprocessed data for a future read
        self.read_buffer.copy_from(temp.get_written());

        Ok(packet)
    }
}

fn write_packet_to_buffer<P: Packet>(packet: &P, temp: &mut Buffer) -> NetResult<()> {
    // Get a write slice of the correct size
    let expected_size =
        header::HEADER_SIZE + packet.expected_size().map_err(NetError::WritingError)? as usize;
    let mut buffer = temp.get_unwritten(expected_size);

    // Leave room for the header
    let header = header::Header::new(&mut buffer);

    // Write the packet into the buffer
    let available = buffer.len();
    packet
        .write_buf(&mut buffer)
        .map_err(NetError::WritingError)?;
    let remaining = buffer.len();

    // Retrospectively write the header
    let packet_size = available - remaining;
    header
        .write(packet_size)
        .map_err(|_| NetError::OversizedPacket(packet_size))?;

    // Advance the buffer by the amount written
    let total_written = expected_size - remaining;
    unsafe {
        // Safety: We wrote something
        temp.advance_write(total_written);
    }

    Ok(())
}

fn try_read_one_packet_from_buffer<P: Packet>(temp: &mut Buffer) -> NetResult<Option<P>> {
    let mut maybe_complete_packet_buf = temp.get_written();

    // Check if a complete packet is available
    let len = header::Header::read(&mut maybe_complete_packet_buf);
    if let Some(len) = len {
        let available = maybe_complete_packet_buf.len();

        // If there is a packet available
        // Read it
        if available >= len {
            // We've already read the header, discard it
            temp.advance_read(header::HEADER_SIZE);
            // Get the packet slice
            let mut complete_packet_buf = temp.advance_read(len);

            // Try to parse the packet
            let packet = P::read_buf(&mut complete_packet_buf).map_err(NetError::ParsingError)?;

            // There was an issue parsing the packet
            if !complete_packet_buf.is_empty() {
                warn!("Packet not completely read");
            }

            // Found a good packet
            return Ok(Some(packet));
        }
    }

    // No complete packets found
    Ok(None)
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use bincode::{DefaultOptions, Options};
    use serde::{Deserialize, Serialize};

    use crate::{
        buf::Buffer,
        peer::{try_read_one_packet_from_buffer, write_packet_to_buffer},
        Packet,
    };

    #[test]
    fn roundtrip_packet() {
        #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
        struct Proto {
            int: u64,
            float: f64,
            string: String,
        }

        impl Packet for Proto {
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

        let mut buffer = Buffer::new();

        let packet_1 = Proto {
            int: 42,
            float: core::f64::consts::PI,
            string: "Hello world".to_owned(),
        };

        let packet_2 = Proto {
            int: 101,
            float: core::f64::consts::E,
            string: "Random Thing".to_owned(),
        };

        let packet_3 = Proto {
            int: u64::MAX,
            float: core::f64::consts::FRAC_2_SQRT_PI,
            string: "This is a packet".to_owned(),
        };

        write_packet_to_buffer(&packet_1, &mut buffer).expect("Write packet");
        write_packet_to_buffer(&packet_2, &mut buffer).expect("Write packet");
        write_packet_to_buffer(&packet_3, &mut buffer).expect("Write packet");

        let packet: Proto = try_read_one_packet_from_buffer(&mut buffer)
            .expect("Read packet")
            .expect("Parse packet");
        assert_eq!(packet, packet_1, "Packet 1");

        let packet: Proto = try_read_one_packet_from_buffer(&mut buffer)
            .expect("Read packet")
            .expect("Parse packet");
        assert_eq!(packet, packet_2, "Packet 2");

        let packet: Proto = try_read_one_packet_from_buffer(&mut buffer)
            .expect("Read packet")
            .expect("Parse packet");
        assert_eq!(packet, packet_3, "Packet 3");
    }
}
