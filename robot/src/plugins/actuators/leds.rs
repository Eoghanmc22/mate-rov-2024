use std::{iter::zip, sync::Arc, thread};

use anyhow::Context;
use bevy::prelude::*;
use common::error::{self, Errors};
use crossbeam::channel::{self, Sender};
use rgb::RGB8;
use rppal::gpio::{Bias, Gpio, Mode};
use tracing::{span, Level};

use crate::peripheral::neopixel::{Neopixel, NeopixelBuffer};

pub struct LedPlugin;

impl Plugin for LedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_leds.pipe(error::handle_errors));
        app.add_systems(Update, update_leds.run_if(resource_exists::<Leds>()));
        app.add_systems(PostUpdate, write_state.run_if(resource_exists::<Leds>()));
    }
}

#[derive(Resource)]
struct Leds(Sender<LedUpdate>, Arc<NeopixelBuffer>, [LedState; 3]);

enum LedUpdate {
    Neopixel(Arc<NeopixelBuffer>),
    LedStates([LedState; 3]),
}

#[derive(Default, Clone, Copy)]
enum LedState {
    Full,
    Dim,
    #[default]
    Off,
}

fn start_leds(mut cmds: Commands, errors: Res<Errors>) -> anyhow::Result<()> {
    let (tx_data, rx_data) = channel::bounded(30);

    let gpio = Gpio::new().context("Open GPIO")?;

    let mut neopixel = Neopixel::new(
        80,
        Neopixel::SPI_BUS,
        Neopixel::SPI_SELECT,
        Neopixel::SPI_CLOCK,
    )
    .context("Open Neopixel")?;

    // Green
    let led_1 = gpio.get(24).context("Open led 1")?.into_io(Mode::Output);
    // Blue
    let led_2 = gpio.get(25).context("Open led 2")?.into_io(Mode::Output);
    // Red
    let led_3 = gpio.get(11).context("Open led 3")?.into_io(Mode::Output);
    let mut leds = [led_1, led_2, led_3];

    cmds.insert_resource(Leds(
        tx_data,
        neopixel.buffer.clone().into(),
        [LedState::default(); 3],
    ));

    let errors = errors.0.clone();
    thread::spawn(move || {
        let _span = span!(Level::INFO, "LED Thread").entered();

        for event in rx_data {
            match event {
                LedUpdate::Neopixel(buffer) => {
                    let res = neopixel
                        .spi
                        .write(buffer.to_slice())
                        .context("Write neopixels");
                    if let Err(err) = res {
                        let _ = errors.send(err);
                    }
                }
                LedUpdate::LedStates(states) => {
                    for (led, state) in zip(&mut leds, states) {
                        match state {
                            LedState::Full => {
                                led.set_mode(Mode::Output);
                                led.set_low();
                            }
                            LedState::Dim => {
                                led.set_mode(Mode::Input);
                                led.set_bias(Bias::PullDown);
                            }
                            LedState::Off => {
                                led.set_mode(Mode::Output);
                                led.set_high();
                            }
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

fn update_leds(mut leds: ResMut<Leds>) {
    let neopixel = Arc::make_mut(&mut leds.1);
    // neopixel.fill(.., RGB8::new(255, 127, 0), false);
    leds.2 = [LedState::Dim; 3];
}

fn write_state(leds: Res<Leds>) {
    let _ = leds.0.send(LedUpdate::Neopixel(leds.1.clone()));
    let _ = leds.0.send(LedUpdate::LedStates(leds.2.clone()));
}
