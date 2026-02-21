//! Camera System
//!
//! Implements perspective/orthographic camera with orbit, turntable,
//! walkthrough, animation, and named viewpoint support.

use glam::{Mat4, Vec3};

/// A saved camera viewpoint (position, target, settings).
#[derive(Debug, Clone)]
pub struct CameraViewpoint {
    pub name: String,
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub orthographic: bool,
}

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

    // -- Orthographic projection --
    /// When true, use orthographic projection instead of perspective
    orthographic: bool,

    // -- Walkthrough (first-person) mode --
    /// When true, camera is in first-person walkthrough mode
    walkthrough_mode: bool,

    // -- Turntable orbit mode --
    /// When true, orbit is constrained to turntable (Y-up) rotation
    turntable_mode: bool,

    // -- Scene scale (set from fit_to_bounds, used for speed floors) --
    /// Diagonal size of the loaded scene. Used to set minimum pan/zoom speeds
    /// so movement never stalls regardless of camera-to-target distance.
    scene_scale: f32,

    // -- Smooth camera animation --
    animation_active: bool,
    animation_start_position: Vec3,
    animation_start_target: Vec3,
    animation_end_position: Vec3,
    animation_end_target: Vec3,
    animation_progress: f32,
    animation_duration: f32,
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
            orthographic: false,
            walkthrough_mode: false,
            turntable_mode: true,
            scene_scale: 100.0, // sensible default, updated on fit_to_bounds
            animation_active: false,
            animation_start_position: Vec3::ZERO,
            animation_start_target: Vec3::ZERO,
            animation_end_position: Vec3::ZERO,
            animation_end_target: Vec3::ZERO,
            animation_progress: 0.0,
            animation_duration: 0.0,
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

    /// Get camera target as array
    pub fn target(&self) -> [f32; 3] {
        self.target.to_array()
    }

    /// Set aspect ratio
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    /// Set near and far clipping planes (auto-adjusted when fitting to bounds)
    pub fn set_clip_planes(&mut self, near: f32, far: f32) {
        self.near = near;
        self.far = far;
    }

    /// Get view matrix (transforms world space to camera space)
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Get projection matrix (perspective or orthographic)
    pub fn projection_matrix(&self) -> Mat4 {
        if self.orthographic {
            let distance = (self.position - self.target).length();
            let half_height = distance * (self.fov / 2.0).to_radians().tan();
            let half_width = half_height * self.aspect_ratio;
            Mat4::orthographic_rh(
                -half_width,
                half_width,
                -half_height,
                half_height,
                self.near,
                self.far,
            )
        } else {
            Mat4::perspective_rh(
                self.fov.to_radians(),
                self.aspect_ratio,
                self.near,
                self.far,
            )
        }
    }

    /// Get combined view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    // ----------------------------------------------------------------
    // Orthographic projection
    // ----------------------------------------------------------------

    /// Enable or disable orthographic projection
    pub fn set_orthographic(&mut self, enabled: bool) {
        self.orthographic = enabled;
    }

    /// Check if orthographic projection is active
    pub fn is_orthographic(&self) -> bool {
        self.orthographic
    }

    // ----------------------------------------------------------------
    // Orbit
    // ----------------------------------------------------------------

    /// Orbit around target (rotate camera position).
    /// If turntable mode is active, delegates to `turntable_orbit`.
    /// Orbit speed scales with distance for consistent feel at any zoom level.
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        if self.turntable_mode {
            self.turntable_orbit(delta_x, delta_y);
            return;
        }

        let radius = (self.position - self.target).length();
        let mut theta = (self.position.z - self.target.z).atan2(self.position.x - self.target.x);
        let mut phi =
            ((self.position.y - self.target.y) / radius).clamp(-1.0, 1.0).acos();

        let speed = 0.003 + 0.002 * (radius * 0.001).atan();
        theta -= delta_x * speed;
        phi = (phi - delta_y * speed).clamp(0.1, std::f32::consts::PI - 0.1);

        self.position.x = self.target.x + radius * phi.sin() * theta.cos();
        self.position.y = self.target.y + radius * phi.cos();
        self.position.z = self.target.z + radius * phi.sin() * theta.sin();
    }

    // ----------------------------------------------------------------
    // Turntable orbit (constrained Y-up rotation)
    // ----------------------------------------------------------------

    /// Enable or disable turntable orbit mode
    pub fn set_turntable_mode(&mut self, enabled: bool) {
        self.turntable_mode = enabled;
    }

    /// Check if turntable mode is active
    pub fn is_turntable_mode(&self) -> bool {
        self.turntable_mode
    }

    /// Turntable orbit: rotates around the world Y axis with clamped pitch.
    /// `delta_x` controls azimuth (yaw), `delta_y` controls elevation (pitch).
    /// Orbit speed scales with distance: slower when zoomed in, faster when zoomed out.
    pub fn turntable_orbit(&mut self, delta_x: f32, delta_y: f32) {
        let radius = (self.position - self.target).length();
        let mut theta = (self.position.z - self.target.z).atan2(self.position.x - self.target.x);

        // Compute current elevation angle (pitch) from Y axis
        let current_elevation = ((self.position.y - self.target.y) / radius).clamp(-1.0, 1.0).asin();

        // Scale orbit speed with distance: use atan so it's bounded and feels natural.
        // At very close range the sensitivity is low, at far range it's higher.
        // Base: 0.003 at distance=1, approaches ~0.005 at large distances.
        let speed = 0.003 + 0.002 * (radius * 0.001).atan();

        // Apply deltas
        theta -= delta_x * speed;
        // Clamp pitch to +/- 85 degrees to avoid gimbal lock / flipping
        let max_pitch: f32 = 85.0_f32.to_radians();
        let new_elevation = (current_elevation + delta_y * speed).clamp(-max_pitch, max_pitch);

        let cos_elev = new_elevation.cos();
        self.position.x = self.target.x + radius * cos_elev * theta.cos();
        self.position.y = self.target.y + radius * new_elevation.sin();
        self.position.z = self.target.z + radius * cos_elev * theta.sin();

        // Keep up vector as world Y
        self.up = Vec3::Y;
    }

    // ----------------------------------------------------------------
    // Pan & Zoom
    // ----------------------------------------------------------------

    /// Pan camera (move target and position together).
    /// Speed uses the larger of distance-based and scene-scale-based values,
    /// so panning is always usable — even after flying deep inside a building.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let forward = (self.target - self.position).normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward);

        let distance = (self.position - self.target).length();
        // Floor derived from scene diagonal — ~0.5% of scene per full-drag
        let min_speed = self.scene_scale * 0.00005;
        let speed = (distance * 0.001).max(min_speed);
        let offset = right * delta_x * speed + up * delta_y * speed;

        self.position += offset;
        self.target += offset;
    }

    /// Zoom in/out — constant-speed fly along the view direction.
    /// Speed is derived from scene scale so it feels the same at any distance.
    /// When the camera reaches the orbit target, the target is pushed forward
    /// so you fly straight through walls and into buildings.
    pub fn zoom(&mut self, delta: f32) {
        let direction = (self.target - self.position).normalize();
        let distance = (self.position - self.target).length();

        // Proportional at long range (responsive overview), constant floor at
        // close range (never stalls). Floor = 0.1% of scene diagonal so it
        // kicks in when you're roughly 1/3 of the way in.
        let speed = (distance * 0.002).max(self.scene_scale * 0.001);
        let move_amount = delta * speed;

        // Always move camera along the view direction
        self.position += direction * move_amount;

        // Check if camera passed through or is very close to the target
        let new_vec = self.target - self.position;
        let still_ahead = new_vec.dot(direction) > 0.0;
        let min_target_dist = self.scene_scale * 0.02;

        if !still_ahead || new_vec.length() < min_target_dist {
            // Reposition target ahead of camera so orbit center follows us
            self.target = self.position + direction * distance.max(min_target_dist);
        }
    }

    /// Fit view to bounding box
    pub fn fit_to_bounds(&mut self, min: Vec3, max: Vec3) {
        let center = (min + max) * 0.5;
        let size = (max - min).length();

        self.target = center;
        self.position = center + Vec3::new(1.0, 1.0, 1.0).normalize() * size * 1.5;
        self.scene_scale = size;
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

    // ----------------------------------------------------------------
    // Walkthrough (first-person) mode
    // ----------------------------------------------------------------

    /// Enable or disable first-person walkthrough mode.
    /// On enter, keeps current position; target is placed 1 unit in front.
    pub fn set_walkthrough_mode(&mut self, enabled: bool) {
        self.walkthrough_mode = enabled;
        if enabled {
            // Compute current forward direction, keep position, put target 1 unit ahead
            let forward = (self.target - self.position).normalize();
            self.target = self.position + forward;
            self.up = Vec3::Y;
        }
    }

    /// Check if walkthrough mode is active
    pub fn is_walkthrough_mode(&self) -> bool {
        self.walkthrough_mode
    }

    /// Move camera forward/backward along the view direction (walkthrough)
    pub fn walk_forward(&mut self, amount: f32) {
        let forward = (self.target - self.position).normalize();
        let offset = forward * amount;
        self.position += offset;
        self.target += offset;
    }

    /// Strafe camera left/right (walkthrough)
    pub fn walk_right(&mut self, amount: f32) {
        let forward = (self.target - self.position).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let offset = right * amount;
        self.position += offset;
        self.target += offset;
    }

    /// Move camera up/down (walkthrough)
    pub fn walk_up(&mut self, amount: f32) {
        let offset = Vec3::Y * amount;
        self.position += offset;
        self.target += offset;
    }

    /// Fly-mode movement: constant speed based on scene scale.
    /// `forward`/`right`/`up` are raw gesture deltas (e.g. pinch scale delta,
    /// two-finger pan dx/dy). Speed is derived from scene_scale so it feels
    /// consistent regardless of where the camera is.
    pub fn fly_move(&mut self, forward: f32, right: f32, up: f32) {
        let speed = self.scene_scale * 0.001;
        let dir = (self.target - self.position).normalize();
        let right_vec = dir.cross(Vec3::Y).normalize();

        let offset = dir * forward * speed + right_vec * right * speed + Vec3::Y * up * speed;
        self.position += offset;
        self.target += offset;
    }

    /// Set the orbit center (target) to a specific world-space point,
    /// keeping camera position unchanged.
    pub fn set_orbit_target(&mut self, target: Vec3) {
        self.target = target;
    }

    /// Rotate the view direction (yaw/pitch) without orbiting around target (walkthrough).
    /// `delta_x` rotates yaw (left/right), `delta_y` rotates pitch (up/down).
    pub fn look_around(&mut self, delta_x: f32, delta_y: f32) {
        let forward = (self.target - self.position).normalize();

        // Yaw: rotate around world Y axis
        let yaw = -delta_x * 0.005;
        let cos_yaw = yaw.cos();
        let sin_yaw = yaw.sin();
        let rotated_x = forward.x * cos_yaw - forward.z * sin_yaw;
        let rotated_z = forward.x * sin_yaw + forward.z * cos_yaw;
        let mut new_forward = Vec3::new(rotated_x, forward.y, rotated_z);

        // Pitch: rotate around the local right axis, clamped to avoid flipping
        let right = new_forward.cross(Vec3::Y).normalize();
        let pitch = -delta_y * 0.005;
        let current_pitch = new_forward.y.asin();
        let max_pitch: f32 = 85.0_f32.to_radians();
        let clamped_pitch = (current_pitch + pitch).clamp(-max_pitch, max_pitch);
        let actual_pitch = clamped_pitch - current_pitch;

        let cos_p = actual_pitch.cos();
        let sin_p = actual_pitch.sin();
        // Rodrigues' rotation formula around `right`
        new_forward = new_forward * cos_p
            + right.cross(new_forward) * sin_p
            + right * right.dot(new_forward) * (1.0 - cos_p);

        new_forward = new_forward.normalize();
        self.target = self.position + new_forward;
    }

    // ----------------------------------------------------------------
    // Named viewpoints (save / restore)
    // ----------------------------------------------------------------

    /// Capture the current camera state as a named viewpoint
    pub fn save_viewpoint(&self, name: String) -> CameraViewpoint {
        CameraViewpoint {
            name,
            position: self.position,
            target: self.target,
            up: self.up,
            fov: self.fov,
            orthographic: self.orthographic,
        }
    }

    /// Restore camera state from a viewpoint
    pub fn restore_viewpoint(&mut self, vp: &CameraViewpoint) {
        self.position = vp.position;
        self.target = vp.target;
        self.up = vp.up;
        self.fov = vp.fov;
        self.orthographic = vp.orthographic;
    }

    // ----------------------------------------------------------------
    // Smooth animated camera transitions
    // ----------------------------------------------------------------

    /// Begin a smooth transition to a new position/target over `duration` seconds.
    pub fn start_transition(
        &mut self,
        target_position: Vec3,
        target_target: Vec3,
        duration: f32,
    ) {
        self.animation_start_position = self.position;
        self.animation_start_target = self.target;
        self.animation_end_position = target_position;
        self.animation_end_target = target_target;
        self.animation_progress = 0.0;
        self.animation_duration = duration.max(0.001); // avoid division by zero
        self.animation_active = true;
    }

    /// Advance the animation by `delta_time` seconds.
    /// Returns `true` if the animation is still active after this tick.
    pub fn tick_animation(&mut self, delta_time: f32) -> bool {
        if !self.animation_active {
            return false;
        }

        self.animation_progress += delta_time / self.animation_duration;
        if self.animation_progress >= 1.0 {
            self.animation_progress = 1.0;
            self.animation_active = false;
        }

        // Smoothstep easing: t * t * (3 - 2t)
        let t = self.animation_progress;
        let smooth = t * t * (3.0 - 2.0 * t);

        self.position = self.animation_start_position.lerp(self.animation_end_position, smooth);
        self.target = self.animation_start_target.lerp(self.animation_end_target, smooth);

        self.animation_active
    }

    /// Check if a camera animation is currently in progress
    pub fn is_animating(&self) -> bool {
        self.animation_active
    }

    // ----------------------------------------------------------------
    // Ray casting
    // ----------------------------------------------------------------

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

/// Frustum extracted from a view-projection matrix for culling.
///
/// Uses the Gribb-Hartmann method: each plane [a, b, c, d] represents
/// the half-space ax + by + cz + d >= 0.
pub struct Frustum {
    planes: [[f32; 4]; 6],
}

impl Frustum {
    /// Extract 6 frustum planes from a view-projection matrix.
    pub fn from_view_projection(vp: &Mat4) -> Self {
        let m = vp.to_cols_array_2d(); // m[col][row]
        let mut planes = [[0.0f32; 4]; 6];

        for c in 0..4 {
            planes[0][c] = m[c][3] + m[c][0]; // Left
            planes[1][c] = m[c][3] - m[c][0]; // Right
            planes[2][c] = m[c][3] + m[c][1]; // Bottom
            planes[3][c] = m[c][3] - m[c][1]; // Top
            planes[4][c] = m[c][2];            // Near (z in [0,1])
            planes[5][c] = m[c][3] - m[c][2]; // Far
        }

        // Normalize each plane
        for plane in &mut planes {
            let len = (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
            if len > 0.0001 {
                for v in plane.iter_mut() {
                    *v /= len;
                }
            }
        }

        Frustum { planes }
    }

    /// Get the 6 frustum planes as [a, b, c, d] equations (ax + by + cz + d = 0).
    pub fn planes(&self) -> [[f32; 4]; 6] {
        self.planes
    }

    /// Test if an AABB is at least partially inside the frustum.
    /// Uses the p-vertex (positive vertex) optimization for fast rejection.
    pub fn intersects_aabb(&self, min: [f32; 3], max: [f32; 3]) -> bool {
        for plane in &self.planes {
            // p-vertex: the AABB corner most in the direction of the plane normal
            let px = if plane[0] >= 0.0 { max[0] } else { min[0] };
            let py = if plane[1] >= 0.0 { max[1] } else { min[1] };
            let pz = if plane[2] >= 0.0 { max[2] } else { min[2] };

            // If p-vertex is behind the plane → AABB is fully outside this plane
            if plane[0] * px + plane[1] * py + plane[2] * pz + plane[3] < 0.0 {
                return false;
            }
        }
        true
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
