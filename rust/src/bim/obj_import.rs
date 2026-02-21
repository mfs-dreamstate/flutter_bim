//! OBJ File Import
//!
//! Parses Wavefront OBJ files into mesh data compatible with the BIM renderer.
//! Supports vertex positions, normals, faces with multiple formats, fan triangulation,
//! negative indices, and named objects/groups.

/// A named object/group within an OBJ file with its index range.
#[derive(Debug, Clone)]
pub struct ObjObject {
    pub name: String,
    pub index_start: u32,
    pub index_count: u32,
}

/// Parsed OBJ mesh data ready for rendering.
#[derive(Debug, Clone)]
pub struct ObjMesh {
    /// Flat vertex positions [x,y,z, x,y,z, ...]
    pub vertices: Vec<f32>,
    /// Flat vertex normals [nx,ny,nz, ...]
    pub normals: Vec<f32>,
    /// Triangle indices
    pub indices: Vec<u32>,
    /// Named objects with their index ranges
    pub objects: Vec<ObjObject>,
    /// Bounding box minimum
    pub bounds_min: [f32; 3],
    /// Bounding box maximum
    pub bounds_max: [f32; 3],
}

/// A single face vertex index set (position, texcoord, normal).
/// All indices are 0-based after resolution.
#[derive(Debug, Clone, Copy)]
struct FaceVertex {
    position: usize,
    normal: Option<usize>,
}

