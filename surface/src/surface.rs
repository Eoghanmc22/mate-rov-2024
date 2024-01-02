use bevy::prelude::*;
use common::{
    components::{Singleton, Surface},
    ecs_sync::Replicate,
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
        let surface = app
            .world
            .spawn((
                Name::new("Control Station"),
                Surface,
                LocalSurfaceMarker,
                Replicate,
                Singleton,
            ))
            .id();

        app.world.insert_resource(LocalSurface { entity: surface })
    }
}
