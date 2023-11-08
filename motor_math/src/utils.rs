use glam::{vec3a, Vec3A};

// https://stackoverflow.com/questions/30011741/3d-vector-defined-by-2-angles
pub fn vec_from_angles(angle_xy: f32, angle_yz: f32) -> Vec3A {
    let x = angle_xy.cos() * angle_yz.cos();
    let y = angle_xy.sin() * angle_yz.cos();
    let z = angle_yz.sin();

    vec3a(x, y, z)
}

#[derive(Clone, Copy, Debug)]
pub enum VectorTransform {
    ReflectXY,
    ReflectYZ,
    ReflectXZ,
}

impl VectorTransform {
    pub fn transform(&self, vec: Vec3A) -> Vec3A {
        match self {
            VectorTransform::ReflectXY => vec3a(vec.x, vec.y, -vec.z),
            VectorTransform::ReflectYZ => vec3a(-vec.x, vec.y, vec.z),
            VectorTransform::ReflectXZ => vec3a(vec.x, -vec.y, vec.z),
        }
    }
}
