//! Code shared between both the surface and robot projects
#![feature(try_blocks, const_fn_floating_point_arithmetic, const_float_classify)]
#![allow(clippy::type_complexity)]

use bevy::{
    app::{Plugin, PluginGroup, PluginGroupBuilder},
    prelude::App,
};
use ctrlc::CtrlCPlugin;
use ecs_sync::{apply_changes::ChangeApplicationPlugin, detect_changes::ChangeDetectionPlugin};
use error::ErrorPlugin;
use sync::SyncPlugin;

pub mod adapters;
pub mod bundles;
pub mod components;
pub mod ctrlc;
pub mod ecs_sync;
pub mod error;
pub mod protocol;
pub mod sync;
pub mod types;

pub struct CommunicationTypes;

impl Plugin for CommunicationTypes {
    fn build(&self, app: &mut App) {
        types::register_types(app);
        components::register_components(app);
    }
}

pub struct CommonPlugins;

impl PluginGroup for CommonPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(SyncPlugin)
            .add(CommunicationTypes)
            .add(ChangeDetectionPlugin)
            .add(ChangeApplicationPlugin)
            .add(CtrlCPlugin)
            .add(ErrorPlugin)
    }
}
