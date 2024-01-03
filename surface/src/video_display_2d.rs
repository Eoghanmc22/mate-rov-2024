use bevy::{
    prelude::*,
    render::{camera::Camera as BevyCamera, view::RenderLayers},
};
use common::components::Camera;

const RENDER_LAYERS: RenderLayers = RenderLayers::layer(2);

pub struct VideoDisplay2DPlugin;

impl Plugin for VideoDisplay2DPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VideoDisplay2DSettings>()
            // .init_resource::<VideoTree>()
            .add_systems(Startup, setup)
            .add_systems(Update, (create_display, enable_camera));
    }
}

#[derive(Component)]
struct DisplayCamera;
#[derive(Component)]
struct DisplayParent;
#[derive(Component)]
struct DisplayMarker;

#[derive(Resource, Default)]
pub struct VideoDisplay2DSettings {
    pub enabled: bool,
}

fn setup(mut cmds: Commands) {
    cmds.spawn((
        Camera2dBundle {
            camera: BevyCamera {
                is_active: false,
                ..default()
            },
            ..default()
        },
        DisplayCamera,
        UiCameraConfig { show_ui: true },
        RENDER_LAYERS,
    ));

    // Root
    cmds.spawn((
        //
        Name::new("Cameras 2D"),
        vertical_root(),
        DisplayParent,
    ))
    .with_children(|builder| {
        builder
            .spawn(horizontal_subroot())
            .with_children(|builder| {
                builder
                    .spawn(horizontal_container())
                    .with_children(|builder| {
                        builder.spawn(horizontal_feed());
                    });

                builder.spawn(horizontal_separator());

                builder
                    .spawn(horizontal_container())
                    .with_children(|builder| {
                        builder.spawn(horizontal_feed());
                    });
            });

        builder.spawn(vertical_separator());

        // builder
        //     .spawn(vertical_container())
        //     .with_children(|builder| {
        //         builder.spawn(vertical_feed());
        //     });
        builder
            .spawn(horizontal_subroot())
            .with_children(|builder| {
                builder
                    .spawn(horizontal_container())
                    .with_children(|builder| {
                        builder.spawn(horizontal_feed());
                    });

                builder.spawn(horizontal_separator());

                builder
                    .spawn(horizontal_container())
                    .with_children(|builder| {
                        builder.spawn(horizontal_feed());
                    });
            });
    });
}

fn horizontal_root() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            background_color: Color::RED.into(),
            ..default()
        },
        RENDER_LAYERS,
    )
}

fn horizontal_subroot() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            background_color: Color::RED.into(),
            ..default()
        },
        RENDER_LAYERS,
    )
}

fn horizontal_container() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                flex_grow: 1.0,
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            background_color: Color::BLUE.into(),
            ..default()
        },
        RENDER_LAYERS,
    )
}

fn horizontal_feed() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                max_height: Val::Percent(100.0),
                aspect_ratio: Some(16.0 / 9.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            background_color: Color::ORANGE.into(),
            ..default()
        },
        RENDER_LAYERS,
    )
}

fn horizontal_separator() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                height: Val::Px(25.0),
                width: Val::Px(5.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            background_color: Color::BLACK.into(),
            ..default()
        },
        RENDER_LAYERS,
    )
}

fn vertical_root() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            background_color: Color::RED.into(),
            ..default()
        },
        RENDER_LAYERS,
    )
}

fn vertical_subroot() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            background_color: Color::RED.into(),
            ..default()
        },
        RENDER_LAYERS,
    )
}

fn vertical_container() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                flex_grow: 1.0,
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            background_color: Color::BLUE.into(),
            ..default()
        },
        RENDER_LAYERS,
    )
}

fn vertical_feed() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                height: Val::Percent(100.0),
                max_width: Val::Percent(100.0),
                aspect_ratio: Some(16.0 / 9.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            background_color: Color::ORANGE.into(),
            ..default()
        },
        RENDER_LAYERS,
    )
}

fn vertical_separator() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                height: Val::Px(5.0),
                width: Val::Px(25.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            background_color: Color::BLACK.into(),
            ..default()
        },
        RENDER_LAYERS,
    )
}

fn create_display(
    mut cmds: Commands,
    new_cameras: Query<
        (Entity, &Handle<Image>, Option<&Transform>),
        (With<Camera>, Added<Handle<Image>>),
    >,
    cameras: Query<(Entity, &Style, &Handle<Image>, &DisplayMarker)>,
    parent: Query<Entity, With<DisplayParent>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
) {
    // for (entity, handle, transform) in &new_cameras {
    //     // let material = materials.add(StandardMaterial {
    //     //     base_color: Color::WHITE,
    //     //     base_color_texture: Some(handle.clone_weak()),
    //     //     unlit: true,
    //     //     ..default()
    //     // });
    //
    //     error!("HIT");
    //
    //     cmds.entity(entity).insert((
    //         ImageBundle {
    //             // style: Style {
    //             //     width: Val::Px(200.0),
    //             //     height: Val::Px(200.0),
    //             //     position_type: PositionType::Absolute,
    //             //     left: Val::Px(210.),
    //             //     bottom: Val::Px(10.),
    //             //     border: UiRect::all(Val::Px(20.)),
    //             //     ..default()
    //             // },
    //             image: UiImage {
    //                 texture: handle.clone(),
    //                 ..default()
    //             },
    //             ..default()
    //         },
    //         DisplayMarker,
    //         RENDER_LAYERS,
    //     ));
    //
    //     let parent = parent.single();
    //     cmds.entity(parent).add_child(entity);
    // }
    //
    // for (entity, style, handle, display) in &cameras {
    //     let Some(image) = images.get(handle) else {
    //         continue;
    //     };
    //
    //     if image.size() != display.0 {
    //         let material = materials.add(StandardMaterial {
    //             base_color: Color::WHITE,
    //             base_color_texture: Some(handle.clone()),
    //             unlit: true,
    //             ..default()
    //         });
    //
    //         let aspect_ratio = image.aspect_ratio();
    //
    //         let mesh_width = 2.0;
    //         let mesh_height = mesh_width * aspect_ratio;
    //
    //         let mesh = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
    //             mesh_width,
    //             mesh_height,
    //         ))));
    //
    //         cmds.entity(entity)
    //             .insert((mesh, material, DisplayMarker(image.size())));
    //     }
    // }
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
