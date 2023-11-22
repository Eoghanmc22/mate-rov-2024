use std::mem;

use tracing::instrument;

pub const HEADER_SIZE: usize = 4;

pub struct Header<'a>(&'a mut [u8; HEADER_SIZE]);

impl<'a> Header<'a> {
    /// Needs at least `HEADER_SIZE` bytes in `buffer`
    #[instrument(level = "trace", skip_all)]
    pub fn new(buffer: &mut &'a mut [u8]) -> Self {
        // Lifetime dance taken from `impl Write for &mut [u8]`.
        let (header, remaining) = mem::take(buffer).split_array_mut();
        *buffer = remaining;

        Self(header)
    }

    /// Returns Err if len doesn't fit
    #[instrument(level = "trace", skip(self))]
    pub fn write(self, len: usize) -> Result<(), ()> {
        let header: u32 = len.try_into().map_err(|_| ())?;
        let header: [u8; HEADER_SIZE] = header.to_le_bytes();

        *self.0 = header;

        Ok(())
    }

    #[instrument(level = "trace", skip_all, ret)]
    pub fn read(buffer: &mut &[u8]) -> Option<usize> {
        if buffer.len() < HEADER_SIZE {
            return None;
        }

        let (header, remaining) = buffer.split_array_ref();
        *buffer = remaining;

        Some(u32::from_le_bytes(*header) as _)
    }
}
