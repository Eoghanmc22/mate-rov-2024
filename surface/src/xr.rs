use bevy::prelude::*;
use bevy_oxr::xr_input::{
    hands::common::{HandInputDebugRenderer, OpenXrHandInput},
    interactions::{XRDirectInteractor, XRInteractorState, XRRayInteractor},
    prototype_locomotion::{proto_locomotion, PrototypeLocomotionConfig},
    trackers::{
        AimPose, OpenXRController, OpenXRLeftController, OpenXRRightController, OpenXRTracker,
    },
};

pub struct OpenXrPlugin;

impl Plugin for OpenXrPlugin {
    fn build(&self, app: &mut App) {
        // Open XR stuff
        app.add_plugins(OpenXrHandInput)
            .add_plugins(HandInputDebugRenderer)
            .add_systems(Update, proto_locomotion)
            .insert_resource(PrototypeLocomotionConfig::default());

        app.add_systems(Startup, spawn_controllers_example);
    }
}

fn spawn_controllers_example(mut commands: Commands) {
    //left hand
    commands.spawn((
        OpenXRLeftController,
        OpenXRController,
        OpenXRTracker,
        SpatialBundle::default(),
        XRRayInteractor,
        AimPose(Transform::default()),
        XRInteractorState::default(),
    ));
    //right hand
    commands.spawn((
        OpenXRRightController,
        OpenXRController,
        OpenXRTracker,
        SpatialBundle::default(),
        XRDirectInteractor,
        XRInteractorState::default(),
    ));
}