/// Parse an OBJ file from its text content.
///
/// Handles:
/// - `v x y z` vertex positions
/// - `vn nx ny nz` vertex normals
/// - `f` faces in all formats: `v`, `v/vt`, `v/vt/vn`, `v//vn`
/// - Faces with more than 3 vertices (fan triangulation)
/// - Negative indices (relative to end of vertex/normal list)
/// - `o`/`g` object/group names
/// - Computes face normals when no vertex normals are provided
pub fn parse_obj(content: &str) -> Result<ObjMesh, String> {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals_pool: Vec<[f32; 3]> = Vec::new();
    let mut faces: Vec<Vec<FaceVertex>> = Vec::new();

    let mut objects: Vec<ObjObject> = Vec::new();
    let mut current_object_name = String::from("default");
    let mut current_object_face_start: usize = 0;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let keyword = match parts.next() {
            Some(k) => k,
            None => continue,
        };

        match keyword {
            "v" => {
                let coords: Vec<f32> = parts
                    .take(3)
                    .map(|s| s.parse::<f32>())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("Invalid vertex coordinate: {}", e))?;
                if coords.len() < 3 {
                    return Err("Vertex requires 3 coordinates".to_string());
                }
                positions.push([coords[0], coords[1], coords[2]]);
            }
            "vn" => {
                let coords: Vec<f32> = parts
                    .take(3)
                    .map(|s| s.parse::<f32>())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("Invalid normal coordinate: {}", e))?;
                if coords.len() < 3 {
                    return Err("Normal requires 3 coordinates".to_string());
                }
                normals_pool.push([coords[0], coords[1], coords[2]]);
            }
            "vt" => {
                // Texture coordinates -- skip but consume
                continue;
            }
            "o" | "g" => {
                // Finalize previous object
                let face_count = faces.len() - current_object_face_start;
                if face_count > 0 {
                    objects.push(ObjObject {
                        name: current_object_name.clone(),
                        index_start: 0, // will be fixed up later
                        index_count: 0, // will be fixed up later
                    });
                }
                current_object_name = parts.collect::<Vec<&str>>().join(" ");
                if current_object_name.is_empty() {
                    current_object_name = "unnamed".to_string();
                }
                current_object_face_start = faces.len();
            }
            "f" => {
                let verts: Vec<FaceVertex> = parts
                    .map(|tok| parse_face_vertex(tok, positions.len(), normals_pool.len()))
                    .collect::<Result<Vec<_>, _>>()?;
                if verts.len() < 3 {
                    return Err("Face requires at least 3 vertices".to_string());
                }
                faces.push(verts);
            }
            "mtllib" | "usemtl" | "s" | "l" => {
                // Ignore material library, material usage, smoothing groups, lines
                continue;
            }
            _ => {
                // Unknown keyword, skip
                continue;
            }
        }
    }

    // Finalize the last object
    let face_count = faces.len() - current_object_face_start;
    if face_count > 0 || objects.is_empty() {
        objects.push(ObjObject {
            name: current_object_name,
            index_start: 0,
            index_count: 0,
        });
    }

    if positions.is_empty() {
        return Err("OBJ file contains no vertices".to_string());
    }

    // Determine if we have per-vertex normals
    let has_normals = !normals_pool.is_empty()
        && faces.iter().all(|f| f.iter().all(|v| v.normal.is_some()));

    // Build output mesh
    let mut out_vertices: Vec<f32> = Vec::new();
    let mut out_normals: Vec<f32> = Vec::new();
    let mut out_indices: Vec<u32> = Vec::new();

    let mut bounds_min = [f32::MAX; 3];
    let mut bounds_max = [f32::MIN; 3];

    // Update bounding box from positions
    for p in &positions {
        for i in 0..3 {
            bounds_min[i] = bounds_min[i].min(p[i]);
            bounds_max[i] = bounds_max[i].max(p[i]);
        }
    }

    // Compute which face belongs to which object
    // Re-derive object boundaries from the parsing state
    // We need to re-derive since we lost the exact boundaries
    // Instead, use the stored objects and assign faces proportionally.
    // Actually, let's track it properly.

    // Re-parse to get face boundaries per object. Instead, let me use a simpler approach:
    // We stored objects in order. The first N objects correspond to objects finalized
    // during parsing, and the last one is the remainder.
    // But we lost the exact face indices. Let me fix this by tracking face indices.

    // Actually, let's rebuild the object face ranges properly.
    // The objects were pushed when o/g was encountered or at the end.
    // We need to re-derive face start indices. Let me re-parse just the o/g lines.
    let mut obj_face_ranges: Vec<(usize, usize)> = Vec::new();
    {
        let mut obj_face_starts: Vec<usize> = Vec::new();
        let mut face_idx = 0;
        let mut pending_start: usize = 0;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let keyword = line.split_whitespace().next().unwrap_or("");
            match keyword {
                "f" => {
                    face_idx += 1;
                }
                "o" | "g" => {
                    if face_idx > pending_start || obj_face_starts.is_empty() {
                        obj_face_starts.push(pending_start);
                    }
                    pending_start = face_idx;
                }
                _ => {}
            }
        }
        // Last group
        obj_face_starts.push(pending_start);

        // Build ranges
        for i in 0..obj_face_starts.len() {
            let start = obj_face_starts[i];
            let end = if i + 1 < obj_face_starts.len() {
                obj_face_starts[i + 1]
            } else {
                faces.len()
            };
            if end > start {
                obj_face_ranges.push((start, end));
            }
        }

        // Handle case where no o/g directives exist
        if obj_face_ranges.is_empty() {
            obj_face_ranges.push((0, faces.len()));
        }
    }

    // Ensure objects vec matches face ranges
    while objects.len() < obj_face_ranges.len() {
        objects.push(ObjObject {
            name: "unnamed".to_string(),
            index_start: 0,
            index_count: 0,
        });
    }

    // Now generate the output mesh
    if has_normals {
        // Indexed mesh with provided normals
        // We need to create unique (position, normal) pairs
        use std::collections::HashMap;
        let mut vertex_map: HashMap<(usize, usize), u32> = HashMap::new();
        let mut current_vertex_idx: u32 = 0;

        for (range_idx, &(face_start, face_end)) in obj_face_ranges.iter().enumerate() {
            let index_start = out_indices.len() as u32;

            for face in &faces[face_start..face_end] {
                // Fan triangulation: triangle fan from vertex 0
                for tri in 0..(face.len() - 2) {
                    let tri_verts = [face[0], face[tri + 1], face[tri + 2]];
                    for fv in &tri_verts {
                        let key = (fv.position, fv.normal.unwrap());
                        let idx = vertex_map.entry(key).or_insert_with(|| {
                            let pos = positions[fv.position];
                            let nrm = normals_pool[fv.normal.unwrap()];
                            out_vertices.extend_from_slice(&pos);
                            out_normals.extend_from_slice(&nrm);
                            let idx = current_vertex_idx;
                            current_vertex_idx += 1;
                            idx
                        });
                        out_indices.push(*idx);
                    }
                }
            }

            let index_count = out_indices.len() as u32 - index_start;
            if range_idx < objects.len() {
                objects[range_idx].index_start = index_start;
                objects[range_idx].index_count = index_count;
            }
        }
    } else {
        // Flat (non-indexed) output with computed face normals
        // Each triangle gets its own vertices with the face normal
        for (range_idx, &(face_start, face_end)) in obj_face_ranges.iter().enumerate() {
            let index_start = out_indices.len() as u32;

            for face in &faces[face_start..face_end] {
                // Compute face normal from first 3 vertices
                let p0 = positions[face[0].position];
                let p1 = positions[face[1].position];
                let p2 = positions[face[2].position];

                let face_normal = compute_face_normal(p0, p1, p2);

                // Fan triangulation
                for tri in 0..(face.len() - 2) {
                    let tri_positions = [
                        positions[face[0].position],
                        positions[face[tri + 1].position],
                        positions[face[tri + 2].position],
                    ];

                    for pos in &tri_positions {
                        let idx = (out_vertices.len() / 3) as u32;
                        out_vertices.extend_from_slice(pos);
                        out_normals.extend_from_slice(&face_normal);
                        out_indices.push(idx);
                    }
                }
            }

            let index_count = out_indices.len() as u32 - index_start;
            if range_idx < objects.len() {
                objects[range_idx].index_start = index_start;
                objects[range_idx].index_count = index_count;
            }
        }
    }

    // Handle edge case: no faces parsed but vertices exist
    if faces.is_empty() && !positions.is_empty() {
        // Return empty mesh with correct bounds
        return Ok(ObjMesh {
            vertices: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            objects: Vec::new(),
            bounds_min,
            bounds_max,
        });
    }

    Ok(ObjMesh {
        vertices: out_vertices,
        normals: out_normals,
        indices: out_indices,
        objects,
        bounds_min,
        bounds_max,
    })
}

