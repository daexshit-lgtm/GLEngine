use glam::{Mat4, Vec2, Vec3, Vec4};

pub struct Camera {
    position:        Vec3,
    yaw:             f32,
    pitch:           f32,
    near:            f32,
    far:             f32,
    fov:             f32,
    sensitivity:     f32,
    projection:      Mat4,
    forward:         Vec3,
    up:              Vec3,
    vp:              Mat4,
    pub speed:       f32,
    pub arr_vp:      [[f32; 4]; 4],
    pub dimensions:  Vec2,
    pub planes:      [Vec4; 6],
}

impl Camera {
    /// Initializes + consists on the UniformBuffer(GPU) cache fastening the rendering process
    pub fn new(dimensions: &Vec2) -> Self {
        let mut r = Self {
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: 0.0,
            near: 0.1,
            far: 100.1,
            fov: 1.1170,
            position: Vec3::new(0.0, 0.0, 3.0),
            forward: Vec3::ZERO,
            up: Vec3::Y,
            speed: 0.1,
            sensitivity: 0.001745,
            projection: Mat4::IDENTITY,
            vp: Mat4::IDENTITY,
            dimensions: *dimensions,
            planes: [Vec4::ZERO; 6],
            arr_vp: [[0.0; 4]; 4],
        };
        r.sync_forward();
        r.update_dimensions();
        r
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Direction it is looking at
    fn sync_forward(&mut self) {
        let yaw = &self.yaw;
        let pitch = &self.pitch;
        self.forward = Vec3::new(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos()).normalize();
    }

    /// Transformation
    fn sync_projection(&mut self) {
        let dimensions = &self.dimensions;
        self.projection = Mat4::perspective_rh(self.fov,  dimensions.x / dimensions.y, self.near, self.far);
    }

    /// Gribb-Hartmann frustum plane extraction.
    fn sync_planes(&mut self) {
        let vp = &self.vp;
        let r = vp.row(3);
        self.planes = [
            r + vp.row(0), // left
            r - vp.row(0), // right
            r + vp.row(1), // bottom
            r - vp.row(1), // top
            r + vp.row(2), // near
            r - vp.row(2), // far
        ];
    }

    /// Projection * View
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
        let fwd  = self.forward;
        let yaw   = self.yaw;
        // Right Direction
        let right = {
            let r = fwd.cross(self.up);
            if r.length_squared() > 0.0001 { r }
            else { Vec3::new(-yaw.sin(), 0.0, yaw.cos()) }
        }.normalize();
        // Upside Direction
        let up  = right.cross(fwd).normalize();
        // Moving towards: forward + up + right
        let delta = fwd * direction.x + up * direction.y + right * direction.z;
        if delta.length_squared() > 0.0 {                    // Can't be NaN
            self.position += delta.normalize() * self.speed; // += <=1.0f * speed
            self.sync_vp();
        }
    }

    // yaw += x, pitch -= y
    pub fn rotate_by(&mut self, mut direction: Vec2) {
        direction *= self.sensitivity;
        // X
        self.yaw   += direction.x;
        // Y (Limited)
        self.pitch  = (self.pitch - direction.y).clamp(-1.569, 1.569);
        // Update
        self.sync_forward();
        self.sync_vp();
    }
}