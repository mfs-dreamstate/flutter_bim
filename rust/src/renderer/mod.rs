//! 3D Rendering Module
//!
//! High-performance 3D rendering using wgpu (WebGPU/Vulkan/Metal).
//! Handles scene rendering, camera management, and GPU resource management.

pub mod bvh;
pub mod camera;
pub mod gpu;
pub mod overlay;
pub mod pipeline;
pub mod scene;
pub mod texture_atlas;
pub mod vertex;

pub use bvh::BvhNode;
pub use camera::{Camera, Frustum, ray_aabb_intersect};
pub use gpu::GpuContext;
pub use overlay::DrawingOverlay;
pub use pipeline::{RenderMode, RenderPipeline};
pub use scene::{ElementDrawRange, SceneRenderer};
pub use vertex::{generate_test_cube, BoxVertex, InstanceData, Vertex};

/// Renderer state and configuration
pub struct Renderer {
    pub gpu: GpuContext,
    pub scene: Option<SceneRenderer>,
    pub camera: Camera,
    pub initialized: bool,
    /// Cached geometry centroid (average vertex position) for robust orbit targeting
    pub geometry_centroid: Option<[f32; 3]>,
}

impl Renderer {
    /// Create a new renderer
    pub fn new() -> Self {
        Self {
            gpu: GpuContext::new(),
            scene: None,
            camera: Camera::default(),
            initialized: false,
            geometry_centroid: None,
        }
    }

    /// Initialize the renderer
    pub async fn initialize(&mut self) -> Result<(), String> {
        self.gpu
            .initialize()
            .await
            .map_err(|e| format!("Failed to initialize GPU: {}", e))?;

        Ok(())
    }

    /// Initialize scene renderer with given dimensions
    pub fn init_scene(&mut self, width: u32, height: u32) -> Result<(), String> {
        let device = self.gpu.device().ok_or("GPU not initialized")?;
        let wireframe_supported = self.gpu.wireframe_supported();

        let mut scene = SceneRenderer::new(width, height);
        scene.initialize_with_features(device, wireframe_supported);

        // Upload test cube
        let (vertices, indices) = generate_test_cube();
        scene.upload_mesh(device, &vertices, &indices);

        self.scene = Some(scene);
        self.camera.set_aspect_ratio(width as f32 / height as f32);
        self.initialized = true;

        Ok(())
    }

    /// Render a frame and return pixel data as RGBA
    pub fn render_frame(&mut self) -> Result<Vec<u8>, String> {
        let device = self.gpu.device().ok_or("GPU not initialized")?;
        let queue = self.gpu.queue().ok_or("GPU queue not initialized")?;
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;

        let pixels = scene.render_frame(device, queue, &self.camera);
        Ok(pixels)
    }

    /// Update camera position/rotation
    pub fn update_camera(&mut self, position: [f32; 3], target: [f32; 3]) {
        self.camera.set_position(position);
        self.camera.set_target(target);
    }

