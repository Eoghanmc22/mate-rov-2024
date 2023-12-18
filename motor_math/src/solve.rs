pub mod forward;
pub mod reverse;

#[cfg(test)]
mod tests {
    extern crate test;
    use ahash::HashMap;
    use std::time::Instant;
    use test::Bencher;

    use glam::vec3a;

    use crate::{
        blue_rov::HeavyMotorId,
        motor_preformance::{self},
        solve::forward,
        utils::vec_from_angles,
        x3d::X3dMotorId,
        Direction, Motor, MotorConfig, Movement,
    };

    use super::reverse;

    #[test]
    fn solve_roundtrip_x3d() {
        let seed_motor = Motor {
            position: vec3a(1.0, 1.0, 1.0).normalize(),
            orientation: vec_from_angles(60.0, 40.0),
            direction: Direction::Clockwise,
        };

        let motor_data =
            motor_preformance::read_motor_data("../robot/motor_data.csv").expect("Read motor data");
        let motor_config = MotorConfig::<X3dMotorId>::new(seed_motor);

        let movement = Movement {
            force: vec3a(-0.6, 0.5, 0.3),
            torque: vec3a(0.2, 0.1, 0.4),
        };

        let start = Instant::now();
        let forces = reverse::reverse_solve(movement, &motor_config);
        let motor_cmds = reverse::forces_to_cmds(forces, &motor_config, &motor_data);
        let elapsed = start.elapsed();

        println!("motor_cmds: {motor_cmds:#?} in {}us", elapsed.as_micros());

        let actual_movement = forward::forward_solve(
            &motor_config,
            &motor_cmds
                .iter()
                .map(|(id, data)| (*id, data.force))
                .collect(),
        );

        let movement_error = movement - actual_movement;
        assert!(movement_error.force.length_squared() < 0.0001);
        assert!(movement_error.torque.length_squared() < 0.0001);
    }

    #[test]
    fn solve_roundtrip_blue_rov() {
        let lateral = Motor {
            position: vec3a(1.0, 1.0, 0.0),
            orientation: vec3a(-1.0, 1.0, 0.0).normalize(),
            direction: Direction::Clockwise,
        };
        let vertical = Motor {
            position: vec3a(1.0, 1.0, 0.0),
            orientation: vec3a(0.0, 0.0, 1.0).normalize(),
            direction: Direction::Clockwise,
        };

        let motor_data =
            motor_preformance::read_motor_data("../robot/motor_data.csv").expect("Read motor data");
        let motor_config = MotorConfig::<HeavyMotorId>::new(lateral, vertical);

        let movement = Movement {
            force: vec3a(0.5, 0.1, 0.4),
            torque: vec3a(0.2, 0.5, -0.3),
        };

        let start = Instant::now();
        let forces = reverse::reverse_solve(movement, &motor_config);
        let motor_cmds = reverse::forces_to_cmds(forces, &motor_config, &motor_data);
        let elapsed = start.elapsed();

        println!("motor_cmds: {motor_cmds:#?} in {}us", elapsed.as_micros());

        let actual_movement = forward::forward_solve(
            &motor_config,
            &motor_cmds
                .iter()
                .map(|(id, data)| (*id, data.force))
                .collect(),
        );

        let movement_error = movement - actual_movement;
        assert!(movement_error.force.length_squared() < 0.0001);
        assert!(movement_error.torque.length_squared() < 0.0001);
    }

