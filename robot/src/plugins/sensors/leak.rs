use anyhow::Context;
use bevy::{app::AppExit, prelude::*};
use common::{components::Leak, error};
use crossbeam::channel::Receiver;
use rppal::gpio::{Gpio, InputPin, Level, Trigger};

use crate::plugins::core::robot::LocalRobot;

pub struct LeakPlugin;

impl Plugin for LeakPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_leak_interupt.pipe(error::handle_errors));
        app.add_systems(
            PreUpdate,
            read_new_data.run_if(resource_exists::<LeakChannels>()),
        );
        app.add_systems(Last, shutdown.run_if(resource_exists::<LeakChannels>()));
    }
}

#[derive(Resource)]
struct LeakChannels(Receiver<bool>, InputPin);

const LEAK_PIN: u8 = 27;

fn setup_leak_interupt(mut cmds: Commands, robot: Res<LocalRobot>) -> anyhow::Result<()> {
    let (tx, rx) = crossbeam::channel::bounded(5);

    let gpio = Gpio::new().context("Open gpio")?;
    let mut leak_pin = gpio
        .get(LEAK_PIN)
        .context("Open leak pin")?
        .into_input_pulldown();

    let initial_leak = leak_pin.is_high();
    cmds.entity(robot.entity).insert(Leak(initial_leak));

    leak_pin
        .set_async_interrupt(Trigger::Both, move |level| {
            let level = match level {
                Level::High => true,
                Level::Low => false,
            };

            warn!(?level, "Leak interrupt triggered");

            tx.send(level).expect("Peer disconnected");
        })
        .context("Set async leak interrupt")?;

    cmds.insert_resource(LeakChannels(rx, leak_pin));

    Ok(())
}

fn read_new_data(mut cmds: Commands, channels: Res<LeakChannels>, robot: Res<LocalRobot>) {
    let mut leak = None;

    for event in channels.0.try_iter() {
        leak = Some(event);
    }

    if let Some(leak) = leak {
        cmds.entity(robot.entity).insert(Leak(leak));
    }
}

fn shutdown(mut channels: ResMut<LeakChannels>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        let _ = channels.1.clear_async_interrupt();
    }
}