    /// Orbit camera around target
    pub fn orbit_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.orbit(delta_x, delta_y);
    }

    /// Zoom camera
    pub fn zoom_camera(&mut self, delta: f32) {
        self.camera.zoom(delta);
    }

    /// Set interaction active state (FastNav: skip expensive post-processing during interaction)
    pub fn set_interaction_active(&mut self, active: bool) {
        if let Some(scene) = &mut self.scene {
            scene.set_interaction_active(active);
        }
    }

    /// Get scene bounds (min, max) in world coordinates, or None if no geometry loaded
    pub fn get_scene_bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let scene = self.scene.as_ref()?;
        if scene.element_draw_ranges.is_empty() {
            return None;
        }
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for range in &scene.element_draw_ranges {
            for i in 0..3 {
                min[i] = min[i].min(range.aabb_min[i]);
                max[i] = max[i].max(range.aabb_max[i]);
            }
        }
        Some((min, max))
    }

    /// Get the effective Y (height) range where most geometry lives.
    /// Uses element centroids at the 2nd and 98th percentile to exclude outliers,
    /// then adds 5% padding on each side.
    pub fn get_effective_height_range(&self) -> Option<(f32, f32)> {
        let scene = self.scene.as_ref()?;
        if scene.element_draw_ranges.is_empty() {
            return None;
        }
        // Collect Y centroids of all elements
        let mut y_centers: Vec<f32> = scene
            .element_draw_ranges
            .iter()
            .map(|r| (r.aabb_min[1] + r.aabb_max[1]) * 0.5)
            .collect();
        y_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = y_centers.len();
        if n == 0 {
            return None;
        }

        // 2nd and 98th percentile indices
        let lo = (n as f32 * 0.02) as usize;
        let hi = ((n as f32 * 0.98) as usize).min(n - 1);
        let min_y = y_centers[lo];
        let max_y = y_centers[hi];

        // Add 5% padding on each side
        let range = (max_y - min_y).max(1.0);
        let padding = range * 0.05;
        Some((min_y - padding, max_y + padding))
    }

    /// Get frame dimensions
    pub fn get_dimensions(&self) -> Option<(u32, u32)> {
        self.scene.as_ref().map(|s| (s.width, s.height))
    }

    /// Load mesh data from flat arrays (from BimModel::generate_meshes)
    pub fn load_mesh(
        &mut self,
        vertices: &[f32],
        normals: &[f32],
        colors: &[f32],
        indices: &[u32],
    ) -> Result<(), String> {
        let device = self.gpu.device().ok_or("GPU not initialized")?;
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;

        scene.upload_mesh_from_arrays(device, vertices, normals, colors, indices);
        Ok(())
    }

    /// Set per-element draw ranges for frustum culling (non-instanced fallback)
    pub fn set_element_draw_ranges(&mut self, ranges: Vec<ElementDrawRange>) -> Result<(), String> {
        let device = self.gpu.device().ok_or("GPU not initialized")?;
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;

        // Auto-initialize compute culling for models with many elements
        let element_count = ranges.len();
        if element_count > 0 && scene.compute_cull_auto {
            if scene.compute_cull_resources.is_none() {
                scene.init_compute_culling(device);
            }
            // Upload AABBs for compute culling
            let aabbs: Vec<[f32; 6]> = ranges
                .iter()
                .map(|r| [
                    r.aabb_min[0], r.aabb_min[1], r.aabb_min[2],
                    r.aabb_max[0], r.aabb_max[1], r.aabb_max[2],
                ])
                .collect();
            scene.upload_aabbs_for_compute(device, &aabbs);
        }

        scene.set_element_draw_ranges(ranges);
        Ok(())
    }

    /// Set per-instance data for GPU instancing (replaces mesh path for BIM models)
    pub fn set_instances(&mut self, instances: Vec<InstanceData>) -> Result<(), String> {
        let device = self.gpu.device().ok_or("GPU not initialized")?;
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        scene.set_instances(device, instances);
        Ok(())
    }

    /// Fit camera to bounding box, using cached centroid if available.
    pub fn fit_camera_to_bounds(&mut self, min: [f32; 3], max: [f32; 3]) {
        let centroid = self.geometry_centroid;
        self.fit_camera_impl(min, max, centroid);
    }

    /// Fit camera with an explicit centroid as orbit target (also caches it).
    pub fn fit_camera_to_bounds_with_centroid(
        &mut self,
        min: [f32; 3],
        max: [f32; 3],
        centroid: Option<[f32; 3]>,
    ) {
        if centroid.is_some() {
            self.geometry_centroid = centroid;
        }
        self.fit_camera_impl(min, max, centroid);
    }

    fn fit_camera_impl(
        &mut self,
        min: [f32; 3],
        max: [f32; 3],
        centroid: Option<[f32; 3]>,
    ) {
        // Use centroid as orbit target if provided, otherwise bounding box center
        let center = centroid.unwrap_or([
            (min[0] + max[0]) / 2.0,
            (min[1] + max[1]) / 2.0,
            (min[2] + max[2]) / 2.0,
        ]);

        let size = [
            max[0] - min[0],
            max[1] - min[1],
            max[2] - min[2],
        ];

        // Find the largest dimension
        let max_size = size[0].max(size[1]).max(size[2]);

        // Calculate camera distance (1.5x the max size, minimum of 10 units)
        let distance = (max_size * 1.5).max(10.0);

        // Set camera target to center
        self.camera.set_target(center);

        // Position camera from a sensible angle:
        // ~30 degree elevation, looking from front-right (positive X and Z)
        let elevation: f32 = 30.0_f32.to_radians();
        let azimuth: f32 = 45.0_f32.to_radians();
        let cam_x = center[0] + distance * elevation.cos() * azimuth.cos();
        let cam_y = center[1] + distance * elevation.sin();
        let cam_z = center[2] + distance * elevation.cos() * azimuth.sin();
        self.camera.set_position([cam_x, cam_y, cam_z]);

        // Adjust near/far planes to fit the model scale
        self.camera.set_clip_planes(
            (distance * 0.001).max(0.1),
            distance * 4.0,
        );
    }

    /// Set directional light direction (will be normalized)
    pub fn set_light_direction(&mut self, x: f32, y: f32, z: f32) -> Result<(), String> {
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        scene.set_light_direction(x, y, z);
        if let Some(queue) = self.gpu.queue() {
            scene.update_light(queue);
        }
        Ok(())
    }

    /// Set directional light color (RGB, 0.0-1.0)
    pub fn set_light_color(&mut self, r: f32, g: f32, b: f32) -> Result<(), String> {
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        scene.set_light_color(r, g, b);
        if let Some(queue) = self.gpu.queue() {
            scene.update_light(queue);
        }
        Ok(())
    }

    /// Set directional light intensity (0.0+)
    pub fn set_light_intensity(&mut self, intensity: f32) -> Result<(), String> {
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        scene.set_light_intensity(intensity);
        if let Some(queue) = self.gpu.queue() {
            scene.update_light(queue);
        }
        Ok(())
    }

    /// Set ambient light color (RGB, 0.0-1.0)
    pub fn set_ambient_color(&mut self, r: f32, g: f32, b: f32) -> Result<(), String> {
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        scene.set_ambient_color(r, g, b);
        if let Some(queue) = self.gpu.queue() {
            scene.update_light(queue);
        }
        Ok(())
    }

    /// Set the render mode (shaded or wireframe)
    pub fn set_render_mode(&mut self, mode: RenderMode) -> Result<(), String> {
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        scene.set_render_mode(mode);
        Ok(())
    }

    /// Get the current render mode
    pub fn get_render_mode(&self) -> Result<RenderMode, String> {
        let scene = self.scene.as_ref().ok_or("Scene not initialized")?;
        Ok(scene.get_render_mode())
    }

    /// Set the section plane for clipping geometry (backward compatible, sets plane index 0)
    /// plane: Option<(origin: [f32; 3], normal: [f32; 3])>
    /// None to disable clipping
    pub fn set_section_plane(&mut self, plane: Option<([f32; 3], [f32; 3])>) -> Result<(), String> {
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        scene.set_section_plane(plane);
        if let Some(queue) = self.gpu.queue() {
            scene.update_section_plane(queue);
        }
        Ok(())
    }

    /// Add a section plane, returns the plane index (0-5)
    pub fn add_section_plane(&mut self, origin: [f32; 3], normal: [f32; 3]) -> Result<usize, String> {
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        let index = scene.add_section_plane(origin, normal)
            .ok_or("Maximum of 6 section planes reached")?;
        if let Some(queue) = self.gpu.queue() {
            scene.update_section_plane(queue);
        }
        Ok(index)
    }

    /// Remove a section plane at the given index
    pub fn remove_section_plane(&mut self, index: usize) -> Result<(), String> {
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        if !scene.remove_section_plane(index) {
            return Err(format!("Invalid section plane index: {}", index));
        }
        if let Some(queue) = self.gpu.queue() {
            scene.update_section_plane(queue);
        }
        Ok(())
    }

    /// Set multiple section planes at once
    pub fn set_section_planes(&mut self, planes: Vec<([f32; 3], [f32; 3])>) -> Result<(), String> {
        if planes.len() > 6 {
            return Err("Maximum of 6 section planes supported".to_string());
        }
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        scene.set_section_planes(&planes);
        if let Some(queue) = self.gpu.queue() {
            scene.update_section_plane(queue);
        }
        Ok(())
    }

    /// Get the current number of active section planes
    pub fn get_section_plane_count(&self) -> Result<usize, String> {
        let scene = self.scene.as_ref().ok_or("Scene not initialized")?;
        Ok(scene.get_section_plane_count())
    }

    /// Clear all section planes
    pub fn clear_section_planes(&mut self) -> Result<(), String> {
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        scene.clear_section_planes();
        if let Some(queue) = self.gpu.queue() {
            scene.update_section_plane(queue);
        }
        Ok(())
    }

    /// Set section box for 6-plane clipping
    /// bounds: Option<(min: [f32; 3], max: [f32; 3])>
    /// None to disable section box
    pub fn set_section_box(&mut self, bounds: Option<([f32; 3], [f32; 3])>) -> Result<(), String> {
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        scene.set_section_box(bounds);
        if let Some(queue) = self.gpu.queue() {
            scene.update_section_box(queue);
        }
        Ok(())
    }

    /// Enable or disable edge rendering (wireframe overlay on solid geometry)
    pub fn set_edge_rendering(&mut self, enabled: bool) -> Result<(), String> {
        let scene = self.scene.as_mut().ok_or("Scene not initialized")?;
        scene.set_edge_rendering(enabled);
        Ok(())
    }

    /// Check if edge rendering is currently enabled
    pub fn is_edge_rendering(&self) -> Result<bool, String> {
        let scene = self.scene.as_ref().ok_or("Scene not initialized")?;
        Ok(scene.is_edge_rendering_enabled())
    }

    /// Set the color of a specific element by index
    /// TODO: Implement per-element coloring in renderer
    pub fn set_element_color(&mut self, _element_index: usize, _r: f32, _g: f32, _b: f32) -> Result<(), String> {
        // TODO: Modify vertex colors in GPU buffer for specific element
        Ok(())
    }

    /// Reset all element colors to their defaults
    /// TODO: Implement color reset in renderer
    pub fn reset_element_colors(&mut self) -> Result<(), String> {
        // TODO: Reset vertex colors to type-based defaults
        Ok(())
    }
}
