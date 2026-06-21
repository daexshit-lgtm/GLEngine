use glam::{Mat4, Vec2, Vec3, Vec4};

pub struct Camera {
    position:    Vec3,
    yaw:         f32,
    pitch:       f32,
    near:        f32,
    far:         f32,
    fov:         f32,
    sensitivity: f32,
    projection:  Mat4,
    forward:     Vec3,
    up:          Vec3,
    vp:          Mat4,
    pub speed:       f32,
    pub arr_vp:      [[f32; 4]; 4],
    pub dimensions:  Vec2,
    pub planes:      [Vec4; 6],
}

impl Camera {
    pub fn new(dimensions: &Vec2) -> Self {
        let mut cam = Self {
            yaw:         -std::f32::consts::FRAC_PI_2,
            pitch:       0.0,
            near:        0.1,
            far:         100.1,
            fov:         1.1170,
            position:    Vec3::new(0.0, 0.0, 3.0),
            forward:     Vec3::ZERO,
            up:          Vec3::Y,
            speed:       0.1,
            sensitivity: 0.001_745,
            projection:  Mat4::IDENTITY,
            vp:          Mat4::IDENTITY,
            dimensions:  *dimensions,
            planes:      [Vec4::ZERO; 6],
            arr_vp:      [[0.0; 4]; 4],
        };
        cam.sync_forward();
        cam.update_dimensions();
        cam
    }

    // ── Sync helpers ──────────────────────────────────────────────────────────

    fn sync_forward(&mut self) {
        self.forward = Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        ).normalize();
    }

    fn sync_projection(&mut self) {
        self.projection = Mat4::perspective_rh(
            self.fov,
            self.dimensions.x / self.dimensions.y,
            self.near,
            self.far,
        );
    }

    /// Gribb-Hartmann frustum plane extraction.
    fn sync_planes(&mut self) {
        let r = self.vp.row(3);
        self.planes = [
            r + self.vp.row(0), // left
            r - self.vp.row(0), // right
            r + self.vp.row(1), // bottom
            r - self.vp.row(1), // top
            r + self.vp.row(2), // near
            r - self.vp.row(2), // far
        ];
    }

    fn sync_vp(&mut self) {
        self.vp     = self.projection * Mat4::look_to_rh(self.position, self.forward, self.up);
        self.arr_vp = self.vp.to_cols_array_2d();
        self.sync_planes();
    }

    // ── Public API ────────────────────────────────────────────────────────────

    pub fn update_dimensions(&mut self) {
        self.sync_projection();
        self.sync_vp();
    }

    pub fn set_far(&mut self, far: f32) {
        self.far = far;
        self.update_dimensions();
    }

    pub fn move_by(&mut self, direction: Vec3) {
        let right = {
            let r = self.forward.cross(self.up);
            if r.length_squared() > 0.0001 { r }
            else { Vec3::new(-self.yaw.sin(), 0.0, self.yaw.cos()) }
        }.normalize();
        let up    = right.cross(self.forward).normalize();
        let delta = self.forward * direction.x + up * direction.y + right * direction.z;
        if delta.length_squared() > 0.0 {
            self.position += delta.normalize() * self.speed;
            self.sync_vp();
        }
    }

    pub fn rotate_by(&mut self, mut delta: Vec2) {
        delta *= self.sensitivity;
        self.yaw   += delta.x;
        self.pitch  = (self.pitch - delta.y).clamp(-1.569, 1.569);
        self.sync_forward();
        self.sync_vp();
    }
}
