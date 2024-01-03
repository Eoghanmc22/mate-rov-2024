use bevy::{
    prelude::*,
    render::{camera::Camera as BevyCamera, view::RenderLayers},
};
use bevy_panorbit_camera::PanOrbitCamera;
use common::components::Camera;

const RENDER_LAYERS: RenderLayers = RenderLayers::layer(3);

pub struct VideoDisplay3DPlugin;

impl Plugin for VideoDisplay3DPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VideoDisplay3DSettings>()
            .add_systems(Startup, setup)
            .add_systems(Update, (create_display, enable_camera));
    }
}

#[derive(Component)]
struct DisplayCamera;
#[derive(Component)]
struct DisplayParent;
#[derive(Component)]
struct DisplayMarker(UVec2);

#[derive(Resource, Default)]
pub struct VideoDisplay3DSettings {
    pub enabled: bool,
}

fn setup(mut cmds: Commands) {
    cmds.spawn((
        Camera3dBundle {
            camera: BevyCamera {
                is_active: false,
                ..default()
            },
            transform: Transform::default().looking_at(Vec3::Z, Vec3::Y),
            ..default()
        },
        PanOrbitCamera::default(),
        DisplayCamera,
        UiCameraConfig { show_ui: false },
        RENDER_LAYERS,
    ));

    cmds.spawn((
        Name::new("Cameras 3D"),
        SpatialBundle::default(),
        DisplayParent,
        RENDER_LAYERS,
    ));
}

fn create_display(
    mut cmds: Commands,
    new_cameras: Query<
        (Entity, &Handle<Image>, Option<&Transform>),
        (With<Camera>, Added<Handle<Image>>),
    >,
    cameras: Query<(Entity, &Handle<Image>, &DisplayMarker)>,
    parent: Query<Entity, With<DisplayParent>>,
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
            RENDER_LAYERS,
        ));

        let parent = parent.single();
        cmds.entity(parent).add_child(entity);
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

            let mesh_width = 2.0;
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

fn enable_camera(
    mut last: Local<bool>,
    mut camera: Query<&mut BevyCamera, With<DisplayCamera>>,
    settings: Res<VideoDisplay3DSettings>,
) {
    if *last != settings.enabled {
        for mut camera in camera.iter_mut() {
            camera.is_active = settings.enabled;
        }

        *last = settings.enabled;
    }
}
