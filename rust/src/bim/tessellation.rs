//! Geometry Type Dispatch and Tessellation
//!
//! Entry point for converting IFC geometry representations into triangle meshes.
//! Supports: FacetedBrep, ExtrudedAreaSolid, RevolvedAreaSolid, TriangulatedFaceSet,
//! PolygonalFaceSet, ShellBasedSurfaceModel, FaceBasedSurfaceModel, SweptDiskSolid,
//! MappedItem, BooleanClippingResult.

use std::collections::HashMap;
use std::sync::Mutex;

use glam::{Mat4, Vec3};

use super::entities::{EntityId, IfcEntity, IfcValue};
use super::geometry::Mesh;
use super::placement::{
    axis2_placement_3d_to_matrix, axis2_placement_2d_offset, cartesian_transform_operator_3d,
    read_cartesian_point, read_direction, PlacementCache,
};
use super::triangulate::triangulate_polygon;

// ---------------------------------------------------------------------------
// Geometry Cache (MappedItem deduplication)
// ---------------------------------------------------------------------------

/// A cached tessellated mesh for a shared geometry representation source.
#[derive(Debug, Clone)]
pub struct CachedMesh {
    pub vertices: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

/// Cache for shared geometry (MappedItem deduplication).
///
/// IFC uses `IFCMAPPEDITEM` to share geometry between similar elements
/// (e.g., all windows of the same type share one geometry definition,
/// each with a different transform). This cache avoids re-tessellating
/// the same representation source multiple times.
///
/// The cache is wrapped in a `Mutex` so it can be safely shared across
/// rayon threads.
pub struct GeometryCache {
    cached_meshes: HashMap<EntityId, CachedMesh>,
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of cache misses
    pub cache_misses: usize,
    /// Estimated bytes saved by deduplication
    pub bytes_saved: usize,
}

impl GeometryCache {
    /// Create a new empty geometry cache.
    pub fn new() -> Self {
        Self {
            cached_meshes: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            bytes_saved: 0,
        }
    }

    /// Check if a representation source is already cached.
    pub fn get(&self, source_id: EntityId) -> Option<&CachedMesh> {
        self.cached_meshes.get(&source_id)
    }

    /// Insert a tessellated mesh for a representation source.
    pub fn insert(&mut self, source_id: EntityId, mesh: CachedMesh) {
        self.cached_meshes.insert(source_id, mesh);
    }

    /// Get cache statistics: (hits, misses, bytes_saved).
    pub fn stats(&self) -> (usize, usize, usize) {
        (self.cache_hits, self.cache_misses, self.bytes_saved)
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.cached_meshes.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
        self.bytes_saved = 0;
    }

