use ahash::HashSet;
use bevy::{
    math::{vec3a, Vec3A},
    prelude::*,
};
use common::{
    bundles::MovementContributionBundle,
    components::{
        Armed, Depth, DepthTarget, MovementAxisMaximums, MovementContribution, OrientationTarget,
        Robot, RobotId,
    },
    ecs_sync::{NetId, Replicate},
    types::units::Meters,
};
use leafwing_input_manager::{
    action_state::ActionState, axislike::SingleAxis, input_map::InputMap,
    plugin::InputManagerPlugin, Actionlike, InputManagerBundle,
};
use motor_math::{solve::reverse::Axis, Movement};

// TODO(low): Handle multiple gamepads better
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<InputInterpolation>()
            .add_plugins(InputManagerPlugin::<Action>::default())
            .add_systems(
                Update,
                (
                    attach_to_new_robots,
                    handle_disconnected_robots,
                    movement,
                    arm,
                    depth_hold,
                    leveling,
                    trim_orientation,
                    trim_depth,
                ),
            );
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct InputInterpolation {
    depth_mps: f32,
    trim_dps: f32,
}

impl Default for InputInterpolation {
    fn default() -> Self {
        Self {
            depth_mps: 0.3,
            trim_dps: 90.0,
        }
    }
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum Action {
    Arm,
    Disarm,

    // SetControlMapping(&'static str),
    // CenterServo,
    // SelectServoIncrement,
    // SelectServoDecrement,
    // RotateServoForward,
    // RotateServoBackward,
    // IncreaseGain,
    // DecreaseGain,
    // ResetGain,
    ToggleDepthHold,
    ToggleLeveling(LevelingType),

    // SetRobotMode(),
    Surge,
    SurgeInverted,
    Heave,
    HeaveInverted,
    Sway,
    SwayInverted,

    Pitch,
    PitchInverted,
    Roll,
    RollInverted,
    Yaw,
    YawInverted,
    // HoldAxis,
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect, Default)]
pub enum LevelingType {
    #[default]
    Upright,
    Inverted,
}

#[derive(Component)]
struct InputMarker;

fn attach_to_new_robots(mut cmds: Commands, new_robots: Query<(&NetId, &Name), Added<Robot>>) {
    for (robot, name) in &new_robots {
        let mut input_map = InputMap::default();

        input_map.insert(Action::Disarm, GamepadButtonType::Select);
        input_map.insert(Action::Arm, GamepadButtonType::Start);

        input_map.insert(Action::Disarm, KeyCode::Space);
        input_map.insert(Action::Arm, KeyCode::Enter);

        input_map.insert(
            Action::ToggleLeveling(LevelingType::Upright),
            GamepadButtonType::North,
        );
        input_map.insert(
            Action::ToggleLeveling(LevelingType::Inverted),
            GamepadButtonType::South,
        );
        input_map.insert(Action::ToggleDepthHold, GamepadButtonType::East);

        input_map.insert(
            Action::Yaw,
            SingleAxis::symmetric(GamepadAxisType::LeftStickX, 0.05),
        );
        input_map.insert(
            Action::Surge,
            SingleAxis::symmetric(GamepadAxisType::LeftStickY, 0.05),
        );

        input_map.insert(
            Action::Sway,
            SingleAxis::symmetric(GamepadAxisType::RightStickX, 0.05),
        );
        input_map.insert(
            Action::Heave,
            SingleAxis::symmetric(GamepadAxisType::RightStickY, 0.05),
        );

        // input_map.insert(
        //     Action::Pitch,
        //     SingleAxis::symmetric(GamepadAxisType::RightZ, 0.05),
        // );
        // input_map.insert(
        //     Action::PitchInverted,
        //     SingleAxis::symmetric(GamepadAxisType::LeftZ, 0.05),
        // );

        input_map.insert(Action::Pitch, GamepadButtonType::RightTrigger);
        input_map.insert(Action::PitchInverted, GamepadButtonType::LeftTrigger);

        input_map.insert(Action::Roll, GamepadButtonType::RightTrigger2);
        input_map.insert(Action::RollInverted, GamepadButtonType::LeftTrigger2);

        cmds.spawn((
            InputManagerBundle::<Action> {
                // Stores "which actions are currently pressed"
                action_state: ActionState::default(),
                // Describes how to convert from player inputs into those actions
                input_map,
            },
            MovementContributionBundle {
                name: Name::new(format!("HID {name}")),
                contribution: MovementContribution(Movement::default()),
                robot: RobotId(*robot),
            },
            InputInterpolation::default(),
            InputMarker,
            Replicate,
        ));
    }
}

