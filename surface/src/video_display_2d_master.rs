use bevy::{
    math::f32,
    prelude::*,
    render::{camera::Camera as BevyCamera, view::RenderLayers},
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use bevy_mod_picking::prelude::*;
use common::components::Camera;

const RENDER_LAYERS: RenderLayers = RenderLayers::layer(2);

pub struct VideoDisplay2DPlugin;

impl Plugin for VideoDisplay2DPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VideoDisplay2DSettings>()
            // .init_resource::<VideoTree>()
            .add_event::<MakeMaster>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    create_display,
                    update_aspect_ratio.after(create_display),
                    handle_new_masters,
                    enable_camera,
                ),
            );
    }
}

#[derive(Resource)]
struct MeshResource(Handle<Mesh>);

#[derive(Default, Component)]
struct Video {
    master_camera: Option<Entity>,
    cameras: Vec<Entity>,
}

#[derive(Component, Clone, Copy)]
struct DisplayCamera;
#[derive(Component, Clone, Copy)]
struct DisplayParent;
#[derive(Component, Clone, Copy)]
struct DisplayMarker(u16);

#[derive(Event, Clone, Copy)]
struct MakeMaster(Entity);

impl From<ListenerInput<Pointer<Click>>> for MakeMaster {
    fn from(value: ListenerInput<Pointer<Click>>) -> Self {
        MakeMaster(value.listener())
    }
}

#[derive(Resource, Default)]
pub struct VideoDisplay2DSettings {
    pub enabled: bool,
}

fn setup(mut cmds: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let camera = cmds
        .spawn((
            Camera2dBundle {
                camera: BevyCamera {
                    is_active: false,
                    ..default()
                },
                ..default()
            },
            DisplayCamera,
            RENDER_LAYERS,
        ))
        .id();

    // Root
    cmds.spawn((
        Name::new("Cameras 2D"),
        TargetCamera(camera),
        Video::default(),
        SpatialBundle::default(),
        DisplayParent,
    ));

    let mesh = meshes.add(Rectangle::new(1.0, 1.0));
    cmds.insert_resource(MeshResource(mesh));
}

fn create_display(
    mut cmds: Commands,

    mesh: Res<MeshResource>,
    mut materials: ResMut<Assets<ColorMaterial>>,

    new_cameras: Query<Entity, (With<Camera>, Added<Handle<Image>>)>,
    mut lost_cameras: RemovedComponents<Camera>,

    cameras: Query<&Handle<Image>>,
    mut parent: Query<(Entity, &mut Video), With<DisplayParent>>,
) {
    let (parent, mut tree) = parent.single_mut();
    let mut tree_changed = false;

    for entity in &new_cameras {
        tree.cameras.push(entity);
        if tree.master_camera.is_none() {
            tree.master_camera = Some(entity);
        }
        tree_changed = true;
    }

    for entity in lost_cameras.read() {
        tree.cameras.retain(|it| *it != entity);
        if tree.master_camera == Some(entity) {
            tree.master_camera = tree.cameras.iter().cloned().next()
        }
        tree_changed = true;
    }

    if tree_changed {
        for (idx, &camera) in tree.cameras.iter().enumerate() {
            let weak_texture = cameras
                .get(camera)
                .map(|it| it.clone_weak())
                .unwrap_or_else(|_| Default::default());
            let material = materials.add(weak_texture);

            cmds.entity(camera).insert((
                MaterialMesh2dBundle {
                    mesh: Mesh2dHandle(mesh.0.clone()),
                    material,
                    transform: Transform::default(),
                    ..default()
                },
                DisplayMarker(idx as _),
                PickableBundle::default(),
                On::<Pointer<Click>>::send_event::<MakeMaster>(),
                RENDER_LAYERS,
            ));
            cmds.entity(parent).add_child(camera);
        }
    }
}

fn update_aspect_ratio(
    mut displays: Query<(&Handle<Image>, &DisplayMarker, &mut Transform)>,
    images: Res<Assets<Image>>,

    camera: Query<&BevyCamera, With<DisplayCamera>>,
) {
    // TODO: Handle Errors
    let camera = camera.single();
    let logical = camera.logical_viewport_size().unwrap();

    let other_max_width_pct = 1.0 / 3.0;

    // height/width
    let mut master_aspect_ratio = 0.0f32;
    let mut aspect_ratios = Vec::default();
    let mut other_aspect_ratio = 0.0f32;
    let mut count = 0;

    for (handle, display, _transform) in &displays {
        let Some(image) = images.get(handle) else {
            continue;
        };

        aspect_ratios.push((display.0, 1.0f32 / f32::from(image.aspect_ratio())));

        if display.0 != 0 {
            other_aspect_ratio += 1.0f32 / f32::from(image.aspect_ratio());
            count += 1;
        } else {
            master_aspect_ratio = 1.0f32 / f32::from(image.aspect_ratio());
        }
    }

    aspect_ratios.sort_by_key(|it| it.0);

    let other_width_needed = other_aspect_ratio * logical.y;
    let other_width = if other_width_needed < other_max_width_pct * logical.x {
        other_width_needed
    } else {
        other_max_width_pct * logical.x
    };
    let other_width = if other_width * other_aspect_ratio > logical.y {
        (1.0 / other_aspect_ratio) * logical.y
    } else {
        other_width
    };

    let other_remaining_height = logical.y - other_width * other_aspect_ratio;

    let master_width_needed = logical.x - other_width;
    let master_width = if master_width_needed * master_aspect_ratio > logical.y {
        (1.0 / master_aspect_ratio) * logical.y
    } else {
        master_width_needed
    };

    for (handle, display, mut transform) in &mut displays {
        let Some(image) = images.get(handle) else {
            continue;
        };

        if display.0 != 0 {
            let total_aspect_ratio = aspect_ratios
                .iter()
                .skip(1)
                .take(display.0 as usize - 1)
                .map(|it| it.1)
                .sum::<f32>();
            let height_so_far = total_aspect_ratio * other_width
                + other_remaining_height / (count as f32 + 1.0) * display.0 as f32;

            *transform = transform
                .with_translation(Vec3::new(
                    logical.x / 2.0 - other_width / 2.0,
                    logical.y / 2.0
                        - height_so_far
                        - 0.5 / f32::from(image.aspect_ratio()) * other_width,
                    0.0,
                ))
                .with_scale(Vec3::new(
                    other_width,
                    1.0 / f32::from(image.aspect_ratio()) * other_width,
                    1.0,
                ));
        } else {
            *transform = transform
                .with_translation(Vec3::new(
                    master_width_needed / 2.0 - logical.x / 2.0,
                    0.0,
                    0.0,
                ))
                .with_scale(Vec3::new(
                    master_width,
                    master_aspect_ratio * master_width,
                    1.0,
                ));
        }
    }
}

fn handle_new_masters(mut events: EventReader<MakeMaster>, mut query: Query<&mut DisplayMarker>) {
    for event in events.read() {
        let Ok(&new_master) = query.get(event.0) else {
            continue;
        };

        for mut display in &mut query {
            if display.0 == 0 {
                display.0 = new_master.0;
            } else if display.0 == new_master.0 {
                display.0 = 0;
            }
        }
    }
}

fn enable_camera(
    mut last: Local<bool>,
    mut camera: Query<&mut BevyCamera, With<DisplayCamera>>,
    settings: Res<VideoDisplay2DSettings>,
) {
    if *last != settings.enabled {
        for mut camera in camera.iter_mut() {
            camera.is_active = settings.enabled;
        }

        *last = settings.enabled;
    }
}
