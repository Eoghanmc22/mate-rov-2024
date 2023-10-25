use parallax::system::launch::SystemManager;
use tracing::{info, Level};

pub mod peripheral;
pub mod systems;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    info!("Starting robot");

    let mut systems = SystemManager::default();

    info!("---------- Registering systems ----------");
    info!("--------------------------------------");

    systems.start("robot_config.toml");

    info!("Robot stopped");
}
