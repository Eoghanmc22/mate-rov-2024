use std::io::{ErrorKind, Read, Write};

use tracing::{instrument, trace};

use crate::{
    buf::Buffer,
    error::{NetError, NetResult},
    PROBE_LENGTH,
};

// Returns if the socket is still writeable
// Callees need to handle any data remaining in `buffer`
#[instrument(level = "trace", skip(socket))]
pub fn raw_write<S: Write>(mut socket: S, buffer: &mut Buffer) -> NetResult<bool> {
    while !buffer.is_empty() {
        let to_write = buffer.get_written();

        let res = socket.write(to_write);
        trace!(to_write = to_write.len(), result = ?res, "Socket write");

        match res {
            Ok(0) => {
                // Write zero means that the connection got closed
                return Err(NetError::PeerClosed);
            }
            Ok(count) => {
                // Data has been read from the buffer and written to the socket
                // Advance the read idx so data doesn't get written multiple times
                buffer.advance_read(count);
            }

            // An error case means nothing has been written
            // Don't need to update `buffer`
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                return Ok(false);
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => {
                continue;
            }
            Err(err) => {
                return Err(err.into());
            }
        }
    }

    Ok(true)
}

// Returns true if the socket is still readable
#[allow(unreachable_code)]
#[instrument(level = "trace", skip(socket))]
pub fn raw_read_once<S: Read>(mut socket: S, buffer: &mut Buffer) -> NetResult<bool> {
    let read_dest = buffer.get_unwritten(PROBE_LENGTH);

    // Need loop in the unlikely case of an interruption
    loop {
        let res = socket.read(read_dest);
        trace!(result = ?res, "Socket read");

        match res {
            Ok(0) => {
                // Read zero means that the connection got closed
                return Err(NetError::PeerClosed);
            }
            Ok(count) => {
                // Data has been read from the socket and written to the buffer
                // Advance the write idx so data doesn't get overwritten
                // Safety: We read something
                unsafe {
                    buffer.advance_write(count);
                }

                return Ok(true);
            }

            // An error case means nothing has been read
            // Don't need to update `buffer`
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                return Ok(false);
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => {
                continue;
            }
            Err(err) => {
                return Err(err.into());
            }
        }

        unreachable!()
    }
}
