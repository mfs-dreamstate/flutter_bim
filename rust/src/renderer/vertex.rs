//! Vertex Definitions
//!
//! Vertex structures for GPU rendering.

use bytemuck::{Pod, Zeroable};

/// Compressed vertex structure for 3D mesh rendering (20 bytes, down from 40).
///
/// Normals are packed as Snorm8x4 (i8 per component, GPU auto-converts to float).
/// Colors are packed as Unorm8x4 (u8 per component, GPU auto-converts to float).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// Position in 3D space [x, y, z] — full f32 precision
    pub position: [f32; 3],
    /// Normal vector packed as Snorm8x4 [nx, ny, nz, 0]
    pub normal_packed: [i8; 4],
    /// Color packed as Unorm8x4 [r, g, b, a]
    pub color_packed: [u8; 4],
}

impl Vertex {
    /// Create a new vertex, packing normal and color into compact formats.
    pub fn new(position: [f32; 3], normal: [f32; 3], color: [f32; 4]) -> Self {
        Self {
            position,
            normal_packed: [
                (normal[0] * 127.0) as i8,
                (normal[1] * 127.0) as i8,
                (normal[2] * 127.0) as i8,
                0,
            ],
            color_packed: [
                (color[0] * 255.0) as u8,
                (color[1] * 255.0) as u8,
                (color[2] * 255.0) as u8,
                (color[3] * 255.0) as u8,
            ],
        }
    }

    /// Get vertex buffer layout description for wgpu
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress, // 20 bytes
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position: Float32x3 at offset 0
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normal: Snorm8x4 at offset 12 (auto-converts to vec4<f32>)
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Snorm8x4,
                },
                // Color: Unorm8x4 at offset 16 (auto-converts to vec4<f32>)
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Unorm8x4,
                },
            ],
        }
    }
}

/// Vertex for the shared unit box (position + normal only, 16 bytes).
/// Color comes from per-instance data instead.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BoxVertex {
    pub position: [f32; 3],
    pub normal_packed: [i8; 4],
}

impl BoxVertex {
    pub fn new(position: [f32; 3], normal: [f32; 3]) -> Self {
        Self {
            position,
            normal_packed: [
                (normal[0] * 127.0) as i8,
                (normal[1] * 127.0) as i8,
                (normal[2] * 127.0) as i8,
                0,
            ],
        }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<BoxVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Snorm8x4,
                },
            ],
        }
    }
}

/// Per-instance data for GPU instancing (28 bytes per instance).
///
/// Each instance represents one BIM element box:
///   - position: center of the box in world space
///   - scale: half-extents (vertex.position * scale + position = world position)
///   - color_packed: Unorm8x4 element color
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct InstanceData {
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub color_packed: [u8; 4],
}

impl InstanceData {
    pub fn new(position: [f32; 3], scale: [f32; 3], color: [f32; 4]) -> Self {
        Self {
            position,
            scale,
            color_packed: [
                (color[0] * 255.0) as u8,
                (color[1] * 255.0) as u8,
                (color[2] * 255.0) as u8,
                (color[3] * 255.0) as u8,
            ],
        }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Unorm8x4,
                },
            ],
        }
    }
}

/// Generate a unit box [-1, 1] with proper normals per face.
/// Shared geometry for all instanced BIM elements.
pub fn generate_unit_box() -> (Vec<BoxVertex>, Vec<u32>) {
    let vertices = vec![
        // Front face (+Z)
        BoxVertex::new([-1.0, -1.0,  1.0], [0.0, 0.0, 1.0]),
        BoxVertex::new([ 1.0, -1.0,  1.0], [0.0, 0.0, 1.0]),
        BoxVertex::new([ 1.0,  1.0,  1.0], [0.0, 0.0, 1.0]),
        BoxVertex::new([-1.0,  1.0,  1.0], [0.0, 0.0, 1.0]),
        // Back face (-Z)
        BoxVertex::new([ 1.0, -1.0, -1.0], [0.0, 0.0, -1.0]),
        BoxVertex::new([-1.0, -1.0, -1.0], [0.0, 0.0, -1.0]),
        BoxVertex::new([-1.0,  1.0, -1.0], [0.0, 0.0, -1.0]),
        BoxVertex::new([ 1.0,  1.0, -1.0], [0.0, 0.0, -1.0]),
        // Top face (+Y)
        BoxVertex::new([-1.0,  1.0,  1.0], [0.0, 1.0, 0.0]),
        BoxVertex::new([ 1.0,  1.0,  1.0], [0.0, 1.0, 0.0]),
        BoxVertex::new([ 1.0,  1.0, -1.0], [0.0, 1.0, 0.0]),
        BoxVertex::new([-1.0,  1.0, -1.0], [0.0, 1.0, 0.0]),
        // Bottom face (-Y)
        BoxVertex::new([-1.0, -1.0, -1.0], [0.0, -1.0, 0.0]),
        BoxVertex::new([ 1.0, -1.0, -1.0], [0.0, -1.0, 0.0]),
        BoxVertex::new([ 1.0, -1.0,  1.0], [0.0, -1.0, 0.0]),
        BoxVertex::new([-1.0, -1.0,  1.0], [0.0, -1.0, 0.0]),
        // Right face (+X)
        BoxVertex::new([ 1.0, -1.0,  1.0], [1.0, 0.0, 0.0]),
        BoxVertex::new([ 1.0, -1.0, -1.0], [1.0, 0.0, 0.0]),
        BoxVertex::new([ 1.0,  1.0, -1.0], [1.0, 0.0, 0.0]),
        BoxVertex::new([ 1.0,  1.0,  1.0], [1.0, 0.0, 0.0]),
        // Left face (-X)
        BoxVertex::new([-1.0, -1.0, -1.0], [-1.0, 0.0, 0.0]),
        BoxVertex::new([-1.0, -1.0,  1.0], [-1.0, 0.0, 0.0]),
        BoxVertex::new([-1.0,  1.0,  1.0], [-1.0, 0.0, 0.0]),
        BoxVertex::new([-1.0,  1.0, -1.0], [-1.0, 0.0, 0.0]),
    ];

    let indices = vec![
        0, 1, 2, 2, 3, 0,
        4, 5, 6, 6, 7, 4,
        8, 9, 10, 10, 11, 8,
        12, 13, 14, 14, 15, 12,
        16, 17, 18, 18, 19, 16,
        20, 21, 22, 22, 23, 20,
    ];

    (vertices, indices)
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
