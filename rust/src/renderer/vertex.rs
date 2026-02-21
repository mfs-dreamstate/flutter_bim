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

// ========================================================================
// Compressed Vertex Format (12 bytes)
// ========================================================================

/// Ultra-compressed vertex for LOD rendering (12 bytes instead of 20).
///
/// Layout:
/// - Position: 3x f16 (half-float stored as u16 bits) = 6 bytes
/// - Normal: octahedral encoding 2x i8 = 2 bytes
/// - Color: RGB565 = 2 bytes
/// - Padding: 2 bytes for alignment
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CompressedVertex {
    /// X position as f16 bits
    pub position_x: u16,
    /// Y position as f16 bits
    pub position_y: u16,
    /// Z position as f16 bits
    pub position_z: u16,
    /// Octahedral encoded normal [s, t] in i8 range
    pub normal_oct: [i8; 2],
    /// RGB565 packed color
    pub color_565: u16,
    /// Padding for 4-byte alignment
    pub _pad: u16,
}

impl CompressedVertex {
    /// Get vertex buffer layout description for wgpu.
    ///
    /// Uses:
    /// - Uint16x2 for position_x, position_y (shader unpacks f16)
    /// - Uint16x2 for position_z + normal_oct (shader unpacks)
    /// - Uint16x2 for color_565 + _pad
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CompressedVertex>() as wgpu::BufferAddress, // 12 bytes
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position_x, position_y as Uint16x2 at offset 0
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Uint16x2,
                },
                // position_z (u16) + normal_oct ([i8; 2] = 2 bytes) packed as Uint16x2 at offset 4
                wgpu::VertexAttribute {
                    offset: 4,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint16x2,
                },
                // color_565 + _pad as Uint16x2 at offset 8
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Uint16x2,
                },
            ],
        }
    }
}

/// Encode a f32 value to IEEE 754 half-precision float (f16) stored as u16 bits.
///
/// Handles special values: 0, infinity, NaN, denormals, and overflow.
pub fn f32_to_f16(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = (bits >> 31) & 1;
    let exponent = ((bits >> 23) & 0xFF) as i32;
    let mantissa = bits & 0x7F_FFFF;

    // Zero (positive or negative)
    if exponent == 0 && mantissa == 0 {
        return (sign << 15) as u16;
    }

    // NaN
    if exponent == 0xFF && mantissa != 0 {
        return ((sign << 15) | 0x7C00 | 0x0200) as u16; // qNaN
    }

    // Infinity
    if exponent == 0xFF && mantissa == 0 {
        return ((sign << 15) | 0x7C00) as u16;
    }

    // Biased exponent for f16 (bias difference: 127 - 15 = 112)
    let new_exp = exponent - 112;

    // Overflow to infinity
    if new_exp >= 31 {
        return ((sign << 15) | 0x7C00) as u16;
    }

    // Underflow to zero (too small for denormal)
    if new_exp <= -10 {
        return (sign << 15) as u16;
    }

    // Denormal in f16
    if new_exp <= 0 {
        let shift = (1 - new_exp) as u32;
        // Add implicit leading 1 bit
        let full_mantissa = mantissa | 0x80_0000;
        let shifted = full_mantissa >> (13 + shift);
        return ((sign << 15) | shifted) as u16;
    }

    // Normal number
    let f16_mantissa = mantissa >> 13;
    ((sign << 15) | ((new_exp as u32) << 10) | f16_mantissa) as u16
}

/// Decode an IEEE 754 half-precision float (f16 stored as u16 bits) back to f32.
pub fn f16_to_f32(bits: u16) -> f32 {
    let sign = ((bits >> 15) & 1) as u32;
    let exponent = ((bits >> 10) & 0x1F) as u32;
    let mantissa = (bits & 0x3FF) as u32;

    if exponent == 0 && mantissa == 0 {
        // Zero
        return f32::from_bits(sign << 31);
    }

    if exponent == 0x1F {
        if mantissa != 0 {
            // NaN
            return f32::from_bits((sign << 31) | 0x7F80_0000 | (mantissa << 13));
        } else {
            // Infinity
            return f32::from_bits((sign << 31) | 0x7F80_0000);
        }
    }

    if exponent == 0 {
        // Denormal: find the leading 1 bit
        let mut m = mantissa;
        let mut e = 0i32;
        while (m & 0x400) == 0 {
            m <<= 1;
            e += 1;
        }
        m &= 0x3FF; // remove the leading 1
        let f32_exp = (127 - 15 - e) as u32;
        return f32::from_bits((sign << 31) | (f32_exp << 23) | (m << 13));
    }

    // Normal number
    let f32_exp = exponent + 112; // 127 - 15
    f32::from_bits((sign << 31) | (f32_exp << 23) | (mantissa << 13))
}

