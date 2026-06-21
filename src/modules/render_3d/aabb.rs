use glam::Vec3;

/// Axis-Aligned Bounding Box
#[derive(Clone, Copy)]
#[allow(unused)]
pub struct Aabb {
    pub min:          Vec3,
    pub max:          Vec3,
    pub center:       Vec3,
    pub radius:       f32,
    pub half_extents: Vec3,
}

impl Aabb {
    pub fn from_positions(iter: impl Iterator<Item = [f32; 3]>) -> Self {
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for p in iter {
            let v = Vec3::from(p);
            min = min.min(v);
            max = max.max(v);
        }
        let center       = (min + max) * 0.5;
        let half_extents = (max - min) * 0.5;
        let radius       = center.distance(max);
        Aabb { min, max, center, radius, half_extents }
    }
}
