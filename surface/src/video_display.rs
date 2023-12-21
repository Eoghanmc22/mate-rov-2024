use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;
use common::components::Camera;

pub struct VideoDisplayPlugin;

impl Plugin for VideoDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, create_display);
    }
}

#[derive(Component)]
struct DisplayCamera;
#[derive(Component)]
struct DisplayMarker(UVec2);

fn setup(
    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    cmds.spawn((
        Camera3dBundle {
            transform: Transform::default().looking_at(Vec3::Z, Vec3::Y),
            ..default()
        },
        PanOrbitCamera::default(),
        DisplayCamera,
    ));

}

fn create_display(
    mut cmds: Commands,
    new_cameras: Query<
        (Entity, &Handle<Image>, Option<&Transform>),
        (With<Camera>, Added<Handle<Image>>),
    >,
    cameras: Query<(Entity, &Handle<Image>, &DisplayMarker)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
) {
    for (entity, handle, transform) in &new_cameras {
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(handle.clone_weak()),
            unlit: true,
            ..default()
        });

        cmds.entity(entity).insert((
            PbrBundle {
                transform: transform.cloned().unwrap_or_default(),
                material,
                ..default()
            },
            DisplayMarker(UVec2::default()),
        ));
    }

    for (entity, handle, display) in &cameras {
        let Some(image) = images.get(handle) else {
            continue;
        };

        if image.size() != display.0 {
            let material = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(handle.clone()),
                unlit: true,
                ..default()
            });

            let aspect_ratio = image.aspect_ratio();

            let mesh_width = 1.0;
            let mesh_height = mesh_width * aspect_ratio;

            let mesh = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
                mesh_width,
                mesh_height,
            ))));

            cmds.entity(entity)
                .insert((mesh, material, DisplayMarker(image.size())));
        }
    }
}