/// Encode a unit normal vector to octahedral 2-byte encoding.
///
/// Maps a 3D unit vector to a 2D point in [-1, 1]^2 using octahedral mapping,
/// then quantizes to i8 range [-127, 127].
///
/// Reference: "Survey of Efficient Representations for Independent Unit Vectors" (Cigolle et al.)
pub fn encode_octahedral(normal: [f32; 3]) -> [i8; 2] {
    let [mut nx, mut ny, nz] = normal;

    // Project onto L1-norm unit octahedron
    let inv_l1 = 1.0 / (nx.abs() + ny.abs() + nz.abs()).max(1e-10);
    nx *= inv_l1;
    ny *= inv_l1;

    // Wrap the bottom hemisphere
    let (ox, oy) = if nz < 0.0 {
        let wx = (1.0 - ny.abs()) * if nx >= 0.0 { 1.0 } else { -1.0 };
        let wy = (1.0 - nx.abs()) * if ny >= 0.0 { 1.0 } else { -1.0 };
        (wx, wy)
    } else {
        (nx, ny)
    };

    // Quantize to i8 range [-127, 127]
    let sx = (ox * 127.0).round().max(-127.0).min(127.0) as i8;
    let sy = (oy * 127.0).round().max(-127.0).min(127.0) as i8;

    [sx, sy]
}

