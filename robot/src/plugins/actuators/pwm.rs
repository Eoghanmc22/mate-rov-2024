use std::{
    mem, thread,
    time::{Duration, Instant},
};

use ahash::HashMap;
use anyhow::{anyhow, Context};
use bevy::{app::AppExit, prelude::*};
use common::{
    components::{Armed, PwmChannel, PwmSignal, RobotId, RobotMarker},
    ecs_sync::NetworkId,
    types::PwmChannelId,
};
use crossbeam::channel::{self, Sender};
use tracing::{span, Level};

use crate::{
    peripheral::pca9685::Pca9685,
    plugins::core::error::{self, Errors},
};

pub struct PwmOutputPlugin;

impl Plugin for PwmOutputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_pwm_thread.pipe(error::handle_errors));

        app.add_systems(
            Update,
            (listen_to_pwms.pipe(error::handle_errors), shutdown),
        );
    }
}

#[derive(Resource)]
struct PwmChannels(Sender<PwmEvent>);

enum PwmEvent {
    Arm(Armed),
    UpdateChannel(PwmChannelId, Duration),
    BatchComplete,
    Shutdown,
}

// TODO: Output should be disabled when disarmed
pub fn start_pwm_thread(mut cmds: Commands, errors: Res<Errors>) -> anyhow::Result<()> {
    let interval = Duration::from_secs_f32(1.0 / 100.0);
    let max_inactive = Duration::from_secs_f32(1.0 / 10.0);

    let (tx_data, rx_data) = channel::bounded(30);

    let mut pwm_controller =
        Pca9685::new(Pca9685::I2C_BUS, Pca9685::I2C_ADDRESS, interval).context("PCA9685")?;

    const STOP_PWMS: [Duration; 16] = [Duration::from_micros(1500); 16];
    pwm_controller
        .set_pwms(STOP_PWMS)
        .context("Set initial pwms")?;

    pwm_controller.output_disable();

    cmds.insert_resource(PwmChannels(tx_data));

    let errors = errors.0.clone();
    thread::spawn(move || {
        let span = span!(Level::INFO, "Pwm Output Thread");
        let _enter = span.enter();

        let mut deadline = Instant::now();

        let mut next_channel_pwms = HashMap::default();
        let mut batch_started = false;

        let mut armed = Armed::Disarmed;
        let mut channel_pwms = HashMap::default();
        let mut last_batch = Instant::now();

        let mut do_shutdown = false;

        while !do_shutdown {
            // Process events
            for event in rx_data.try_iter() {
                match event {
                    PwmEvent::Arm(Armed::Armed) => {
                        batch_started = true;
                        next_channel_pwms.clear();
                    }
                    PwmEvent::Arm(Armed::Disarmed) => {
                        batch_started = false;
                        armed = Armed::Disarmed;
                    }
                    PwmEvent::UpdateChannel(channel, pwm) => {
                        if batch_started {
                            next_channel_pwms.insert(channel, pwm);
                        }
                    }
                    PwmEvent::BatchComplete => {
                        if batch_started {
                            batch_started = false;

                            armed = Armed::Armed;
                            channel_pwms = mem::take(&mut next_channel_pwms);
                            last_batch = Instant::now();
                        }
                    }
                    PwmEvent::Shutdown => {
                        armed = Armed::Disarmed;
                        do_shutdown = true;

                        break;
                    }
                }
            }

            // Update state
            if last_batch.elapsed() > max_inactive {
                // TODO: Should this notify bevy?
                let _ = errors.send(anyhow!("Motors disarmed due to inactivity"));
                armed = Armed::Disarmed;
            }

            // Sync state with pwm chip
            match armed {
                Armed::Armed => {
                    pwm_controller.output_enable();
                }
                Armed::Disarmed => {
                    pwm_controller.output_disable();

                    // No motors should be active when disarmed
                    channel_pwms.clear();
                }
            }

            // Generate the pwm states for each channel
            let pwms = {
                // By default all motors should be stopped
                let mut pwms = STOP_PWMS;

                // Copy pwm values from `channel_pwms` into `pwms`
                // `channel_pwms` is cleared in the disarmed case
                for (channel, new_pwm) in &channel_pwms {
                    let channel_pwm = pwms.get_mut(*channel as usize);

                    // If this is a valid channel, set the corresponding channel's pwm
                    if let Some(channel_pwm) = channel_pwm {
                        *channel_pwm = *new_pwm;
                    }
                }

                pwms
            };

            // Write the current pwms to the pwm chip
            let rst = pwm_controller
                .set_pwms(pwms)
                .context("Could not communicate with PCA9685");

            if let Err(err) = rst {
                let _ = errors.send(err);
            }

            deadline += interval;
            let remaining = deadline - Instant::now();
            thread::sleep(remaining);
        }
    });

    Ok(())
}

// TODO: Handle errors
pub fn listen_to_pwms(
    channels: Res<PwmChannels>,
    robot: Query<(&NetworkId, &Armed), With<RobotMarker>>,
    pwms: Query<(&RobotId, &PwmChannel, &PwmSignal)>,
) -> anyhow::Result<()> {
    let (net_id, armed) = robot.single();

    channels
        .0
        .send(PwmEvent::Arm(*armed))
        .context("Send data to pwm thread")?;

    for (RobotId(robot_net_id), pwm_channel, pwm) in &pwms {
        if robot_net_id == net_id {
            channels
                .0
                .send(PwmEvent::UpdateChannel(pwm_channel.0, pwm.0))
                .context("Send data to pwm thread")?;
        }
    }

    channels
        .0
        .send(PwmEvent::BatchComplete)
        .context("Send data to pwm thread")?;

    Ok(())
}

pub fn shutdown(channels: Res<PwmChannels>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        let _ = channels.0.send(PwmEvent::Shutdown);
    }
}