fn handle_disconnected_robots(
    mut cmds: Commands,
    robots: Query<&NetId, With<Robot>>,
    inputs: Query<(Entity, &RobotId), With<InputMarker>>,
    mut removed_robots: RemovedComponents<Robot>,
) {
    for _robot in removed_robots.read() {
        let robots: HashSet<NetId> = robots.iter().copied().collect();

        inputs
            .iter()
            .filter(|(_, &RobotId(robot))| !robots.contains(&robot))
            .for_each(|(entity, _)| cmds.entity(entity).despawn());
    }
}

// TODO(mid): Remap sticks to square. See http://theinstructionlimit.com/squaring-the-thumbsticks
fn movement(
    mut cmds: Commands,
    inputs: Query<(Entity, &RobotId, &ActionState<Action>), With<InputMarker>>,
    robots: Query<
        (
            &MovementAxisMaximums,
            Option<&DepthTarget>,
            Option<&OrientationTarget>,
            &RobotId,
        ),
        With<Robot>,
    >,
) {
    for (entity, robot, action_state) in &inputs {
        let Some((MovementAxisMaximums(maximums), depth_target, orientation_target, _)) = robots
            .iter()
            .find(|(_, _, _, robot_id)| robot_id.0 == robot.0)
        else {
            error!("Could not find robot for input");

            continue;
        };

        let x = action_state.value(&Action::Sway) * maximums[&Axis::X].0
            - action_state.value(&Action::SwayInverted) * maximums[&Axis::X].0;
        let y = action_state.value(&Action::Surge) * maximums[&Axis::Y].0
            - action_state.value(&Action::SurgeInverted) * maximums[&Axis::Y].0;
        let z = action_state.value(&Action::Heave) * maximums[&Axis::Z].0
            - action_state.value(&Action::HeaveInverted) * maximums[&Axis::Z].0;

        let x_rot = action_state.value(&Action::Pitch) * maximums[&Axis::XRot].0
            - action_state.value(&Action::PitchInverted) * maximums[&Axis::XRot].0;
        let y_rot = action_state.value(&Action::Roll) * maximums[&Axis::YRot].0
            - action_state.value(&Action::RollInverted) * maximums[&Axis::YRot].0;
        let z_rot = action_state.value(&Action::Yaw) * maximums[&Axis::ZRot].0
            - action_state.value(&Action::YawInverted) * maximums[&Axis::ZRot].0;

        let z = if depth_target.is_none() { z } else { 0.0 };

        let (x_rot, y_rot) = if orientation_target.is_none() {
            (x_rot, y_rot)
        } else {
            (0.0, 0.0)
        };

        let movement = Movement {
            force: vec3a(x, y, z),
            torque: vec3a(x_rot, y_rot, z_rot),
        };

        cmds.entity(entity).insert(MovementContribution(movement));
    }
}

fn arm(
    mut cmds: Commands,
    inputs: Query<(&RobotId, &ActionState<Action>), With<InputMarker>>,
    robots: Query<(Entity, &RobotId), With<Robot>>,
) {
    for (robot, action_state) in &inputs {
        let disarm = action_state.just_pressed(&Action::Disarm);
        let arm = action_state.just_pressed(&Action::Arm);

        let robot = robots.iter().find(|&(_, other_robot)| robot == other_robot);

        if let Some((robot, _)) = robot {
            if disarm {
                info!("Disarming");
                cmds.entity(robot).insert(Armed::Disarmed);
            } else if arm {
                info!("Arming");
                cmds.entity(robot).insert(Armed::Armed);
            }
        } else if arm || disarm {
            warn!("No ROV attached");
        }
    }
}

fn depth_hold(
    mut cmds: Commands,
    inputs: Query<(&RobotId, &ActionState<Action>), With<InputMarker>>,
    robots: Query<(Entity, &Depth, Option<&DepthTarget>, &RobotId), With<Robot>>,
) {
    for (robot, action_state) in &inputs {
        let toggle = action_state.just_pressed(&Action::ToggleDepthHold);

        let robot = robots
            .iter()
            .find(|&(_, _, _, other_robot)| robot == other_robot);

        if let Some((robot, depth, depth_target, _)) = robot {
            if toggle {
                match depth_target {
                    Some(_) => {
                        info!("Clear Depth Hold");
                        cmds.entity(robot).remove::<DepthTarget>();
                    }
                    None => {
                        let depth = depth.0.depth;

                        info!("Set Depth Hold: {:.2}", depth);
                        cmds.entity(robot).insert(DepthTarget(depth));
                    }
                }
            }
        } else if toggle {
            warn!("No ROV attached");
        }
    }
}

