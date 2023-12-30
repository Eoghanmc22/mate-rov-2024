use core::slice;
use std::{fmt::Debug, iter, ops::DerefMut, slice::SliceIndex, usize};

use anyhow::Context;
use rgb::{ComponentMap, RGB8};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use tracing::instrument;

pub struct Neopixel {
    pub spi: Spi,
    pub buffer: NeopixelBuffer,
}

impl Neopixel {
    pub const SPI_BUS: Bus = Bus::Spi0;
    pub const SPI_SELECT: SlaveSelect = SlaveSelect::Ss0;
    pub const SPI_CLOCK: u32 = 6_000_000;

    #[instrument(level = "debug")]
    pub fn new(
        len: usize,
        bus: Bus,
        slave_select: SlaveSelect,
        clock_speed: u32,
    ) -> anyhow::Result<Self> {
        let spi = Spi::new(bus, slave_select, clock_speed, Mode::Mode0).context("Open spi")?;
        let buffer = NeopixelBuffer::new(len);

        let mut this = Self { spi, buffer };
        this.show()?;

        Ok(this)
    }
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn fill<T, I>(&mut self, idx: I, color: RGB8, gamma_correction: bool)
    where
        T: AsSlice<u8> + ?Sized,
        I: SliceIndex<[u8], Output = T> + Debug + Copy,
    {
        self.buffer.fill(idx, color, gamma_correction)
    }

    /// Sets the range of LEDs specified by `idx` to the colors from the iterator `colors`
    /// If the colors iterator finishes before the end of the range, no more LEDs will be set
    pub fn set<T, I, C>(&mut self, idx: I, colors: C, gamma_correction: bool)
    where
        T: AsSlice<u8> + ?Sized,
        I: SliceIndex<[u8], Output = T> + Debug + Copy,
        C: Iterator<Item = RGB8>,
    {
        self.buffer.set(idx, colors, gamma_correction)
    }

    /// Replaces the buffer
    ///
    /// # Panics
    ///
    /// This  function may panic if the new buffer is a different length than the current one
    pub fn replace_buffer(&mut self, new_buffer: NeopixelBuffer) -> NeopixelBuffer {
        std::mem::replace(&mut self.buffer, new_buffer)
    }

    pub fn show(&mut self) -> anyhow::Result<()> {
        self.spi
            .write(self.buffer.to_slice())
            .context("Write neopixels")?;

        Ok(())
    }
}

impl Drop for Neopixel {
    fn drop(&mut self) {
        self.fill(.., RGB8::default(), false);
        let _ = self.show();
    }
}

#[derive(Clone)]
pub struct NeopixelBuffer {
    buffer: Vec<u8>,
}

impl NeopixelBuffer {
    pub fn new(len: usize) -> Self {
        Self {
            buffer: vec![0; len * 3],
        }
    }

    pub fn len(&self) -> usize {
        self.buffer.len() / 3
    }

    pub fn fill<T, I>(&mut self, idx: I, color: RGB8, gamma_correction: bool)
    where
        T: AsSlice<u8> + ?Sized,
        I: SliceIndex<[u8], Output = T> + Debug + Copy,
    {
        self.set(idx, iter::repeat(color), gamma_correction)
    }

    /// Sets the range of LEDs specified by `idx` to the colors from the iterator `colors`
    /// If the colors iterator finishes before the end of the range, no more LEDs will be set
    pub fn set<T, I, C>(&mut self, idx: I, colors: C, gamma_correction: bool)
    where
        T: AsSlice<u8> + ?Sized,
        I: SliceIndex<[u8], Output = T> + Debug + Copy,
        C: Iterator<Item = RGB8>,
    {
        let Some(buffer) = self.buffer.deref_mut().get_mut(idx) else {
            panic!(
                "Could not set neopixel. index ({idx:?}) out of bounds for length {}",
                self.len()
            );
        };
        let dst_iter = buffer.as_slice_mut().iter_mut();
        let src_iter = colors
            .map(|color| {
                if gamma_correction {
                    correct_color(color)
                } else {
                    color
                }
            })
            .flat_map(color_to_data);

        for (dst, src) in iter::zip(dst_iter, src_iter) {
            *dst = src
        }
    }

    pub fn to_slice(&self) -> &[u8] {
        &self.buffer
    }
}

// From smart_led crate
const GAMMA8: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5,
    5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 12, 12, 13, 13, 13, 14,
    14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 24, 24, 25, 25, 26, 27,
    27, 28, 29, 29, 30, 31, 32, 32, 33, 34, 35, 35, 36, 37, 38, 39, 39, 40, 41, 42, 43, 44, 45, 46,
    47, 48, 49, 50, 50, 51, 52, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 66, 67, 68, 69, 70, 72,
    73, 74, 75, 77, 78, 79, 81, 82, 83, 85, 86, 87, 89, 90, 92, 93, 95, 96, 98, 99, 101, 102, 104,
    105, 107, 109, 110, 112, 114, 115, 117, 119, 120, 122, 124, 126, 127, 129, 131, 133, 135, 137,
    138, 140, 142, 144, 146, 148, 150, 152, 154, 156, 158, 160, 162, 164, 167, 169, 171, 173, 175,
    177, 180, 182, 184, 186, 189, 191, 193, 196, 198, 200, 203, 205, 208, 210, 213, 215, 218, 220,
    223, 225, 228, 231, 233, 236, 239, 241, 244, 247, 249, 252, 255,
];

#[must_use]
pub fn correct_color(color: RGB8) -> RGB8 {
    color.map(|it| GAMMA8[it as usize])
}

fn color_to_data(color: RGB8) -> impl Iterator<Item = u8> {
    iter::from_coroutine(move || {
        yield byte_to_data(color.g);
        yield byte_to_data(color.r);
        yield byte_to_data(color.b);
    })
    .flatten()
}

fn byte_to_data(byte: u8) -> impl Iterator<Item = u8> {
    const LED_T0: u8 = 0b1100_0000;
    const LED_T1: u8 = 0b1111_1000;

    iter::from_coroutine(move || {
        for bit in 0..8 {
            if byte & (0x80 >> bit) != 0 {
                yield LED_T1;
            } else {
                yield LED_T0;
            }
        }
    })
}

trait AsSlice<T> {
    fn as_slice(&self) -> &[T];
    fn as_slice_mut(&mut self) -> &mut [T];
}

impl<T> AsSlice<T> for T {
    fn as_slice(&self) -> &[T] {
        slice::from_ref(self)
    }
    fn as_slice_mut(&mut self) -> &mut [T] {
        slice::from_mut(self)
    }
}

impl<T> AsSlice<T> for [T] {
    fn as_slice(&self) -> &[T] {
        self
    }
    fn as_slice_mut(&mut self) -> &mut [T] {
        self
    }
}
