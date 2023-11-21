use std::{thread, time::Duration};

use anyhow::anyhow;
use bevy::{app::AppExit, prelude::*};
use common::{
    components::{
        Cores, CpuTotal, Disks, LoadAverage, Memory, Networks, OperatingSystem, Processes,
        RobotMarker, Temperatures, Uptime,
    },
    types::{
        system::{ComponentTemperature, Cpu, Disk, Network, Process},
        units::Celsius,
    },
};
use crossbeam::channel::{self, Receiver, Sender};
use sysinfo::{
    ComponentExt, CpuExt, DiskExt, NetworkExt, NetworksExt, PidExt, ProcessExt, System, SystemExt,
    UserExt,
};
use tracing::{span, Level};

use crate::plugins::core::error::Errors;

pub struct HwStatPlugin;

impl Plugin for HwStatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_hw_stat_thread);
        app.add_systems(Update, (read_new_data, shutdown));
    }
}

#[derive(Resource)]
struct HwStatChannels(Receiver<SystemInfo>, Sender<()>);

#[derive(Bundle, Debug, Clone)]
pub struct SystemInfo {
    pub processes: Processes,
    pub load_average: LoadAverage,
    pub networks: Networks,
    pub cpu_total: CpuTotal,
    pub cpus: Cores,
    pub memory: Memory,
    pub components: Temperatures,
    pub disks: Disks,
    pub uptime: Uptime,
    pub os: OperatingSystem,
}

pub fn start_hw_stat_thread(mut cmds: Commands, errors: Res<Errors>) {
    let (tx_data, rx_data) = channel::bounded(10);
    let (tx_exit, rx_exit) = channel::bounded(1);

    cmds.insert_resource(HwStatChannels(rx_data, tx_exit));

    let errors = errors.0.clone();
    thread::spawn(move || {
        let span = span!(Level::INFO, "Hardware monitor");
        let _enter = span.enter();

        let mut system = System::new();
        loop {
            system.refresh_all();
            system.refresh_disks_list();
            system.refresh_disks();
            system.refresh_components_list();
            system.refresh_components();
            system.refresh_networks_list();
            system.refresh_networks();
            system.refresh_users_list();

            match collect_system_state(&system) {
                Ok(hw_state) => {
                    // TODO: Handle?
                    let _ = tx_data.send(hw_state);
                }
                Err(err) => {
                    let _ = errors.send(anyhow!(err).context("Could not collect system state"));
                }
            }

            if let Ok(()) = rx_exit.try_recv() {
                return;
            }

            thread::sleep(Duration::from_secs(1));
        }
    });
}

pub fn read_new_data(
    mut cmds: Commands,
    channels: Res<HwStatChannels>,
    robot: Query<Entity, With<RobotMarker>>,
) {
    for info in channels.0.try_iter() {
        let robot = robot.single();
        // FIXME/TODO: This will clobber change detection
        cmds.entity(robot).insert(info);
    }
}

pub fn shutdown(channels: Res<HwStatChannels>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        let _ = channels.1.send(());
    }
}

fn collect_system_state(system: &System) -> anyhow::Result<SystemInfo> {
    // TODO sorting?
    let hw_state = SystemInfo {
        processes: Processes(
            system
                .processes()
                .values()
                .map(|process| Process {
                    name: process.name().to_owned(),
                    pid: process.pid().as_u32(),
                    memory: process.memory(),
                    cpu_usage: process.cpu_usage(),
                    user: process
                        .user_id()
                        .and_then(|user| system.get_user_by_id(user))
                        .map(|user| user.name().to_owned()),
                })
                .collect(),
        ),
        load_average: LoadAverage {
            one_min: system.load_average().one,
            five_min: system.load_average().five,
            fifteen_min: system.load_average().fifteen,
        },
        networks: Networks(
            system
                .networks()
                .iter()
                .map(|(name, data)| Network {
                    name: name.clone(),
                    rx_bytes: data.total_received(),
                    tx_bytes: data.total_transmitted(),
                    rx_packets: data.total_packets_received(),
                    tx_packets: data.total_packets_transmitted(),
                    rx_errors: data.total_errors_on_received(),
                    tx_errors: data.total_errors_on_transmitted(),
                })
                .collect(),
        ),
        cpu_total: CpuTotal(Cpu {
            frequency: system.global_cpu_info().frequency(),
            usage: system.global_cpu_info().cpu_usage(),
            name: system.global_cpu_info().name().to_owned(),
        }),
        cpus: Cores(
            system
                .cpus()
                .iter()
                .map(|cpu| Cpu {
                    frequency: cpu.frequency(),
                    usage: cpu.cpu_usage(),
                    name: cpu.name().to_owned(),
                })
                .collect(),
        ),
        memory: Memory {
            total_mem: system.total_memory(),
            used_mem: system.used_memory(),
            free_mem: system.free_memory(),
            total_swap: system.total_swap(),
            used_swap: system.used_swap(),
            free_swap: system.free_swap(),
        },
        components: Temperatures(
            system
                .components()
                .iter()
                .map(|component| ComponentTemperature {
                    tempature: Celsius(component.temperature()),
                    tempature_max: Celsius(component.max()),
                    tempature_critical: component.critical().map(Celsius),
                    name: component.label().to_owned(),
                })
                .collect(),
        ),
        disks: Disks(
            system
                .disks()
                .iter()
                .map(|disk| Disk {
                    name: disk.name().to_string_lossy().to_string(),
                    mount_point: disk.mount_point().to_string_lossy().to_string(),
                    total_space: disk.total_space(),
                    available_space: disk.available_space(),
                    removable: disk.is_removable(),
                })
                .collect(),
        ),
        uptime: Uptime(Duration::from_secs(system.uptime())),
        os: OperatingSystem {
            name: system.name(),
            kernel_version: system.kernel_version(),
            os_version: system.long_os_version(),
            distro: Some(system.distribution_id()),
            host_name: system.host_name(),
        },
    };

    Ok(hw_state)
}
