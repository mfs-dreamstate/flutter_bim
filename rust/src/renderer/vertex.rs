//! Vertex Definitions
//!
//! Vertex structures for GPU rendering.

use bytemuck::{Pod, Zeroable};

/// Vertex structure for 3D mesh rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// Position in 3D space [x, y, z]
    pub position: [f32; 3],
    /// Normal vector [x, y, z]
    pub normal: [f32; 3],
    /// Color [r, g, b, a]
    pub color: [f32; 4],
}

impl Vertex {
    /// Create a new vertex
    pub fn new(position: [f32; 3], normal: [f32; 3], color: [f32; 4]) -> Self {
        Self {
            position,
            normal,
            color,
        }
    }

    /// Get vertex buffer layout description for wgpu
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normal
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Generate a test cube mesh
pub fn generate_test_cube() -> (Vec<Vertex>, Vec<u32>) {
    let vertices = vec![
        // Front face (red)
        Vertex::new([-1.0, -1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 1.0]),
        Vertex::new([1.0, -1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 1.0]),
        Vertex::new([1.0, 1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 1.0]),
        Vertex::new([-1.0, 1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 1.0]),
        // Back face (green)
        Vertex::new([1.0, -1.0, -1.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0, 1.0]),
        Vertex::new([-1.0, -1.0, -1.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0, 1.0]),
        Vertex::new([-1.0, 1.0, -1.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0, 1.0]),
        Vertex::new([1.0, 1.0, -1.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0, 1.0]),
        // Top face (blue)
        Vertex::new([-1.0, 1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0, 1.0]),
        Vertex::new([1.0, 1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0, 1.0]),
        Vertex::new([1.0, 1.0, -1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0, 1.0]),
        Vertex::new([-1.0, 1.0, -1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0, 1.0]),
        // Bottom face (yellow)
        Vertex::new([-1.0, -1.0, -1.0], [0.0, -1.0, 0.0], [1.0, 1.0, 0.0, 1.0]),
        Vertex::new([1.0, -1.0, -1.0], [0.0, -1.0, 0.0], [1.0, 1.0, 0.0, 1.0]),
        Vertex::new([1.0, -1.0, 1.0], [0.0, -1.0, 0.0], [1.0, 1.0, 0.0, 1.0]),
        Vertex::new([-1.0, -1.0, 1.0], [0.0, -1.0, 0.0], [1.0, 1.0, 0.0, 1.0]),
        // Right face (magenta)
        Vertex::new([1.0, -1.0, 1.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0, 1.0]),
        Vertex::new([1.0, -1.0, -1.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0, 1.0]),
        Vertex::new([1.0, 1.0, -1.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0, 1.0]),
        Vertex::new([1.0, 1.0, 1.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0, 1.0]),
        // Left face (cyan)
        Vertex::new([-1.0, -1.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 1.0, 1.0]),
        Vertex::new([-1.0, -1.0, 1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 1.0, 1.0]),
        Vertex::new([-1.0, 1.0, 1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 1.0, 1.0]),
        Vertex::new([-1.0, 1.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 1.0, 1.0]),
    ];

    let indices = vec![
        0, 1, 2, 2, 3, 0, // Front
        4, 5, 6, 6, 7, 4, // Back
        8, 9, 10, 10, 11, 8, // Top
        12, 13, 14, 14, 15, 12, // Bottom
        16, 17, 18, 18, 19, 16, // Right
        20, 21, 22, 22, 23, 20, // Left
    ];

    (vertices, indices)
}