fn leveling(
    mut cmds: Commands,
    inputs: Query<(&RobotId, &ActionState<Action>), With<InputMarker>>,
    robots: Query<(Entity, Option<&OrientationTarget>, &RobotId), With<Robot>>,
) {
    for (robot, action_state) in &inputs {
        let toggle_upright =
            action_state.just_pressed(&Action::ToggleLeveling(LevelingType::Upright));
        let toggle_inverted =
            action_state.just_pressed(&Action::ToggleLeveling(LevelingType::Inverted));

        let robot = robots
            .iter()
            .find(|&(_, _, other_robot)| robot == other_robot);

        if let Some((robot, orientation_target, _)) = robot {
            if toggle_upright || toggle_inverted {
                let new_target = if toggle_upright {
                    Vec3A::Z
                } else {
                    Vec3A::NEG_Z
                };

                match orientation_target {
                    Some(old_target) if old_target.0 == new_target => {
                        info!("Clear Depth Hold");
                        cmds.entity(robot).remove::<OrientationTarget>();
                    }
                    _ => {
                        if toggle_upright {
                            info!("Set Level Upright");
                        } else {
                            info!("Set Level Inverted");
                        }

                        cmds.entity(robot).insert(OrientationTarget(new_target));
                    }
                }
            }
        } else if toggle_upright || toggle_inverted {
            warn!("No ROV attached");
        }
    }
}

fn trim_orientation(
    mut cmds: Commands,
    inputs: Query<(&RobotId, &ActionState<Action>, &InputInterpolation), With<InputMarker>>,
    robots: Query<(Entity, Option<&OrientationTarget>, &RobotId), With<Robot>>,
    time: Res<Time<Real>>,
) {
    for (robot, action_state, interpolation) in &inputs {
        let pitch = action_state.value(&Action::Pitch) - action_state.value(&Action::PitchInverted);
        let roll = action_state.value(&Action::Roll) - action_state.value(&Action::RollInverted);

        let robot = robots
            .iter()
            .find(|&(_, _, other_robot)| robot == other_robot);

        if let Some((robot, orientation_target, _)) = robot {
            let Some(&OrientationTarget(mut orientation_target)) = orientation_target else {
                continue;
            };

            if pitch != 0.0 {
                let input = pitch * interpolation.trim_dps * time.delta_seconds();
                orientation_target = Quat::from_rotation_x(input.to_radians()) * orientation_target;
            }

            if roll != 0.0 {
                let input = roll * interpolation.trim_dps * time.delta_seconds();
                orientation_target = Quat::from_rotation_y(input.to_radians()) * orientation_target;
            }

            if pitch != 0.0 || roll != 0.0 {
                cmds.entity(robot)
                    .insert(OrientationTarget(orientation_target));
            }
        } else if pitch != 0.0 || roll != 0.0 {
            warn!("No ROV attached");
        }
    }
}

fn trim_depth(
    mut cmds: Commands,
    inputs: Query<(&RobotId, &ActionState<Action>, &InputInterpolation), With<InputMarker>>,
    robots: Query<(Entity, Option<&DepthTarget>, &RobotId), With<Robot>>,
    time: Res<Time<Real>>,
) {
    for (robot, action_state, interpolation) in &inputs {
        let z = action_state.value(&Action::Heave) - action_state.value(&Action::HeaveInverted);

        let robot = robots
            .iter()
            .find(|&(_, _, other_robot)| robot == other_robot);

        if let Some((robot, depth_target, _)) = robot {
            let Some(&DepthTarget(Meters(mut depth_target))) = depth_target else {
                continue;
            };

            if z != 0.0 {
                let input = z * interpolation.depth_mps * time.delta_seconds();
                depth_target -= input;
                if depth_target < 0.1 {
                    depth_target = 0.1;
                }
                cmds.entity(robot).insert(DepthTarget(depth_target.into()));
            }
        } else if z != 0.0 {
            warn!("No ROV attached");
        }
    }
}
