use rppal::i2c::I2c;
use tracing::instrument;

use anyhow::Context;

pub struct Ads1115 {
    i2c: I2c,
}

impl Ads1115 {
    pub const I2C_BUS: u8 = 1;
    pub const I2C_ADDRESS: u8 = 0x48;

    #[instrument(level = "debug")]
    pub fn new(bus: u8, address: u8) -> anyhow::Result<Self> {
        let mut i2c = I2c::with_bus(bus).context("Open i2c")?;

        i2c.set_slave_address(address as u16)
            .context("Set address for ADS1115")?;

        let mut this = Self { i2c };

        Ok(this)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalogChannel {
    Ch0,
    Ch1,
    Ch2,
    Ch3,
}

impl AnalogChannel {
    pub fn selector(&self) -> u16 {
        match self {
            AnalogChannel::Ch0 => 0b100,
            AnalogChannel::Ch1 => 0b101,
            AnalogChannel::Ch2 => 0b110,
            AnalogChannel::Ch3 => 0b111,
        }
    }
}

// Implementation based on https://github.com/bluerobotics/ads1115-python
impl Ads1115 {
    const POINTER_CONVERSION: u8 = 0x00;
    const POINTER_CONFIG: u8 = 0x01;

    #[instrument(level = "trace", skip(self), ret)]
    pub fn request_conversion(&mut self, channel: AnalogChannel) -> anyhow::Result<()> {
        let config = 1 << 15 | channel.selector() << 12 | 0b001 << 9 | 1 << 8 | 0b111 << 5;

        self.i2c
            .block_write(Self::POINTER_CONFIG, &config.to_be_bytes())
            .context("Begin ADC convert")?;

        Ok(())
    }

    #[instrument(level = "trace", skip(self), ret)]
    pub fn ready(&mut self) -> anyhow::Result<bool> {
        let mut buffer = [0u8; 2];

        self.i2c
            .block_read(Self::POINTER_CONFIG, &mut buffer)
            .context("Check ADC conversion status")?;

        let value = i16::from_be_bytes(buffer);

        Ok(value & 1 << 15 != 0)
    }

    #[instrument(level = "trace", skip(self), ret)]
    pub fn read(&mut self) -> anyhow::Result<f32> {
        let mut buffer = [0u8; 2];

        self.i2c
            .block_read(Self::POINTER_CONVERSION, &mut buffer)
            .context("Check ADC conversion status")?;

        let value = u16::from_be_bytes(buffer);

        Ok(value as f32 / 0xffff as f32 * 2.0 * 4.096)
    }
}