    #[test]
    fn solve_roundtrip_arbitrary() {
        let motor_data =
            motor_preformance::read_motor_data("../robot/motor_data.csv").expect("Read motor data");

        let mut motors = HashMap::default();

        #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
        enum MotorIds {
            Right,
            Left,
            Lateral,
            Up1,
            Up2,
            Up3,
        }

        motors.insert(
            MotorIds::Right,
            Motor {
                position: vec3a(1.0, 1.0, 0.0).normalize(),
                orientation: vec3a(0.0, 1.0, 0.0),
                direction: Direction::Clockwise,
            },
        );

        motors.insert(
            MotorIds::Left,
            Motor {
                position: vec3a(-1.0, 1.0, 0.0).normalize(),
                orientation: vec3a(0.0, 1.0, 0.0),
                direction: Direction::CounterClockwise,
            },
        );

        motors.insert(
            MotorIds::Lateral,
            Motor {
                position: vec3a(0.0, 0.0, 0.0),
                orientation: vec3a(1.0, 0.0, 0.0),
                direction: Direction::Clockwise,
            },
        );

        motors.insert(
            MotorIds::Up1,
            Motor {
                position: vec3a(1.0, 1.0, 0.0).normalize() * 2.0,
                orientation: vec3a(0.0, 0.0, 1.0),
                direction: Direction::Clockwise,
            },
        );

        motors.insert(
            MotorIds::Up2,
            Motor {
                position: vec3a(-1.0, 1.0, 0.0).normalize() * 2.0,
                orientation: vec3a(0.0, 0.0, 1.0),
                direction: Direction::CounterClockwise,
            },
        );

        motors.insert(
            MotorIds::Up3,
            Motor {
                position: vec3a(0.0, -1.0, 0.0).normalize() * 2.0,
                orientation: vec3a(0.0, 0.0, 1.0),
                direction: Direction::Clockwise,
            },
        );

        let motor_config = MotorConfig::new_raw(motors);

        let movement = Movement {
            force: vec3a(0.9, -0.5, 0.3),
            torque: vec3a(-0.2, 0.1, 0.4),
        };

        let start = Instant::now();
        let forces = reverse::reverse_solve(movement, &motor_config);
        let motor_cmds = reverse::forces_to_cmds(forces, &motor_config, &motor_data);
        let elapsed = start.elapsed();

        println!("motor_cmds: {motor_cmds:#?} in {}us", elapsed.as_micros());

        let actual_movement = forward::forward_solve(
            &motor_config,
            &motor_cmds
                .iter()
                .map(|(id, data)| (*id, data.force))
                .collect(),
        );

        let movement_error = movement - actual_movement;
        assert!(movement_error.force.length_squared() < 0.0001);
        assert!(movement_error.torque.length_squared() < 0.0001);
    }

    #[bench]
    fn bench_reverse_solver_x3d(b: &mut Bencher) {
        let seed_motor = Motor {
            position: vec3a(0.3, 0.5, 0.4).normalize(),
            orientation: vec_from_angles(60.0, 40.0),
            direction: Direction::Clockwise,
        };

        let motor_data =
            motor_preformance::read_motor_data("../robot/motor_data.csv").expect("Read motor data");
        let motor_config = MotorConfig::<X3dMotorId>::new(seed_motor);

        let movement = Movement {
            force: vec3a(0.6, 0.0, 0.3),
            torque: vec3a(0.2, 0.1, 0.3),
        };

        b.iter(|| {
            let forces = reverse::reverse_solve(movement, &motor_config);
            let motor_cmds = reverse::forces_to_cmds(forces, &motor_config, &motor_data);
            motor_cmds
        });
    }

    #[bench]
    fn bench_reverse_solver_blue_rov(b: &mut Bencher) {
        let lateral = Motor {
            position: vec3a(1.0, 1.0, 0.0),
            orientation: vec3a(-1.0, 1.0, 0.0).normalize(),
            direction: Direction::Clockwise,
        };
        let vertical = Motor {
            position: vec3a(1.0, 1.0, 0.0),
            orientation: vec3a(0.0, 0.0, 1.0).normalize(),
            direction: Direction::Clockwise,
        };

        let motor_data =
            motor_preformance::read_motor_data("../robot/motor_data.csv").expect("Read motor data");
        let motor_config = MotorConfig::<HeavyMotorId>::new(lateral, vertical);

        let movement = Movement {
            force: vec3a(0.6, 0.0, 0.3),
            torque: vec3a(0.2, 0.1, 0.3),
        };

        b.iter(|| {
            let forces = reverse::reverse_solve(movement, &motor_config);
            let motor_cmds = reverse::forces_to_cmds(forces, &motor_config, &motor_data);
            motor_cmds
        });
    }
}
