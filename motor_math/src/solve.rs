pub mod forward;
pub mod reverse;

#[cfg(test)]
mod tests {
    extern crate test;
    use std::time::Instant;
    use test::Bencher;

    use glam::vec3a;

    use crate::{
        blue_rov::HeavyMotorId,
        motor_preformance::{self},
        solve::forward,
        utils::vec_from_angles,
        x3d::X3dMotorId,
        Motor, MotorConfig, Movement,
    };

    use super::reverse;

    #[test]
    fn solve_roundtrip() {
        let seed_motor = Motor {
            position: vec3a(1.0, 1.0, 1.0).normalize(),
            orientation: vec_from_angles(60.0, 40.0),
        };

        let motor_data =
            motor_preformance::read_motor_data("../robot/motor_data.csv").expect("Read motor data");
        let motor_config = MotorConfig::<X3dMotorId>::new(seed_motor);

        // let lateral = Motor {
        //     position: vec3a(1.0, 1.0, 0.0),
        //     orientation: vec3a(-1.0, 1.0, 0.0).normalize(),
        // };
        // let vertical = Motor {
        //     position: vec3a(1.0, 1.0, 0.0),
        //     orientation: vec3a(0.0, 0.0, 1.0).normalize(),
        // };
        //
        // let motor_data =
        //     motor_preformance::read_motor_data("../robot/motor_data.csv").expect("Read motor data");
        // let motor_config = MotorConfig::<HeavyMotorId>::new(lateral, vertical);

        // let motor_data =
        //     motor_preformance::read_motor_data("../robot/motor_data.csv").expect("Read motor data");
        //
        // let mut motors = BTreeMap::default();
        //
        // #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
        // enum MotorIds {
        //     Right,
        //     Left,
        //     Lateral,
        //     Up1,
        //     Up2,
        //     Up3,
        // }
        //
        // motors.insert(
        //     MotorIds::Right,
        //     Motor {
        //         position: vec3a(1.0, 1.0, 0.0).normalize(),
        //         orientation: vec3a(0.0, 1.0, 0.0),
        //     },
        // );
        //
        // motors.insert(
        //     MotorIds::Left,
        //     Motor {
        //         position: vec3a(-1.0, 1.0, 0.0).normalize(),
        //         orientation: vec3a(0.0, 1.0, 0.0),
        //     },
        // );
        //
        // motors.insert(
        //     MotorIds::Lateral,
        //     Motor {
        //         position: vec3a(0.0, 0.0, 0.0),
        //         orientation: vec3a(1.0, 0.0, 0.0),
        //     },
        // );
        //
        // motors.insert(
        //     MotorIds::Up1,
        //     Motor {
        //         position: vec3a(1.0, 1.0, 0.0).normalize() * 2.0,
        //         orientation: vec3a(0.0, 0.0, 1.0),
        //     },
        // );
        //
        // motors.insert(
        //     MotorIds::Up2,
        //     Motor {
        //         position: vec3a(-1.0, 1.0, 0.0).normalize() * 2.0,
        //         orientation: vec3a(0.0, 0.0, 1.0),
        //     },
        // );
        //
        // motors.insert(
        //     MotorIds::Up3,
        //     Motor {
        //         position: vec3a(0.0, -1.0, 0.0).normalize() * 2.0,
        //         orientation: vec3a(0.0, 0.0, 1.0),
        //     },
        // );
        //
        // let motor_config = MotorConfig::new_raw(motors);

        let movement = Movement {
            force: vec3a(0.6, 0.0, 0.3),
            torque: vec3a(0.2, 0.1, 0.3),
        };

        let start = Instant::now();
        let motor_cmds = reverse::reverse_solve(movement, &motor_config, &motor_data);
        let elapsed = start.elapsed();

        println!("motor_cmds: {motor_cmds:#?} in {}us", elapsed.as_micros());

        let actual_movement = forward::forward_solve(
            &motor_config,
            &motor_cmds
                .iter()
                .map(|(id, data)| (*id, data.force))
                .collect(),
        );

        assert_eq!(movement, actual_movement);
    }

    #[bench]
    fn bench_reverse_solver_x3d(b: &mut Bencher) {
        let seed_motor = Motor {
            position: vec3a(0.3, 0.5, 0.4).normalize(),
            orientation: vec_from_angles(60.0, 40.0),
        };

        let motor_data =
            motor_preformance::read_motor_data("../robot/motor_data.csv").expect("Read motor data");
        let motor_config = MotorConfig::<X3dMotorId>::new(seed_motor);

        let movement = Movement {
            force: vec3a(0.6, 0.0, 0.3),
            torque: vec3a(0.2, 0.1, 0.3),
        };

        b.iter(|| reverse::reverse_solve(movement, &motor_config, &motor_data));
    }

    #[bench]
    fn bench_reverse_solver_blue_rov(b: &mut Bencher) {
        let lateral = Motor {
            position: vec3a(1.0, 1.0, 0.0),
            orientation: vec3a(-1.0, 1.0, 0.0).normalize(),
        };
        let vertical = Motor {
            position: vec3a(1.0, 1.0, 0.0),
            orientation: vec3a(0.0, 0.0, 1.0).normalize(),
        };

        let motor_data =
            motor_preformance::read_motor_data("../robot/motor_data.csv").expect("Read motor data");
        let motor_config = MotorConfig::<HeavyMotorId>::new(lateral, vertical);

        let movement = Movement {
            force: vec3a(0.6, 0.0, 0.3),
            torque: vec3a(0.2, 0.1, 0.3),
        };

        b.iter(|| reverse::reverse_solve(movement, &motor_config, &motor_data));
    }
}