/// Decode octahedral encoding back to a unit normal vector.
pub fn decode_octahedral(oct: [i8; 2]) -> [f32; 3] {
    let mut x = oct[0] as f32 / 127.0;
    let mut y = oct[1] as f32 / 127.0;
    let z = 1.0 - x.abs() - y.abs();

    if z < 0.0 {
        let old_x = x;
        x = (1.0 - y.abs()) * if old_x >= 0.0 { 1.0 } else { -1.0 };
        y = (1.0 - old_x.abs()) * if y >= 0.0 { 1.0 } else { -1.0 };
    }

    // Normalize to unit length
    let len = (x * x + y * y + z * z).sqrt();
    if len > 1e-10 {
        [x / len, y / len, z / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Pack RGB color (0-1 range) to RGB565 format.
///
/// - Red: 5 bits (0-31)
/// - Green: 6 bits (0-63)
/// - Blue: 5 bits (0-31)
pub fn pack_rgb565(r: f32, g: f32, b: f32) -> u16 {
    let r5 = (r.clamp(0.0, 1.0) * 31.0).round() as u16;
    let g6 = (g.clamp(0.0, 1.0) * 63.0).round() as u16;
    let b5 = (b.clamp(0.0, 1.0) * 31.0).round() as u16;
    (r5 << 11) | (g6 << 5) | b5
}

/// Unpack RGB565 to float RGB (0-1 range).
pub fn unpack_rgb565(packed: u16) -> (f32, f32, f32) {
    let r5 = (packed >> 11) & 0x1F;
    let g6 = (packed >> 5) & 0x3F;
    let b5 = packed & 0x1F;
    (
        r5 as f32 / 31.0,
        g6 as f32 / 63.0,
        b5 as f32 / 31.0,
    )
}

/// Convert a Vertex to CompressedVertex.
pub fn compress_vertex(v: &Vertex) -> CompressedVertex {
    // Unpack the normal from the Vertex's Snorm8x4 encoding
    let normal = [
        v.normal_packed[0] as f32 / 127.0,
        v.normal_packed[1] as f32 / 127.0,
        v.normal_packed[2] as f32 / 127.0,
    ];

    // Unpack the color from the Vertex's Unorm8x4 encoding
    let r = v.color_packed[0] as f32 / 255.0;
    let g = v.color_packed[1] as f32 / 255.0;
    let b = v.color_packed[2] as f32 / 255.0;

    CompressedVertex {
        position_x: f32_to_f16(v.position[0]),
        position_y: f32_to_f16(v.position[1]),
        position_z: f32_to_f16(v.position[2]),
        normal_oct: encode_octahedral(normal),
        color_565: pack_rgb565(r, g, b),
        _pad: 0,
    }
}

/// Convert a CompressedVertex back to Vertex (lossy).
pub fn decompress_vertex(cv: &CompressedVertex) -> Vertex {
    let position = [
        f16_to_f32(cv.position_x),
        f16_to_f32(cv.position_y),
        f16_to_f32(cv.position_z),
    ];

    let normal = decode_octahedral(cv.normal_oct);
    let (r, g, b) = unpack_rgb565(cv.color_565);

    Vertex::new(position, normal, [r, g, b, 1.0])
}

/// Batch compress vertices from standard Vertex to CompressedVertex.
pub fn compress_vertices(vertices: &[Vertex]) -> Vec<CompressedVertex> {
    vertices.iter().map(compress_vertex).collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressed_vertex_size() {
        assert_eq!(std::mem::size_of::<CompressedVertex>(), 12);
    }

    #[test]
    fn test_vertex_size() {
        assert_eq!(std::mem::size_of::<Vertex>(), 20);
    }

    #[test]
    fn test_f32_f16_roundtrip_normal_values() {
        let test_values = [0.0f32, 1.0, -1.0, 0.5, -0.5, 100.0, -100.0, 0.333, 3.14];
        for &val in &test_values {
            let encoded = f32_to_f16(val);
            let decoded = f16_to_f32(encoded);
            let tolerance = val.abs() * 0.01 + 0.001; // ~1% relative + small absolute
            assert!(
                (decoded - val).abs() < tolerance,
                "f16 roundtrip failed for {}: got {} (diff {})",
                val, decoded, (decoded - val).abs()
            );
        }
    }

    #[test]
    fn test_f32_f16_zero() {
        assert_eq!(f32_to_f16(0.0), 0);
        assert_eq!(f16_to_f32(0), 0.0);

        // Negative zero
        let neg_zero = f32_to_f16(-0.0);
        assert_eq!(f16_to_f32(neg_zero), -0.0f32);
    }

    #[test]
    fn test_f32_f16_infinity() {
        let pos_inf = f32_to_f16(f32::INFINITY);
        assert!(f16_to_f32(pos_inf).is_infinite());
        assert!(f16_to_f32(pos_inf) > 0.0);

        let neg_inf = f32_to_f16(f32::NEG_INFINITY);
        assert!(f16_to_f32(neg_inf).is_infinite());
        assert!(f16_to_f32(neg_inf) < 0.0);
    }

    #[test]
    fn test_f32_f16_nan() {
        let nan_bits = f32_to_f16(f32::NAN);
        assert!(f16_to_f32(nan_bits).is_nan());
    }

    #[test]
    fn test_octahedral_encode_decode_axis_normals() {
        let normals = [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ];

        for normal in &normals {
            let encoded = encode_octahedral(*normal);
            let decoded = decode_octahedral(encoded);

            // Check unit length
            let len = (decoded[0] * decoded[0] + decoded[1] * decoded[1] + decoded[2] * decoded[2]).sqrt();
            assert!(
                (len - 1.0).abs() < 0.02,
                "Decoded normal not unit length: {:?} -> {:?} (len {})",
                normal, decoded, len
            );

            // Check direction (dot product should be close to 1)
            let dot = normal[0] * decoded[0] + normal[1] * decoded[1] + normal[2] * decoded[2];
            assert!(
                dot > 0.95,
                "Octahedral roundtrip too lossy for {:?}: decoded {:?}, dot {}",
                normal, decoded, dot
            );
        }
    }

    #[test]
    fn test_octahedral_encode_decode_diagonal_normals() {
        let inv_sqrt3 = 1.0 / 3.0f32.sqrt();
        let normals = [
            [inv_sqrt3, inv_sqrt3, inv_sqrt3],
            [-inv_sqrt3, inv_sqrt3, inv_sqrt3],
            [inv_sqrt3, -inv_sqrt3, inv_sqrt3],
            [inv_sqrt3, inv_sqrt3, -inv_sqrt3],
        ];

        for normal in &normals {
            let encoded = encode_octahedral(*normal);
            let decoded = decode_octahedral(encoded);

            let len = (decoded[0] * decoded[0] + decoded[1] * decoded[1] + decoded[2] * decoded[2]).sqrt();
            assert!(
                (len - 1.0).abs() < 0.02,
                "Decoded normal not unit length: {:?} (len {})",
                decoded, len
            );

            let dot = normal[0] * decoded[0] + normal[1] * decoded[1] + normal[2] * decoded[2];
            assert!(
                dot > 0.9,
                "Octahedral roundtrip too lossy for diagonal {:?}: decoded {:?}, dot {}",
                normal, decoded, dot
            );
        }
    }

    #[test]
    fn test_rgb565_pack_unpack() {
        // Pure red
        let packed = pack_rgb565(1.0, 0.0, 0.0);
        let (r, g, b) = unpack_rgb565(packed);
        assert!((r - 1.0).abs() < 0.04);
        assert!(g.abs() < 0.02);
        assert!(b.abs() < 0.04);

        // Pure green
        let packed = pack_rgb565(0.0, 1.0, 0.0);
        let (r, g, b) = unpack_rgb565(packed);
        assert!(r.abs() < 0.04);
        assert!((g - 1.0).abs() < 0.02);
        assert!(b.abs() < 0.04);

        // Pure blue
        let packed = pack_rgb565(0.0, 0.0, 1.0);
        let (r, g, b) = unpack_rgb565(packed);
        assert!(r.abs() < 0.04);
        assert!(g.abs() < 0.02);
        assert!((b - 1.0).abs() < 0.04);

        // White
        let packed = pack_rgb565(1.0, 1.0, 1.0);
        let (r, g, b) = unpack_rgb565(packed);
        assert!((r - 1.0).abs() < 0.04);
        assert!((g - 1.0).abs() < 0.02);
        assert!((b - 1.0).abs() < 0.04);

        // Black
        let packed = pack_rgb565(0.0, 0.0, 0.0);
        assert_eq!(packed, 0);
        let (r, g, b) = unpack_rgb565(packed);
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn test_rgb565_clamping() {
        // Values outside [0, 1] should be clamped
        let packed = pack_rgb565(1.5, -0.5, 2.0);
        let (r, g, b) = unpack_rgb565(packed);
        assert!((r - 1.0).abs() < 0.04);
        assert!(g.abs() < 0.02);
        assert!((b - 1.0).abs() < 0.04);
    }

    #[test]
    fn test_compress_decompress_vertex() {
        let v = Vertex::new([1.0, 2.0, 3.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0, 1.0]);
        let cv = compress_vertex(&v);
        let dv = decompress_vertex(&cv);

        // Position roundtrip (f16 precision)
        assert!((dv.position[0] - 1.0).abs() < 0.01);
        assert!((dv.position[1] - 2.0).abs() < 0.01);
        assert!((dv.position[2] - 3.0).abs() < 0.01);

        // Normal should be approximately [0, 1, 0]
        let dn = [
            dv.normal_packed[0] as f32 / 127.0,
            dv.normal_packed[1] as f32 / 127.0,
            dv.normal_packed[2] as f32 / 127.0,
        ];
        assert!(dn[1] > 0.9, "Y normal should be close to 1.0, got {}", dn[1]);

        // Color should be approximately red
        let dr = dv.color_packed[0] as f32 / 255.0;
        let dg = dv.color_packed[1] as f32 / 255.0;
        let db = dv.color_packed[2] as f32 / 255.0;
        assert!(dr > 0.9, "Red channel should be close to 1.0, got {}", dr);
        assert!(dg < 0.1, "Green channel should be close to 0.0, got {}", dg);
        assert!(db < 0.1, "Blue channel should be close to 0.0, got {}", db);
    }

    #[test]
    fn test_batch_compress() {
        let (vertices, _) = generate_test_cube();
        let compressed = compress_vertices(&vertices);

        assert_eq!(compressed.len(), vertices.len());
        assert_eq!(compressed.len(), 24);

        // Each compressed vertex is 12 bytes
        let total_bytes = compressed.len() * std::mem::size_of::<CompressedVertex>();
        let original_bytes = vertices.len() * std::mem::size_of::<Vertex>();
        assert_eq!(total_bytes, 24 * 12);
        assert_eq!(original_bytes, 24 * 20);
        assert!(total_bytes < original_bytes, "Compressed should be smaller");
    }

    #[test]
    fn test_batch_compress_roundtrip() {
        let vertices = vec![
            Vertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
            Vertex::new([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0, 1.0]),
            Vertex::new([0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0, 1.0]),
        ];

        let compressed = compress_vertices(&vertices);
        assert_eq!(compressed.len(), 3);

        for (i, cv) in compressed.iter().enumerate() {
            let dv = decompress_vertex(cv);
            // Positions should be very close (f16 is exact for small integers)
            for j in 0..3 {
                assert!(
                    (dv.position[j] - vertices[i].position[j]).abs() < 0.01,
                    "Position mismatch at vertex {}, component {}: {} vs {}",
                    i, j, dv.position[j], vertices[i].position[j]
                );
            }
        }
    }
}
