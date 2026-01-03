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

    /// Compute union of two bounding boxes
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min: [
                self.min[0].min(other.min[0]),
                self.min[1].min(other.min[1]),
                self.min[2].min(other.min[2]),
            ],
            max: [
                self.max[0].max(other.max[0]),
                self.max[1].max(other.max[1]),
                self.max[2].max(other.max[2]),
            ],
        }
    }

    /// Create a bounding box from min/max arrays
    pub fn from_min_max(min: [f32; 3], max: [f32; 3]) -> BoundingBox {
        BoundingBox { min, max }
    }
}

/// Get color for IFC element type
pub fn color_for_element_type(element_type: &str) -> [f32; 4] {
    match element_type.to_uppercase().as_str() {
        // === ARCHITECTURAL ===
        // Walls - light gray/beige
        s if s.contains("WALL") => [0.85, 0.82, 0.75, 1.0],
        // Slabs/floors - darker gray
        s if s.contains("SLAB") || s.contains("FLOOR") => [0.6, 0.6, 0.65, 1.0],
        // Doors - brown
        s if s.contains("DOOR") => [0.6, 0.45, 0.3, 1.0],
        // Windows - light blue (glass)
        s if s.contains("WINDOW") => [0.7, 0.85, 0.95, 0.7],
        // Roofs - terracotta
        s if s.contains("ROOF") => [0.75, 0.5, 0.4, 1.0],
        // Stairs - concrete gray
        s if s.contains("STAIR") => [0.65, 0.65, 0.65, 1.0],
        // Railings - dark gray
        s if s.contains("RAILING") => [0.4, 0.4, 0.4, 1.0],
        // Furniture - wood tone
        s if s.contains("FURNITURE") => [0.65, 0.5, 0.35, 1.0],

        // === STRUCTURAL ===
        // Columns - steel blue
        s if s.contains("COLUMN") => [0.5, 0.55, 0.7, 1.0],
        // Beams - steel gray
        s if s.contains("BEAM") => [0.55, 0.55, 0.6, 1.0],
        // Footings - concrete
        s if s.contains("FOOTING") || s.contains("FOUNDATION") => [0.5, 0.5, 0.5, 1.0],

        // === MEP (Mechanical/Electrical/Plumbing) ===
        // Pipes - copper/green for water
        s if s.contains("PIPE") => [0.2, 0.7, 0.5, 1.0],
        // Ducts - silver/metal
        s if s.contains("DUCT") => [0.7, 0.75, 0.8, 1.0],
        // Flow terminals (vents, outlets) - light metal
        s if s.contains("FLOWTERMINAL") || s.contains("TERMINAL") => [0.6, 0.65, 0.7, 1.0],

        // === ELECTRICAL ===
        // Cable carriers/trays - orange
        s if s.contains("CABLE") || s.contains("CONDUIT") => [0.9, 0.5, 0.2, 1.0],
        // Electrical equipment - yellow
        s if s.contains("ELECTRIC") => [0.9, 0.8, 0.2, 1.0],

        // === GENERIC ===
        // Building element proxy - purple tint
        s if s.contains("PROXY") => [0.6, 0.5, 0.7, 1.0],

        // Default - neutral gray
        _ => [0.7, 0.7, 0.7, 1.0],
    }
}

/// Generate a box mesh with proper normals per face
pub fn generate_box_with_normals(
    center: [f32; 3],
    size: [f32; 3],
    color: [f32; 4],
) -> Mesh {
    let mut mesh = Mesh::new();

    let hx = size[0] / 2.0;
    let hy = size[1] / 2.0;
    let hz = size[2] / 2.0;
    let cx = center[0];
    let cy = center[1];
    let cz = center[2];

    // Each face has 4 vertices with proper normals
    // Front face (+Z)
    let base = mesh.vertex_count() as u32;
    mesh.add_vertex(cx - hx, cy - hy, cz + hz);
    mesh.add_vertex(cx + hx, cy - hy, cz + hz);
    mesh.add_vertex(cx + hx, cy + hy, cz + hz);
    mesh.add_vertex(cx - hx, cy + hy, cz + hz);
    for _ in 0..4 {
        mesh.add_normal(0.0, 0.0, 1.0);
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 2, base + 3, base);

    // Back face (-Z)
    let base = mesh.vertex_count() as u32;
    mesh.add_vertex(cx + hx, cy - hy, cz - hz);
    mesh.add_vertex(cx - hx, cy - hy, cz - hz);
    mesh.add_vertex(cx - hx, cy + hy, cz - hz);
    mesh.add_vertex(cx + hx, cy + hy, cz - hz);
    for _ in 0..4 {
        mesh.add_normal(0.0, 0.0, -1.0);
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 2, base + 3, base);

    // Top face (+Y)
    let base = mesh.vertex_count() as u32;
    mesh.add_vertex(cx - hx, cy + hy, cz + hz);
    mesh.add_vertex(cx + hx, cy + hy, cz + hz);
    mesh.add_vertex(cx + hx, cy + hy, cz - hz);
    mesh.add_vertex(cx - hx, cy + hy, cz - hz);
    for _ in 0..4 {
        mesh.add_normal(0.0, 1.0, 0.0);
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 2, base + 3, base);

    // Bottom face (-Y)
    let base = mesh.vertex_count() as u32;
    mesh.add_vertex(cx - hx, cy - hy, cz - hz);
    mesh.add_vertex(cx + hx, cy - hy, cz - hz);
    mesh.add_vertex(cx + hx, cy - hy, cz + hz);
    mesh.add_vertex(cx - hx, cy - hy, cz + hz);
    for _ in 0..4 {
        mesh.add_normal(0.0, -1.0, 0.0);
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 2, base + 3, base);

    // Right face (+X)
    let base = mesh.vertex_count() as u32;
    mesh.add_vertex(cx + hx, cy - hy, cz + hz);
    mesh.add_vertex(cx + hx, cy - hy, cz - hz);
    mesh.add_vertex(cx + hx, cy + hy, cz - hz);
    mesh.add_vertex(cx + hx, cy + hy, cz + hz);
    for _ in 0..4 {
        mesh.add_normal(1.0, 0.0, 0.0);
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 2, base + 3, base);

    // Left face (-X)
    let base = mesh.vertex_count() as u32;
    mesh.add_vertex(cx - hx, cy - hy, cz - hz);
    mesh.add_vertex(cx - hx, cy - hy, cz + hz);
    mesh.add_vertex(cx - hx, cy + hy, cz + hz);
    mesh.add_vertex(cx - hx, cy + hy, cz - hz);
    for _ in 0..4 {
        mesh.add_normal(-1.0, 0.0, 0.0);
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 2, base + 3, base);

    mesh
}

/// Merge multiple meshes into one
pub fn merge_meshes(meshes: Vec<Mesh>) -> Mesh {
    let mut result = Mesh::new();

    for mesh in meshes {
        let base = result.vertex_count() as u32;

        // Add vertices
        result.vertices.extend(&mesh.vertices);
        result.normals.extend(&mesh.normals);
        result.colors.extend(&mesh.colors);

        // Add indices with offset
        for idx in &mesh.indices {
            result.indices.push(idx + base);
        }
    }

    result
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
