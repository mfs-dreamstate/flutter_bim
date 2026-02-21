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
        self.vertices.extend_from_slice(&[x, y, z]);
    }

    /// Add a normal
    pub fn add_normal(&mut self, x: f32, y: f32, z: f32) {
        self.normals.extend_from_slice(&[x, y, z]);
    }

    /// Add a color
    pub fn add_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.colors.extend_from_slice(&[r, g, b, a]);
    }

    /// Add a triangle
    pub fn add_triangle(&mut self, i0: u32, i1: u32, i2: u32) {
        self.indices.extend_from_slice(&[i0, i1, i2]);
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

/// Case-insensitive ASCII substring match without allocation.
fn ascii_contains_ci(haystack: &str, needle: &str) -> bool {
    let n = needle.as_bytes();
    haystack.as_bytes().windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

/// Get color for IFC element type (zero-alloc — no .to_uppercase())
pub fn color_for_element_type(element_type: &str) -> [f32; 4] {
    // === ARCHITECTURAL ===
    if ascii_contains_ci(element_type, "WALL") { return [0.85, 0.82, 0.75, 1.0]; }
    if ascii_contains_ci(element_type, "SLAB") || ascii_contains_ci(element_type, "FLOOR") { return [0.6, 0.6, 0.65, 1.0]; }
    if ascii_contains_ci(element_type, "DOOR") { return [0.6, 0.45, 0.3, 1.0]; }
    if ascii_contains_ci(element_type, "WINDOW") { return [0.7, 0.85, 0.95, 0.7]; }
    if ascii_contains_ci(element_type, "ROOF") { return [0.75, 0.5, 0.4, 1.0]; }
    if ascii_contains_ci(element_type, "STAIR") { return [0.65, 0.65, 0.65, 1.0]; }
    if ascii_contains_ci(element_type, "RAILING") { return [0.4, 0.4, 0.4, 1.0]; }
    if ascii_contains_ci(element_type, "FURNITURE") { return [0.65, 0.5, 0.35, 1.0]; }
    // === STRUCTURAL ===
    if ascii_contains_ci(element_type, "COLUMN") { return [0.5, 0.55, 0.7, 1.0]; }
    if ascii_contains_ci(element_type, "BEAM") { return [0.55, 0.55, 0.6, 1.0]; }
    if ascii_contains_ci(element_type, "FOOTING") || ascii_contains_ci(element_type, "FOUNDATION") { return [0.5, 0.5, 0.5, 1.0]; }
    // === MEP ===
    if ascii_contains_ci(element_type, "PIPE") { return [0.2, 0.7, 0.5, 1.0]; }
    if ascii_contains_ci(element_type, "DUCT") { return [0.7, 0.75, 0.8, 1.0]; }
    if ascii_contains_ci(element_type, "FLOWTERMINAL") || ascii_contains_ci(element_type, "TERMINAL") { return [0.6, 0.65, 0.7, 1.0]; }
    // === ELECTRICAL ===
    if ascii_contains_ci(element_type, "CABLE") || ascii_contains_ci(element_type, "CONDUIT") { return [0.9, 0.5, 0.2, 1.0]; }
    if ascii_contains_ci(element_type, "ELECTRIC") { return [0.9, 0.8, 0.2, 1.0]; }
    // === GENERIC ===
    if ascii_contains_ci(element_type, "PROXY") { return [0.6, 0.5, 0.7, 1.0]; }
    // Default
    [0.7, 0.7, 0.7, 1.0]
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

/// Append source mesh into target mesh with correct index offsets.
pub fn append_mesh(target: &mut Mesh, source: &Mesh) {
    let base = target.vertex_count() as u32;
    target.vertices.extend_from_slice(&source.vertices);
    target.normals.extend_from_slice(&source.normals);
    target.colors.extend_from_slice(&source.colors);
    target.indices.extend(source.indices.iter().map(|i| i + base));
}

/// Transform all vertices and normals in a mesh by a 4x4 matrix.
pub fn transform_mesh(mesh: &mut Mesh, matrix: &glam::Mat4) {
    let normal_matrix = matrix.inverse().transpose();
    for i in (0..mesh.vertices.len()).step_by(3) {
        let pos = glam::Vec3::new(mesh.vertices[i], mesh.vertices[i + 1], mesh.vertices[i + 2]);
        let transformed = matrix.transform_point3(pos);
        mesh.vertices[i] = transformed.x;
        mesh.vertices[i + 1] = transformed.y;
        mesh.vertices[i + 2] = transformed.z;
    }
    for i in (0..mesh.normals.len()).step_by(3) {
        let n = glam::Vec3::new(mesh.normals[i], mesh.normals[i + 1], mesh.normals[i + 2]);
        let tn = normal_matrix.transform_vector3(n).normalize_or_zero();
        mesh.normals[i] = tn.x;
        mesh.normals[i + 1] = tn.y;
        mesh.normals[i + 2] = tn.z;
    }
}

/// Merge multiple meshes into one
pub fn merge_meshes(meshes: Vec<Mesh>) -> Mesh {
    // Pre-allocate totals
    let total_verts: usize = meshes.iter().map(|m| m.vertices.len()).sum();
    let total_indices: usize = meshes.iter().map(|m| m.indices.len()).sum();
    let total_normals: usize = meshes.iter().map(|m| m.normals.len()).sum();
    let total_colors: usize = meshes.iter().map(|m| m.colors.len()).sum();

    let mut result = Mesh {
        vertices: Vec::with_capacity(total_verts),
        indices: Vec::with_capacity(total_indices),
        normals: Vec::with_capacity(total_normals),
        colors: Vec::with_capacity(total_colors),
    };

    for mesh in meshes {
        let base = result.vertex_count() as u32;

        result.vertices.extend_from_slice(&mesh.vertices);
        result.normals.extend_from_slice(&mesh.normals);
        result.colors.extend_from_slice(&mesh.colors);

        // Add indices with offset — single extend avoids per-element push
        result.indices.extend(mesh.indices.iter().map(|idx| idx + base));
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
