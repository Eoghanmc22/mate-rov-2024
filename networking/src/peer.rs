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
    pub fn write_packet<P: Packet>(&mut self, packet: P, temp: &mut Buffer) -> NetResult<()> {
        temp.reset();

        // Write the packet to the buffer
        {
            let expected_size = header::HEADER_SIZE
                + packet.expected_size().map_err(NetError::WritingError)? as usize;
            let mut buffer = temp.get_unwritten(expected_size);

            let header = header::Header::new(&mut buffer);

            let available = buffer.len();
            packet
                .write_buf(&mut buffer)
                .map_err(NetError::WritingError)?;
            let remaining = buffer.len();

            let packet_size = available - remaining;
            header
                .write(packet_size)
                .map_err(|_| NetError::OversizedPacket(packet_size))?;

            let total_written = expected_size - remaining;

            // Safety: We wrote something
            unsafe {
                temp.advance_write(total_written);
            }
        }

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
            {
                let mut maybe_complete_packet_buf = temp.get_written();

                // Check if a complete packet is available
                let len = header::Header::read(&mut maybe_complete_packet_buf);
                if let Some(len) = len {
                    let available = maybe_complete_packet_buf.len();
                    if available >= len {
                        // There is a packet available
                        // Read it
                        temp.advance_read(header::HEADER_SIZE);
                        let mut complete_packet_buf = temp.advance_read(len);
                        let packet = P::read_buf(&mut complete_packet_buf)
                            .map_err(NetError::ParsingError)?;

                        if !complete_packet_buf.is_empty() {
                            warn!("Packet not completely read");
                        }

                        break Some(packet);
                    }
                }
            }

            // Not enough data was available
            // Read some more for the next irreration
            let readable = raw::raw_read_once(&mut self.socket, temp)?;
            if !readable {
                break None;
            }
        };

        // Keep unprocessed data for a future read
        self.read_buffer.copy_from(temp.get_written());

        Ok(packet)
    }
}
