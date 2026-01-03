//! 3D Rendering Module
//!
//! High-performance 3D rendering using wgpu (WebGPU/Vulkan/Metal).
//! Handles scene rendering, camera management, and GPU resource management.

pub mod camera;
pub mod gpu;
pub mod overlay;
pub mod pipeline;
pub mod scene;
pub mod vertex;

pub use camera::{Camera, ray_aabb_intersect};
pub use gpu::GpuContext;
pub use overlay::DrawingOverlay;
pub use pipeline::{RenderMode, RenderPipeline};
pub use scene::SceneRenderer;
pub use vertex::{generate_test_cube, Vertex};

/// Renderer state and configuration
pub struct Renderer {
    pub gpu: GpuContext,
    pub scene: Option<SceneRenderer>,
    pub camera: Camera,
    pub initialized: bool,
}

impl Renderer {
    /// Create a new renderer
    pub fn new() -> Self {
        Self {
            gpu: GpuContext::new(),
            scene: None,
            camera: Camera::default(),
            initialized: false,
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
    pub fn render_frame(&self) -> Result<Vec<u8>, String> {
        let device = self.gpu.device().ok_or("GPU not initialized")?;
        let queue = self.gpu.queue().ok_or("GPU queue not initialized")?;
        let scene = self.scene.as_ref().ok_or("Scene not initialized")?;

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

    /// Fit camera to bounding box
    pub fn fit_camera_to_bounds(&mut self, min: [f32; 3], max: [f32; 3]) {
        // Calculate center and size
        let center = [
            (min[0] + max[0]) / 2.0,
            (min[1] + max[1]) / 2.0,
            (min[2] + max[2]) / 2.0,
        ];

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

        // Set camera distance
        self.camera.set_distance(distance);
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

    /// Set the section plane for clipping geometry
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
