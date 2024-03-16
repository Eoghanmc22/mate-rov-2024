//! Code shared between both the surface and robot projects
#![feature(
    try_blocks,
    const_fn_floating_point_arithmetic,
    const_float_classify,
    hash_extract_if
)]
#![allow(clippy::type_complexity)]

use bevy::{
    app::{Plugin, PluginGroup, PluginGroupBuilder},
    core::Name,
    ecs::system::Resource,
    prelude::App,
    transform::components::Transform,
};
use ctrlc::CtrlCPlugin;
use ecs_sync::{
    apply_changes::ChangeApplicationPlugin, detect_changes::ChangeDetectionPlugin, AppReplicateExt,
    NetId, Replicate,
};
use error::ErrorPlugin;
use over_run::OverRunPligin;
use sync::{Latency, SyncPlugin, SyncRole};

pub mod adapters;
pub mod bundles;
pub mod components;
pub mod ctrlc;
pub mod ecs_sync;
pub mod error;
pub mod events;
pub mod over_run;
pub mod protocol;
pub mod reflect;
pub mod sync;
pub mod types;

pub struct CommunicationTypes;

impl Plugin for CommunicationTypes {
    fn build(&self, app: &mut App) {
        types::register_types(app);
        components::register_components(app);
        events::register_events(app);

        app.register_type::<NetId>()
            .register_type::<Replicate>()
            .register_type::<Latency>();
        // .register_type::<Peer>();

        app.replicate::<Transform>().replicate_reflect::<Name>();
    }
}

pub struct CommonPlugins {
    pub name: String,
    pub role: SyncRole,
}

#[derive(Resource, Debug, Clone)]
pub struct InstanceName(String);

impl PluginGroup for CommonPlugins {
    fn build(self) -> PluginGroupBuilder {
        let name = self.name;

        PluginGroupBuilder::start::<Self>()
            .add(move |app: &mut App| {
                app.insert_resource(InstanceName(name.clone()));
            })
            .add(SyncPlugin(self.role))
            .add(CommunicationTypes)
            .add(ChangeDetectionPlugin)
            .add(ChangeApplicationPlugin)
            .add(CtrlCPlugin)
            .add(ErrorPlugin)
            .add(OverRunPligin)
    }
}