    /// Estimate total memory used by all cached meshes, in bytes.
    pub fn memory_bytes(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        for mesh in self.cached_meshes.values() {
            total += mesh.vertices.len() * std::mem::size_of::<f32>();
            total += mesh.normals.len() * std::mem::size_of::<f32>();
            total += mesh.indices.len() * std::mem::size_of::<u32>();
            total += std::mem::size_of::<CachedMesh>();
        }
        total
    }
}

impl Default for GeometryCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about shared geometry (MappedItem) usage in a model.
pub struct SharedGeometryStats {
    /// Total number of IFCMAPPEDITEM entities found
    pub total_mapped_items: usize,
    /// Number of unique representation sources
    pub unique_representations: usize,
    /// Average duplication ratio (e.g., 3.0 means 3x geometry sharing on average)
    pub duplication_ratio: f32,
    /// Estimated memory savings in megabytes from deduplication
    pub estimated_savings_mb: f32,
}

/// Count how many elements share geometry via MappedItem and compute statistics.
pub fn count_shared_geometry(entities: &HashMap<EntityId, IfcEntity>) -> SharedGeometryStats {
    // Find all IFCMAPPEDITEM entities and their representation map sources
    let mut source_counts: HashMap<EntityId, usize> = HashMap::new();
    let mut total_mapped_items = 0usize;

    for entity in entities.values() {
        if entity.entity_type.eq_ignore_ascii_case("IFCMAPPEDITEM") {
            total_mapped_items += 1;
            // MappingSource (attribute 0) -> IFCREPRESENTATIONMAP
            if let Some(source_id) = entity.get_entity_ref(0) {
                *source_counts.entry(source_id).or_insert(0) += 1;
            }
        }
    }

    let unique_representations = source_counts.len();
    let duplication_ratio = if unique_representations > 0 {
        total_mapped_items as f32 / unique_representations as f32
    } else {
        0.0
    };

    // Estimate savings: for each source used N times, we save (N-1) copies.
    // Assume average mesh is ~50KB (rough estimate for a BIM element).
    let avg_mesh_bytes: f32 = 50_000.0;
    let saved_copies: usize = source_counts.values().map(|&c| if c > 1 { c - 1 } else { 0 }).sum();
    let estimated_savings_mb = (saved_copies as f32 * avg_mesh_bytes) / (1024.0 * 1024.0);

    SharedGeometryStats {
        total_mapped_items,
        unique_representations,
        duplication_ratio,
        estimated_savings_mb,
    }
}

// ---------------------------------------------------------------------------
// Geometry Compression Utilities
// ---------------------------------------------------------------------------

/// Mesh statistics for analysis and debugging.
pub struct MeshStats {
    /// Number of vertex positions (vertices.len() / 3)
    pub vertex_count: usize,
    /// Number of triangles (indices.len() / 3)
    pub triangle_count: usize,
    /// Number of unique vertex positions (after deduplication)
    pub unique_vertex_count: usize,
    /// Number of degenerate (zero-area) triangles
    pub degenerate_triangles: usize,
    /// Average edge length across all triangles
    pub average_edge_length: f32,
    /// Axis-aligned bounding box minimum
    pub bounds_min: [f32; 3],
    /// Axis-aligned bounding box maximum
    pub bounds_max: [f32; 3],
    /// Total memory used by vertices + normals + indices, in bytes
    pub memory_bytes: usize,
}

/// Compute mesh statistics for a given mesh.
pub fn mesh_statistics(
    vertices: &[f32],
    normals: &[f32],
    indices: &[u32],
) -> MeshStats {
    let vertex_count = vertices.len() / 3;
    let triangle_count = indices.len() / 3;

    // Compute bounding box
    let mut bounds_min = [f32::MAX; 3];
    let mut bounds_max = [f32::MIN; 3];
    for i in 0..vertex_count {
        for c in 0..3 {
            let v = vertices[i * 3 + c];
            if v < bounds_min[c] { bounds_min[c] = v; }
            if v > bounds_max[c] { bounds_max[c] = v; }
        }
    }
    if vertex_count == 0 {
        bounds_min = [0.0; 3];
        bounds_max = [0.0; 3];
    }

    // Count unique vertices and degenerate triangles
    let mut unique_positions: Vec<[f32; 3]> = Vec::new();
    for i in 0..vertex_count {
        let pos = [vertices[i * 3], vertices[i * 3 + 1], vertices[i * 3 + 2]];
        let already_exists = unique_positions.iter().any(|p| {
            (p[0] - pos[0]).abs() < 1e-6
                && (p[1] - pos[1]).abs() < 1e-6
                && (p[2] - pos[2]).abs() < 1e-6
        });
        if !already_exists {
            unique_positions.push(pos);
        }
    }
    let unique_vertex_count = unique_positions.len();

    let mut degenerate_triangles = 0usize;
    let mut total_edge_length = 0.0f32;
    let mut edge_count = 0usize;

    for t in 0..triangle_count {
        let i0 = indices[t * 3] as usize;
        let i1 = indices[t * 3 + 1] as usize;
        let i2 = indices[t * 3 + 2] as usize;

        if i0 * 3 + 2 >= vertices.len() || i1 * 3 + 2 >= vertices.len() || i2 * 3 + 2 >= vertices.len() {
            degenerate_triangles += 1;
            continue;
        }

        let v0 = Vec3::new(vertices[i0 * 3], vertices[i0 * 3 + 1], vertices[i0 * 3 + 2]);
        let v1 = Vec3::new(vertices[i1 * 3], vertices[i1 * 3 + 1], vertices[i1 * 3 + 2]);
        let v2 = Vec3::new(vertices[i2 * 3], vertices[i2 * 3 + 1], vertices[i2 * 3 + 2]);

        let area = (v1 - v0).cross(v2 - v0).length() * 0.5;
        if area < 1e-10 {
            degenerate_triangles += 1;
        }

        let e0 = (v1 - v0).length();
        let e1 = (v2 - v1).length();
        let e2 = (v0 - v2).length();
        total_edge_length += e0 + e1 + e2;
        edge_count += 3;
    }

    let average_edge_length = if edge_count > 0 {
        total_edge_length / edge_count as f32
    } else {
        0.0
    };

    let memory_bytes = vertices.len() * std::mem::size_of::<f32>()
        + normals.len() * std::mem::size_of::<f32>()
        + indices.len() * std::mem::size_of::<u32>();

    MeshStats {
        vertex_count,
        triangle_count,
        unique_vertex_count,
        degenerate_triangles,
        average_edge_length,
        bounds_min,
        bounds_max,
        memory_bytes,
    }
}

/// Remove duplicate vertices (within epsilon tolerance), remapping indices.
///
/// Two vertices are considered duplicates if both their position and normal
/// components are within `epsilon` of each other.
pub fn deduplicate_vertices(
    vertices: &[f32],
    normals: &[f32],
    indices: &[u32],
    epsilon: f32,
) -> (Vec<f32>, Vec<f32>, Vec<u32>) {
    let vertex_count = vertices.len() / 3;
    let eps2 = epsilon * epsilon;

    // Map from old index to new index
    let mut remap = vec![0u32; vertex_count];
    let mut new_vertices = Vec::new();
    let mut new_normals = Vec::new();
    let mut new_count = 0u32;

    for i in 0..vertex_count {
        let px = vertices[i * 3];
        let py = vertices[i * 3 + 1];
        let pz = vertices[i * 3 + 2];

        let nx = if i * 3 + 2 < normals.len() { normals[i * 3] } else { 0.0 };
        let ny = if i * 3 + 2 < normals.len() { normals[i * 3 + 1] } else { 0.0 };
        let nz = if i * 3 + 2 < normals.len() { normals[i * 3 + 2] } else { 0.0 };

        // Check if this vertex matches an existing one
        let mut found = None;
        let existing_count = new_vertices.len() / 3;
        for j in 0..existing_count {
            let dx = new_vertices[j * 3] - px;
            let dy = new_vertices[j * 3 + 1] - py;
            let dz = new_vertices[j * 3 + 2] - pz;
            if dx * dx + dy * dy + dz * dz < eps2 {
                let dnx = new_normals[j * 3] - nx;
                let dny = new_normals[j * 3 + 1] - ny;
                let dnz = new_normals[j * 3 + 2] - nz;
                if dnx * dnx + dny * dny + dnz * dnz < eps2 {
                    found = Some(j as u32);
                    break;
                }
            }
        }

        if let Some(existing_idx) = found {
            remap[i] = existing_idx;
        } else {
            remap[i] = new_count;
            new_vertices.push(px);
            new_vertices.push(py);
            new_vertices.push(pz);
            new_normals.push(nx);
            new_normals.push(ny);
            new_normals.push(nz);
            new_count += 1;
        }
    }

    // Remap indices
    let new_indices: Vec<u32> = indices
        .iter()
        .map(|&idx| {
            if (idx as usize) < remap.len() {
                remap[idx as usize]
            } else {
                idx
            }
        })
        .collect();

    (new_vertices, new_normals, new_indices)
}

/// Optimize vertex cache order using a simplified Forsyth algorithm.
///
/// Reorders triangles to improve GPU vertex cache utilization. Uses a
/// simulated FIFO cache of `CACHE_SIZE` entries and emits triangles
/// in a best-score-first order.
pub fn optimize_vertex_cache(indices: &mut [u32], vertex_count: usize) {
    const CACHE_SIZE: usize = 16;

    let triangle_count = indices.len() / 3;
    if triangle_count <= 1 || vertex_count == 0 {
        return;
    }

    // Build adjacency: for each vertex, which triangles use it
    let mut vert_tris: Vec<Vec<usize>> = vec![Vec::new(); vertex_count];
    for t in 0..triangle_count {
        for k in 0..3 {
            let v = indices[t * 3 + k] as usize;
            if v < vertex_count {
                vert_tris[v].push(t);
            }
        }
    }

    // Simulated cache (FIFO)
    let mut cache: Vec<u32> = Vec::with_capacity(CACHE_SIZE);
    let mut emitted = vec![false; triangle_count];
    let mut result = Vec::with_capacity(indices.len());

    // Score a triangle: count how many of its 3 vertices are in the cache
    let score_tri = |tri_idx: usize, cache: &[u32], indices: &[u32]| -> usize {
        let mut score = 0;
        for k in 0..3 {
            let v = indices[tri_idx * 3 + k];
            if cache.contains(&v) {
                score += 1;
            }
        }
        score
    };

    // Emit all triangles
    let mut emitted_count = 0;
    while emitted_count < triangle_count {
        // Find the best triangle to emit next
        // First, search among triangles that share vertices with the cache
        let mut best_tri = None;
        let mut best_score = 0;

        for &v in &cache {
            let v = v as usize;
            if v >= vertex_count { continue; }
            for &t in &vert_tris[v] {
                if emitted[t] { continue; }
                let score = score_tri(t, &cache, indices);
                if score > best_score || best_tri.is_none() {
                    best_score = score;
                    best_tri = Some(t);
                }
            }
        }

        // If no cache-friendly triangle found, pick the first unemitted one
        if best_tri.is_none() {
            for t in 0..triangle_count {
                if !emitted[t] {
                    best_tri = Some(t);
                    break;
                }
            }
        }

        let tri = match best_tri {
            Some(t) => t,
            None => break,
        };

        emitted[tri] = true;
        emitted_count += 1;

        // Emit the triangle
        for k in 0..3 {
            let v = indices[tri * 3 + k];
            result.push(v);

            // Update cache (FIFO)
            if !cache.contains(&v) {
                if cache.len() >= CACHE_SIZE {
                    cache.remove(0);
                }
                cache.push(v);
            }
        }
    }

    // Copy result back
    indices[..result.len()].copy_from_slice(&result);
}

/// Remove degenerate triangles (triangles with area below `min_area`).
///
/// Returns cleaned vertices, normals, and indices. Vertices that are no longer
/// referenced after removing degenerate triangles are also removed.
pub fn remove_degenerate_triangles(
    vertices: &[f32],
    normals: &[f32],
    indices: &[u32],
    min_area: f32,
) -> (Vec<f32>, Vec<f32>, Vec<u32>) {
    let triangle_count = indices.len() / 3;
    let mut kept_indices = Vec::new();

    for t in 0..triangle_count {
        let i0 = indices[t * 3] as usize;
        let i1 = indices[t * 3 + 1] as usize;
        let i2 = indices[t * 3 + 2] as usize;

        if i0 * 3 + 2 >= vertices.len() || i1 * 3 + 2 >= vertices.len() || i2 * 3 + 2 >= vertices.len() {
            continue; // skip out-of-bounds
        }

        let v0 = Vec3::new(vertices[i0 * 3], vertices[i0 * 3 + 1], vertices[i0 * 3 + 2]);
        let v1 = Vec3::new(vertices[i1 * 3], vertices[i1 * 3 + 1], vertices[i1 * 3 + 2]);
        let v2 = Vec3::new(vertices[i2 * 3], vertices[i2 * 3 + 1], vertices[i2 * 3 + 2]);

        let area = (v1 - v0).cross(v2 - v0).length() * 0.5;
        if area >= min_area {
            kept_indices.push(indices[t * 3]);
            kept_indices.push(indices[t * 3 + 1]);
            kept_indices.push(indices[t * 3 + 2]);
        }
    }

    // Compact: collect only referenced vertices
    let vertex_count = vertices.len() / 3;
    let mut used = vec![false; vertex_count];
    for &idx in &kept_indices {
        if (idx as usize) < vertex_count {
            used[idx as usize] = true;
        }
    }

    let mut remap = vec![0u32; vertex_count];
    let mut new_vertices = Vec::new();
    let mut new_normals = Vec::new();
    let mut new_count = 0u32;

    for i in 0..vertex_count {
        if used[i] {
            remap[i] = new_count;
            new_vertices.push(vertices[i * 3]);
            new_vertices.push(vertices[i * 3 + 1]);
            new_vertices.push(vertices[i * 3 + 2]);
            if i * 3 + 2 < normals.len() {
                new_normals.push(normals[i * 3]);
                new_normals.push(normals[i * 3 + 1]);
                new_normals.push(normals[i * 3 + 2]);
            } else {
                new_normals.push(0.0);
                new_normals.push(1.0);
                new_normals.push(0.0);
            }
            new_count += 1;
        }
    }

    let new_indices: Vec<u32> = kept_indices
        .iter()
        .map(|&idx| remap[idx as usize])
        .collect();

    (new_vertices, new_normals, new_indices)
}

/// Number of segments for circular approximation.
const CIRCLE_SEGMENTS: usize = 24;
/// Number of steps for revolution approximation.
const REVOLVE_STEPS: usize = 24;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Tessellate a product's geometry into a world-space `Mesh`.
///
/// Returns `None` if the product has no usable geometry representation.
///
/// If `geometry_cache` is `Some`, MappedItem deduplication is used: shared
/// representation sources are only tessellated once and then the cached mesh
/// is cloned + transformed for subsequent uses.
pub fn tessellate_product(
    product_id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    placement_cache: &mut PlacementCache,
    color: [f32; 4],
) -> Option<Mesh> {
    tessellate_product_cached(product_id, entities, placement_cache, color, None)
}

/// Like [`tessellate_product`] but accepts an optional `GeometryCache` wrapped
/// in a `Mutex` for MappedItem deduplication across parallel threads.
pub fn tessellate_product_cached(
    product_id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    placement_cache: &mut PlacementCache,
    color: [f32; 4],
    geometry_cache: Option<&Mutex<GeometryCache>>,
) -> Option<Mesh> {
    let product = entities.get(&product_id)?;

    // Resolve world placement (attribute 5 = ObjectPlacement)
    let world_matrix = super::placement::resolve_product_placement(product_id, entities, placement_cache);

    // Get representation (attribute 6 = Representation -> IFCPRODUCTDEFINITIONSHAPE)
    let rep_id = product.get_entity_ref(6)?;
    let rep_entity = entities.get(&rep_id)?;

    // Find shape representations
    let shape_rep_ids = find_body_representations(rep_entity, entities);
    if shape_rep_ids.is_empty() {
        return None;
    }

    let mut combined_mesh = Mesh::new();

    for shape_rep_id in shape_rep_ids {
        let shape_rep = match entities.get(&shape_rep_id) {
            Some(e) => e,
            None => continue,
        };

        // Get items list (attribute 3 for IFCSHAPEREPRESENTATION)
        let items = match shape_rep.get_list(3) {
            Some(list) => list.clone(),
            None => continue,
        };

        for item_val in &items {
            let item_id = match item_val {
                IfcValue::EntityRef(id) => *id,
                _ => continue,
            };

            if let Some(item_mesh) = tessellate_geometry_item_cached(item_id, entities, placement_cache, color, geometry_cache) {
                append_mesh(&mut combined_mesh, &item_mesh);
            }
        }
    }

    if combined_mesh.vertices.is_empty() {
        return None;
    }

    // Apply world transform
    transform_mesh(&mut combined_mesh, &world_matrix);

    Some(combined_mesh)
}

// ---------------------------------------------------------------------------
// Representation finding
// ---------------------------------------------------------------------------

/// Find Body shape representations from an IFCPRODUCTDEFINITIONSHAPE.
/// IFCPRODUCTDEFINITIONSHAPE has Representations at attribute 2.
fn find_body_representations(
    prod_def_shape: &IfcEntity,
    entities: &HashMap<EntityId, IfcEntity>,
) -> Vec<EntityId> {
    let mut result = Vec::new();

    let reps = match prod_def_shape.get_list(2) {
        Some(list) => list,
        None => return result,
    };

    for val in reps {
        let rep_id = match val {
            IfcValue::EntityRef(id) => *id,
            _ => continue,
        };
        let rep = match entities.get(&rep_id) {
            Some(e) => e,
            None => continue,
        };

        if !rep.entity_type.eq_ignore_ascii_case("IFCSHAPEREPRESENTATION") {
            continue;
        }

        // Check RepresentationIdentifier (attribute 1) for "Body"
        let identifier = rep.get_string(1).unwrap_or_default();
        if identifier.eq_ignore_ascii_case("Body")
            || identifier.eq_ignore_ascii_case("Facetation")
            || identifier.eq_ignore_ascii_case("Tessellation")
        {
            result.push(rep_id);
        }
    }

    // If no "Body" found, try all representations
    if result.is_empty() {
        for val in reps {
            if let IfcValue::EntityRef(id) = val {
                if entities.get(id).map_or(false, |e| {
                    e.entity_type.eq_ignore_ascii_case("IFCSHAPEREPRESENTATION")
                }) {
                    result.push(*id);
                }
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Geometry item dispatch
// ---------------------------------------------------------------------------

/// Dispatch a geometry item by entity type.
fn tessellate_geometry_item(
    item_id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    placement_cache: &mut PlacementCache,
    color: [f32; 4],
) -> Option<Mesh> {
    tessellate_geometry_item_cached(item_id, entities, placement_cache, color, None)
}

/// Dispatch a geometry item by entity type, with optional geometry cache.
fn tessellate_geometry_item_cached(
    item_id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    placement_cache: &mut PlacementCache,
    color: [f32; 4],
    geometry_cache: Option<&Mutex<GeometryCache>>,
) -> Option<Mesh> {
    let entity = entities.get(&item_id)?;
    let entity_type = entity.entity_type.as_str();

    match entity_type {
        t if t.eq_ignore_ascii_case("IFCFACETEDBREP") => {
            tessellate_faceted_brep(item_id, entities, color)
        }
        t if t.eq_ignore_ascii_case("IFCEXTRUDEDAREASOLID") => {
            tessellate_extruded_area_solid(item_id, entities, color)
        }
        t if t.eq_ignore_ascii_case("IFCREVOLVEDAREASOLID") => {
            tessellate_revolved_area_solid(item_id, entities, color)
        }
        t if t.eq_ignore_ascii_case("IFCBOOLEANCLIPPINGRESULT")
            || t.eq_ignore_ascii_case("IFCBOOLEANRESULT") =>
        {
            tessellate_boolean_result(item_id, entities, placement_cache, color)
        }
        t if t.eq_ignore_ascii_case("IFCHALFSPACESOLID")
            || t.eq_ignore_ascii_case("IFCPOLYGONALBOUNDEDHALFSPACE") =>
        {
            // These are handled inside boolean operations; skip if encountered standalone
            None
        }
        t if t.eq_ignore_ascii_case("IFCTRIANGULATEDFACESET") => {
            tessellate_triangulated_face_set(item_id, entities, color)
        }
        t if t.eq_ignore_ascii_case("IFCPOLYGONALFACESET") => {
            tessellate_polygonal_face_set(item_id, entities, color)
        }
        t if t.eq_ignore_ascii_case("IFCSHELLBASEDSURFACEMODEL") => {
            tessellate_shell_based_surface_model(item_id, entities, color)
        }
        t if t.eq_ignore_ascii_case("IFCFACEBASEDSURFACEMODEL") => {
            tessellate_face_based_surface_model(item_id, entities, color)
        }
        t if t.eq_ignore_ascii_case("IFCSWEPTDISKSOLID") => {
            tessellate_swept_disk_solid(item_id, entities, color)
        }
        t if t.eq_ignore_ascii_case("IFCMAPPEDITEM") => {
            tessellate_mapped_item_cached(item_id, entities, placement_cache, color, geometry_cache)
        }
        t if t.eq_ignore_ascii_case("IFCSURFACECURVESWEPTAREASOLID") => {
            tessellate_surface_curve_swept(item_id, entities, color)
        }
        _ => {
            tracing::debug!("Unsupported geometry item: {} (#{item_id})", entity_type);
            None
        }
    }
}

// ---------------------------------------------------------------------------
// IFCFACETEDBREP
// ---------------------------------------------------------------------------

/// IFCFACETEDBREP(Outer: IFCCLOSEDSHELL)
fn tessellate_faceted_brep(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
) -> Option<Mesh> {
    let entity = entities.get(&id)?;
    let shell_id = entity.get_entity_ref(0)?;
    tessellate_shell(shell_id, entities, color)
}

/// Tessellate an IFCCLOSEDSHELL or IFCOPENSHELL.
/// Shell(CfsFaces: LIST OF IFCFACE)
fn tessellate_shell(
    shell_id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
) -> Option<Mesh> {
    let shell = entities.get(&shell_id)?;
    let faces = shell.get_list(0)?;

    let mut mesh = Mesh::new();

    for face_val in faces {
        let face_id = match face_val {
            IfcValue::EntityRef(id) => *id,
            _ => continue,
        };
        tessellate_face(face_id, entities, color, &mut mesh);
    }

    if mesh.vertices.is_empty() {
        None
    } else {
        Some(mesh)
    }
}

/// Tessellate a single IFCFACE.
/// IFCFACE(Bounds: LIST OF IFCFACEBOUND)
fn tessellate_face(
    face_id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
    mesh: &mut Mesh,
) {
    let face = match entities.get(&face_id) {
        Some(e) => e,
        None => return,
    };

    let bounds = match face.get_list(0) {
        Some(list) => list.clone(),
        None => return,
    };

    for bound_val in &bounds {
        let bound_id = match bound_val {
            IfcValue::EntityRef(id) => *id,
            _ => continue,
        };

        let bound = match entities.get(&bound_id) {
            Some(e) => e,
            None => continue,
        };

        // IFCFACEOUTERBOUND / IFCFACEBOUND(Bound: IFCLOOP, Orientation: BOOLEAN)
        let loop_id = match bound.get_entity_ref(0) {
            Some(id) => id,
            None => continue,
        };

        let points = read_loop_points(loop_id, entities);
        if points.len() < 3 {
            continue;
        }

        // Compute face normal from first 3 points
        let normal = compute_polygon_normal(&points);

        let triangles = triangulate_polygon(&points, normal);
        let base = (mesh.vertices.len() / 3) as u32;

        // Add vertices
        for pt in &points {
            mesh.add_vertex(pt.x, pt.y, pt.z);
            mesh.add_normal(normal.x, normal.y, normal.z);
            mesh.add_color(color[0], color[1], color[2], color[3]);
        }

        // Add triangles
        for tri in &triangles {
            mesh.add_triangle(base + tri[0] as u32, base + tri[1] as u32, base + tri[2] as u32);
        }
    }
}

/// Read loop points from IFCPOLYLOOP.
/// IFCPOLYLOOP(Polygon: LIST OF IFCCARTESIANPOINT)
fn read_loop_points(loop_id: EntityId, entities: &HashMap<EntityId, IfcEntity>) -> Vec<Vec3> {
    let loop_entity = match entities.get(&loop_id) {
        Some(e) => e,
        None => return Vec::new(),
    };

    if loop_entity.entity_type.eq_ignore_ascii_case("IFCPOLYLOOP") {
        let point_list = match loop_entity.get_list(0) {
            Some(list) => list,
            None => return Vec::new(),
        };

        point_list
            .iter()
            .filter_map(|v| match v {
                IfcValue::EntityRef(id) => read_cartesian_point(*id, entities),
                _ => None,
            })
            .collect()
    } else {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// IFCEXTRUDEDAREASOLID
// ---------------------------------------------------------------------------

/// IFCEXTRUDEDAREASOLID(SweptArea, Position, ExtrudedDirection, Depth)
fn tessellate_extruded_area_solid(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
) -> Option<Mesh> {
    let entity = entities.get(&id)?;

    // SweptArea (attribute 0) - profile
    let profile_id = entity.get_entity_ref(0)?;

    // Position (attribute 1) - IFCAXIS2PLACEMENT3D (local position of extrusion)
    let local_matrix = entity
        .get_entity_ref(1)
        .map(|pos_id| axis2_placement_3d_to_matrix(pos_id, entities))
        .unwrap_or(Mat4::IDENTITY);

    // ExtrudedDirection (attribute 2) - IFCDIRECTION
    let extrude_dir = entity
        .get_entity_ref(2)
        .and_then(|dir_id| read_direction(dir_id, entities))
        .unwrap_or(Vec3::Z);

    // Depth (attribute 3) - REAL
    let depth = entity.get_real(3)? as f32;

    // Get profile points
    let profile_points = get_profile_points(profile_id, entities);
    if profile_points.len() < 3 {
        return None;
    }

    let extrusion_vec = extrude_dir * depth;
    let mut mesh = build_extrusion_mesh(&profile_points, extrusion_vec, color);

    // Apply local position
    transform_mesh(&mut mesh, &local_matrix);

    Some(mesh)
}

/// Build a mesh from extruding a profile along a vector.
fn build_extrusion_mesh(profile: &[Vec3], extrusion: Vec3, color: [f32; 4]) -> Mesh {
    let mut mesh = Mesh::new();
    let n = profile.len();

    // Bottom cap normal (pointing opposite to extrusion)
    let cap_normal = compute_polygon_normal(profile);
    let bottom_normal = -cap_normal;
    let top_normal = cap_normal;

    // Bottom cap vertices
    let bottom_base = 0u32;
    for pt in profile {
        mesh.add_vertex(pt.x, pt.y, pt.z);
        mesh.add_normal(bottom_normal.x, bottom_normal.y, bottom_normal.z);
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }

    // Top cap vertices
    let top_base = n as u32;
    for pt in profile {
        let top = *pt + extrusion;
        mesh.add_vertex(top.x, top.y, top.z);
        mesh.add_normal(top_normal.x, top_normal.y, top_normal.z);
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }

    // Triangulate bottom cap (reverse winding for outward-facing normal)
    let bottom_tris = triangulate_polygon(profile, cap_normal);
    for tri in &bottom_tris {
        mesh.add_triangle(
            bottom_base + tri[0] as u32,
            bottom_base + tri[2] as u32,
            bottom_base + tri[1] as u32,
        );
    }

    // Triangulate top cap (normal winding)
    let top_profile: Vec<Vec3> = profile.iter().map(|p| *p + extrusion).collect();
    let top_tris = triangulate_polygon(&top_profile, cap_normal);
    for tri in &top_tris {
        mesh.add_triangle(
            top_base + tri[0] as u32,
            top_base + tri[1] as u32,
            top_base + tri[2] as u32,
        );
    }

    // Side faces
    let side_base = (2 * n) as u32;
    for i in 0..n {
        let j = (i + 1) % n;
        let p0 = profile[i];
        let p1 = profile[j];
        let p2 = p1 + extrusion;
        let p3 = p0 + extrusion;

        // Compute side normal
        let edge = p1 - p0;
        let up = extrusion;
        let side_normal = edge.cross(up).normalize();

        let base = side_base + (i as u32) * 4;
        mesh.add_vertex(p0.x, p0.y, p0.z);
        mesh.add_vertex(p1.x, p1.y, p1.z);
        mesh.add_vertex(p2.x, p2.y, p2.z);
        mesh.add_vertex(p3.x, p3.y, p3.z);
        for _ in 0..4 {
            mesh.add_normal(side_normal.x, side_normal.y, side_normal.z);
            mesh.add_color(color[0], color[1], color[2], color[3]);
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base + 2, base + 3, base);
    }

    mesh
}

// ---------------------------------------------------------------------------
// IFCREVOLVEDAREASOLID
// ---------------------------------------------------------------------------

/// IFCREVOLVEDAREASOLID(SweptArea, Position, Axis, Angle)
fn tessellate_revolved_area_solid(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
) -> Option<Mesh> {
    let entity = entities.get(&id)?;

    let profile_id = entity.get_entity_ref(0)?;
    let local_matrix = entity
        .get_entity_ref(1)
        .map(|pos_id| axis2_placement_3d_to_matrix(pos_id, entities))
        .unwrap_or(Mat4::IDENTITY);

    // Axis (attribute 2) - IFCAXIS1PLACEMENT(Location, Axis)
    let axis_entity_id = entity.get_entity_ref(2)?;
    let axis_entity = entities.get(&axis_entity_id)?;
    let axis_origin = axis_entity
        .get_entity_ref(0)
        .and_then(|id| read_cartesian_point(id, entities))
        .unwrap_or(Vec3::ZERO);
    let axis_dir = axis_entity
        .get_entity_ref(1)
        .and_then(|id| read_direction(id, entities))
        .unwrap_or(Vec3::Y);

    let angle_degrees = entity.get_real(3).unwrap_or(360.0) as f32;
    let angle_rad = angle_degrees.to_radians();

    let profile_points = get_profile_points(profile_id, entities);
    if profile_points.len() < 2 {
        return None;
    }

    let steps = REVOLVE_STEPS;
    let mut mesh = Mesh::new();

    // Generate revolution strips
    for step in 0..steps {
        let t0 = step as f32 / steps as f32 * angle_rad;
        let t1 = (step + 1) as f32 / steps as f32 * angle_rad;

        let rot0 = Mat4::from_axis_angle(axis_dir, t0)
            * Mat4::from_translation(-axis_origin);
        let rot1 = Mat4::from_axis_angle(axis_dir, t1)
            * Mat4::from_translation(-axis_origin);

        let translate_back = Mat4::from_translation(axis_origin);
        let m0 = translate_back * rot0;
        let m1 = translate_back * rot1;

        for i in 0..profile_points.len() {
            let j = (i + 1) % profile_points.len();

            let p0 = m0.transform_point3(profile_points[i]);
            let p1 = m0.transform_point3(profile_points[j]);
            let p2 = m1.transform_point3(profile_points[j]);
            let p3 = m1.transform_point3(profile_points[i]);

            let normal = (p1 - p0).cross(p3 - p0).normalize();
            let base = (mesh.vertices.len() / 3) as u32;

            mesh.add_vertex(p0.x, p0.y, p0.z);
            mesh.add_vertex(p1.x, p1.y, p1.z);
            mesh.add_vertex(p2.x, p2.y, p2.z);
            mesh.add_vertex(p3.x, p3.y, p3.z);
            for _ in 0..4 {
                mesh.add_normal(normal.x, normal.y, normal.z);
                mesh.add_color(color[0], color[1], color[2], color[3]);
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 2, base + 3, base);
        }
    }

    transform_mesh(&mut mesh, &local_matrix);

    if mesh.vertices.is_empty() {
        None
    } else {
        Some(mesh)
    }
}

// ---------------------------------------------------------------------------
// IFCBOOLEANCLIPPINGRESULT / IFCBOOLEANRESULT
// ---------------------------------------------------------------------------

/// IFCBOOLEANCLIPPINGRESULT / IFCBOOLEANRESULT(Operator, FirstOperand, SecondOperand)
///
/// Supports DIFFERENCE operations where the second operand is an IFCHALFSPACESOLID
/// or IFCPOLYGONALBOUNDEDHALFSPACE.  Falls back to tessellating the first operand
/// only when the second operand type is not recognised.
fn tessellate_boolean_result(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    placement_cache: &mut PlacementCache,
    color: [f32; 4],
) -> Option<Mesh> {
    let entity = entities.get(&id)?;

    // Operator (attribute 0) — enum, e.g. ".DIFFERENCE."
    let is_difference = match entity.get_attr(0) {
        Some(IfcValue::Enum(s)) => s.eq_ignore_ascii_case("DIFFERENCE")
            || s.eq_ignore_ascii_case(".DIFFERENCE."),
        _ => false,
    };

    // FirstOperand (attribute 1)
    let first_operand_id = entity.get_entity_ref(1)?;
    let mut mesh = tessellate_geometry_item(first_operand_id, entities, placement_cache, color)?;

    // SecondOperand (attribute 2)
    let second_operand_id = match entity.get_entity_ref(2) {
        Some(id) => id,
        None => return Some(mesh),
    };

    if !is_difference {
        // Only DIFFERENCE is supported; for other operators return the first operand.
        return Some(mesh);
    }

    let second = match entities.get(&second_operand_id) {
        Some(e) => e,
        None => return Some(mesh),
    };

    let second_type = second.entity_type.as_str();

    if second_type.eq_ignore_ascii_case("IFCHALFSPACESOLID")
        || second_type.eq_ignore_ascii_case("IFCPOLYGONALBOUNDEDHALFSPACE")
    {
        // Parse the half-space plane
        if let Some(clipped) = apply_half_space_clip(&mesh, second, entities, color) {
            mesh = clipped;
        }
    } else if second_type.eq_ignore_ascii_case("IFCBOOLEANCLIPPINGRESULT")
        || second_type.eq_ignore_ascii_case("IFCBOOLEANRESULT")
    {
        // Recursive boolean — the second operand is itself a boolean result.
        // We cannot subtract a boolean tree directly, so just return first operand.
        // (Full CSG would require volume representations.)
    }

    if mesh.vertices.is_empty() {
        None
    } else {
        Some(mesh)
    }
}

/// Extract plane parameters from an IFCHALFSPACESOLID and clip the mesh.
fn apply_half_space_clip(
    mesh: &Mesh,
    half_space: &IfcEntity,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
) -> Option<Mesh> {
    // BaseSurface (attribute 0) → IFCPLANE → Position: IFCAXIS2PLACEMENT3D
    let surface_id = half_space.get_entity_ref(0)?;
    let surface = entities.get(&surface_id)?;

    // IFCPLANE(Position: IFCAXIS2PLACEMENT3D)
    let placement_id = surface.get_entity_ref(0)?;
    let placement = entities.get(&placement_id)?;

    let plane_origin = placement
        .get_entity_ref(0)
        .and_then(|id| read_cartesian_point(id, entities))
        .unwrap_or(Vec3::ZERO);

    // Axis (attribute 1) of the placement gives the plane normal
    let plane_normal = placement
        .get_entity_ref(1)
        .and_then(|id| read_direction(id, entities))
        .unwrap_or(Vec3::Z);

    // AgreementFlag (attribute 1 of half-space)
    let agreement = match half_space.get_attr(1) {
        Some(IfcValue::Boolean(b)) => *b,
        Some(IfcValue::Enum(s)) => s.eq_ignore_ascii_case(".T.") || s.eq_ignore_ascii_case("TRUE"),
        _ => true,
    };

    // For a DIFFERENCE boolean, we want to *remove* the half-space solid from the
    // first operand.  The half-space solid represents the set of points on the
    // agreement side of the plane.  So we *keep* the points on the opposite side.
    clip_mesh_by_plane(mesh, plane_origin, plane_normal, !agreement, color)
}

/// Clip a triangle mesh against an infinite plane.
///
/// Keeps the geometry on one side of the plane, splits triangles that straddle it.
///
/// * `plane_origin` — a point on the plane
/// * `plane_normal` — the outward normal of the plane (unit length)
/// * `keep_positive` — if true, keep vertices where dot(v - origin, normal) >= 0
/// * `color` — vertex colour for newly created intersection vertices
fn clip_mesh_by_plane(
    mesh: &Mesh,
    plane_origin: Vec3,
    plane_normal: Vec3,
    keep_positive: bool,
    color: [f32; 4],
) -> Option<Mesh> {
    let normal = plane_normal.normalize();
    let mut out = Mesh::new();

    let num_triangles = mesh.indices.len() / 3;
    for t in 0..num_triangles {
        let idx0 = mesh.indices[t * 3] as usize;
        let idx1 = mesh.indices[t * 3 + 1] as usize;
        let idx2 = mesh.indices[t * 3 + 2] as usize;

        let v0 = Vec3::new(
            mesh.vertices[idx0 * 3],
            mesh.vertices[idx0 * 3 + 1],
            mesh.vertices[idx0 * 3 + 2],
        );
        let v1 = Vec3::new(
            mesh.vertices[idx1 * 3],
            mesh.vertices[idx1 * 3 + 1],
            mesh.vertices[idx1 * 3 + 2],
        );
        let v2 = Vec3::new(
            mesh.vertices[idx2 * 3],
            mesh.vertices[idx2 * 3 + 1],
            mesh.vertices[idx2 * 3 + 2],
        );

        let n0 = Vec3::new(
            mesh.normals[idx0 * 3],
            mesh.normals[idx0 * 3 + 1],
            mesh.normals[idx0 * 3 + 2],
        );
        let n1 = Vec3::new(
            mesh.normals[idx1 * 3],
            mesh.normals[idx1 * 3 + 1],
            mesh.normals[idx1 * 3 + 2],
        );
        let n2 = Vec3::new(
            mesh.normals[idx2 * 3],
            mesh.normals[idx2 * 3 + 1],
            mesh.normals[idx2 * 3 + 2],
        );

        let d0 = (v0 - plane_origin).dot(normal);
        let d1 = (v1 - plane_origin).dot(normal);
        let d2 = (v2 - plane_origin).dot(normal);

        let keep0 = if keep_positive { d0 >= 0.0 } else { d0 <= 0.0 };
        let keep1 = if keep_positive { d1 >= 0.0 } else { d1 <= 0.0 };
        let keep2 = if keep_positive { d2 >= 0.0 } else { d2 <= 0.0 };

        let kept_count = keep0 as u8 + keep1 as u8 + keep2 as u8;

        if kept_count == 3 {
            // All inside — keep triangle as-is
            emit_triangle(&mut out, v0, v1, v2, n0, n1, n2, color);
        } else if kept_count == 0 {
            // All outside — discard
        } else {
            // Mixed — need to clip
            let verts = [v0, v1, v2];
            let norms = [n0, n1, n2];
            let dists = [d0, d1, d2];
            let keeps = [keep0, keep1, keep2];

            clip_triangle(&mut out, &verts, &norms, &dists, &keeps, color);
        }
    }

    if out.vertices.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Emit a single triangle into the output mesh.
fn emit_triangle(
    mesh: &mut Mesh,
    v0: Vec3, v1: Vec3, v2: Vec3,
    n0: Vec3, n1: Vec3, n2: Vec3,
    color: [f32; 4],
) {
    let base = (mesh.vertices.len() / 3) as u32;
    mesh.add_vertex(v0.x, v0.y, v0.z);
    mesh.add_vertex(v1.x, v1.y, v1.z);
    mesh.add_vertex(v2.x, v2.y, v2.z);
    mesh.add_normal(n0.x, n0.y, n0.z);
    mesh.add_normal(n1.x, n1.y, n1.z);
    mesh.add_normal(n2.x, n2.y, n2.z);
    for _ in 0..3 {
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }
    mesh.add_triangle(base, base + 1, base + 2);
}

/// Clip a mixed triangle (some vertices inside, some outside) and emit 1-2 triangles.
fn clip_triangle(
    mesh: &mut Mesh,
    verts: &[Vec3; 3],
    norms: &[Vec3; 3],
    dists: &[f32; 3],
    keeps: &[bool; 3],
    color: [f32; 4],
) {
    let kept_count = keeps[0] as u8 + keeps[1] as u8 + keeps[2] as u8;

    if kept_count == 1 {
        // One vertex inside → single output triangle
        let alone = if keeps[0] { 0 } else if keeps[1] { 1 } else { 2 };
        let next = (alone + 1) % 3;
        let prev = (alone + 2) % 3;

        let (int_a, norm_a) = edge_plane_intersection(
            verts[alone], verts[next], norms[alone], norms[next], dists[alone], dists[next],
        );
        let (int_b, norm_b) = edge_plane_intersection(
            verts[alone], verts[prev], norms[alone], norms[prev], dists[alone], dists[prev],
        );

        emit_triangle(mesh, verts[alone], int_a, int_b, norms[alone], norm_a, norm_b, color);
    } else {
        // Two vertices inside → two output triangles (a quad)
        let alone = if !keeps[0] { 0 } else if !keeps[1] { 1 } else { 2 };
        let next = (alone + 1) % 3;
        let prev = (alone + 2) % 3;

        let (int_a, norm_a) = edge_plane_intersection(
            verts[alone], verts[next], norms[alone], norms[next], dists[alone], dists[next],
        );
        let (int_b, norm_b) = edge_plane_intersection(
            verts[alone], verts[prev], norms[alone], norms[prev], dists[alone], dists[prev],
        );

        // Triangle 1: next, int_a is not right — need to preserve winding.
        // The two kept vertices are `next` and `prev`.
        // int_a is on edge alone→next, int_b is on edge alone→prev.
        emit_triangle(mesh, verts[next], verts[prev], int_b, norms[next], norms[prev], norm_b, color);
        emit_triangle(mesh, verts[next], int_b, int_a, norms[next], norm_b, norm_a, color);
    }
}

/// Compute the intersection point of an edge with the clipping plane.
/// Returns the interpolated position and normal.
fn edge_plane_intersection(
    v0: Vec3, v1: Vec3,
    n0: Vec3, n1: Vec3,
    d0: f32, d1: f32,
) -> (Vec3, Vec3) {
    let denom = d0 - d1;
    let t = if denom.abs() < 1e-10 { 0.5 } else { d0 / denom };
    let t = t.clamp(0.0, 1.0);
    let pos = v0 + (v1 - v0) * t;
    let norm = (n0 + (n1 - n0) * t).normalize_or_zero();
    (pos, norm)
}

// ---------------------------------------------------------------------------
// IFCTRIANGULATEDFACESET (IFC4)
// ---------------------------------------------------------------------------

/// IFCTRIANGULATEDFACESET(Coordinates, Normals, Closed, CoordIndex, NormalIndex)
/// Coordinates: IFCCARTESIANPOINTLIST3D
fn tessellate_triangulated_face_set(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
) -> Option<Mesh> {
    let entity = entities.get(&id)?;

    // Coordinates (attribute 0) -> IFCCARTESIANPOINTLIST3D
    let coord_list_id = entity.get_entity_ref(0)?;
    let points = read_cartesian_point_list_3d(coord_list_id, entities)?;

    // CoordIndex (attribute 3) - list of index triples
    let coord_index = entity.get_list(3)?;

    let mut mesh = Mesh::new();

    // Add all points as vertices
    for pt in &points {
        mesh.add_vertex(pt.x, pt.y, pt.z);
        mesh.add_normal(0.0, 1.0, 0.0); // placeholder, will compute per-face
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }

    // Process index triples
    for tri_val in coord_index {
        let tri_list = match tri_val {
            IfcValue::List(list) => list,
            _ => continue,
        };
        if tri_list.len() < 3 {
            continue;
        }
        // IFC indices are 1-based
        let i0 = value_as_u32(&tri_list[0])?.wrapping_sub(1);
        let i1 = value_as_u32(&tri_list[1])?.wrapping_sub(1);
        let i2 = value_as_u32(&tri_list[2])?.wrapping_sub(1);

        if (i0 as usize) < points.len()
            && (i1 as usize) < points.len()
            && (i2 as usize) < points.len()
        {
            mesh.add_triangle(i0, i1, i2);

            // Compute and set face normal
            let normal = (points[i1 as usize] - points[i0 as usize])
                .cross(points[i2 as usize] - points[i0 as usize])
                .normalize_or_zero();
            for idx in [i0, i1, i2] {
                let n_offset = idx as usize * 3;
                if n_offset + 2 < mesh.normals.len() {
                    mesh.normals[n_offset] = normal.x;
                    mesh.normals[n_offset + 1] = normal.y;
                    mesh.normals[n_offset + 2] = normal.z;
                }
            }
        }
    }

    if mesh.indices.is_empty() {
        None
    } else {
        Some(mesh)
    }
}

// ---------------------------------------------------------------------------
// IFCPOLYGONALFACESET (IFC4)
// ---------------------------------------------------------------------------

/// IFCPOLYGONALFACESET(Coordinates, Closed, Faces, PnIndex)
fn tessellate_polygonal_face_set(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
) -> Option<Mesh> {
    let entity = entities.get(&id)?;

    // Coordinates (attribute 0) -> IFCCARTESIANPOINTLIST3D
    let coord_list_id = entity.get_entity_ref(0)?;
    let points = read_cartesian_point_list_3d(coord_list_id, entities)?;

    // PnIndex (attribute 3) - optional reindexing
    let pn_index: Option<Vec<u32>> = entity.get_list(3).map(|list| {
        list.iter()
            .filter_map(|v| value_as_u32(v))
            .collect()
    });

    // Faces (attribute 2) - list of IFCINDEXEDPOLYGONALFACE
    let faces = entity.get_list(2)?;

    let mut mesh = Mesh::new();

    for face_val in faces {
        let face_id = match face_val {
            IfcValue::EntityRef(id) => *id,
            _ => continue,
        };
        let face_entity = match entities.get(&face_id) {
            Some(e) => e,
            None => continue,
        };

        // IFCINDEXEDPOLYGONALFACE(CoordIndex: LIST OF INTEGER)
        let coord_indices = match face_entity.get_list(0) {
            Some(list) => list,
            None => continue,
        };

        let face_points: Vec<Vec3> = coord_indices
            .iter()
            .filter_map(|v| {
                let mut idx = value_as_u32(v)? as usize;
                // Apply PnIndex remapping if present
                if let Some(ref pn) = pn_index {
                    if idx >= 1 && idx <= pn.len() {
                        idx = pn[idx - 1] as usize;
                    }
                }
                // IFC indices are 1-based
                if idx >= 1 {
                    points.get(idx - 1).copied()
                } else {
                    None
                }
            })
            .collect();

        if face_points.len() < 3 {
            continue;
        }

        let normal = compute_polygon_normal(&face_points);
        let tris = triangulate_polygon(&face_points, normal);
        let base = (mesh.vertices.len() / 3) as u32;

        for pt in &face_points {
            mesh.add_vertex(pt.x, pt.y, pt.z);
            mesh.add_normal(normal.x, normal.y, normal.z);
            mesh.add_color(color[0], color[1], color[2], color[3]);
        }
        for tri in &tris {
            mesh.add_triangle(base + tri[0] as u32, base + tri[1] as u32, base + tri[2] as u32);
        }
    }

    if mesh.vertices.is_empty() {
        None
    } else {
        Some(mesh)
    }
}

// ---------------------------------------------------------------------------
// IFCSHELLBASEDSURFACEMODEL
// ---------------------------------------------------------------------------

/// IFCSHELLBASEDSURFACEMODEL(SbsmBoundary: SET OF SHELL)
fn tessellate_shell_based_surface_model(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
) -> Option<Mesh> {
    let entity = entities.get(&id)?;
    let shells = entity.get_list(0)?;

    let mut combined = Mesh::new();
    for shell_val in shells {
        if let IfcValue::EntityRef(shell_id) = shell_val {
            if let Some(shell_mesh) = tessellate_shell(*shell_id, entities, color) {
                append_mesh(&mut combined, &shell_mesh);
            }
        }
    }

    if combined.vertices.is_empty() {
        None
    } else {
        Some(combined)
    }
}

// ---------------------------------------------------------------------------
// IFCFACEBASEDSURFACEMODEL
// ---------------------------------------------------------------------------

/// IFCFACEBASEDSURFACEMODEL(FbsmFaces: SET OF IFCCONNECTEDFACESET)
fn tessellate_face_based_surface_model(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
) -> Option<Mesh> {
    let entity = entities.get(&id)?;
    let face_sets = entity.get_list(0)?;

    let mut combined = Mesh::new();
    for face_set_val in face_sets {
        if let IfcValue::EntityRef(face_set_id) = face_set_val {
            // IFCCONNECTEDFACESET has same structure as shell (CfsFaces at attr 0)
            if let Some(shell_mesh) = tessellate_shell(*face_set_id, entities, color) {
                append_mesh(&mut combined, &shell_mesh);
            }
        }
    }

    if combined.vertices.is_empty() {
        None
    } else {
        Some(combined)
    }
}

// ---------------------------------------------------------------------------
// IFCSWEPTDISKSOLID
// ---------------------------------------------------------------------------

/// IFCSWEPTDISKSOLID(Directrix, Radius, InnerRadius, StartParam, EndParam)
fn tessellate_swept_disk_solid(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
) -> Option<Mesh> {
    let entity = entities.get(&id)?;

    // Directrix (attribute 0) - spine curve
    let directrix_id = entity.get_entity_ref(0)?;
    let spine_points = read_curve_points(directrix_id, entities);
    if spine_points.len() < 2 {
        return None;
    }

    // Radius (attribute 1)
    let radius = entity.get_real(1)? as f32;

    let segments = CIRCLE_SEGMENTS;
    let mut mesh = Mesh::new();

    // Generate tube by sweeping circle along spine
    let mut rings: Vec<Vec<Vec3>> = Vec::new();

    for i in 0..spine_points.len() {
        // Compute tangent direction
        let tangent = if i == 0 {
            (spine_points[1] - spine_points[0]).normalize()
        } else if i == spine_points.len() - 1 {
            (spine_points[i] - spine_points[i - 1]).normalize()
        } else {
            ((spine_points[i + 1] - spine_points[i]).normalize()
                + (spine_points[i] - spine_points[i - 1]).normalize())
                .normalize()
        };

        // Build local coordinate frame
        let up = if tangent.y.abs() < 0.99 {
            Vec3::Y
        } else {
            Vec3::X
        };
        let right = tangent.cross(up).normalize();
        let actual_up = right.cross(tangent).normalize();

        let mut ring = Vec::with_capacity(segments);
        for s in 0..segments {
            let angle = s as f32 * std::f32::consts::TAU / segments as f32;
            let offset = right * angle.cos() * radius + actual_up * angle.sin() * radius;
            ring.push(spine_points[i] + offset);
        }
        rings.push(ring);
    }

    // Connect rings with quads
    for i in 0..rings.len() - 1 {
        for s in 0..segments {
            let s_next = (s + 1) % segments;

            let p0 = rings[i][s];
            let p1 = rings[i][s_next];
            let p2 = rings[i + 1][s_next];
            let p3 = rings[i + 1][s];

            let normal = (p1 - p0).cross(p3 - p0).normalize();
            let base = (mesh.vertices.len() / 3) as u32;

            mesh.add_vertex(p0.x, p0.y, p0.z);
            mesh.add_vertex(p1.x, p1.y, p1.z);
            mesh.add_vertex(p2.x, p2.y, p2.z);
            mesh.add_vertex(p3.x, p3.y, p3.z);
            for _ in 0..4 {
                mesh.add_normal(normal.x, normal.y, normal.z);
                mesh.add_color(color[0], color[1], color[2], color[3]);
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 2, base + 3, base);
        }
    }

    // Cap the ends
    add_disk_cap(&mut mesh, &rings[0], color, true);
    add_disk_cap(&mut mesh, rings.last().unwrap(), color, false);

    if mesh.vertices.is_empty() {
        None
    } else {
        Some(mesh)
    }
}

/// Add a circular cap (fan triangulation).
fn add_disk_cap(mesh: &mut Mesh, ring: &[Vec3], color: [f32; 4], flip: bool) {
    if ring.len() < 3 {
        return;
    }

    let center: Vec3 = ring.iter().copied().sum::<Vec3>() / ring.len() as f32;
    let normal = compute_polygon_normal(ring);
    let normal = if flip { -normal } else { normal };

    let center_idx = (mesh.vertices.len() / 3) as u32;
    mesh.add_vertex(center.x, center.y, center.z);
    mesh.add_normal(normal.x, normal.y, normal.z);
    mesh.add_color(color[0], color[1], color[2], color[3]);

    let ring_base = center_idx + 1;
    for pt in ring {
        mesh.add_vertex(pt.x, pt.y, pt.z);
        mesh.add_normal(normal.x, normal.y, normal.z);
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }

    for i in 0..ring.len() as u32 {
        let j = (i + 1) % ring.len() as u32;
        if flip {
            mesh.add_triangle(center_idx, ring_base + j, ring_base + i);
        } else {
            mesh.add_triangle(center_idx, ring_base + i, ring_base + j);
        }
    }
}

// ---------------------------------------------------------------------------
// IFCSURFACECURVESWEPTAREASOLID
// ---------------------------------------------------------------------------

/// IFCSURFACECURVESWEPTAREASOLID(SweptArea, Position, Directrix, StartParam, EndParam, ReferenceSurface)
/// Simplified: sweep profile along directrix curve (similar to SweptDiskSolid but with arbitrary profile)
fn tessellate_surface_curve_swept(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    color: [f32; 4],
) -> Option<Mesh> {
    let entity = entities.get(&id)?;

    let profile_id = entity.get_entity_ref(0)?;
    let local_matrix = entity
        .get_entity_ref(1)
        .map(|pos_id| axis2_placement_3d_to_matrix(pos_id, entities))
        .unwrap_or(Mat4::IDENTITY);

    let directrix_id = entity.get_entity_ref(2)?;
    let spine_points = read_curve_points(directrix_id, entities);
    if spine_points.len() < 2 {
        return None;
    }

    let profile_points = get_profile_points(profile_id, entities);
    if profile_points.len() < 3 {
        return None;
    }

    // Sweep profile along spine (similar to swept disk but with profile shape)
    let mut mesh = Mesh::new();
    let mut rings: Vec<Vec<Vec3>> = Vec::new();

    for i in 0..spine_points.len() {
        let tangent = if i == 0 {
            (spine_points[1] - spine_points[0]).normalize()
        } else if i == spine_points.len() - 1 {
            (spine_points[i] - spine_points[i - 1]).normalize()
        } else {
            ((spine_points[i + 1] - spine_points[i]).normalize()
                + (spine_points[i] - spine_points[i - 1]).normalize())
                .normalize()
        };

        let up = if tangent.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
        let right = tangent.cross(up).normalize();
        let actual_up = right.cross(tangent).normalize();

        // Transform profile points to this spine position
        let ring: Vec<Vec3> = profile_points
            .iter()
            .map(|p| spine_points[i] + right * p.x + actual_up * p.y)
            .collect();
        rings.push(ring);
    }

    let n = profile_points.len();

    // Connect rings with quads
    for i in 0..rings.len() - 1 {
        for s in 0..n {
            let s_next = (s + 1) % n;
            let p0 = rings[i][s];
            let p1 = rings[i][s_next];
            let p2 = rings[i + 1][s_next];
            let p3 = rings[i + 1][s];

            let normal = (p1 - p0).cross(p3 - p0).normalize();
            let base = (mesh.vertices.len() / 3) as u32;

            mesh.add_vertex(p0.x, p0.y, p0.z);
            mesh.add_vertex(p1.x, p1.y, p1.z);
            mesh.add_vertex(p2.x, p2.y, p2.z);
            mesh.add_vertex(p3.x, p3.y, p3.z);
            for _ in 0..4 {
                mesh.add_normal(normal.x, normal.y, normal.z);
                mesh.add_color(color[0], color[1], color[2], color[3]);
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 2, base + 3, base);
        }
    }

    // Cap ends
    if !rings.is_empty() {
        add_polygon_cap(&mut mesh, &rings[0], color, true);
        add_polygon_cap(&mut mesh, rings.last().unwrap(), color, false);
    }

    transform_mesh(&mut mesh, &local_matrix);
    if mesh.vertices.is_empty() { None } else { Some(mesh) }
}

/// Add a polygon cap (fan triangulation from centroid)
fn add_polygon_cap(mesh: &mut Mesh, ring: &[Vec3], color: [f32; 4], flip: bool) {
    if ring.len() < 3 { return; }
    let center: Vec3 = ring.iter().copied().sum::<Vec3>() / ring.len() as f32;
    let normal = compute_polygon_normal(ring);
    let normal = if flip { -normal } else { normal };

    let center_idx = (mesh.vertices.len() / 3) as u32;
    mesh.add_vertex(center.x, center.y, center.z);
    mesh.add_normal(normal.x, normal.y, normal.z);
    mesh.add_color(color[0], color[1], color[2], color[3]);

    let ring_base = center_idx + 1;
    for pt in ring {
        mesh.add_vertex(pt.x, pt.y, pt.z);
        mesh.add_normal(normal.x, normal.y, normal.z);
        mesh.add_color(color[0], color[1], color[2], color[3]);
    }

    for i in 0..ring.len() as u32 {
        let j = (i + 1) % ring.len() as u32;
        if flip {
            mesh.add_triangle(center_idx, ring_base + j, ring_base + i);
        } else {
            mesh.add_triangle(center_idx, ring_base + i, ring_base + j);
        }
    }
}

// ---------------------------------------------------------------------------
// IFCMAPPEDITEM
// ---------------------------------------------------------------------------

/// IFCMAPPEDITEM(MappingSource, MappingTarget)
/// MappingSource: IFCREPRESENTATIONMAP(MappingOrigin, MappedRepresentation)
/// MappingTarget: IFCCARTESIANTRANSFORMATIONOPERATOR3D
#[allow(dead_code)]
fn tessellate_mapped_item(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    placement_cache: &mut PlacementCache,
    color: [f32; 4],
) -> Option<Mesh> {
    tessellate_mapped_item_cached(id, entities, placement_cache, color, None)
}

/// IFCMAPPEDITEM with optional geometry cache for deduplication.
///
/// When a `GeometryCache` is provided:
/// 1. Extract the IFCREPRESENTATIONMAP source ID
/// 2. Check the cache for an existing tessellation
/// 3. If hit: clone the cached mesh and apply the instance transform
/// 4. If miss: tessellate normally, cache the result, then apply transform
fn tessellate_mapped_item_cached(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    placement_cache: &mut PlacementCache,
    color: [f32; 4],
    geometry_cache: Option<&Mutex<GeometryCache>>,
) -> Option<Mesh> {
    let entity = entities.get(&id)?;

    // MappingSource (attribute 0) -> IFCREPRESENTATIONMAP
    let source_id = entity.get_entity_ref(0)?;
    let source = entities.get(&source_id)?;

    // MappingOrigin (attribute 0 of IFCREPRESENTATIONMAP) -> IFCAXIS2PLACEMENT3D
    let origin_matrix = source
        .get_entity_ref(0)
        .map(|origin_id| axis2_placement_3d_to_matrix(origin_id, entities))
        .unwrap_or(Mat4::IDENTITY);

    // MappingTarget (attribute 1) -> IFCCARTESIANTRANSFORMATIONOPERATOR3D
    let target_matrix = entity
        .get_entity_ref(1)
        .map(|target_id| cartesian_transform_operator_3d(target_id, entities))
        .unwrap_or(Mat4::IDENTITY);

    let combined_transform = target_matrix * origin_matrix;

    // Try cache lookup
    if let Some(cache_mutex) = geometry_cache {
        if let Ok(cache) = cache_mutex.lock() {
            if let Some(cached) = cache.get(source_id) {
                // Cache hit: clone the cached mesh and apply instance transform
                let mut mesh = Mesh {
                    vertices: cached.vertices.clone(),
                    normals: cached.normals.clone(),
                    indices: cached.indices.clone(),
                    colors: Vec::new(),
                };
                // Reconstruct colors (not stored in cache to save space; use current color)
                let vertex_count = mesh.vertices.len() / 3;
                mesh.colors.reserve(vertex_count * 4);
                for _ in 0..vertex_count {
                    mesh.colors.push(color[0]);
                    mesh.colors.push(color[1]);
                    mesh.colors.push(color[2]);
                    mesh.colors.push(color[3]);
                }

                transform_mesh(&mut mesh, &combined_transform);

                // Update stats (need to drop the read lock and acquire write)
                drop(cache);
                if let Ok(mut cache_w) = cache_mutex.lock() {
                    let mesh_bytes = mesh.vertices.len() * 4 + mesh.normals.len() * 4 + mesh.indices.len() * 4;
                    cache_w.cache_hits += 1;
                    cache_w.bytes_saved += mesh_bytes;
                }

                return Some(mesh);
            }
        }
    }

    // Cache miss: tessellate normally
    let mapped_rep_id = source.get_entity_ref(1)?;
    let mapped_rep = entities.get(&mapped_rep_id)?;

    let items = mapped_rep.get_list(3)?;
    let mut combined = Mesh::new();

    for item_val in items {
        let item_id = match item_val {
            IfcValue::EntityRef(id) => *id,
            _ => continue,
        };
        if let Some(item_mesh) = tessellate_geometry_item(item_id, entities, placement_cache, color) {
            append_mesh(&mut combined, &item_mesh);
        }
    }

    if combined.vertices.is_empty() {
        return None;
    }

    // Cache the untransformed mesh before applying instance transform
    if let Some(cache_mutex) = geometry_cache {
        if let Ok(mut cache) = cache_mutex.lock() {
            // Compute bounds for the cached mesh
            let vertex_count = combined.vertices.len() / 3;
            let mut bmin = [f32::MAX; 3];
            let mut bmax = [f32::MIN; 3];
            for i in 0..vertex_count {
                for c in 0..3 {
                    let v = combined.vertices[i * 3 + c];
                    if v < bmin[c] { bmin[c] = v; }
                    if v > bmax[c] { bmax[c] = v; }
                }
            }
            if vertex_count == 0 {
                bmin = [0.0; 3];
                bmax = [0.0; 3];
            }

            cache.insert(source_id, CachedMesh {
                vertices: combined.vertices.clone(),
                normals: combined.normals.clone(),
                indices: combined.indices.clone(),
                bounds_min: bmin,
                bounds_max: bmax,
            });
            cache.cache_misses += 1;
        }
    }

    // Apply: target_transform * origin_transform
    transform_mesh(&mut combined, &combined_transform);

    Some(combined)
}

// ---------------------------------------------------------------------------
// Profile extraction
// ---------------------------------------------------------------------------

/// Get profile outline points from various IFCPROFILEDEF types.
pub fn get_profile_points(
    profile_id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
) -> Vec<Vec3> {
    let entity = match entities.get(&profile_id) {
        Some(e) => e,
        None => return Vec::new(),
    };
    let etype = entity.entity_type.as_str();

    // Get profile position offset (attribute 2 for parameterized profiles, 0 for arbitrary)
    let position_offset = if etype.eq_ignore_ascii_case("IFCARBITRARYCLOSEDPROFILEDEF")
        || etype.eq_ignore_ascii_case("IFCARBITRARYPROFILEDEFWITHVOIDS")
    {
        Vec3::ZERO
    } else {
        entity
            .get_entity_ref(2)
            .map(|pos_id| axis2_placement_2d_offset(pos_id, entities))
            .unwrap_or(Vec3::ZERO)
    };

    let mut points = match etype {
        t if t.eq_ignore_ascii_case("IFCRECTANGLEPROFILEDEF") => {
            profile_rectangle(entity)
        }
        t if t.eq_ignore_ascii_case("IFCRECTANGLEHOLLOWPROFILEDEF") => {
            // Use outer rectangle (ignore hollow for now)
            profile_rectangle(entity)
        }
        t if t.eq_ignore_ascii_case("IFCCIRCLEPROFILEDEF") => {
            profile_circle(entity)
        }
        t if t.eq_ignore_ascii_case("IFCCIRCLEHOLLOWPROFILEDEF") => {
            profile_circle(entity) // Outer circle only
        }
        t if t.eq_ignore_ascii_case("IFCISHAPEPROFILEDEF") => {
            profile_i_shape(entity)
        }
        t if t.eq_ignore_ascii_case("IFCLSHAPEPROFILEDEF") => {
            profile_l_shape(entity)
        }
        t if t.eq_ignore_ascii_case("IFCTSHAPEPROFILEDEF") => {
            profile_t_shape(entity)
        }
        t if t.eq_ignore_ascii_case("IFCUSHAPEPROFILEDEF") => {
            profile_u_shape(entity)
        }
        t if t.eq_ignore_ascii_case("IFCARBITRARYCLOSEDPROFILEDEF") => {
            profile_arbitrary_closed(entity, entities)
        }
        t if t.eq_ignore_ascii_case("IFCARBITRARYPROFILEDEFWITHVOIDS") => {
            // Use outer curve only (ignore voids for now)
            profile_arbitrary_closed(entity, entities)
        }
        t if t.eq_ignore_ascii_case("IFCCOMPOSITEPROFILEDEF") => {
            // Use first sub-profile only
            profile_composite(entity, entities)
        }
        t if t.eq_ignore_ascii_case("IFCCSHAPEPROFILEDEF") => {
            profile_c_shape(entity)
        }
        t if t.eq_ignore_ascii_case("IFCASYMMETRICISHAPEPROFILEDEF") => {
            profile_asymmetric_i_shape(entity)
        }
        _ => {
            tracing::debug!("Unsupported profile type: {}", etype);
            Vec::new()
        }
    };

    // Apply position offset
    if position_offset != Vec3::ZERO {
        for pt in &mut points {
            *pt += position_offset;
        }
    }

    points
}

/// IFCRECTANGLEPROFILEDEF(ProfileType, ProfileName, Position, XDim, YDim)
fn profile_rectangle(entity: &IfcEntity) -> Vec<Vec3> {
    let x_dim = entity.get_real(3).unwrap_or(1.0) as f32;
    let y_dim = entity.get_real(4).unwrap_or(1.0) as f32;
    let hx = x_dim / 2.0;
    let hy = y_dim / 2.0;
    vec![
        Vec3::new(-hx, -hy, 0.0),
        Vec3::new(hx, -hy, 0.0),
        Vec3::new(hx, hy, 0.0),
        Vec3::new(-hx, hy, 0.0),
    ]
}

/// IFCCIRCLEPROFILEDEF(ProfileType, ProfileName, Position, Radius)
fn profile_circle(entity: &IfcEntity) -> Vec<Vec3> {
    let radius = entity.get_real(3).unwrap_or(0.5) as f32;
    (0..CIRCLE_SEGMENTS)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / CIRCLE_SEGMENTS as f32;
            Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.0)
        })
        .collect()
}

/// IFCISHAPEPROFILEDEF(ProfileType, ProfileName, Position, OverallWidth, OverallDepth,
///                     WebThickness, FlangeThickness, FilletRadius)
fn profile_i_shape(entity: &IfcEntity) -> Vec<Vec3> {
    let w = entity.get_real(3).unwrap_or(0.2) as f32; // OverallWidth
    let d = entity.get_real(4).unwrap_or(0.4) as f32; // OverallDepth
    let tw = entity.get_real(5).unwrap_or(0.01) as f32; // WebThickness
    let tf = entity.get_real(6).unwrap_or(0.02) as f32; // FlangeThickness

    let hw = w / 2.0;
    let hd = d / 2.0;
    let htw = tw / 2.0;

    // I-shape outline (12 points, clockwise)
    vec![
        Vec3::new(-hw, -hd, 0.0),
        Vec3::new(hw, -hd, 0.0),
        Vec3::new(hw, -hd + tf, 0.0),
        Vec3::new(htw, -hd + tf, 0.0),
        Vec3::new(htw, hd - tf, 0.0),
        Vec3::new(hw, hd - tf, 0.0),
        Vec3::new(hw, hd, 0.0),
        Vec3::new(-hw, hd, 0.0),
        Vec3::new(-hw, hd - tf, 0.0),
        Vec3::new(-htw, hd - tf, 0.0),
        Vec3::new(-htw, -hd + tf, 0.0),
        Vec3::new(-hw, -hd + tf, 0.0),
    ]
}

/// IFCLSHAPEPROFILEDEF(ProfileType, ProfileName, Position, Depth, Width,
///                     Thickness, FilletRadius, EdgeRadius, LegSlope)
fn profile_l_shape(entity: &IfcEntity) -> Vec<Vec3> {
    let depth = entity.get_real(3).unwrap_or(0.1) as f32;
    let width = entity.get_real(4).unwrap_or(0.1) as f32;
    let thickness = entity.get_real(5).unwrap_or(0.01) as f32;

    vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(width, 0.0, 0.0),
        Vec3::new(width, thickness, 0.0),
        Vec3::new(thickness, thickness, 0.0),
        Vec3::new(thickness, depth, 0.0),
        Vec3::new(0.0, depth, 0.0),
    ]
}

/// IFCTSHAPEPROFILEDEF(ProfileType, ProfileName, Position, Depth, FlangeWidth,
///                     WebThickness, FlangeThickness, ...)
fn profile_t_shape(entity: &IfcEntity) -> Vec<Vec3> {
    let depth = entity.get_real(3).unwrap_or(0.2) as f32;
    let flange_w = entity.get_real(4).unwrap_or(0.15) as f32;
    let web_t = entity.get_real(5).unwrap_or(0.01) as f32;
    let flange_t = entity.get_real(6).unwrap_or(0.02) as f32;

    let hfw = flange_w / 2.0;
    let hwt = web_t / 2.0;

    vec![
        Vec3::new(-hfw, depth - flange_t, 0.0),
        Vec3::new(hfw, depth - flange_t, 0.0),
        Vec3::new(hfw, depth, 0.0),
        Vec3::new(-hfw, depth, 0.0),
        // This creates a simplified T by just doing the flange and web as separate rects
        // For simplicity, return the T outline
        Vec3::new(-hwt, 0.0, 0.0),
        Vec3::new(hwt, 0.0, 0.0),
        Vec3::new(hwt, depth - flange_t, 0.0),
        Vec3::new(-hwt, depth - flange_t, 0.0),
    ]
}

/// IFCUSHAPEPROFILEDEF(ProfileType, ProfileName, Position, Depth, FlangeWidth,
///                     WebThickness, FlangeThickness, ...)
fn profile_u_shape(entity: &IfcEntity) -> Vec<Vec3> {
    let depth = entity.get_real(3).unwrap_or(0.2) as f32;
    let flange_w = entity.get_real(4).unwrap_or(0.1) as f32;
    let web_t = entity.get_real(5).unwrap_or(0.01) as f32;
    let flange_t = entity.get_real(6).unwrap_or(0.02) as f32;

    vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(flange_w, 0.0, 0.0),
        Vec3::new(flange_w, flange_t, 0.0),
        Vec3::new(web_t, flange_t, 0.0),
        Vec3::new(web_t, depth - flange_t, 0.0),
        Vec3::new(flange_w, depth - flange_t, 0.0),
        Vec3::new(flange_w, depth, 0.0),
        Vec3::new(0.0, depth, 0.0),
    ]
}

/// IFCCSHAPEPROFILEDEF(ProfileType, ProfileName, Position, Depth, Width,
///                     WallThickness, Girth, InternalFilletRadius)
fn profile_c_shape(entity: &IfcEntity) -> Vec<Vec3> {
    let depth = entity.get_real(3).unwrap_or(0.2) as f32;
    let width = entity.get_real(4).unwrap_or(0.1) as f32;
    let wall_t = entity.get_real(5).unwrap_or(0.01) as f32;
    let girth = entity.get_real(6).unwrap_or(0.02) as f32;

    // C-shape is like a U with inward-facing lips (girth)
    vec![
        Vec3::new(girth, 0.0, 0.0),
        Vec3::new(width, 0.0, 0.0),
        Vec3::new(width, wall_t, 0.0),
        Vec3::new(wall_t, wall_t, 0.0),
        Vec3::new(wall_t, depth - wall_t, 0.0),
        Vec3::new(width, depth - wall_t, 0.0),
        Vec3::new(width, depth, 0.0),
        Vec3::new(girth, depth, 0.0),
        Vec3::new(girth, depth - wall_t, 0.0),
        Vec3::new(0.0, depth - wall_t, 0.0),
        Vec3::new(0.0, wall_t, 0.0),
        Vec3::new(girth, wall_t, 0.0),
    ]
}

/// IFCASYMMETRICISHAPEPROFILEDEF - I-beam with different top and bottom flanges
/// Attrs: ProfileType, ProfileName, Position, BottomFlangeWidth, OverallDepth,
///        WebThickness, BottomFlangeThickness, BottomFlangeFilletRadius,
///        TopFlangeWidth, TopFlangeThickness, TopFlangeFilletRadius
fn profile_asymmetric_i_shape(entity: &IfcEntity) -> Vec<Vec3> {
    let bot_w = entity.get_real(3).unwrap_or(0.2) as f32;
    let depth = entity.get_real(4).unwrap_or(0.4) as f32;
    let tw = entity.get_real(5).unwrap_or(0.01) as f32;
    let bot_tf = entity.get_real(6).unwrap_or(0.02) as f32;
    // Skip attr 7 (fillet radius)
    let top_w = entity.get_real(8).unwrap_or(0.15) as f32;
    let top_tf = entity.get_real(9).unwrap_or(0.02) as f32;

    let hbw = bot_w / 2.0;
    let htw = tw / 2.0;
    let htop = top_w / 2.0;
    let hd = depth / 2.0;

    vec![
        // Bottom flange
        Vec3::new(-hbw, -hd, 0.0),
        Vec3::new(hbw, -hd, 0.0),
        Vec3::new(hbw, -hd + bot_tf, 0.0),
        Vec3::new(htw, -hd + bot_tf, 0.0),
        // Web
        Vec3::new(htw, hd - top_tf, 0.0),
        // Top flange
        Vec3::new(htop, hd - top_tf, 0.0),
        Vec3::new(htop, hd, 0.0),
        Vec3::new(-htop, hd, 0.0),
        Vec3::new(-htop, hd - top_tf, 0.0),
        // Web (left)
        Vec3::new(-htw, hd - top_tf, 0.0),
        Vec3::new(-htw, -hd + bot_tf, 0.0),
        // Bottom flange (left)
        Vec3::new(-hbw, -hd + bot_tf, 0.0),
    ]
}

/// IFCARBITRARYCLOSEDPROFILEDEF(ProfileType, ProfileName, OuterCurve)
fn profile_arbitrary_closed(
    entity: &IfcEntity,
    entities: &HashMap<EntityId, IfcEntity>,
) -> Vec<Vec3> {
    // OuterCurve is at attribute 2 for ArbitraryClosedProfileDef
    let curve_id = match entity.get_entity_ref(2) {
        Some(id) => id,
        None => return Vec::new(),
    };
    read_curve_points(curve_id, entities)
}

/// IFCCOMPOSITEPROFILEDEF(ProfileType, ProfileName, Profiles, Label)
fn profile_composite(
    entity: &IfcEntity,
    entities: &HashMap<EntityId, IfcEntity>,
) -> Vec<Vec3> {
    // Profiles (attribute 2) - list of sub-profiles
    let profiles = match entity.get_list(2) {
        Some(list) => list,
        None => return Vec::new(),
    };

    // Use first sub-profile
    for val in profiles {
        if let IfcValue::EntityRef(id) = val {
            let pts = get_profile_points(*id, entities);
            if !pts.is_empty() {
                return pts;
            }
        }
    }
    Vec::new()
}

// ---------------------------------------------------------------------------
// Curve reading
// ---------------------------------------------------------------------------

/// Read points from various curve types.
pub fn read_curve_points(
    curve_id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
) -> Vec<Vec3> {
    let entity = match entities.get(&curve_id) {
        Some(e) => e,
        None => return Vec::new(),
    };

    match entity.entity_type.as_str() {
        t if t.eq_ignore_ascii_case("IFCPOLYLINE") => {
            read_polyline(entity, entities)
        }
        t if t.eq_ignore_ascii_case("IFCTRIMMEDCURVE") => {
            read_trimmed_curve(entity, entities)
        }
        t if t.eq_ignore_ascii_case("IFCCOMPOSITECURVE") => {
            read_composite_curve(entity, entities)
        }
        t if t.eq_ignore_ascii_case("IFCCIRCLE") => {
            read_circle(entity, entities)
        }
        t if t.eq_ignore_ascii_case("IFCINDEXEDPOLYCURVE") => {
            read_indexed_polycurve(entity, entities)
        }
        t if t.eq_ignore_ascii_case("IFCLINE") => {
            read_line(entity, entities)
        }
        t if t.eq_ignore_ascii_case("IFCBSPLINECURVEWITHKNOTS") => {
            read_bspline_curve(entity, entities, None)
        }
        t if t.eq_ignore_ascii_case("IFCRATIONALBSPLINECURVEWITHKNOTS") => {
            // Weights at attribute 8
            let weights: Vec<f64> = entity
                .get_list(8)
                .map(|list| {
                    list.iter()
                        .filter_map(|v| match v {
                            IfcValue::Real(r) => Some(*r),
                            IfcValue::Integer(i) => Some(*i as f64),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            read_bspline_curve(
                entity,
                entities,
                if weights.is_empty() { None } else { Some(&weights) },
            )
        }
        _ => {
            tracing::debug!("Unsupported curve type: {}", entity.entity_type);
            Vec::new()
        }
    }
}

/// IFCPOLYLINE(Points: LIST OF IFCCARTESIANPOINT)
fn read_polyline(entity: &IfcEntity, entities: &HashMap<EntityId, IfcEntity>) -> Vec<Vec3> {
    let points = match entity.get_list(0) {
        Some(list) => list,
        None => return Vec::new(),
    };
    let mut result: Vec<Vec3> = points
        .iter()
        .filter_map(|v| match v {
            IfcValue::EntityRef(id) => read_cartesian_point(*id, entities),
            _ => None,
        })
        .collect();

    // Remove closing point if it duplicates the first (polylines sometimes close explicitly)
    if result.len() > 1 {
        if let (Some(first), Some(last)) = (result.first(), result.last()) {
            if (*first - *last).length_squared() < 1e-8 {
                result.pop();
            }
        }
    }
    result
}

/// IFCTRIMMEDCURVE(BasisCurve, Trim1, Trim2, SenseAgreement, MasterRepresentation)
/// Approximate as a line between trim points.
fn read_trimmed_curve(entity: &IfcEntity, entities: &HashMap<EntityId, IfcEntity>) -> Vec<Vec3> {
    // Try to read the basis curve and approximate
    let basis_id = match entity.get_entity_ref(0) {
        Some(id) => id,
        None => return Vec::new(),
    };

    // If the basis curve is a circle, approximate the arc
    if let Some(basis) = entities.get(&basis_id) {
        if basis.entity_type.eq_ignore_ascii_case("IFCCIRCLE") {
            // For now, approximate with the full circle (or read trim params)
            return read_circle(basis, entities);
        }
    }

    // Fallback: read basis curve points
    read_curve_points(basis_id, entities)
}

/// IFCCOMPOSITECURVE(Segments: LIST OF IFCCOMPOSITECURVESEGMENT, SelfIntersect)
fn read_composite_curve(entity: &IfcEntity, entities: &HashMap<EntityId, IfcEntity>) -> Vec<Vec3> {
    let segments = match entity.get_list(0) {
        Some(list) => list,
        None => return Vec::new(),
    };

    let mut result = Vec::new();
    for seg_val in segments {
        let seg_id = match seg_val {
            IfcValue::EntityRef(id) => *id,
            _ => continue,
        };
        let seg = match entities.get(&seg_id) {
            Some(e) => e,
            None => continue,
        };

        // IFCCOMPOSITECURVESEGMENT(Transition, SameSense, ParentCurve)
        let curve_id = match seg.get_entity_ref(2) {
            Some(id) => id,
            None => continue,
        };

        let mut pts = read_curve_points(curve_id, entities);
        // Remove first point if it duplicates the last added point
        if !result.is_empty() && !pts.is_empty() {
            let last: Vec3 = *result.last().unwrap();
            let first: Vec3 = *pts.first().unwrap();
            if (last - first).length_squared() < 1e-8 {
                pts.remove(0);
            }
        }
        result.extend(pts);
    }

    // Remove closing point duplicate
    if result.len() > 1 {
        let first: Vec3 = result[0];
        let last: Vec3 = *result.last().unwrap();
        if (first - last).length_squared() < 1e-8 {
            result.pop();
        }
    }
    result
}

/// IFCCIRCLE(Position, Radius) — approximate as polygon.
fn read_circle(entity: &IfcEntity, entities: &HashMap<EntityId, IfcEntity>) -> Vec<Vec3> {
    let radius = entity.get_real(1).unwrap_or(0.5) as f32;
    let center = entity
        .get_entity_ref(0)
        .map(|pos_id| axis2_placement_2d_offset(pos_id, entities))
        .unwrap_or(Vec3::ZERO);

    (0..CIRCLE_SEGMENTS)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / CIRCLE_SEGMENTS as f32;
            Vec3::new(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
                center.z,
            )
        })
        .collect()
}

/// IFCINDEXEDPOLYCURVE(Points, Segments, SelfIntersect)
/// Points: IFCCARTESIANPOINTLIST2D or IFCCARTESIANPOINTLIST3D
fn read_indexed_polycurve(entity: &IfcEntity, entities: &HashMap<EntityId, IfcEntity>) -> Vec<Vec3> {
    let points_id = match entity.get_entity_ref(0) {
        Some(id) => id,
        None => return Vec::new(),
    };

    // Try 3D first, then 2D
    let points = if let Some(pts) = read_cartesian_point_list_3d(points_id, entities) {
        pts
    } else if let Some(pts) = read_cartesian_point_list_2d(points_id, entities) {
        pts
    } else {
        return Vec::new();
    };

    // If no Segments attribute, use all points in order
    let segments = match entity.get_list(1) {
        Some(list) => list.clone(),
        None => {
            let mut result = points;
            // Remove closing duplicate
            if result.len() > 1 {
                if let (Some(first), Some(last)) = (result.first(), result.last()) {
                    if (*first - *last).length_squared() < 1e-8 {
                        result.pop();
                    }
                }
            }
            return result;
        }
    };

    // Process segments (IFCLINEINDEX or IFCARCINDEX)
    let mut result = Vec::new();
    for seg_val in &segments {
        if let IfcValue::List(indices) = seg_val {
            // Line segment: just add the indexed points
            for idx_val in indices {
                if let Some(idx) = value_as_u32(idx_val) {
                    if idx >= 1 && (idx as usize - 1) < points.len() {
                        let pt = points[idx as usize - 1];
                        // Avoid duplicating consecutive points
                        if result.last().map_or(true, |last: &Vec3| (*last - pt).length_squared() > 1e-8) {
                            result.push(pt);
                        }
                    }
                }
            }
        }
    }

    // Remove closing duplicate
    if result.len() > 1 {
        if let (Some(first), Some(last)) = (result.first(), result.last()) {
            if (*first - *last).length_squared() < 1e-8 {
                result.pop();
            }
        }
    }
    result
}

/// IFCLINE(Pnt: IFCCARTESIANPOINT, Dir: IFCVECTOR)
fn read_line(entity: &IfcEntity, entities: &HashMap<EntityId, IfcEntity>) -> Vec<Vec3> {
    let start = entity
        .get_entity_ref(0)
        .and_then(|id| read_cartesian_point(id, entities))
        .unwrap_or(Vec3::ZERO);

    let dir_id = match entity.get_entity_ref(1) {
        Some(id) => id,
        None => return vec![start],
    };

    // IFCVECTOR(Orientation: IFCDIRECTION, Magnitude: REAL)
    if let Some(vector) = entities.get(&dir_id) {
        let direction = vector
            .get_entity_ref(0)
            .and_then(|id| read_direction(id, entities))
            .unwrap_or(Vec3::X);
        let magnitude = vector.get_real(1).unwrap_or(1.0) as f32;
        let end = start + direction * magnitude;
        vec![start, end]
    } else {
        vec![start]
    }
}

// ---------------------------------------------------------------------------
// B-Spline curves
// ---------------------------------------------------------------------------

/// Read an IFCBSPLINECURVEWITHKNOTS (or rational variant with supplied weights).
///
/// Attributes:
///  0: Degree (INTEGER)
///  1: ControlPointsList (LIST OF IFCCARTESIANPOINT)
///  2: CurveForm (ENUM)
///  3: ClosedCurve (BOOLEAN)
///  4: SelfIntersect (BOOLEAN)
///  5: KnotMultiplicities (LIST OF INTEGER)
///  6: Knots (LIST OF REAL)
///  7: KnotSpec (ENUM)
///  8: WeightsData (LIST OF REAL) — only for IFCRATIONALBSPLINECURVEWITHKNOTS
fn read_bspline_curve(
    entity: &IfcEntity,
    entities: &HashMap<EntityId, IfcEntity>,
    weights: Option<&[f64]>,
) -> Vec<Vec3> {
    // Degree (attribute 0)
    let degree = match entity.get_attr(0) {
        Some(IfcValue::Integer(i)) => *i as usize,
        Some(IfcValue::Real(r)) => *r as usize,
        _ => return Vec::new(),
    };

    // Control points (attribute 1)
    let cp_list = match entity.get_list(1) {
        Some(list) => list,
        None => return Vec::new(),
    };
    let control_points: Vec<Vec3> = cp_list
        .iter()
        .filter_map(|v| match v {
            IfcValue::EntityRef(id) => read_cartesian_point(*id, entities),
            _ => None,
        })
        .collect();

    if control_points.len() < 2 {
        return Vec::new();
    }

    // Knot multiplicities (attribute 5)
    let knot_mults: Vec<usize> = entity
        .get_list(5)
        .map(|list| {
            list.iter()
                .filter_map(|v| match v {
                    IfcValue::Integer(i) => Some(*i as usize),
                    IfcValue::Real(r) => Some(*r as usize),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    // Knots (attribute 6)
    let knots: Vec<f64> = entity
        .get_list(6)
        .map(|list| {
            list.iter()
                .filter_map(|v| match v {
                    IfcValue::Real(r) => Some(*r),
                    IfcValue::Integer(i) => Some(*i as f64),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    if knots.is_empty() || knot_mults.is_empty() || knots.len() != knot_mults.len() {
        // Fallback: return control polygon
        return control_points;
    }

    // Build expanded knot vector by repeating each knot according to its multiplicity
    let mut knot_vector: Vec<f64> = Vec::new();
    for (k, m) in knots.iter().zip(knot_mults.iter()) {
        for _ in 0..*m {
            knot_vector.push(*k);
        }
    }

    let num_samples = CIRCLE_SEGMENTS * 2;

    let mut result = evaluate_bspline(&control_points, &knot_vector, degree, weights, num_samples);

    // Remove closing duplicate
    if result.len() > 1 {
        if let (Some(first), Some(last)) = (result.first(), result.last()) {
            if (*first - *last).length_squared() < 1e-8 {
                result.pop();
            }
        }
    }

    result
}

/// Evaluate a B-spline curve at `num_samples` evenly spaced parameter values
/// using De Boor's algorithm.
///
/// For rational B-splines, supply `weights` (one per control point).
fn evaluate_bspline(
    control_points: &[Vec3],
    knot_vector: &[f64],
    degree: usize,
    weights: Option<&[f64]>,
    num_samples: usize,
) -> Vec<Vec3> {
    let n = control_points.len(); // number of control points
    if n == 0 || knot_vector.len() < n + degree + 1 || degree == 0 || num_samples == 0 {
        return control_points.to_vec();
    }

    // Parameter range: [knot[degree], knot[n])
    let t_start = knot_vector[degree];
    let t_end = knot_vector[n];
    if (t_end - t_start).abs() < 1e-12 {
        return control_points.to_vec();
    }

    let mut result = Vec::with_capacity(num_samples);

    for sample in 0..num_samples {
        // Map sample index to parameter t in [t_start, t_end]
        // Use num_samples - 1 intervals so we include both endpoints.
        let t = if num_samples == 1 {
            t_start
        } else {
            t_start + (t_end - t_start) * (sample as f64) / ((num_samples - 1) as f64)
        };

        // Clamp t slightly below t_end to avoid boundary issues
        let t = if t >= t_end { t_end - 1e-10 } else { t };

        // Find span k: largest index such that knot[k] <= t < knot[k+1]
        let mut k = degree;
        for i in degree..n {
            if knot_vector[i] <= t && t < knot_vector[i + 1] {
                k = i;
                break;
            }
        }
        // Handle the case where t is very close to t_end
        if t >= knot_vector[n] - 1e-10 {
            // Use the last valid span
            k = n - 1;
            // Make sure k >= degree
            if k < degree {
                k = degree;
            }
        }

        // De Boor's algorithm
        // Copy the relevant control points: indices k-degree .. k
        let start_idx = if k >= degree { k - degree } else { 0 };
        let count = degree + 1;
        let end_idx = (start_idx + count).min(n);
        let actual_count = end_idx - start_idx;

        if actual_count < degree + 1 {
            // Not enough control points for this span; skip
            continue;
        }

        // For rational B-splines, work in homogeneous coordinates:
        // multiply control points by their weights before interpolation.
        let mut d: Vec<Vec3> = if let Some(wt) = weights {
            control_points[start_idx..end_idx]
                .iter()
                .zip(&wt[start_idx..end_idx])
                .map(|(p, &wi)| *p * wi as f32)
                .collect()
        } else {
            control_points[start_idx..end_idx].to_vec()
        };
        let mut w: Vec<f64> = if let Some(wt) = weights {
            wt[start_idx..end_idx].to_vec()
        } else {
            vec![1.0; actual_count]
        };

        for r in 1..=degree {
            for j in (r..=degree).rev() {
                let left = k as isize - degree as isize + j as isize;
                let right = k as isize + 1 + j as isize - r as isize;

                if left < 0 || right < 0
                    || left as usize >= knot_vector.len()
                    || right as usize >= knot_vector.len()
                {
                    continue;
                }

                let kl = knot_vector[left as usize];
                let kr = knot_vector[right as usize];
                let denom = kr - kl;
                if denom.abs() < 1e-14 {
                    continue;
                }
                let alpha = (t - kl) / denom;
                let alpha = alpha.clamp(0.0, 1.0);
                let one_minus_alpha = 1.0 - alpha;

                d[j] = d[j - 1] * one_minus_alpha as f32 + d[j] * alpha as f32;
                w[j] = w[j - 1] * one_minus_alpha + w[j] * alpha;
            }
        }

        let point = if weights.is_some() && w[degree].abs() > 1e-14 {
            d[degree] / w[degree] as f32
        } else {
            d[degree]
        };
        result.push(point);
    }

    result
}

// ---------------------------------------------------------------------------
// Point list readers
// ---------------------------------------------------------------------------

/// Read IFCCARTESIANPOINTLIST3D(CoordList: LIST OF LIST OF REAL)
fn read_cartesian_point_list_3d(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
) -> Option<Vec<Vec3>> {
    let entity = entities.get(&id)?;
    if !entity.entity_type.eq_ignore_ascii_case("IFCCARTESIANPOINTLIST3D") {
        return None;
    }
    let coord_list = entity.get_list(0)?;
    let points: Vec<Vec3> = coord_list
        .iter()
        .filter_map(|v| {
            if let IfcValue::List(coords) = v {
                let x = value_as_f32(coords.get(0)?)?;
                let y = value_as_f32(coords.get(1)?)?;
                let z = coords.get(2).and_then(|v| value_as_f32(v)).unwrap_or(0.0);
                Some(Vec3::new(x, y, z))
            } else {
                None
            }
        })
        .collect();
    if points.is_empty() {
        None
    } else {
        Some(points)
    }
}

/// Read IFCCARTESIANPOINTLIST2D(CoordList: LIST OF LIST OF REAL)
fn read_cartesian_point_list_2d(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
) -> Option<Vec<Vec3>> {
    let entity = entities.get(&id)?;
    if !entity.entity_type.eq_ignore_ascii_case("IFCCARTESIANPOINTLIST2D") {
        return None;
    }
    let coord_list = entity.get_list(0)?;
    let points: Vec<Vec3> = coord_list
        .iter()
        .filter_map(|v| {
            if let IfcValue::List(coords) = v {
                let x = value_as_f32(coords.get(0)?)?;
                let y = value_as_f32(coords.get(1)?)?;
                Some(Vec3::new(x, y, 0.0))
            } else {
                None
            }
        })
        .collect();
    if points.is_empty() {
        None
    } else {
        Some(points)
    }
}

// ---------------------------------------------------------------------------
// Mesh helpers (also used from geometry.rs)
// ---------------------------------------------------------------------------

/// Append source mesh into target (with index offset).
pub fn append_mesh(target: &mut Mesh, source: &Mesh) {
    let base = (target.vertices.len() / 3) as u32;
    target.vertices.extend_from_slice(&source.vertices);
    target.normals.extend_from_slice(&source.normals);
    target.colors.extend_from_slice(&source.colors);
    target.indices.extend(source.indices.iter().map(|i| i + base));
}

/// Transform all vertices and normals in a mesh by a 4x4 matrix.
pub fn transform_mesh(mesh: &mut Mesh, matrix: &Mat4) {
    let normal_matrix = matrix.inverse().transpose();

    for i in (0..mesh.vertices.len()).step_by(3) {
        let pos = Vec3::new(mesh.vertices[i], mesh.vertices[i + 1], mesh.vertices[i + 2]);
        let transformed = matrix.transform_point3(pos);
        mesh.vertices[i] = transformed.x;
        mesh.vertices[i + 1] = transformed.y;
        mesh.vertices[i + 2] = transformed.z;
    }

    for i in (0..mesh.normals.len()).step_by(3) {
        let n = Vec3::new(mesh.normals[i], mesh.normals[i + 1], mesh.normals[i + 2]);
        let tn = normal_matrix.transform_vector3(n).normalize_or_zero();
        mesh.normals[i] = tn.x;
        mesh.normals[i + 1] = tn.y;
        mesh.normals[i + 2] = tn.z;
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Compute polygon normal from the first 3 non-degenerate points using Newell's method.
fn compute_polygon_normal(points: &[Vec3]) -> Vec3 {
    if points.len() < 3 {
        return Vec3::Y;
    }

    // Newell's method for robust normal computation
    let mut normal = Vec3::ZERO;
    let n = points.len();
    for i in 0..n {
        let curr = points[i];
        let next = points[(i + 1) % n];
        normal.x += (curr.y - next.y) * (curr.z + next.z);
        normal.y += (curr.z - next.z) * (curr.x + next.x);
        normal.z += (curr.x - next.x) * (curr.y + next.y);
    }

    if normal.length_squared() < 1e-12 {
        Vec3::Y
    } else {
        normal.normalize()
    }
}

fn value_as_f32(v: &IfcValue) -> Option<f32> {
    match v {
        IfcValue::Real(r) => Some(*r as f32),
        IfcValue::Integer(i) => Some(*i as f32),
        _ => None,
    }
}

fn value_as_u32(v: &IfcValue) -> Option<u32> {
    match v {
        IfcValue::Integer(i) => Some(*i as u32),
        IfcValue::Real(r) => Some(*r as u32),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_bspline_evaluation_linear() {
        // Degree 1 B-spline with appropriate knots = piecewise linear interpolation
        // through the control points.
        let control_points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 1.0, 0.0),
        ];
        // For degree 1 with 3 control points we need n + degree + 1 = 5 knots.
        let knots = vec![0.0, 0.0, 0.5, 1.0, 1.0];
        let result = evaluate_bspline(&control_points, &knots, 1, None, 5);
        assert!(result.len() >= 3, "Expected at least 3 sample points, got {}", result.len());
        // First point should be near the first control point
        assert!(
            (result[0] - control_points[0]).length() < 0.1,
            "First point {:?} not near control point {:?}",
            result[0],
            control_points[0]
        );
        // Last point should be near the last control point
        assert!(
            (*result.last().unwrap() - control_points[2]).length() < 0.1,
            "Last point {:?} not near control point {:?}",
            result.last().unwrap(),
            control_points[2]
        );
    }

    #[test]
    fn test_bspline_evaluation_quadratic() {
        // Degree 2 B-spline: the curve should start at the first control point
        // and end at the last, but not pass through interior points.
        let control_points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        // n=3 control points, degree=2, need 3+2+1=6 knots
        let knots = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let result = evaluate_bspline(&control_points, &knots, 2, None, 20);
        assert!(!result.is_empty());
        // Endpoints should match control points
        assert!(
            (result[0] - control_points[0]).length() < 0.1,
            "Start: {:?} vs {:?}",
            result[0],
            control_points[0]
        );
        assert!(
            (*result.last().unwrap() - control_points[2]).length() < 0.1,
            "End: {:?} vs {:?}",
            result.last().unwrap(),
            control_points[2]
        );
        // Midpoint should NOT be at (1, 2, 0) — it should be pulled toward but not reach it
        let mid = result[result.len() / 2];
        assert!(
            mid.y < 2.0,
            "Midpoint y={} should be less than 2.0 (not interpolating through control point)",
            mid.y
        );
        assert!(mid.y > 0.0, "Midpoint y={} should be above 0", mid.y);
    }

    #[test]
    fn test_bspline_rational_weights() {
        // A rational B-spline with equal weights should produce the same result
        // as a non-rational one.
        let control_points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let knots = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let uniform_weights = vec![1.0, 1.0, 1.0];

        let result_no_weight = evaluate_bspline(&control_points, &knots, 2, None, 10);
        let result_with_weight =
            evaluate_bspline(&control_points, &knots, 2, Some(&uniform_weights), 10);

        assert_eq!(result_no_weight.len(), result_with_weight.len());
        for (a, b) in result_no_weight.iter().zip(result_with_weight.iter()) {
            assert!(
                (*a - *b).length() < 1e-4,
                "Mismatch: {:?} vs {:?}",
                a,
                b
            );
        }
    }

    #[test]
    fn test_bspline_high_weight_pulls_curve() {
        // Increasing the weight of the middle control point should pull the curve
        // closer to it.
        let control_points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let knots = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0];

        let low_weights = vec![1.0, 1.0, 1.0];
        let high_weights = vec![1.0, 10.0, 1.0];

        let result_low = evaluate_bspline(&control_points, &knots, 2, Some(&low_weights), 11);
        let result_high = evaluate_bspline(&control_points, &knots, 2, Some(&high_weights), 11);

        // The midpoint with high weight should be closer to (1, 1, 0) than with low weight
        let mid_low = result_low[result_low.len() / 2];
        let mid_high = result_high[result_high.len() / 2];
        let target = Vec3::new(1.0, 1.0, 0.0);

        assert!(
            (mid_high - target).length() < (mid_low - target).length(),
            "High weight midpoint {:?} should be closer to {:?} than low weight midpoint {:?}",
            mid_high,
            target,
            mid_low
        );
    }

    #[test]
    fn test_clip_mesh_keeps_above() {
        // Create a simple quad mesh: two triangles forming a square from -1 to 1 in X and Y at Z=0.
        let mut mesh = Mesh::new();
        let color = [0.5, 0.5, 0.5, 1.0];

        // Vertices
        mesh.add_vertex(-1.0, -1.0, 0.0); // 0
        mesh.add_vertex(1.0, -1.0, 0.0);  // 1
        mesh.add_vertex(1.0, 1.0, 0.0);   // 2
        mesh.add_vertex(-1.0, 1.0, 0.0);  // 3

        for _ in 0..4 {
            mesh.add_normal(0.0, 0.0, 1.0);
            mesh.add_color(color[0], color[1], color[2], color[3]);
        }

        // Two triangles forming the quad
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);

        // Clip at X=0 plane (normal pointing in +X direction), keeping X >= 0
        let plane_origin = Vec3::new(0.0, 0.0, 0.0);
        let plane_normal = Vec3::new(1.0, 0.0, 0.0);
        let result = clip_mesh_by_plane(&mesh, plane_origin, plane_normal, true, color);

        assert!(result.is_some(), "Clipping should produce a non-empty mesh");
        let clipped = result.unwrap();

        // All resulting vertices should have X >= -0.01 (small epsilon for floating point)
        let vertex_count = clipped.vertices.len() / 3;
        for i in 0..vertex_count {
            let x = clipped.vertices[i * 3];
            assert!(
                x >= -0.01,
                "Vertex {} has X={}, should be >= 0 after clipping",
                i,
                x
            );
        }

        // Should have at least some triangles
        assert!(
            clipped.indices.len() >= 3,
            "Expected at least 1 triangle, got {} indices",
            clipped.indices.len()
        );
    }

    #[test]
    fn test_clip_mesh_keeps_below() {
        // Same quad, but keep X <= 0
        let mut mesh = Mesh::new();
        let color = [0.5, 0.5, 0.5, 1.0];

        mesh.add_vertex(-1.0, -1.0, 0.0);
        mesh.add_vertex(1.0, -1.0, 0.0);
        mesh.add_vertex(1.0, 1.0, 0.0);
        mesh.add_vertex(-1.0, 1.0, 0.0);

        for _ in 0..4 {
            mesh.add_normal(0.0, 0.0, 1.0);
            mesh.add_color(color[0], color[1], color[2], color[3]);
        }

        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);

        let plane_origin = Vec3::new(0.0, 0.0, 0.0);
        let plane_normal = Vec3::new(1.0, 0.0, 0.0);
        let result = clip_mesh_by_plane(&mesh, plane_origin, plane_normal, false, color);

        assert!(result.is_some(), "Clipping should produce a non-empty mesh");
        let clipped = result.unwrap();

        let vertex_count = clipped.vertices.len() / 3;
        for i in 0..vertex_count {
            let x = clipped.vertices[i * 3];
            assert!(
                x <= 0.01,
                "Vertex {} has X={}, should be <= 0 after clipping",
                i,
                x
            );
        }
    }

    #[test]
    fn test_clip_mesh_all_inside() {
        // All vertices are on the keep side — mesh should be unchanged
        let mut mesh = Mesh::new();
        let color = [1.0, 0.0, 0.0, 1.0];

        mesh.add_vertex(1.0, 0.0, 0.0);
        mesh.add_vertex(2.0, 0.0, 0.0);
        mesh.add_vertex(1.5, 1.0, 0.0);

        for _ in 0..3 {
            mesh.add_normal(0.0, 0.0, 1.0);
            mesh.add_color(color[0], color[1], color[2], color[3]);
        }
        mesh.add_triangle(0, 1, 2);

        let plane_origin = Vec3::ZERO;
        let plane_normal = Vec3::X;
        let result = clip_mesh_by_plane(&mesh, plane_origin, plane_normal, true, color);

        assert!(result.is_some());
        let clipped = result.unwrap();
        // Should have exactly 1 triangle (3 indices)
        assert_eq!(clipped.indices.len(), 3);
    }

    #[test]
    fn test_clip_mesh_all_outside() {
        // All vertices are on the discard side — mesh should be None
        let mut mesh = Mesh::new();
        let color = [1.0, 0.0, 0.0, 1.0];

        mesh.add_vertex(-1.0, 0.0, 0.0);
        mesh.add_vertex(-2.0, 0.0, 0.0);
        mesh.add_vertex(-1.5, 1.0, 0.0);

        for _ in 0..3 {
            mesh.add_normal(0.0, 0.0, 1.0);
            mesh.add_color(color[0], color[1], color[2], color[3]);
        }
        mesh.add_triangle(0, 1, 2);

        let plane_origin = Vec3::ZERO;
        let plane_normal = Vec3::X;
        let result = clip_mesh_by_plane(&mesh, plane_origin, plane_normal, true, color);

        assert!(result.is_none(), "All-outside clipping should return None");
    }

    #[test]
    fn test_bspline_empty_inputs() {
        // Edge case: empty control points
        let result = evaluate_bspline(&[], &[0.0, 1.0], 1, None, 10);
        assert!(result.is_empty() || result.len() <= 1);

        // Edge case: degree 0
        let cp = vec![Vec3::ZERO, Vec3::X];
        let result = evaluate_bspline(&cp, &[0.0, 0.0, 1.0, 1.0], 0, None, 10);
        // Should fall back to returning control points
        assert!(!result.is_empty());
    }

    // -----------------------------------------------------------------------
    // GeometryCache tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_geometry_cache_insert_and_get() {
        let mut cache = GeometryCache::new();
        assert!(cache.get(42).is_none());

        let mesh = CachedMesh {
            vertices: vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
            normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            indices: vec![0, 1, 0],
            bounds_min: [0.0, 1.0, 2.0],
            bounds_max: [3.0, 4.0, 5.0],
        };
        cache.insert(42, mesh);

        let got = cache.get(42);
        assert!(got.is_some());
        let got = got.unwrap();
        assert_eq!(got.vertices.len(), 6);
        assert_eq!(got.indices.len(), 3);
        assert_eq!(got.bounds_min, [0.0, 1.0, 2.0]);
        assert_eq!(got.bounds_max, [3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_geometry_cache_stats() {
        let mut cache = GeometryCache::new();
        assert_eq!(cache.stats(), (0, 0, 0));

        cache.cache_hits = 5;
        cache.cache_misses = 3;
        cache.bytes_saved = 1024;
        assert_eq!(cache.stats(), (5, 3, 1024));

        cache.clear();
        assert_eq!(cache.stats(), (0, 0, 0));
        assert!(cache.get(42).is_none());
    }

    #[test]
    fn test_geometry_cache_memory_estimation() {
        let mut cache = GeometryCache::new();
        let base_mem = cache.memory_bytes();
        assert!(base_mem > 0, "Base memory should include struct overhead");

        // Insert a mesh with known sizes
        let mesh = CachedMesh {
            vertices: vec![1.0; 300],  // 300 floats = 1200 bytes
            normals: vec![0.0; 300],   // 300 floats = 1200 bytes
            indices: vec![0; 100],     // 100 u32s = 400 bytes
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
        };
        cache.insert(1, mesh);

        let mem_with_one = cache.memory_bytes();
        assert!(
            mem_with_one > base_mem,
            "Memory should increase after inserting a mesh: {} vs {}",
            mem_with_one,
            base_mem
        );

        // The increase should include at least the raw data
        let expected_data_bytes = (300 + 300) * 4 + 100 * 4; // 2800 bytes
        assert!(
            mem_with_one - base_mem >= expected_data_bytes,
            "Memory increase {} should be >= raw data {}",
            mem_with_one - base_mem,
            expected_data_bytes
        );
    }

    #[test]
    fn test_shared_geometry_stats_no_mapped_items() {
        let entities: HashMap<EntityId, IfcEntity> = HashMap::new();
        let stats = count_shared_geometry(&entities);
        assert_eq!(stats.total_mapped_items, 0);
        assert_eq!(stats.unique_representations, 0);
        assert_eq!(stats.duplication_ratio, 0.0);
        assert_eq!(stats.estimated_savings_mb, 0.0);
    }

    #[test]
    fn test_shared_geometry_stats_with_mapped_items() {
        use super::super::entities::{IfcEntity, IfcValue};

        let mut entities: HashMap<EntityId, IfcEntity> = HashMap::new();

        // Create a representation map source (id=100)
        entities.insert(100, IfcEntity {
            id: 100,
            entity_type: "IFCREPRESENTATIONMAP".to_string(),
            attributes: vec![],
        });

        // Create 3 mapped items all pointing to the same source (id=100)
        for id in [1, 2, 3] {
            entities.insert(id, IfcEntity {
                id,
                entity_type: "IFCMAPPEDITEM".to_string(),
                attributes: vec![IfcValue::EntityRef(100), IfcValue::Null],
            });
        }

        let stats = count_shared_geometry(&entities);
        assert_eq!(stats.total_mapped_items, 3);
        assert_eq!(stats.unique_representations, 1);
        assert!((stats.duplication_ratio - 3.0).abs() < 0.01);
        assert!(stats.estimated_savings_mb > 0.0);
    }

    // -----------------------------------------------------------------------
    // Geometry compression tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_deduplicate_vertices_basic() {
        // Two triangles sharing an edge, with duplicate vertices
        let vertices = vec![
            0.0, 0.0, 0.0,  // 0
            1.0, 0.0, 0.0,  // 1
            0.0, 1.0, 0.0,  // 2
            1.0, 0.0, 0.0,  // 3 (duplicate of 1)
            1.0, 1.0, 0.0,  // 4
            0.0, 1.0, 0.0,  // 5 (duplicate of 2)
        ];
        let normals = vec![
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
        ];
        let indices = vec![0, 1, 2, 3, 4, 5];

        let (new_verts, new_norms, new_indices) =
            deduplicate_vertices(&vertices, &normals, &indices, 0.001);

        // Should have 4 unique vertices (0, 1, 2, 4)
        assert_eq!(new_verts.len() / 3, 4, "Expected 4 unique vertices, got {}", new_verts.len() / 3);
        assert_eq!(new_norms.len() / 3, 4);
        // All indices should still form valid triangles
        assert_eq!(new_indices.len(), 6);
        for &idx in &new_indices {
            assert!(idx < 4, "Index {} should be < 4", idx);
        }
    }

    #[test]
    fn test_mesh_statistics_basic() {
        // A single triangle
        let vertices = vec![
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
        ];
        let normals = vec![
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
        ];
        let indices = vec![0, 1, 2];

        let stats = mesh_statistics(&vertices, &normals, &indices);
        assert_eq!(stats.vertex_count, 3);
        assert_eq!(stats.triangle_count, 1);
        assert_eq!(stats.unique_vertex_count, 3);
        assert_eq!(stats.degenerate_triangles, 0);
        assert!(stats.average_edge_length > 0.0);
        assert_eq!(stats.bounds_min, [0.0, 0.0, 0.0]);
        assert_eq!(stats.bounds_max, [1.0, 1.0, 0.0]);
        assert!(stats.memory_bytes > 0);
    }

    #[test]
    fn test_mesh_statistics_degenerate() {
        // A degenerate triangle (all vertices at same point)
        let vertices = vec![
            1.0, 1.0, 1.0,
            1.0, 1.0, 1.0,
            1.0, 1.0, 1.0,
        ];
        let normals = vec![0.0; 9];
        let indices = vec![0, 1, 2];

        let stats = mesh_statistics(&vertices, &normals, &indices);
        assert_eq!(stats.triangle_count, 1);
        assert_eq!(stats.degenerate_triangles, 1);
        // Only 1 unique vertex since all are at the same position
        assert_eq!(stats.unique_vertex_count, 1);
    }

    #[test]
    fn test_remove_degenerate_triangles() {
        // Two triangles: one valid, one degenerate (zero area)
        let vertices = vec![
            0.0, 0.0, 0.0,  // 0
            1.0, 0.0, 0.0,  // 1
            0.0, 1.0, 0.0,  // 2
            5.0, 5.0, 5.0,  // 3 (degenerate - all same point)
            5.0, 5.0, 5.0,  // 4
            5.0, 5.0, 5.0,  // 5
        ];
        let normals = vec![
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
        ];
        let indices = vec![0, 1, 2, 3, 4, 5];

        let (new_verts, new_norms, new_indices) =
            remove_degenerate_triangles(&vertices, &normals, &indices, 0.001);

        // Only the valid triangle should remain
        assert_eq!(new_indices.len(), 3, "Should have 1 triangle (3 indices), got {}", new_indices.len());
        // Only 3 vertices should remain (the valid triangle)
        assert_eq!(new_verts.len() / 3, 3);
        assert_eq!(new_norms.len() / 3, 3);
    }

    #[test]
    fn test_optimize_vertex_cache_does_not_lose_triangles() {
        // Create a simple mesh with 4 triangles forming a strip
        let mut indices = vec![
            0, 1, 2,
            2, 1, 3,
            2, 3, 4,
            4, 3, 5,
        ];
        let original_len = indices.len();

        optimize_vertex_cache(&mut indices, 6);

        // Should still have same number of indices
        assert_eq!(indices.len(), original_len);
        // All indices should still be in valid range
        for &idx in &indices {
            assert!(idx < 6, "Index {} out of range", idx);
        }
    }
}