/// Parse a face vertex token like "1", "1/2", "1/2/3", or "1//3".
/// Handles negative indices (relative to end of list).
fn parse_face_vertex(
    token: &str,
    position_count: usize,
    normal_count: usize,
) -> Result<FaceVertex, String> {
    let parts: Vec<&str> = token.split('/').collect();

    let pos_idx = parse_obj_index(parts[0], position_count)?;

    let normal_idx = if parts.len() >= 3 && !parts[2].is_empty() {
        Some(parse_obj_index(parts[2], normal_count)?)
    } else {
        None
    };

    Ok(FaceVertex {
        position: pos_idx,
        normal: normal_idx,
    })
}

/// Parse an OBJ index (1-based, possibly negative) to a 0-based index.
fn parse_obj_index(s: &str, count: usize) -> Result<usize, String> {
    let idx: i64 = s
        .parse()
        .map_err(|e| format!("Invalid index '{}': {}", s, e))?;

    if idx == 0 {
        return Err("OBJ indices are 1-based, got 0".to_string());
    }

    let resolved = if idx > 0 {
        (idx - 1) as usize
    } else {
        // Negative index: relative to end
        if (-idx) as usize > count {
            return Err(format!(
                "Negative index {} out of range (count={})",
                idx, count
            ));
        }
        count - ((-idx) as usize)
    };

    if resolved >= count {
        return Err(format!(
            "Index {} out of range (count={})",
            idx, count
        ));
    }

    Ok(resolved)
}

