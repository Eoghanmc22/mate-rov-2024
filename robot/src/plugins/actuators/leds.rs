use std::{
    f32::consts::TAU,
    iter::{self, zip},
    sync::Arc,
    thread,
};

use anyhow::Context;
use bevy::{app::AppExit, prelude::*, utils::HashMap};
use common::{
    components::{PwmChannel, PwmSignal, RobotId, RobotStatus},
    error::{self, ErrorEvent, Errors},
};
use crossbeam::channel::{self, Sender};
use rgb::RGB8;
use rppal::gpio::{Bias, Gpio, IoPin, Mode};
use tracing::{span, Level};

use crate::{
    peripheral::neopixel::{Neopixel, NeopixelBuffer},
    plugins::core::robot::LocalRobotMarker,
};

pub struct LedPlugin;

impl Plugin for LedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_leds.pipe(error::handle_errors))
            .add_systems(Update, update_leds.run_if(resource_exists::<LedChannels>()))
            .add_systems(
                PostUpdate,
                write_state.run_if(resource_exists::<LedChannels>()),
            )
            .add_systems(Last, shutdown);
    }
}

#[derive(Resource)]
struct LedChannels(Sender<LedUpdate>, Arc<NeopixelBuffer>, [LedState; 3]);

struct Leds([IoPin; 3]);

impl Drop for Leds {
    fn drop(&mut self) {
        for led in &mut self.0 {
            led.set_mode(Mode::Input);
            led.set_bias(Bias::Off);
        }
    }
}

enum LedUpdate {
    Neopixel(Arc<NeopixelBuffer>),
    LedStates([LedState; 3]),
    Shutdown,
}

#[derive(Default, Clone, Copy)]
enum LedState {
    On,
    Dim,
    #[default]
    Off,
}

enum LedType {
    Status,
    Thruster(u8),
    Circle(u8),
    Side(u8),
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
    let mut leds = Leds([led_1, led_2, led_3]);

    cmds.insert_resource(LedChannels(
        tx_data,
        neopixel.buffer.clone().into(),
        [LedState::default(); 3],
    ));

    let errors = errors.0.clone();
    thread::Builder::new()
        .name("LED Thread".to_owned())
        .spawn(move || {
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
                        for (led, state) in zip(&mut leds.0, states) {
                            match state {
                                LedState::On => {
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
                    LedUpdate::Shutdown => return,
                }
            }
        })
        .context("Spawn thread")?;

    Ok(())
}

fn update_leds(
    mut leds: ResMut<LedChannels>,
    robot: Query<(&RobotStatus, &RobotId), With<LocalRobotMarker>>,
    thrusters: Query<(&PwmChannel, &PwmSignal, &RobotId)>,
    time: Res<Time>,
    mut errors: EventReader<ErrorEvent>,
) {
    let (status, id) = robot.single();
    let thrusters = thrusters
        .iter()
        .filter(|(_, _, robot)| **robot == *id)
        .map(|(&channel, &signal, _)| (channel, signal))
        .collect::<HashMap<_, _>>();

    let brightness = 0.5;

    let colors = neopixels().map(|led| {
        match led {
            // Choose color besed on ROV status
            LedType::Status => {
                // TODO(high): Figure out what we want to display here
                RGB8::default()
            }
            // Choose color based on thruster speed
            LedType::Thruster(id) => {
                let signal = thrusters.get(&PwmChannel(id));

                if let Some(signal) = signal {
                    let micros = signal.0.as_micros();

                    if micros >= 1500 {
                        // Forward
                        let green = (micros as u32 - 1500) * 255 / 400;
                        RGB8::new(0, green as u8, 0)
                    } else {
                        // Backward
                        let red = (1500 - micros as u32) * 255 / 400;
                        RGB8::new(red as u8, 0, 0)
                    }
                } else {
                    RGB8::new(0, 0, 127)
                }
            }
            // Rotate
            LedType::Circle(id) => {
                let red = (((time.elapsed_seconds() + 0.0 * TAU / 3.0 + TAU * (id as f32 / 11.0))
                    .sin()
                    / 2.0
                    + 0.5)
                    * 255.0
                    * brightness) as u8;
                let green =
                    (((time.elapsed_seconds() + 1.0 * TAU / 3.0 + TAU * (id as f32 / 11.0)).sin()
                        / 2.0
                        + 0.5)
                        * 255.0) as u8;
                let blue = (((time.elapsed_seconds() + 2.0 * TAU / 3.0 + TAU * (id as f32 / 11.0))
                    .sin()
                    / 2.0
                    + 0.5)
                    * 255.0
                    * brightness) as u8;

                RGB8::new(red, green, blue)
            }
            LedType::Side(id) => {
                let offset = 0.1;

                let red = (((time.elapsed_seconds() + 0.0 * TAU / 3.0 + (id as f32 * offset))
                    .sin()
                    / 2.0
                    + 0.5)
                    * 255.0
                    * brightness) as u8;
                let green = (((time.elapsed_seconds() + 1.0 * TAU / 3.0 + (id as f32 * offset))
                    .sin()
                    / 2.0
                    + 0.5)
                    * 255.0
                    * brightness) as u8;
                let blue = (((time.elapsed_seconds() + 2.0 * TAU / 3.0 + (id as f32 * offset))
                    .sin()
                    / 2.0
                    + 0.5)
                    * 255.0
                    * brightness) as u8;

                RGB8::new(red, green, blue)
            }
        }
    });

    let neopixel = Arc::make_mut(&mut leds.1);
    neopixel.set(.., colors, true);

    // Blue for connected
    // Green for armed
    // Red on error

    leds.2 = [LedState::Dim; 3];
    match status {
        RobotStatus::NoPeer => {}
        RobotStatus::Disarmed => {
            leds.2[1] = LedState::On;
        }
        RobotStatus::Ready | RobotStatus::Moving(_) => {
            leds.2[0] = LedState::On;
            leds.2[1] = LedState::On;
        }
    }

    if !errors.is_empty() {
        leds.2[2] = LedState::On;
        errors.clear();
    }
}

fn shutdown(channels: Res<LedChannels>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        let _ = channels.0.send(LedUpdate::Shutdown);
    }
}

fn write_state(leds: Res<LedChannels>) {
    let _ = leds.0.send(LedUpdate::Neopixel(leds.1.clone()));
    let _ = leds.0.send(LedUpdate::LedStates(leds.2));
}

fn neopixels() -> impl Iterator<Item = LedType> {
    iter::from_coroutine(|| {
        for board in 0..2 {
            yield LedType::Status;

            for led in 0..11 {
                yield LedType::Circle(led);
            }

            for led in 0..12 {
                yield LedType::Side(led);
            }

            for led in (0..4).rev() {
                yield LedType::Thruster(led + board * 4);
            }

            for led in (0..12).rev() {
                yield LedType::Side(led);
            }
        }
    })
}
