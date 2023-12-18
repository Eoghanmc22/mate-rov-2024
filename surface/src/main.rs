use std::{fmt::Formatter, time::Duration};

use bevy::{app::ScheduleRunnerPlugin, prelude::*, reflect::serde::TypedReflectDeserializer};
use common::{
    adapters,
    ecs_sync::{
        apply_changes::ChangeApplicationPlugin, detect_changes::ChangeDetectionPlugin,
        SerializedChange, SerializedChangeInEvent, SerializedChangeOutEvent,
    },
    sync::SyncRole,
    CommonPlugins,
};
use tracing::Level;

fn main() -> anyhow::Result<()> {
    // TODO: tracy support
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let bevy_plugins = MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
        1.0 / 100.0,
    )));

    App::new()
        .add_plugins((
            // DefaultPlugins.build().disable::<bevy::audio::AudioPlugin>(),
            bevy_plugins,
            CommonPlugins(SyncRole::Client).build(), // .disable::<ChangeApplicationPlugin>(),
        ))
        .add_systems(Update, log_updates)
        .run();

    Ok(())
}

fn log_updates(
    mut inbound: EventReader<SerializedChangeInEvent>,
    mut outbound: EventReader<SerializedChangeOutEvent>,
    reg: Res<AppTypeRegistry>,
) {
    info!("---------- FRAME -----------");
    for inbound in inbound.read() {
        info!("{inbound:?}");

        // if let SerializedChangeInEvent(SerializedChange::ComponentUpdated(
        //     net_id,
        //     path,
        //     Some(data),
        // )) = &inbound
        // {
        //     let registry = reg.read();
        //     let registration = registry.get_with_type_path(path).unwrap();
        //
        //     let seed = TypedReflectDeserializer::new(registration, &registry);
        //     let reflect = adapters::options().deserialize_seed(seed, data).unwrap();
        //
        //     info!("{reflect:?}");
        // }
    }
    for outbound in outbound.read() {
        info!("{outbound:?}");

        // if let SerializedChangeOutEvent(SerializedChange::ComponentUpdated(
        //     net_id,
        //     path,
        //     Some(data),
        // )) = &outbound
        // {
        //     let registry = reg.read();
        //     let registration = registry.get_with_type_path(path).unwrap();
        //
        //     let seed = TypedReflectDeserializer::new(registration, &registry);
        //     let reflect = adapters::options().deserialize_seed(seed, data).unwrap();
        //
        //     info!("{reflect:?}");
        // }
    }
}