/// Compute the face normal from three vertex positions using cross product.
fn compute_face_normal(p0: [f32; 3], p1: [f32; 3], p2: [f32; 3]) -> [f32; 3] {
    let u = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let v = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

    let nx = u[1] * v[2] - u[2] * v[1];
    let ny = u[2] * v[0] - u[0] * v[2];
    let nz = u[0] * v[1] - u[1] * v[0];

    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len < 1e-10 {
        [0.0, 1.0, 0.0] // degenerate face, return up vector
    } else {
        [nx / len, ny / len, nz / len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_triangle() {
        let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
";
        let mesh = parse_obj(obj).unwrap();
        assert_eq!(mesh.indices.len(), 3);
        // 3 vertices * 3 floats each = 9
        assert_eq!(mesh.vertices.len(), 9);
        assert_eq!(mesh.normals.len(), 9);
        // Check bounding box
        assert_eq!(mesh.bounds_min, [0.0, 0.0, 0.0]);
        assert_eq!(mesh.bounds_max, [1.0, 1.0, 0.0]);
    }

    #[test]
    fn test_parse_cube() {
        let obj = "\
# Simple cube
v -0.5 -0.5 -0.5
v  0.5 -0.5 -0.5
v  0.5  0.5 -0.5
v -0.5  0.5 -0.5
v -0.5 -0.5  0.5
v  0.5 -0.5  0.5
v  0.5  0.5  0.5
v -0.5  0.5  0.5

f 1 2 3 4
f 5 8 7 6
f 1 5 6 2
f 2 6 7 3
f 3 7 8 4
f 4 8 5 1
";
        let mesh = parse_obj(obj).unwrap();
        // 6 faces * 2 triangles each * 3 indices = 36
        assert_eq!(mesh.indices.len(), 36);
        // Flat output: 36 vertices
        assert_eq!(mesh.vertices.len(), 36 * 3);
        assert_eq!(mesh.normals.len(), 36 * 3);
        // Bounding box
        assert!((mesh.bounds_min[0] - (-0.5)).abs() < 1e-6);
        assert!((mesh.bounds_max[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_face_format_v_vt_vn() {
        let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
vn 0.0 0.0 1.0
f 1/1/1 2/2/1 3/3/1
";
        let mesh = parse_obj(obj).unwrap();
        assert_eq!(mesh.indices.len(), 3);
        assert_eq!(mesh.vertices.len(), 9);
        // Normals should be from vn
        assert!((mesh.normals[2] - 1.0).abs() < 1e-6); // z component of first normal
    }

    #[test]
    fn test_face_format_v_vn() {
        let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vn 0.0 0.0 1.0
f 1//1 2//1 3//1
";
        let mesh = parse_obj(obj).unwrap();
        assert_eq!(mesh.indices.len(), 3);
        assert!((mesh.normals[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_face_format_v_vt() {
        let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
f 1/1 2/2 3/3
";
        let mesh = parse_obj(obj).unwrap();
        assert_eq!(mesh.indices.len(), 3);
        // No vn provided, so computed face normal
        assert_eq!(mesh.normals.len(), 9);
    }

    #[test]
    fn test_negative_indices() {
        let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f -3 -2 -1
";
        let mesh = parse_obj(obj).unwrap();
        assert_eq!(mesh.indices.len(), 3);
        // -3 = vertex 0, -2 = vertex 1, -1 = vertex 2
        // Flat output, so check positions
        assert!((mesh.vertices[0] - 0.0).abs() < 1e-6); // v1.x
        assert!((mesh.vertices[3] - 1.0).abs() < 1e-6); // v2.x
        assert!((mesh.vertices[7] - 1.0).abs() < 1e-6); // v3.y
    }

    #[test]
    fn test_computed_face_normals() {
        // Triangle in XY plane - normal should be +Z
        let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
";
        let mesh = parse_obj(obj).unwrap();
        // Normal should be approximately [0, 0, 1]
        assert!((mesh.normals[0]).abs() < 1e-6); // nx
        assert!((mesh.normals[1]).abs() < 1e-6); // ny
        assert!((mesh.normals[2] - 1.0).abs() < 1e-6); // nz
    }

    #[test]
    fn test_fan_triangulation() {
        // Pentagon face
        let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.5 1.0 0.0
v 0.5 1.5 0.0
v -0.5 1.0 0.0
f 1 2 3 4 5
";
        let mesh = parse_obj(obj).unwrap();
        // 5-gon should produce 3 triangles = 9 indices
        assert_eq!(mesh.indices.len(), 9);
    }

    #[test]
    fn test_objects_tracking() {
        let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
v 2.0 0.0 0.0
v 2.0 1.0 0.0
v 3.0 0.0 0.0

o Object1
f 1 2 3

o Object2
f 4 5 6
";
        let mesh = parse_obj(obj).unwrap();
        assert_eq!(mesh.objects.len(), 2);
        assert_eq!(mesh.objects[0].name, "Object1");
        assert_eq!(mesh.objects[1].name, "Object2");
        assert_eq!(mesh.objects[0].index_count, 3);
        assert_eq!(mesh.objects[1].index_count, 3);
    }

    #[test]
    fn test_empty_file_error() {
        let obj = "# just a comment\n";
        let result = parse_obj(obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_mixed_normals_consistency() {
        // All faces have normals
        let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
v 0.0 0.0 1.0
vn 0.0 0.0 1.0
vn 0.0 1.0 0.0
f 1//1 2//1 3//1
f 1//2 2//2 4//2
";
        let mesh = parse_obj(obj).unwrap();
        assert_eq!(mesh.indices.len(), 6);
        // With normals provided, output is indexed with unique (pos, normal) pairs
        // 4 positions * 2 normals = up to 8 unique pairs, but shared ones are deduped
        // vertex 0 with normal 1, vertex 0 with normal 2, etc.
        // We expect at most 6 unique (pos, normal) combinations (2 faces * 3 verts)
        assert!(mesh.vertices.len() <= 6 * 3);
        // Normals should match vertex count
        assert_eq!(mesh.normals.len(), mesh.vertices.len());
    }

    #[test]
    fn test_parse_obj_index_positive() {
        assert_eq!(parse_obj_index("1", 5).unwrap(), 0);
        assert_eq!(parse_obj_index("5", 5).unwrap(), 4);
    }

    #[test]
    fn test_parse_obj_index_negative() {
        assert_eq!(parse_obj_index("-1", 5).unwrap(), 4);
        assert_eq!(parse_obj_index("-5", 5).unwrap(), 0);
    }

    #[test]
    fn test_parse_obj_index_out_of_range() {
        assert!(parse_obj_index("6", 5).is_err());
        assert!(parse_obj_index("-6", 5).is_err());
        assert!(parse_obj_index("0", 5).is_err());
    }
}
