//! Geometry Type Dispatch and Tessellation
//!
//! Entry point for converting IFC geometry representations into triangle meshes.
//! Supports: FacetedBrep, ExtrudedAreaSolid, RevolvedAreaSolid, TriangulatedFaceSet,
//! PolygonalFaceSet, ShellBasedSurfaceModel, FaceBasedSurfaceModel, SweptDiskSolid,
//! MappedItem, BooleanClippingResult.

use std::collections::HashMap;

use glam::{Mat4, Vec3};

use super::entities::{EntityId, IfcEntity, IfcValue};
use super::geometry::Mesh;
use super::placement::{
    axis2_placement_3d_to_matrix, axis2_placement_2d_offset, cartesian_transform_operator_3d,
    read_cartesian_point, read_direction, PlacementCache,
};
use super::triangulate::triangulate_polygon;

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
pub fn tessellate_product(
    product_id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    placement_cache: &mut PlacementCache,
    color: [f32; 4],
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

            if let Some(item_mesh) = tessellate_geometry_item(item_id, entities, placement_cache, color) {
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
            None // Skip — used only as boolean operand
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
            tessellate_mapped_item(item_id, entities, placement_cache, color)
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

/// Phase 1: tessellate the first operand only (skip clipping).
/// IFCBOOLEANCLIPPINGRESULT(Operator, FirstOperand, SecondOperand)
fn tessellate_boolean_result(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    placement_cache: &mut PlacementCache,
    color: [f32; 4],
) -> Option<Mesh> {
    let entity = entities.get(&id)?;
    // FirstOperand (attribute 1)
    let first_operand = entity.get_entity_ref(1)?;
    tessellate_geometry_item(first_operand, entities, placement_cache, color)
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
// IFCMAPPEDITEM
// ---------------------------------------------------------------------------

/// IFCMAPPEDITEM(MappingSource, MappingTarget)
/// MappingSource: IFCREPRESENTATIONMAP(MappingOrigin, MappedRepresentation)
/// MappingTarget: IFCCARTESIANTRANSFORMATIONOPERATOR3D
fn tessellate_mapped_item(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    placement_cache: &mut PlacementCache,
    color: [f32; 4],
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

    // MappedRepresentation (attribute 1 of IFCREPRESENTATIONMAP) -> IFCSHAPEREPRESENTATION
    let mapped_rep_id = source.get_entity_ref(1)?;
    let mapped_rep = entities.get(&mapped_rep_id)?;

    // MappingTarget (attribute 1) -> IFCCARTESIANTRANSFORMATIONOPERATOR3D
    let target_matrix = entity
        .get_entity_ref(1)
        .map(|target_id| cartesian_transform_operator_3d(target_id, entities))
        .unwrap_or(Mat4::IDENTITY);

    // Tessellate the mapped representation's items
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

    // Apply: target_transform * origin_transform
    let combined_transform = target_matrix * origin_matrix;
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
