use std::{
    thread,
    time::{Duration, Instant},
};

use ahrs::{Ahrs, Madgwick};
use anyhow::{anyhow, Context};
use bevy::{app::AppExit, prelude::*};
use common::{
    components::{Inertial, Magnetic, Orientation},
    error::{self, ErrorEvent, Errors},
    types::hw::{InertialFrame, MagneticFrame},
};
use crossbeam::channel::{self, Receiver, Sender};
use nalgebra::Vector3;
use tracing::{span, Level};
use tracy_client::frame_name;

use crate::{
    peripheral::{icm20602::Icm20602, mmc5983::Mcc5983},
    plugins::core::robot::LocalRobot,
    tracy,
};

pub struct OrientationPlugin;

impl Plugin for OrientationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MadgwickFilter(Madgwick::new(1.0 / 1000.0, 0.041)));

        app.add_systems(Startup, start_inertial_thread.pipe(error::handle_errors));
        app.add_systems(
            PreUpdate,
            read_new_data.run_if(resource_exists::<InertialChannels>()),
        );
        app.add_systems(Last, shutdown.run_if(resource_exists::<InertialChannels>()));
    }
}

#[derive(Resource)]
struct InertialChannels(
    Receiver<([InertialFrame; 10], [MagneticFrame; 1])>,
    Sender<()>,
);

#[derive(Resource)]
struct MadgwickFilter(Madgwick<f32>);

fn start_inertial_thread(mut cmds: Commands, errors: Res<Errors>) -> anyhow::Result<()> {
    let (tx_data, rx_data) = channel::bounded(5);
    let (tx_exit, rx_exit) = channel::bounded(1);

    let mut imu = Icm20602::new(Icm20602::SPI_BUS, Icm20602::SPI_SELECT, Icm20602::SPI_CLOCK)
        .context("Inerital Sensor (ICM20602)")?;
    let mut mag = Mcc5983::new(Mcc5983::SPI_BUS, Mcc5983::SPI_SELECT, Mcc5983::SPI_CLOCK)
        .context("Magnmetic Sensor (MCC5983)")?;

    cmds.insert_resource(InertialChannels(rx_data, tx_exit));

    let errors = errors.0.clone();
    thread::Builder::new()
        .name("IMU Thread".to_owned())
        .spawn(move || {
            let span = span!(Level::INFO, "IMU thread");
            let _enter = span.enter();

            let interval = Duration::from_secs_f32(1.0 / 1000.0);
            let counts = 10;

            let mut counter = 0;

            let mut inertial_buffer = [InertialFrame::default(); 10];
            let mut mag_buffer = [MagneticFrame::default(); 1];

            let inertial_divisor = counts / inertial_buffer.len();
            let mag_divisor = counts / mag_buffer.len();

            let mut deadline = Instant::now();

            let mut first_run = true;

            loop {
                if counter == 0 && !first_run {
                    let res = tx_data.send((inertial_buffer, mag_buffer));
                    if res.is_err() {
                        // Peer disconnected
                        return;
                    }
                }

                if counter % inertial_divisor == 0 {
                    let rst = imu.read_frame().context("Read inertial frame");

                    match rst {
                        Ok(frame) => {
                            inertial_buffer[counter / inertial_divisor] = frame;
                        }
                        Err(err) => {
                            let _ = errors.send(err);
                        }
                    }
                }

                if counter % mag_divisor == 0 {
                    let rst = mag.read_frame().context("Read magnetic frame");

                    match rst {
                        Ok(frame) => {
                            mag_buffer[counter / mag_divisor] = frame;
                        }
                        Err(err) => {
                            let _ = errors.send(err);
                        }
                    }
                }

                if let Ok(()) = rx_exit.try_recv() {
                    return;
                }

                tracy::secondary_frame_mark(frame_name!("IMU"));

                deadline += interval;
                let remaining = deadline - Instant::now();
                thread::sleep(remaining);

                counter += 1;
                counter %= counts;
                first_run = false;
            }
        })
        .context("Spawn thread")?;

    Ok(())
}

fn read_new_data(
    mut cmds: Commands,
    channels: Res<InertialChannels>,
    mut madgwick_filter: ResMut<MadgwickFilter>,
    robot: Res<LocalRobot>,
    mut errors: EventWriter<ErrorEvent>,
) {
    for (inertial, magnetic) in channels.0.try_iter() {
        // We currently ignore mag updates as the compass is not calibrated
        // TODO(high): Calibrate the compass
        for inertial in inertial {
            let gyro = Vector3::new(inertial.gyro_x.0, inertial.gyro_y.0, inertial.gyro_z.0)
                * (std::f32::consts::PI / 180.0);
            let accel = Vector3::new(inertial.accel_x.0, inertial.accel_y.0, inertial.accel_z.0);

            let rst = madgwick_filter.0.update_imu(&gyro, &accel);
            if let Err(msg) = rst {
                errors.send(anyhow!("Process IMU frame: {msg}").into());
            }
        }

        let quat: glam::Quat = madgwick_filter.0.quat.into();
        let orientation = Orientation(quat);

        let inertial = inertial.last().unwrap();
        let inertial = Inertial(*inertial);

        let magnetic = magnetic.last().unwrap();
        let magnetic = Magnetic(*magnetic);

        cmds.entity(robot.entity)
            .insert((orientation, inertial, magnetic));
    }
}

fn shutdown(channels: Res<InertialChannels>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        let _ = channels.1.send(());
    }
}
