use bevy::prelude::*;
use common::{
    components::{Singleton, Surface},
    ecs_sync::Replicate,
    InstanceName,
};

pub struct SurfacePlugin;

// TODO(low): This nameing is kinda bad
#[derive(Component, Debug, Copy, Clone, PartialEq, Default)]
pub struct LocalSurfaceMarker;

#[derive(Resource)]
pub struct LocalSurface {
    pub entity: Entity,
}

impl Plugin for SurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup_surface);
    }
}

fn setup_surface(mut cmds: Commands, name: Res<InstanceName>) {
    let surface = cmds
        .spawn((
            Name::new(name.0.clone()),
            Surface,
            LocalSurfaceMarker,
            Replicate,
            Singleton,
        ))
        .id();

    cmds.insert_resource(LocalSurface { entity: surface })
}
