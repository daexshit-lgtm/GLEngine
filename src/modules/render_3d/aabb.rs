use glam::Vec3;

// ── AABB ──────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy)]
#[allow(unused)]
/// Axis-Aligned Bounding Box
pub struct Aabb {
    pub min:    Vec3,
    pub max:    Vec3,
    pub center: Vec3,
    pub radius: f32,
    pub half_extents: Vec3,
}

impl Aabb {
    pub fn from_positions(iter: impl Iterator<Item = [f32; 3]>) -> Self {
        // Starts at the highest limit
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for p in iter {
            min.x = min.x.min(p[0]);
            min.y = min.y.min(p[1]);
            min.z = min.z.min(p[2]);
            max.x = max.x.max(p[0]);
            max.y = max.y.max(p[1]);
            max.z = max.z.max(p[2]);
        }
        let center = (min + max) * 0.5;      // Center Position
        let half_extents = (max - min) * 0.5;// Half Size
        let radius = center.distance(max);    // O
        Aabb { min, max, center, radius, half_extents }
    }
}