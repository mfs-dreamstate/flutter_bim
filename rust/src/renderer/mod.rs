//! 3D Rendering Module
//!
//! High-performance 3D rendering using wgpu (WebGPU/Vulkan/Metal).
//! Handles scene rendering, camera management, and GPU resource management.

pub mod camera;
pub mod gpu;
pub mod pipeline;
pub mod vertex;

pub use camera::Camera;
pub use gpu::GpuContext;
pub use pipeline::RenderPipeline;
pub use vertex::Vertex;

use crate::bim::Mesh;

/// Renderer state and configuration
pub struct Renderer {
    pub gpu: GpuContext,
    pub pipeline: Option<RenderPipeline>,
    pub camera: Camera,
}

impl Renderer {
    /// Create a new renderer (will be initialized later with surface)
    pub fn new() -> Self {
        Self {
            gpu: GpuContext::new(),
            pipeline: None,
            camera: Camera::default(),
        }
    }

    /// Initialize the renderer with a surface (called from Flutter)
    pub async fn initialize(&mut self) -> Result<(), String> {
        self.gpu
            .initialize()
            .await
            .map_err(|e| format!("Failed to initialize GPU: {}", e))?;

        Ok(())
    }

    /// Render a frame
    pub fn render(&mut self, meshes: &[Mesh]) -> Result<(), String> {
        // TODO: Implement rendering
        Ok(())
    }

    /// Update camera position/rotation
    pub fn update_camera(&mut self, position: [f32; 3], target: [f32; 3]) {
        self.camera.set_position(position);
        self.camera.set_target(target);
    }

    /// Resize the viewport
    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.set_aspect_ratio(width as f32 / height as f32);
    }
}
