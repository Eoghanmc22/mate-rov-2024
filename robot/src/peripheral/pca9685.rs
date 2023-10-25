use core::slice;
use std::{array, thread, time::Duration};

use anyhow::{bail, Context};
use rppal::{
    gpio::{Gpio, OutputPin},
    i2c::I2c,
};

// PWM_OE (GPIO66) is active low
// pwm chip is on i2c4 at address 0x40
// See https://bluerobotics.com/wp-content/uploads/2022/05/PCA9685-DATASHEET.pdf

pub struct Pca9685 {
    i2c: I2c,
    output_enable: OutputPin,
    period: Duration,
}

impl Pca9685 {
    pub const I2C_BUS: u8 = 4;
    pub const I2C_ADDRESS: u8 = 0x40;

    pub fn new(bus: u8, address: u8, period: Duration) -> anyhow::Result<Self> {
        let gpio = Gpio::new().context("Open gpio")?;
        let mut i2c = I2c::with_bus(bus).context("Open i2c")?;
        let output_enable = gpio
            .get(26)
            .context("Get PWM Output Enable pin")?
            .into_output_high();
        i2c.set_slave_address(address as u16)
            .context("Set addres for PCA9685")?;

        let mut this = Self {
            i2c,
            output_enable,
            period,
        };

        this.initialize().context("Init PCA9685")?;

        Ok(this)
    }

    pub fn output_enable(&mut self) {
        self.output_enable.set_low();
    }

    pub fn output_disable(&mut self) {
        self.output_enable.set_high();
    }

    pub fn set_pwm(&mut self, channel: u8, pwm: Duration) -> anyhow::Result<()> {
        let raw = pwm_to_raw(pwm, self.period);
        let upper = ((raw & 0x0f00) >> 8) as u8;
        let lower = ((raw & 0x00ff) >> 0) as u8;
        // let expected = [lower, upper];

        let register = channel_to_reg(channel);
        self.i2c
            .write(&[register, lower, upper])
            .context("Write pwm")?;

        // let mut observed = [0, 0];
        // self.i2c
        //     .write_read(&[register], &mut observed)
        //     .context("Validate pwm")?;
        // if observed != expected {
        //     bail!("Attempted to set pwm to {expected:?}. Instead, {observed:?} was read");
        // }

        Ok(())
    }

    pub fn set_pwms(&mut self, pwm: [Duration; 16]) -> anyhow::Result<()> {
        let raw: [u16; 16] = array::from_fn(|idx| pwm_to_raw(pwm[idx], self.period));

        let mut message: [u8; 65] = [0; 65];
        message[0] = Self::REG_LED0_ON_L;

        for idx in 0..16 {
            let upper = ((raw[idx] & 0x0f00) >> 8) as u8;
            let lower = ((raw[idx] & 0x00ff) >> 0) as u8;

            message[(idx << 2) + 3] = lower;
            message[(idx << 2) + 4] = upper;
        }

        self.i2c.write(&message).context("Write pwm")?;

        // let mut observed = [0; 64];
        // self.i2c
        //     .write_read(&[Self::REG_LED0_ON_L], &mut observed)
        //     .context("Validate pwm")?;
        // if observed != message[1..] {
        //     bail!("Attempted to set pwm to {message:?}. Instead, {observed:?} was read");
        // }

        Ok(())
    }
}

// Implementation based on https://github.com/bluerobotics/pca9685-python
impl Pca9685 {
    const REG_MODE1: u8 = 0x00;
    const REG_PRESCALE: u8 = 0xfe;
    const REG_LED0_ON_L: u8 = 0x06;
    const REG_LED0_OFF_L: u8 = 0x08;
    const REG_ALL_LED_OFF_H: u8 = 0xfd;

    const MODE1_SLEEP: u8 = 1 << 4;
    const MODE1_EXTCLK: u8 = 1 << 6;
    const MODE1_AI: u8 = 1 << 5;

    const EXT_CLOCK: f64 = 24.576e6;

    fn initialize(&mut self) -> anyhow::Result<()> {
        self.i2c
            .write(&[Self::REG_MODE1, Self::MODE1_SLEEP | Self::MODE1_AI])
            .context("Init PCA9685")?;
        self.set_prescale().context("Set prescale")?;

        Ok(())
    }

    fn set_prescale(&mut self) -> anyhow::Result<()> {
        let prescale = calc_prescale(self.period);
        if prescale < 3 {
            bail!("Prescale must be greater then 3, got: {prescale}");
        }

        self.i2c
            .write(&[
                Self::REG_MODE1,
                Self::MODE1_EXTCLK | Self::MODE1_SLEEP | Self::MODE1_AI,
            ])
            .context("Setup for prescale")?;

        self.i2c
            .write(&[Self::REG_PRESCALE, prescale])
            .context("Write prescale")?;

        self.i2c
            .write(&[Self::REG_MODE1, Self::MODE1_EXTCLK | Self::MODE1_AI])
            .context("Unsleep")?;

        let observed_prescale = self
            .read_reg(Self::REG_PRESCALE)
            .context("Verify prescale")?;
        if observed_prescale != prescale {
            bail!("Attempted to set prescale to {prescale}. Instead, {observed_prescale} was read");
        }

        Ok(())
    }

    fn read_reg(&self, reg: u8) -> anyhow::Result<u8> {
        let mut out = 0;
        self.i2c
            .write_read(&[reg], slice::from_mut(&mut out))
            .context("Read reg")?;
        Ok(out)
    }
}

impl Drop for Pca9685 {
    fn drop(&mut self) {
        let _ = self.set_pwms([Duration::ZERO; 16]);

        // Prevent cutting the last pulse short
        thread::sleep(Duration::from_millis(20));

        self.output_disable();
        thread::sleep(Duration::from_millis(5));

        let _ = self.i2c.write(&[Self::REG_ALL_LED_OFF_H, 0x08]);
        let _ = self.i2c.write(&[Self::REG_MODE1, Self::MODE1_SLEEP]);
    }
}

fn calc_prescale(period: Duration) -> u8 {
    let update_rate = 1.0 / period.as_secs_f64();
    ((Pca9685::EXT_CLOCK / (4096.0 * update_rate)).round() - 1.0) as u8
}

const fn pwm_to_raw(pwm: Duration, period: Duration) -> u16 {
    (pwm.as_micros() as u32 * 4096 / period.as_micros() as u32 - 1) as u16
}

const fn channel_to_reg(channel: u8) -> u8 {
    assert!(channel < 16);
    Pca9685::REG_LED0_OFF_L + (4 * channel)
}
