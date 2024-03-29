use std::{
    thread,
    time::{Duration, Instant},
};

use anyhow::Context;
use bevy::{app::AppExit, prelude::*};
use common::{
    components::{Depth, DepthSettings},
    error::{self, Errors},
    events::CalibrateSeaLevel,
    types::hw::DepthFrame,
};
use crossbeam::channel::{self, Receiver, Sender};
use tracing::{span, Level};

use crate::{
    peripheral::ms5937::Ms5837,
    plugins::core::robot::{LocalRobot, LocalRobotMarker},
};

pub struct DepthPlugin;

impl Plugin for DepthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_depth_thread.pipe(error::handle_errors));
        app.add_systems(
            PreUpdate,
            read_new_data.run_if(resource_exists::<DepthChannels>),
        );
        app.add_systems(
            Update,
            (
                calibrate_sea_level,
                listen_for_settings
                    .pipe(error::handle_errors)
                    .after(calibrate_sea_level),
            ),
        );
        app.add_systems(Last, shutdown.run_if(resource_exists::<DepthChannels>));
    }
}

#[derive(Resource)]
struct DepthChannels(Receiver<DepthFrame>, Sender<Message>);

enum Message {
    Settings(DepthSettings),
    Shutdown,
}

fn start_depth_thread(
    mut cmds: Commands,
    robot: Res<LocalRobot>,
    errors: Res<Errors>,
) -> anyhow::Result<()> {
    let (tx_data, rx_data) = channel::bounded(5);
    let (tx_exit, rx_msg) = channel::bounded(1);

    let mut depth =
        Ms5837::new(Ms5837::I2C_BUS, Ms5837::I2C_ADDRESS).context("Depth sensor (Ms5837)")?;

    cmds.insert_resource(DepthChannels(rx_data, tx_exit));

    let sea_level = depth.read_frame().context("Read Sea Level")?;
    depth.sea_level = sea_level.pressure;

    cmds.entity(robot.entity).insert(DepthSettings {
        sea_level: depth.sea_level,
        fluid_density: depth.fluid_density,
    });

    let errors = errors.0.clone();
    thread::Builder::new()
        .name("Depth Thread".to_owned())
        .spawn(move || {
            let _span = span!(Level::INFO, "Depth sensor thread").entered();

            let interval = Duration::from_secs_f64(1.0 / 100.0);
            let mut deadline = Instant::now();

            loop {
                let span = span!(Level::INFO, "Depth sensor cycle").entered();

                let rst = depth.read_frame().context("Read depth frame");

                match rst {
                    Ok(frame) => {
                        let res = tx_data.send(frame);

                        if res.is_err() {
                            // Peer disconected
                            return;
                        }
                    }
                    Err(err) => {
                        let _ = errors.send(err);
                    }
                }

                if let Ok(msg) = rx_msg.try_recv() {
                    match msg {
                        Message::Settings(settings) => {
                            depth.fluid_density = settings.fluid_density;
                            depth.sea_level = settings.sea_level;
                        }
                        Message::Shutdown => return,
                    }
                }

                span.exit();

                deadline += interval;
                let remaining = deadline - Instant::now();
                thread::sleep(remaining);
            }
        })
        .context("Start thread")?;

    Ok(())
}

fn read_new_data(mut cmds: Commands, channels: Res<DepthChannels>, robot: Res<LocalRobot>) {
    for depth in channels.0.try_iter() {
        let depth = Depth(depth);

        cmds.entity(robot.entity).insert(depth);
    }
}

fn calibrate_sea_level(
    mut cmds: Commands,
    mut events: EventReader<CalibrateSeaLevel>,
    mut robot: Query<(&Depth, &mut DepthSettings), With<LocalRobotMarker>>,
) {
    for _ in events.read() {
        info!("Calibrating Sea Level");

        for (depth, mut settings) in &mut robot {
            settings.sea_level = depth.0.pressure;
        }
    }
}

fn listen_for_settings(
    channels: Res<DepthChannels>,
    robot: Query<&DepthSettings, (With<LocalRobotMarker>, Changed<DepthSettings>)>,
) -> anyhow::Result<()> {
    for settings in &robot {
        channels
            .1
            .send(Message::Settings(*settings))
            .context("Send new settings to Depth Thread")?;
    }

    Ok(())
}

fn shutdown(channels: Res<DepthChannels>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        let _ = channels.1.send(Message::Shutdown);
    }
}
