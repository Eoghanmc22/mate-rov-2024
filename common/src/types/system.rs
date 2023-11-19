use serde::{Deserialize, Serialize};

use super::units::Celsius;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Process {
    pub name: String,
    pub pid: u32,
    pub memory: u64,
    pub cpu_usage: f32,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Cpu {
    pub frequency: u64,
    pub usage: f32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ComponentTemperature {
    pub tempature: Celsius,
    pub tempature_max: Celsius,
    pub tempature_critical: Option<Celsius>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Disk {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Network {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
}
