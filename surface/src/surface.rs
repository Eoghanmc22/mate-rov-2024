use bevy::prelude::*;

pub struct SurfacePlugin;

// TODO: This nameing is kinda bad
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
            .spawn((Name::new("Control Station"), LocalSurfaceMarker))
            .id();

        app.world.insert_resource(LocalSurface { entity: surface })
    }
}
