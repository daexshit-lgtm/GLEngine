use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};
use crate::modules::render_3d::aabb::Aabb;

pub struct Transform {
    matrix:           Mat4,
    pub matrix_array: [[f32; 4]; 4],
}

impl Transform {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        let matrix = Mat4::from_translation(Vec3::new(x, y, z));
        Self { matrix, matrix_array: matrix.to_cols_array_2d() }
    }

    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.matrix.w_axis = Vec4::new(x, y, z, 1.0);
    }

    /// Sphere-based frustum cull. Fast but can over-draw at oblique angles.
    #[allow(unused)]
    pub fn frustum_cull_sphere(&self, planes: &[Vec4; 6], bounds: &Aabb) -> bool {
        let center = self.matrix.w_axis.xyz() + bounds.center;
        planes.iter().all(|p| p.xyz().dot(center) + p.w >= -bounds.radius)
    }

    /// AABB frustum cull (accurate for boxes).
    pub fn frustum_cull(&self, planes: &[Vec4; 6], bounds: &Aabb) -> bool {
        let center = self.matrix.w_axis.xyz() + bounds.center;
        let he     = bounds.half_extents;
        planes.iter().all(|p| {
            let n = p.xyz();
            let r = he.x * n.x.abs() + he.y * n.y.abs() + he.z * n.z.abs();
            n.dot(center) + p.w >= -r
        })
    }
}
