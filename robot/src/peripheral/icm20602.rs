use common::types::{
    sensors::InertialFrame,
    units::{Celsius, Dps, GForce},
};
use std::{thread, time::Duration};
use tracing::{debug, instrument};

use anyhow::Context;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};

pub struct Icm20602 {
    spi: Spi,
}

impl Icm20602 {
    pub const SPI_BUS: Bus = Bus::Spi1;
    pub const SPI_SELECT: SlaveSelect = SlaveSelect::Ss2;
    pub const SPI_CLOCK: u32 = 10_000_000;

    #[instrument(level = "debug")]
    pub fn new(bus: Bus, slave_select: SlaveSelect, clock_speed: u32) -> anyhow::Result<Self> {
        let spi = Spi::new(bus, slave_select, clock_speed, Mode::Mode0).context("Open spi")?;

        let mut this = Self { spi };
        this.initialize().context("Initialize")?;

        Ok(this)
    }

    #[instrument(level = "trace", skip(self), ret)]
    pub fn read_frame(&mut self) -> anyhow::Result<InertialFrame> {
        let raw = self.read_raw_frame().context("Read raw frame")?;

        // The first byte is junk
        let raw = &raw[1..];

        let raw_accel_native_x = (raw[0] as u16) << 8 | raw[1] as u16;
        let raw_accel_native_y = (raw[2] as u16) << 8 | raw[3] as u16;
        let raw_accel_native_z = (raw[4] as u16) << 8 | raw[5] as u16;

        let raw_tempature = (raw[6] as u16) << 8 | raw[7] as u16;

        let raw_gyro_native_x = (raw[8] as u16) << 8 | raw[9] as u16;
        let raw_gyro_native_y = (raw[10] as u16) << 8 | raw[11] as u16;
        let raw_gyro_native_z = (raw[12] as u16) << 8 | raw[13] as u16;

        let accel_native_x = raw_accel_native_x as i16 as f32 / 16384.0;
        let accel_native_y = raw_accel_native_y as i16 as f32 / 16384.0;
        let accel_native_z = raw_accel_native_z as i16 as f32 / 16384.0;

        let tempature = raw_tempature as i16 as f32 / 326.8 + 25.0;

        let gyro_native_x = raw_gyro_native_x as i16 as f32 / 65.5;
        let gyro_native_y = raw_gyro_native_y as i16 as f32 / 65.5;
        let gyro_native_z = raw_gyro_native_z as i16 as f32 / 65.5;

        let accel_x = -accel_native_y;
        let accel_y = -accel_native_x;
        let accel_z = -accel_native_z;

        let gyro_x = -gyro_native_y;
        let gyro_y = -gyro_native_x;
        let gyro_z = -gyro_native_z;

        Ok(InertialFrame {
            gyro_x: Dps(gyro_x),
            gyro_y: Dps(gyro_y),
            gyro_z: Dps(gyro_z),
            accel_x: GForce(accel_x),
            accel_y: GForce(accel_y),
            accel_z: GForce(accel_z),
            tempature: Celsius(tempature),
        })
    }
}

// Implementation based on https://github.com/bluerobotics/icm20602-python
impl Icm20602 {
    const REG_I2C_IF: u8 = 0x70;
    const REG_CONFIG: u8 = 0x1A;
    const REG_GYRO_CONFIG: u8 = 0x1B;
    const REG_ACCEL_CONFIG: u8 = 0x1C;
    const REG_ACCEL_CONFIG_2: u8 = 0x1D;
    const REG_ACCEL_INTEL_CTRL: u8 = 0x69;
    const REG_PWR_MGMT_1: u8 = 0x6B;
    const REG_WHO_AM_I: u8 = 0x75;
    const REG_ACCEL_XOUT_H: u8 = 0x3B;

    const READ: u8 = 0x80;

    fn initialize(&mut self) -> anyhow::Result<()> {
        debug!("Initializing ICM20602 (gyro + accelerometer)");

        let mut id = [0, 0];
        self.spi
            .transfer(&mut id, &[Self::REG_WHO_AM_I | Self::READ, 0])
            .context("Request id")?;
        assert_eq!(id[1], 0x12);

        self.spi
            .write(&[Self::REG_I2C_IF, 0x40])
            .context("Disable i2c")?;

        // 1Hz sample rate
        self.spi
            .write(&[Self::REG_CONFIG, 0x1])
            .context("Setup lowpass filter")?;

        // 500 deg range, lowpass filter
        self.spi
            .write(&[Self::REG_GYRO_CONFIG, 0x0 | 0b01 << 3])
            .context("Setup gyro")?;

        // 2g range
        self.spi
            .write(&[Self::REG_ACCEL_CONFIG, 0x0])
            .context("Setup accel")?;

        // lowpass filter
        self.spi
            .write(&[Self::REG_ACCEL_CONFIG_2, 0x0])
            .context("Setup accel")?;

        // Disable output limit
        self.spi
            .write(&[Self::REG_ACCEL_INTEL_CTRL, 0x2])
            .context("Setup accel")?;

        // Exit sleep mode
        self.spi
            .write(&[Self::REG_PWR_MGMT_1, 0x1])
            .context("Exit sleep")?;

        // Delay to allow sensors to start up and stabilize
        thread::sleep(Duration::from_millis(100));

        debug!("Initializing ICM20602 complete");

        Ok(())
    }

    fn read_raw_frame(&mut self) -> anyhow::Result<[u8; 15]> {
        let mut output = [0; 15];
        let mut input = [0; 15];

        output[0] = Self::REG_ACCEL_XOUT_H | Self::READ;

        self.spi
            .transfer(&mut input, &output)
            .context("Begin read imu frame")?;

        Ok(input)
    }
}
