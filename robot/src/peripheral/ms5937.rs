use std::{thread, time::Duration};

use anyhow::{bail, Context};
use common::types::{
    sensors::DepthFrame,
    units::{Celsius, Mbar, Meters},
};
use rppal::i2c::I2c;

pub struct Ms5837 {
    i2c: I2c,
    calibration: [u16; 8],
    fluid_density: f32,
}

impl Ms5837 {
    pub const I2C_BUS: u8 = 6;
    pub const I2C_ADDRESS: u8 = 0x76;

    pub fn new(bus: u8, address: u8) -> anyhow::Result<Self> {
        let mut i2c = I2c::with_bus(bus).context("Open i2c")?;

        i2c.set_slave_address(address as u16)
            .context("Set addres for MS5837")?;

        let mut this = Self {
            i2c,
            calibration: [0; 8],
            // TODO tweak
            fluid_density: 1000.0,
        };

        this.initialize().context("Init MS5837")?;

        Ok(this)
    }

    pub fn read_frame(&mut self) -> anyhow::Result<DepthFrame> {
        let raw = self.read_raw().context("Read raw frame")?;

        let (pressure, temperature) = calculate_pressure_and_temperature(raw, &self.calibration);
        let altitude = pressure_to_altitude(pressure);
        let depth = pressure_to_depth(pressure, self.fluid_density);

        Ok(DepthFrame {
            depth,
            altitude,
            pressure,
            temperature,
        })
    }

    pub fn set_fluid_density(&mut self, density: f32) {
        self.fluid_density = density;
    }
}

impl Ms5837 {
    const CMD_RESET: u8 = 0x1e;
    const CMD_READ_PROM: u8 = 0xA0;
    const CMD_CONVERT_D1_OSR1024: u8 = 0x44;
    const CMD_CONVERT_D2_OSR1024: u8 = 0x54;
    const CMD_READ_ADC: u8 = 0x00;

    fn initialize(&mut self) -> anyhow::Result<()> {
        self.i2c.write(&[Self::CMD_RESET]).context("Reset MS5837")?;
        thread::sleep(Duration::from_millis(10));

        for prom_addrs in 0..7 {
            let mut buffer = [0, 0];
            self.i2c
                .write(&[Self::CMD_READ_PROM | (prom_addrs as u8) << 1])
                .context("Read prom cmd")?;
            self.i2c.read(&mut buffer).context("Read prom")?;

            let val = (buffer[0] as u16) << 8 | buffer[1] as u16;
            self.calibration[prom_addrs] = val;
        }

        let crc = (self.calibration[0] >> 12) as u8;
        if crc != crc4(self.calibration) {
            bail!("Got bad crc");
        }

        // let version = (self.calibration[0] & 0x0FE0) >> 5;
        // if version != 0x1A {
        //     bail!("Got bad version, {version}, {}", self.calibration[0]);
        // }

        Ok(())
    }

    fn read_raw(&mut self) -> anyhow::Result<(u32, u32)> {
        let mut buffer = [0, 0, 0];

        self.i2c
            .write(&[Self::CMD_CONVERT_D1_OSR1024])
            .context("Begin d1 convert")?;
        thread::sleep(Duration::from_millis(3));

        self.i2c
            .write(&[Self::CMD_READ_ADC])
            .context("Begin d1 read")?;
        self.i2c.read(&mut buffer).context("D1 read")?;

        let d1 = (buffer[0] as u32) << 16 | (buffer[1] as u32) << 8 | buffer[0] as u32;

        self.i2c
            .write(&[Self::CMD_CONVERT_D2_OSR1024])
            .context("Begin d2 convert")?;
        thread::sleep(Duration::from_millis(3));

        self.i2c
            .write(&[Self::CMD_READ_ADC])
            .context("Begin d2 read")?;
        self.i2c.read(&mut buffer).context("D2 read")?;

        let d2 = (buffer[0] as u32) << 16 | (buffer[1] as u32) << 8 | buffer[0] as u32;

        Ok((d1, d2))
    }
}

// Hippity hoppity the code in the data sheet is my property
fn calculate_pressure_and_temperature(raw: (u32, u32), calibration: &[u16; 8]) -> (Mbar, Celsius) {
    // Calculate temperature
    let dt = raw.1 as i32 - calibration[5] as i32 * 256;
    let temp = 2000 + dt * calibration[6] as i32 / 8388608;

    // Calculate actual offset and sensitivity
    let off = calibration[2] as i64 * 65536 + (calibration[4] as i64 * dt as i64) / 128;
    let sens = calibration[1] as i64 * 32768 + (calibration[3] as i64 * dt as i64) / 256;

    let t_i;
    let mut off_i;
    let mut sens_i;

    // Second order compensation
    if temp / 100 < 20 {
        // Low temp
        t_i = 3 * dt as i64 * dt as i64 / 8589934592;
        off_i = 3 * (temp as i64 - 2000) * (temp as i64 - 2000) / 2;
        sens_i = 5 * (temp as i64 - 2000) * (temp as i64 - 2000) / 8;

        if temp / 100 < -15 {
            // Very low temp
            off_i += 7 * (temp as i64 + 1500) * (temp as i64 + 1500);
            sens_i += 4 * (temp as i64 + 1500) * (temp as i64 + 1500);
        }
    } else {
        // High temp
        t_i = 2 * dt as i64 * dt as i64 / 137438953472;
        off_i = (temp as i64 - 2000) * (temp as i64 - 2000) / 16;
        sens_i = 0;
    }

    // Calculate corrected offset and sensitivity
    let off = off - off_i;
    let sens = sens - sens_i;

    // Calculate pressure and temperature
    let pressure_raw = ((raw.0 as i64 * sens / 2097152) - off) / 8192;
    let temperature_raw = temp - t_i as i32;

    // Wrap in newtypes
    let pressure = Mbar(pressure_raw as f32 / 10.0);
    let temperature = Celsius(temperature_raw as f32 / 100.0);

    (pressure, temperature)
}

fn pressure_to_depth(pressure: Mbar, density: f32) -> Meters {
    Meters((pressure.0 * 100.0 - 101300.0) / (density * 9.80665))
}

fn pressure_to_altitude(pressure: Mbar) -> Meters {
    Meters((1.0 - f32::powf(pressure.0 / 1013.25, 0.190284)) * 145366.45 * 0.3048)
}

fn crc4(mut data: [u16; 8]) -> u8 {
    let mut n_rem = 0u16;

    data[0] &= 0x0FFF;
    data[7] = 0;

    for i in 0..16 {
        if i % 2 == 1 {
            n_rem ^= data[i >> 1] & 0x00FF;
        } else {
            n_rem ^= data[i >> 1] >> 8;
        }

        for _ in 0..8 {
            if n_rem & 0x8000 != 0 {
                n_rem = (n_rem << 1) ^ 0x3000;
            } else {
                n_rem <<= 1;
            }
        }
    }

    (n_rem >> 12) as u8
}
