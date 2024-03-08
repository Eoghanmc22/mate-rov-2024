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
        app.add_plugins(InputManagerPlugin::<Action>::default())
            .add_systems(
                Update,
                (
                    attach_to_new_robots,
                    handle_disconnected_robots,
                    movement,
                    arm,
                    depth_hold,
                    leveling,
                ),
            );
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
    // TrimPitch,
    // TrimPitchInverted,
    // TrimRoll,
    // TrimRollInverted,

    // SetRobotMode(),
    Surge,
    Heave,
    Sway,

    Pitch,
    Roll,
    Yaw,
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
    robots: Query<(&MovementAxisMaximums, &RobotId), With<Robot>>,
) {
    for (entity, robot, action_state) in &inputs {
        let Some((MovementAxisMaximums(maximums), _)) =
            robots.iter().find(|(_, robot_id)| robot_id.0 == robot.0)
        else {
            error!("Could not find robot for input");

            continue;
        };

        let x = action_state.value(&Action::Sway) * maximums[&Axis::X].0;
        let y = action_state.value(&Action::Surge) * maximums[&Axis::Y].0;
        let z = action_state.value(&Action::Heave) * maximums[&Axis::Z].0;

        let x_rot = action_state.value(&Action::Pitch) * maximums[&Axis::XRot].0;
        let y_rot = action_state.value(&Action::Roll) * maximums[&Axis::YRot].0;
        let z_rot = action_state.value(&Action::Yaw) * maximums[&Axis::ZRot].0;

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
                cmds.entity(robot).insert(Armed::Disarmed);
            } else if arm {
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
                        cmds.entity(robot).remove::<DepthTarget>();
                    }
                    None => {
                        cmds.entity(robot).insert(DepthTarget(depth.0.depth));
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
                match orientation_target {
                    Some(_) => {
                        cmds.entity(robot).remove::<OrientationTarget>();
                    }
                    None => {
                        if toggle_upright {
                            cmds.entity(robot).insert(OrientationTarget(Vec3A::Z));
                        } else if toggle_inverted {
                            cmds.entity(robot).insert(OrientationTarget(Vec3A::NEG_Z));
                        }
                    }
                }
            }
        } else if toggle_upright || toggle_inverted {
            warn!("No ROV attached");
        }
    }
}
