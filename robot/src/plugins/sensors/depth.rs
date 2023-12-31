use std::{
    thread,
    time::{Duration, Instant},
};

use anyhow::Context;
use bevy::{app::AppExit, prelude::*};
use common::{
    components::Depth,
    error::{self, Errors},
    types::hw::DepthFrame,
};
use crossbeam::channel::{self, Receiver, Sender};
use tracing::{span, Level};

use crate::{peripheral::ms5937::Ms5837, plugins::core::robot::LocalRobot};

pub struct DepthPlugin;

impl Plugin for DepthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_depth_thread.pipe(error::handle_errors));
        app.add_systems(
            PreUpdate,
            read_new_data.run_if(resource_exists::<DepthChannels>()),
        );
        app.add_systems(Last, shutdown.run_if(resource_exists::<DepthChannels>()));
    }
}

#[derive(Resource)]
struct DepthChannels(Receiver<DepthFrame>, Sender<()>);

fn start_depth_thread(mut cmds: Commands, errors: Res<Errors>) -> anyhow::Result<()> {
    let (tx_data, rx_data) = channel::bounded(5);
    let (tx_exit, rx_exit) = channel::bounded(1);

    let mut depth =
        Ms5837::new(Ms5837::I2C_BUS, Ms5837::I2C_ADDRESS).context("Depth sensor (Ms5837)")?;

    cmds.insert_resource(DepthChannels(rx_data, tx_exit));

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

                if let Ok(()) = rx_exit.try_recv() {
                    return;
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

fn shutdown(channels: Res<DepthChannels>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        let _ = channels.1.send(());
    }
}
