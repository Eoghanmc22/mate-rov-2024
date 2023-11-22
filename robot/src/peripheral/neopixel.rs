use anyhow::Context;
use rgb::{ComponentMap, RGB8};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use tracing::instrument;

// TODO: Support a whole light strip
pub struct NeoPixel {
    spi: Spi,
}

impl NeoPixel {
    pub const SPI_BUS: Bus = Bus::Spi0;
    pub const SPI_SELECT: SlaveSelect = SlaveSelect::Ss0;
    pub const SPI_CLOCK: u32 = 6_000_000;

    #[instrument(level = "debug")]
    pub fn new(bus: Bus, slave_select: SlaveSelect, clock_speed: u32) -> anyhow::Result<Self> {
        let spi = Spi::new(bus, slave_select, clock_speed, Mode::Mode0).context("Open spi")?;

        let mut this = Self { spi };
        this.write_color_raw(RGB8::default())?;

        Ok(this)
    }

    pub fn write_color_raw(&mut self, color: RGB8) -> anyhow::Result<()> {
        let data = color_to_data(color);
        self.spi.write(&data).context("Write color")?;

        Ok(())
    }

    pub fn write_color_corrected(&mut self, color: RGB8) -> anyhow::Result<()> {
        let color = correct_color(color);
        self.write_color_raw(color)
    }
}

impl Drop for NeoPixel {
    fn drop(&mut self) {
        // Best effort attempt to reset color to black
        let _ = self.write_color_raw(RGB8::default());
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

fn color_to_data(color: RGB8) -> Vec<u8> {
    let mut data = Vec::new();

    byte_to_data(&mut data, color.g);
    byte_to_data(&mut data, color.r);
    byte_to_data(&mut data, color.b);

    data
}

fn byte_to_data(data: &mut Vec<u8>, byte: u8) {
    const LED_T0: u8 = 0b1100_0000;
    const LED_T1: u8 = 0b1111_1000;

    for bit in 0..8 {
        if byte & (0x80 >> bit) != 0 {
            data.push(LED_T1);
        } else {
            data.push(LED_T0);
        }
    }
}
