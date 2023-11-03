use anyhow::Context;
use bevy::{app::AppExit, prelude::*};
use common::components::{Leak, RobotMarker};
use crossbeam::channel::Receiver;
use rppal::gpio::{Gpio, InputPin, Level, Trigger};

use super::error;

pub struct LeakPlugin;

impl Plugin for LeakPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_leak_interupt.pipe(error::handle_errors));
        app.add_systems(
            Update,
            (read_new_data, shutdown).run_if(resource_exists::<LeakChannels>()),
        );
    }
}

#[derive(Resource)]
struct LeakChannels(Receiver<bool>, InputPin);

const LEAK_PIN: u8 = 27;

pub fn setup_leak_interupt(
    mut cmds: Commands,
    robot: Query<Entity, With<RobotMarker>>,
) -> anyhow::Result<()> {
    let (tx, rx) = crossbeam::channel::bounded(5);

    let gpio = Gpio::new().context("Open gpio")?;
    let mut leak_pin = gpio
        .get(LEAK_PIN)
        .context("Open leak pin")?
        .into_input_pulldown();

    let robot = robot.single();
    let initial_leak = leak_pin.is_high();

    cmds.entity(robot).insert(Leak(initial_leak));

    leak_pin
        .set_async_interrupt(Trigger::Both, move |level| {
            let level = match level {
                Level::High => true,
                Level::Low => false,
            };

            // TODO: Handle?
            let _ = tx.send(level);
        })
        .context("Set async leak interrupt")?;

    cmds.insert_resource(LeakChannels(rx, leak_pin));

    Ok(())
}

pub fn read_new_data(
    mut cmds: Commands,
    channels: Res<LeakChannels>,
    robot: Query<Entity, With<RobotMarker>>,
) {
    let mut leak = None;

    for event in channels.0.try_iter() {
        leak = Some(event);
    }

    if let Some(leak) = leak {
        let robot = robot.single();
        cmds.entity(robot).insert(Leak(leak));
    }
}

pub fn shutdown(mut channels: ResMut<LeakChannels>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        // TODO: Handle?
        let _ = channels.1.clear_async_interrupt();
    }
}
