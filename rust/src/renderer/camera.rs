//! Camera System
//!
//! Implements perspective camera with orbit controls.

use glam::{Mat4, Vec3};

/// Camera for 3D scene viewing
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space
    position: Vec3,
    /// Point the camera is looking at
    target: Vec3,
    /// Up vector (usually [0, 1, 0])
    up: Vec3,
    /// Field of view in degrees
    fov: f32,
    /// Aspect ratio (width / height)
    aspect_ratio: f32,
    /// Near clipping plane
    near: f32,
    /// Far clipping plane
    far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(10.0, 10.0, 10.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: 45.0,
            aspect_ratio: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}

impl Camera {
    /// Create a new camera
    pub fn new(position: Vec3, target: Vec3) -> Self {
        Self {
            position,
            target,
            ..Default::default()
        }
    }

    /// Set camera position
    pub fn set_position(&mut self, position: [f32; 3]) {
        self.position = Vec3::from_array(position);
    }

    /// Set camera target
    pub fn set_target(&mut self, target: [f32; 3]) {
        self.target = Vec3::from_array(target);
    }

    /// Get camera position as array
    pub fn position(&self) -> [f32; 3] {
        self.position.to_array()
    }

    /// Set aspect ratio
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    /// Get view matrix (transforms world space to camera space)
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Get projection matrix (perspective)
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov.to_radians(),
            self.aspect_ratio,
            self.near,
            self.far,
        )
    }

    /// Get combined view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Orbit around target (rotate camera position)
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let radius = (self.position - self.target).length();
        let mut theta = (self.position.z - self.target.z).atan2(self.position.x - self.target.x);
        let mut phi =
            ((self.position.y - self.target.y) / radius).clamp(-1.0, 1.0).acos();

        theta -= delta_x * 0.01;
        phi = (phi - delta_y * 0.01).clamp(0.1, std::f32::consts::PI - 0.1);

        self.position.x = self.target.x + radius * phi.sin() * theta.cos();
        self.position.y = self.target.y + radius * phi.cos();
        self.position.z = self.target.z + radius * phi.sin() * theta.sin();
    }

    /// Pan camera (move target and position together)
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let forward = (self.target - self.position).normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward);

        let offset = right * delta_x * 0.01 + up * delta_y * 0.01;

        self.position += offset;
        self.target += offset;
    }

    /// Zoom in/out (move camera closer/farther from target)
    pub fn zoom(&mut self, delta: f32) {
        let direction = (self.target - self.position).normalize();
        let distance = (self.position - self.target).length();
        let new_distance = (distance - delta * 0.1).max(0.1);

        self.position = self.target - direction * new_distance;
    }

    /// Fit view to bounding box
    pub fn fit_to_bounds(&mut self, min: Vec3, max: Vec3) {
        let center = (min + max) * 0.5;
        let size = (max - min).length();

        self.target = center;
        self.position = center + Vec3::new(1.0, 1.0, 1.0).normalize() * size * 1.5;
    }

    /// Set camera distance from target (preserving direction)
    pub fn set_distance(&mut self, distance: f32) {
        let direction = (self.position - self.target).normalize_or_zero();
        if direction.length_squared() < 0.001 {
            // If camera is at target, use a default direction
            self.position = self.target + Vec3::new(1.0, 1.0, 1.0).normalize() * distance;
        } else {
            self.position = self.target + direction * distance;
        }
    }

    /// Convert screen coordinates (0-1 range) to a world-space ray
    /// Returns (origin, direction)
    pub fn screen_to_ray(&self, screen_x: f32, screen_y: f32) -> (Vec3, Vec3) {
        // Convert to NDC (-1 to 1)
        let ndc_x = screen_x * 2.0 - 1.0;
        let ndc_y = 1.0 - screen_y * 2.0; // Flip Y

        // Get inverse view-projection matrix
        let inv_view_proj = self.view_projection_matrix().inverse();

        // Create ray in clip space and transform to world
        let near_point = inv_view_proj.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
        let far_point = inv_view_proj.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));

        let origin = self.position;
        let direction = (far_point - near_point).normalize();

        (origin, direction)
    }
}

/// Ray-AABB intersection test
/// Returns the distance to intersection, or None if no hit
pub fn ray_aabb_intersect(
    ray_origin: Vec3,
    ray_dir: Vec3,
    box_min: Vec3,
    box_max: Vec3,
) -> Option<f32> {
    let inv_dir = Vec3::new(
        1.0 / ray_dir.x,
        1.0 / ray_dir.y,
        1.0 / ray_dir.z,
    );

    let t1 = (box_min.x - ray_origin.x) * inv_dir.x;
    let t2 = (box_max.x - ray_origin.x) * inv_dir.x;
    let t3 = (box_min.y - ray_origin.y) * inv_dir.y;
    let t4 = (box_max.y - ray_origin.y) * inv_dir.y;
    let t5 = (box_min.z - ray_origin.z) * inv_dir.z;
    let t6 = (box_max.z - ray_origin.z) * inv_dir.z;

    let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
    let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

    if tmax < 0.0 || tmin > tmax {
        None
    } else {
        Some(if tmin < 0.0 { tmax } else { tmin })
    }
}
