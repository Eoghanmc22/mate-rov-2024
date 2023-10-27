use std::{
    thread,
    time::{Duration, Instant},
};

use ahrs::{Ahrs, Madgwick};
use bevy::{app::AppExit, prelude::*};
use common::{
    components::{Orientation, RawInertial, RawMagnetic, RobotMarker},
    types::sensors::{InertialFrame, MagneticFrame},
};
use crossbeam::channel::{self, Receiver, Sender};
use nalgebra::Vector3;
use tracing::{span, Level};

use crate::peripheral::{icm20602::Icm20602, mmc5983::Mcc5983};

pub struct OrientationPlugin;

impl Plugin for OrientationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_inertial_thread);
        app.add_systems(Update, read_new_data);
        app.insert_resource(MadgwickFilter(Madgwick::new(1.0 / 1000.0, 0.041)));
    }
}

#[derive(Resource)]
struct InertialChannels(
    Receiver<([InertialFrame; 10], [MagneticFrame; 1])>,
    Sender<()>,
);

#[derive(Resource)]
struct MadgwickFilter(Madgwick<f64>);

pub fn start_inertial_thread(mut cmds: Commands) {
    let (tx_data, rx_data) = channel::bounded(5);
    let (tx_exit, rx_exit) = channel::bounded(1);

    thread::spawn(move || {
        span!(Level::INFO, "Inertial sensor monitor thread").enter();

        let imu = Icm20602::new(Icm20602::SPI_BUS, Icm20602::SPI_SELECT, Icm20602::SPI_CLOCK);
        let mut imu = match imu {
            Ok(imu) => imu,
            Err(err) => {
                // TODO: error handeling
                // events.send(Event::Error(err.context("ICM20602")));
                return;
            }
        };

        let mag = Mcc5983::new(Mcc5983::SPI_BUS, Mcc5983::SPI_SELECT, Mcc5983::SPI_CLOCK);
        let mut mag = match mag {
            Ok(mag) => mag,
            Err(err) => {
                // TODO: error handeling
                // events.send(Event::Error(err.context("MCC5983")));
                return;
            }
        };

        let interval = Duration::from_secs_f64(1.0 / 1000.0);
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
                tx_data.send((inertial_buffer, mag_buffer));
            }

            if counter % inertial_divisor == 0 {
                let rst = imu.read_frame();

                match rst {
                    Ok(frame) => {
                        inertial_buffer[counter / inertial_divisor] = frame;
                    }
                    Err(err) => {
                        // TODO: error handeling
                        // events.send(Event::Error(err.context("Could not read imu")));
                    }
                }
            }

            if counter % mag_divisor == 0 {
                let rst = mag.read_frame();

                match rst {
                    Ok(frame) => {
                        mag_buffer[counter / mag_divisor] = frame;
                    }
                    Err(err) => {
                        // TODO: error handeling
                        // events.send(Event::Error(err.context("Could not read mag")));
                    }
                }
            }

            if let Ok(()) = rx_exit.try_recv() {
                return;
            }

            deadline += interval;
            let remaining = deadline - Instant::now();
            thread::sleep(remaining);

            counter += 1;
            counter %= counts;
            first_run = false;
        }
    });

    cmds.insert_resource(InertialChannels(rx_data, tx_exit));
}

pub fn read_new_data(
    mut cmds: Commands,
    channels: Res<InertialChannels>,
    mut madgwick_filter: ResMut<MadgwickFilter>,
    robot: Query<Entity, With<RobotMarker>>,
) {
    for (inertial, magnetic) in channels.0.try_iter() {
        // We currently ignore mag updates as the compass is not calibrated
        // TODO: Calibrate the compass
        for inertial in inertial {
            let gyro = Vector3::new(inertial.gyro_x.0, inertial.gyro_y.0, inertial.gyro_z.0)
                * (std::f64::consts::PI / 180.0);
            let accel = Vector3::new(inertial.accel_x.0, inertial.accel_y.0, inertial.accel_z.0);

            let rst = madgwick_filter.0.update_imu(&gyro, &accel);
            if let Err(_) = rst {
                // TODO: error handeling
            }
        }

        let quat: nalgebra::UnitQuaternion<f64> = madgwick_filter.0.quat;
        let quat: mint::Quaternion<f32> = quat.cast().into();
        let quat: glam::Quat = quat.into();
        let orientation = Orientation(quat);

        let inertial = inertial.last().unwrap();
        let inertial = RawInertial(*inertial);

        let magnetic = magnetic.last().unwrap();
        let magnetic = RawMagnetic(*magnetic);

        let robot = robot.single();
        cmds.entity(robot).insert((orientation, inertial, magnetic));
    }
}

pub fn shutdown(channels: Res<InertialChannels>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        let _ = channels.1.send(());
    }
}