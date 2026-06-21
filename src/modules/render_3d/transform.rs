use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};

use crate::modules::render_3d::aabb::Aabb;

pub struct Transform {
    matrix: Mat4,
    pub matrix_array: [[f32; 4]; 4],
}

impl Transform {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        let matrix = Mat4::from_translation(Vec3::new(x, y, z)); // Matrix by position
        Self { matrix, matrix_array: matrix.to_cols_array_2d() }
    }

    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.matrix.w_axis = Vec4::new(x, y, z, 1.0);
    }

    /// Returns true if it intersects all 6 Frustum planes
    #[allow(unused)]
    pub fn frustum_cull_sphere(&self, planes: &[Vec4; 6], bounds: &Aabb) -> bool {
        let r = &bounds.radius;
        let center = self.matrix.w_axis.xyz() + bounds.center;
        planes.iter().all(|p| p.xyz().dot(center) + p.w >= -r) // Dot Product: Looking at it within a radius
    }

    /// Returns true if the AABB intersects or is inside all 6 Frustum planes
    /// 
    /// Accurate for boxes
    pub fn frustum_cull(&self, planes: &[Vec4; 6], bounds: &Aabb) -> bool {
        let world_center = self.matrix.w_axis.xyz() + bounds.center;
        let half_extents = bounds.half_extents;
        planes.iter().all(|p| {
            let normal = p.xyz();
            // It's a lil heavier with all 4 points
            let r = half_extents.x * normal.x.abs() 
                + half_extents.y * normal.y.abs() 
                + half_extents.z * normal.z.abs();
            normal.dot(world_center) + p.w >= -r
        })
    }
}
