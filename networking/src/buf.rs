use std::{
    fmt::{self, Debug},
    ptr,
};

use tracing::instrument;

#[derive(Clone)]
pub struct Buffer {
    vec: Vec<u8>,
    write_index: usize,
    read_index: usize,
}

impl Buffer {
    pub const fn new() -> Self {
        Self {
            vec: Vec::new(),
            write_index: 0,
            read_index: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vec: Vec::with_capacity(capacity),
            ..Self::new()
        }
    }

    pub fn len(&self) -> usize {
        self.write_index - self.read_index
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[instrument(level = "trace")]
    pub fn reset(&mut self) {
        self.write_index = 0;
        self.read_index = 0;
    }

    #[instrument(level = "trace")]
    pub fn get_written(&self) -> &[u8] {
        let ptr = self.vec.as_ptr();
        unsafe { std::slice::from_raw_parts(ptr.add(self.read_index), self.len()) }
    }

    #[instrument(level = "trace")]
    pub fn get_unwritten(&mut self, capacity: usize) -> &mut [u8] {
        self.vec.reserve(self.write_index + capacity);

        unsafe {
            let ptr = self.vec.as_mut_ptr().add(self.write_index);
            std::slice::from_raw_parts_mut(ptr, capacity)
        }
    }

    #[instrument(level = "trace")]
    pub fn copy_from(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }

        self.get_unwritten(bytes.len()).copy_from_slice(bytes);
        unsafe {
            self.advance_write(bytes.len());
        }
    }

    #[instrument(level = "trace")]
    pub fn consume(&mut self, amount: usize) {
        debug_assert!(
            amount <= self.len(),
            "amount {} must be <= the length {}",
            amount,
            self.len()
        );

        let remaining = self.len() - amount;

        unsafe {
            let src = self.vec.as_ptr().add(self.read_index + amount);
            let dst = self.vec.as_mut_ptr();
            ptr::copy(src, dst, remaining);
        }

        self.write_index = remaining;
        self.read_index = 0
    }

    /// This function should be used after successfully writing some data with `get_unwritten`
    ///
    /// # Safety
    /// 1. `advance` must be at most the capacity requested in `get_unwritten`
    /// 2.  At least `advance` bytes must have been written to the slice returned by `get_unwritten`,
    ///     otherwise `get_written` will return uninitialized memory
    #[instrument(level = "trace")]
    pub unsafe fn advance_write(&mut self, advance: usize) -> &[u8] {
        debug_assert!(
            self.write_index + advance <= self.vec.capacity(),
            "advance {} must be <= the remaining bytes {}",
            advance,
            self.vec.capacity() - self.write_index
        );

        let ptr = self.vec.as_ptr().add(self.write_index);
        let slice = std::slice::from_raw_parts(ptr, advance);

        self.write_index += advance;

        slice
    }

    #[instrument(level = "trace")]
    pub fn advance_read(&mut self, advance: usize) -> &[u8] {
        assert!(
            self.read_index + advance <= self.write_index,
            "Can not advance read idx ({}) past write ({}), A: {}",
            self.read_index,
            self.write_index,
            advance
        );

        let slice = unsafe {
            let ptr = self.vec.as_ptr().add(self.read_index);
            std::slice::from_raw_parts(ptr, advance)
        };

        self.read_index += advance;

        slice
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Buffer")
            .field("write_index", &self.write_index)
            .field("read_index", &self.read_index)
            .field("len", &self.vec.len())
            .field("cap", &self.vec.capacity())
            .finish_non_exhaustive()
    }
}
