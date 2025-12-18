//! Geometry Extraction and Processing
//!
//! Converts IFC geometry representations to triangle meshes.

use serde::{Deserialize, Serialize};

/// 3D Point
pub type Point3D = [f32; 3];

/// 3D Vector / Normal
pub type Vector3D = [f32; 3];

/// Triangle mesh representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    /// Vertex positions (x, y, z triplets)
    pub vertices: Vec<f32>,

    /// Vertex indices (triangle list)
    pub indices: Vec<u32>,

    /// Vertex normals (x, y, z triplets)
    pub normals: Vec<f32>,

    /// Vertex colors (r, g, b, a)
    pub colors: Vec<f32>,
}

/// Bounding box
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min: Point3D,
    pub max: Point3D,
}

impl Mesh {
    /// Create a new empty mesh
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
        }
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / 3
    }

    /// Get triangle count
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Calculate bounding box
    pub fn bounding_box(&self) -> Option<BoundingBox> {
        if self.vertices.is_empty() {
            return None;
        }

        let mut min = [f32::MAX, f32::MAX, f32::MAX];
        let mut max = [f32::MIN, f32::MIN, f32::MIN];

        for i in (0..self.vertices.len()).step_by(3) {
            let x = self.vertices[i];
            let y = self.vertices[i + 1];
            let z = self.vertices[i + 2];

            min[0] = min[0].min(x);
            min[1] = min[1].min(y);
            min[2] = min[2].min(z);

            max[0] = max[0].max(x);
            max[1] = max[1].max(y);
            max[2] = max[2].max(z);
        }

        Some(BoundingBox { min, max })
    }

    /// Add a vertex
    pub fn add_vertex(&mut self, x: f32, y: f32, z: f32) {
        self.vertices.push(x);
        self.vertices.push(y);
        self.vertices.push(z);
    }

    /// Add a normal
    pub fn add_normal(&mut self, x: f32, y: f32, z: f32) {
        self.normals.push(x);
        self.normals.push(y);
        self.normals.push(z);
    }

    /// Add a color
    pub fn add_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.colors.push(r);
        self.colors.push(g);
        self.colors.push(b);
        self.colors.push(a);
    }

    /// Add a triangle
    pub fn add_triangle(&mut self, i0: u32, i1: u32, i2: u32) {
        self.indices.push(i0);
        self.indices.push(i1);
        self.indices.push(i2);
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

impl BoundingBox {
    /// Get center point
    pub fn center(&self) -> Point3D {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
            (self.min[2] + self.max[2]) / 2.0,
        ]
    }

    /// Get size
    pub fn size(&self) -> Vector3D {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }
}

/// Generate a simple box mesh (for testing)
pub fn generate_box(width: f32, height: f32, depth: f32) -> Mesh {
    let mut mesh = Mesh::new();

    let hw = width / 2.0;
    let hh = height / 2.0;
    let hd = depth / 2.0;

    // Vertices (8 corners of a box)
    let vertices = [
        [-hw, -hh, -hd], // 0
        [hw, -hh, -hd],  // 1
        [hw, hh, -hd],   // 2
        [-hw, hh, -hd],  // 3
        [-hw, -hh, hd],  // 4
        [hw, -hh, hd],   // 5
        [hw, hh, hd],    // 6
        [-hw, hh, hd],   // 7
    ];

    for v in &vertices {
        mesh.add_vertex(v[0], v[1], v[2]);
        mesh.add_normal(0.0, 0.0, 1.0); // Default normal
        mesh.add_color(0.7, 0.7, 0.7, 1.0); // Default gray
    }

    // Indices (12 triangles, 2 per face)
    let indices = [
        // Front
        0, 1, 2, 2, 3, 0, // Back
        4, 6, 5, 6, 4, 7, // Left
        4, 0, 3, 3, 7, 4, // Right
        1, 5, 6, 6, 2, 1, // Bottom
        4, 5, 1, 1, 0, 4, // Top
        3, 2, 6, 6, 7, 3,
    ];

    for &i in &indices {
        mesh.indices.push(i);
    }

    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_creation() {
        let mesh = Mesh::new();
        assert_eq!(mesh.vertex_count(), 0);
        assert_eq!(mesh.triangle_count(), 0);
    }

    #[test]
    fn test_generate_box() {
        let mesh = generate_box(2.0, 2.0, 2.0);
        assert_eq!(mesh.vertex_count(), 8);
        assert_eq!(mesh.triangle_count(), 12);
    }

    #[test]
    fn test_bounding_box() {
        let mesh = generate_box(2.0, 2.0, 2.0);
        let bbox = mesh.bounding_box().unwrap();
        assert_eq!(bbox.center(), [0.0, 0.0, 0.0]);
        assert_eq!(bbox.size(), [2.0, 2.0, 2.0]);
    }
}
