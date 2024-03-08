use std::{
    thread,
    time::{Duration, Instant},
};

use anyhow::Context;
use bevy::{app::AppExit, prelude::*};
use common::{
    components::{CurrentDraw, MeasuredVoltage},
    error::{self, Errors},
};
use crossbeam::channel::{self, Receiver, Sender};
use tracing::{span, Level};

use crate::{
    peripheral::ads1115::{Ads1115, AnalogChannel},
    plugins::core::robot::LocalRobot,
};

pub struct PowerPlugin;

impl Plugin for PowerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_power_thread.pipe(error::handle_errors));
        app.add_systems(
            PreUpdate,
            read_new_data.run_if(resource_exists::<PowerChannels>),
        );
        app.add_systems(Last, shutdown.run_if(resource_exists::<PowerChannels>));
    }
}

#[derive(Resource)]
struct PowerChannels(Receiver<PowerEvent>, Sender<()>);

enum PowerEvent {
    Voltage(f32),
    Amperage(f32),
}

fn start_power_thread(mut cmds: Commands, errors: Res<Errors>) -> anyhow::Result<()> {
    let (tx_data, rx_data) = channel::bounded(5);
    let (tx_exit, rx_exit) = channel::bounded(1);

    let mut adc = Ads1115::new(Ads1115::I2C_BUS, Ads1115::I2C_ADDRESS)
        .context("Analog to Digital converter (Ads1115)")?;

    cmds.insert_resource(PowerChannels(rx_data, tx_exit));

    let errors = errors.0.clone();
    thread::Builder::new()
        .name("Power Thread".to_owned())
        .spawn(move || {
            let _span = span!(Level::INFO, "Power sense thread").entered();

            let interval = Duration::from_secs_f64(1.0 / 100.0);
            let mut deadline = Instant::now();

            loop {
                let span = span!(Level::INFO, "Power sense cycle").entered();

                // Voltage
                let rst = adc.request_conversion(AnalogChannel::Ch3);
                if let Err(err) = rst {
                    let _ = errors.send(err);
                }
                thread::sleep(Duration::from_secs_f64(1.0 / 860.0));
                while !matches!(adc.ready(), Ok(true)) {
                    warn!("ADC not ready");
                }
                let rst = adc.read();

                match rst {
                    Ok(value) => {
                        let value = 11.0 * value;
                        let res = tx_data.send(PowerEvent::Voltage(value));

                        if res.is_err() {
                            // Peer disconected
                            return;
                        }
                    }
                    Err(err) => {
                        let _ = errors.send(err);
                    }
                }

                // Current
                let rst = adc.request_conversion(AnalogChannel::Ch2);
                if let Err(err) = rst {
                    let _ = errors.send(err);
                }
                thread::sleep(Duration::from_secs_f64(1.0 / 860.0));
                while !matches!(adc.ready(), Ok(true)) {
                    warn!("ADC not ready");
                }
                let rst = adc.read();

                match rst {
                    Ok(value) => {
                        let value = 37.8788 * (value - 0.33);
                        let res = tx_data.send(PowerEvent::Amperage(value));

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

fn read_new_data(mut cmds: Commands, channels: Res<PowerChannels>, robot: Res<LocalRobot>) {
    for event in channels.0.try_iter() {
        match event {
            PowerEvent::Voltage(voltage) => {
                cmds.entity(robot.entity)
                    .insert(MeasuredVoltage(voltage.into()));
            }
            PowerEvent::Amperage(amperage) => {
                cmds.entity(robot.entity)
                    .insert(CurrentDraw(amperage.into()));
            }
        }
    }
}

fn shutdown(channels: Res<PowerChannels>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        let _ = channels.1.send(());
    }
}
