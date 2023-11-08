use glam::{vec3, Vec3};

// https://stackoverflow.com/questions/30011741/3d-vector-defined-by-2-angles
pub fn vec_from_angles(angle_xy: f32, angle_yz: f32) -> Vec3 {
    let x = angle_xy.cos() * angle_yz.cos();
    let y = angle_xy.sin() * angle_yz.cos();
    let z = angle_yz.sin();

    vec3(x, y, z)
}

#[derive(Clone, Copy, Debug)]
pub enum VectorTransform {
    ReflectXY,
    ReflectYZ,
    ReflectXZ,
}

impl VectorTransform {
    pub fn transform(&self, vec: Vec3) -> Vec3 {
        let Vec3 { x, y, z } = vec;

        match self {
            VectorTransform::ReflectXY => vec3(x, y, -z),
            VectorTransform::ReflectYZ => vec3(-x, y, z),
            VectorTransform::ReflectXZ => vec3(x, -y, z),
        }
    }
}
