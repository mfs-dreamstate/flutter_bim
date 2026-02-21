//! glTF/GLB File Import
//!
//! Parses glTF 2.0 JSON and GLB binary files into mesh data compatible with the BIM renderer.
//! Uses serde_json for JSON parsing and glam for matrix transforms.
//! Supports embedded data URIs, node transforms (matrix and TRS), and multiple accessor types.

use serde_json::Value;

/// A scene graph node from the glTF file with its index range in the output mesh.
#[derive(Debug, Clone)]
pub struct GltfNode {
    pub name: String,
    pub index_start: u32,
    pub index_count: u32,
}

/// Parsed glTF mesh data ready for rendering.
#[derive(Debug, Clone)]
pub struct GltfMesh {
    /// Flat vertex positions [x,y,z, ...]
    pub vertices: Vec<f32>,
    /// Flat vertex normals [nx,ny,nz, ...]
    pub normals: Vec<f32>,
    /// Triangle indices
    pub indices: Vec<u32>,
    /// RGBA per vertex (default white if not present in source)
    pub colors: Vec<f32>,
    /// Scene graph nodes with their index ranges
    pub nodes: Vec<GltfNode>,
    /// Bounding box minimum
    pub bounds_min: [f32; 3],
    /// Bounding box maximum
    pub bounds_max: [f32; 3],
}

/// glTF component types
const COMPONENT_BYTE: u64 = 5120;
const COMPONENT_UNSIGNED_BYTE: u64 = 5121;
const COMPONENT_SHORT: u64 = 5122;
const COMPONENT_UNSIGNED_SHORT: u64 = 5123;
const COMPONENT_UNSIGNED_INT: u64 = 5125;
const COMPONENT_FLOAT: u64 = 5126;

/// GLB magic number and chunk types
const GLB_MAGIC: u32 = 0x46546C67; // "glTF"
const GLB_JSON_CHUNK: u32 = 0x4E4F534A;
const GLB_BIN_CHUNK: u32 = 0x004E4942;

/// Parse a glTF JSON file with an optional binary buffer.
pub fn parse_gltf(json_content: &str, binary_chunk: Option<&[u8]>) -> Result<GltfMesh, String> {
    let root: Value =
        serde_json::from_str(json_content).map_err(|e| format!("Invalid glTF JSON: {}", e))?;

    // Verify version
    let version = root
        .pointer("/asset/version")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !version.starts_with("2.") {
        return Err(format!(
            "Unsupported glTF version '{}', expected 2.x",
            version
        ));
    }

    // Parse buffers
    let buffers = resolve_buffers(&root, binary_chunk)?;

    // Parse accessors, buffer views
    let buffer_views = parse_buffer_views(&root)?;
    let accessors = parse_accessors(&root)?;

    let mut result = GltfMesh {
        vertices: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
        colors: Vec::new(),
        nodes: Vec::new(),
        bounds_min: [f32::MAX; 3],
        bounds_max: [f32::MIN; 3],
    };

    // Get scene nodes
    let scene_idx = root
        .get("scene")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let root_nodes = root
        .pointer(&format!("/scenes/{}/nodes", scene_idx))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Process scene graph
    let identity = glam::Mat4::IDENTITY;
    for node_ref in &root_nodes {
        let node_idx = node_ref.as_u64().ok_or("Invalid node index")? as usize;
        process_node(
            node_idx,
            &identity,
            &root,
            &buffers,
            &buffer_views,
            &accessors,
            &mut result,
        )?;
    }

    // If no scenes, try processing all meshes directly
    if root_nodes.is_empty() {
        if let Some(nodes) = root.get("nodes").and_then(|v| v.as_array()) {
            for (idx, _) in nodes.iter().enumerate() {
                process_node(
                    idx,
                    &identity,
                    &root,
                    &buffers,
                    &buffer_views,
                    &accessors,
                    &mut result,
                )?;
            }
        }
    }

    // Update bounds from accessor min/max if we have no data yet
    if result.vertices.is_empty() {
        result.bounds_min = [0.0; 3];
        result.bounds_max = [0.0; 3];
    }

    Ok(result)
}

/// Parse a GLB binary file.
pub fn parse_glb(data: &[u8]) -> Result<GltfMesh, String> {
    if data.len() < 12 {
        return Err("GLB data too short for header".to_string());
    }

    // Read header
    let magic = read_u32_le(data, 0);
    if magic != GLB_MAGIC {
        return Err(format!(
            "Invalid GLB magic: 0x{:08X}, expected 0x{:08X}",
            magic, GLB_MAGIC
        ));
    }

    let version = read_u32_le(data, 4);
    if version != 2 {
        return Err(format!("Unsupported GLB version {}, expected 2", version));
    }

    let _total_length = read_u32_le(data, 8);

    // Parse chunks
    let mut offset = 12usize;
    let mut json_chunk: Option<&[u8]> = None;
    let mut bin_chunk: Option<&[u8]> = None;

    while offset + 8 <= data.len() {
        let chunk_length = read_u32_le(data, offset) as usize;
        let chunk_type = read_u32_le(data, offset + 4);
        offset += 8;

        if offset + chunk_length > data.len() {
            return Err("GLB chunk extends beyond data".to_string());
        }

        let chunk_data = &data[offset..offset + chunk_length];

        match chunk_type {
            GLB_JSON_CHUNK => {
                json_chunk = Some(chunk_data);
            }
            GLB_BIN_CHUNK => {
                bin_chunk = Some(chunk_data);
            }
            _ => {
                // Unknown chunk type, skip
            }
        }

        offset += chunk_length;
        // Chunks are padded to 4-byte alignment
        offset = (offset + 3) & !3;
    }

    let json_data = json_chunk.ok_or("GLB missing JSON chunk")?;
    let json_str =
        std::str::from_utf8(json_data).map_err(|e| format!("Invalid UTF-8 in JSON chunk: {}", e))?;

    parse_gltf(json_str, bin_chunk)
}

/// Buffer view descriptor
#[derive(Debug)]
struct BufferViewDesc {
    buffer: usize,
    byte_offset: usize,
    byte_length: usize,
    byte_stride: Option<usize>,
}

/// Accessor descriptor
#[derive(Debug)]
struct AccessorDesc {
    buffer_view: Option<usize>,
    byte_offset: usize,
    component_type: u64,
    count: usize,
    accessor_type: String,
    min: Option<Vec<f64>>,
    max: Option<Vec<f64>>,
}

fn parse_buffer_views(root: &Value) -> Result<Vec<BufferViewDesc>, String> {
    let views = match root.get("bufferViews").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Ok(Vec::new()),
    };

    views
        .iter()
        .enumerate()
        .map(|(i, bv)| {
            Ok(BufferViewDesc {
                buffer: bv
                    .get("buffer")
                    .and_then(|v| v.as_u64())
                    .ok_or(format!("bufferView {} missing buffer", i))?
                    as usize,
                byte_offset: bv
                    .get("byteOffset")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                byte_length: bv
                    .get("byteLength")
                    .and_then(|v| v.as_u64())
                    .ok_or(format!("bufferView {} missing byteLength", i))?
                    as usize,
                byte_stride: bv.get("byteStride").and_then(|v| v.as_u64()).map(|v| v as usize),
            })
        })
        .collect()
}

fn parse_accessors(root: &Value) -> Result<Vec<AccessorDesc>, String> {
    let accs = match root.get("accessors").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Ok(Vec::new()),
    };

    accs.iter()
        .enumerate()
        .map(|(i, acc)| {
            Ok(AccessorDesc {
                buffer_view: acc.get("bufferView").and_then(|v| v.as_u64()).map(|v| v as usize),
                byte_offset: acc
                    .get("byteOffset")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                component_type: acc
                    .get("componentType")
                    .and_then(|v| v.as_u64())
                    .ok_or(format!("accessor {} missing componentType", i))?,
                count: acc
                    .get("count")
                    .and_then(|v| v.as_u64())
                    .ok_or(format!("accessor {} missing count", i))?
                    as usize,
                accessor_type: acc
                    .get("type")
                    .and_then(|v| v.as_str())
                    .ok_or(format!("accessor {} missing type", i))?
                    .to_string(),
                min: acc
                    .get("min")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect()),
                max: acc
                    .get("max")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect()),
            })
        })
        .collect()
}

/// Resolve all buffers from the glTF JSON. Supports embedded data URIs and GLB binary chunk.
fn resolve_buffers(root: &Value, binary_chunk: Option<&[u8]>) -> Result<Vec<Vec<u8>>, String> {
    let buffer_defs = match root.get("buffers").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Ok(Vec::new()),
    };

    let mut buffers = Vec::with_capacity(buffer_defs.len());

    for (i, buf_def) in buffer_defs.iter().enumerate() {
        let uri = buf_def.get("uri").and_then(|v| v.as_str());

        if let Some(uri) = uri {
            if uri.starts_with("data:") {
                // Data URI: data:application/octet-stream;base64,XXXX
                let base64_data = uri
                    .find(",")
                    .map(|pos| &uri[pos + 1..])
                    .ok_or(format!("Invalid data URI in buffer {}", i))?;
                let decoded = decode_base64(base64_data)
                    .map_err(|e| format!("Failed to decode base64 buffer {}: {}", i, e))?;
                buffers.push(decoded);
            } else {
                // External file reference - not supported in this context
                return Err(format!(
                    "External buffer URI '{}' not supported (buffer {}). Use embedded data URIs or GLB format.",
                    uri, i
                ));
            }
        } else if i == 0 {
            // First buffer with no URI = GLB binary chunk
            if let Some(bin) = binary_chunk {
                buffers.push(bin.to_vec());
            } else {
                return Err(format!("Buffer {} has no URI and no binary chunk available", i));
            }
        } else {
            return Err(format!("Buffer {} has no URI", i));
        }
    }

    Ok(buffers)
}

/// Process a glTF node recursively, applying transforms.
fn process_node(
    node_idx: usize,
    parent_transform: &glam::Mat4,
    root: &Value,
    buffers: &[Vec<u8>],
    buffer_views: &[BufferViewDesc],
    accessors: &[AccessorDesc],
    result: &mut GltfMesh,
) -> Result<(), String> {
    let node = root
        .pointer(&format!("/nodes/{}", node_idx))
        .ok_or(format!("Node {} not found", node_idx))?;

    // Compute local transform
    let local_transform = get_node_transform(node)?;
    let world_transform = *parent_transform * local_transform;

    let node_name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&format!("node_{}", node_idx))
        .to_string();

    // Process mesh if present
    if let Some(mesh_idx) = node.get("mesh").and_then(|v| v.as_u64()) {
        let index_start = result.indices.len() as u32;

        process_mesh(
            mesh_idx as usize,
            &world_transform,
            root,
            buffers,
            buffer_views,
            accessors,
            result,
        )?;

        let index_count = result.indices.len() as u32 - index_start;
        if index_count > 0 {
            result.nodes.push(GltfNode {
                name: node_name.clone(),
                index_start,
                index_count,
            });
        }
    }

    // Process children
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child_ref in children {
            let child_idx = child_ref.as_u64().ok_or("Invalid child node index")? as usize;
            process_node(
                child_idx,
                &world_transform,
                root,
                buffers,
                buffer_views,
                accessors,
                result,
            )?;
        }
    }

    Ok(())
}

/// Extract node transform (matrix or TRS).
fn get_node_transform(node: &Value) -> Result<glam::Mat4, String> {
    if let Some(matrix) = node.get("matrix").and_then(|v| v.as_array()) {
        if matrix.len() != 16 {
            return Err("Node matrix must have 16 elements".to_string());
        }
        let m: Vec<f32> = matrix
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        // glTF uses column-major, glam::Mat4::from_cols_array also expects column-major
        Ok(glam::Mat4::from_cols_array(&[
            m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9], m[10], m[11], m[12],
            m[13], m[14], m[15],
        ]))
    } else {
        // TRS (Translation, Rotation, Scale)
        let translation = node
            .get("translation")
            .and_then(|v| v.as_array())
            .map(|arr| {
                glam::Vec3::new(
                    arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                )
            })
            .unwrap_or(glam::Vec3::ZERO);

        let rotation = node
            .get("rotation")
            .and_then(|v| v.as_array())
            .map(|arr| {
                glam::Quat::from_xyzw(
                    arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(3).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                )
            })
            .unwrap_or(glam::Quat::IDENTITY);

        let scale = node
            .get("scale")
            .and_then(|v| v.as_array())
            .map(|arr| {
                glam::Vec3::new(
                    arr.get(0).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                )
            })
            .unwrap_or(glam::Vec3::ONE);

        Ok(glam::Mat4::from_scale_rotation_translation(
            scale,
            rotation,
            translation,
        ))
    }
}

/// Process a glTF mesh (which contains primitives).
fn process_mesh(
    mesh_idx: usize,
    transform: &glam::Mat4,
    root: &Value,
    buffers: &[Vec<u8>],
    buffer_views: &[BufferViewDesc],
    accessors: &[AccessorDesc],
    result: &mut GltfMesh,
) -> Result<(), String> {
    let primitives = root
        .pointer(&format!("/meshes/{}/primitives", mesh_idx))
        .and_then(|v| v.as_array())
        .ok_or(format!("Mesh {} has no primitives", mesh_idx))?;

    let normal_matrix = transform.inverse().transpose();

    for (prim_idx, prim) in primitives.iter().enumerate() {
        // Check primitive mode (default 4 = TRIANGLES)
        let mode = prim.get("mode").and_then(|v| v.as_u64()).unwrap_or(4);
        if mode != 4 {
            // Only support triangles for now
            continue;
        }

        let attributes = prim
            .get("attributes")
            .ok_or(format!("Primitive {}/{} missing attributes", mesh_idx, prim_idx))?;

        // POSITION (required)
        let position_accessor_idx = attributes
            .get("POSITION")
            .and_then(|v| v.as_u64())
            .ok_or(format!(
                "Primitive {}/{} missing POSITION",
                mesh_idx, prim_idx
            ))? as usize;

        let positions = read_accessor_vec3(
            &accessors[position_accessor_idx],
            buffer_views,
            buffers,
        )?;

        // NORMAL (optional but important)
        let normals = if let Some(normal_idx) = attributes.get("NORMAL").and_then(|v| v.as_u64())
        {
            read_accessor_vec3(&accessors[normal_idx as usize], buffer_views, buffers)?
        } else {
            // Generate flat normals
            vec![[0.0, 1.0, 0.0]; positions.len()]
        };

        // COLOR_0 (optional)
        let colors = if let Some(color_idx) = attributes.get("COLOR_0").and_then(|v| v.as_u64()) {
            read_accessor_color(&accessors[color_idx as usize], buffer_views, buffers)?
        } else {
            vec![[1.0, 1.0, 1.0, 1.0]; positions.len()]
        };

        // Indices (optional)
        let indices = if let Some(indices_idx) = prim.get("indices").and_then(|v| v.as_u64()) {
            read_accessor_scalar(&accessors[indices_idx as usize], buffer_views, buffers)?
        } else {
            // Non-indexed: sequential indices
            (0..positions.len() as u32).collect()
        };

        // Apply transform and append to result
        let vertex_base = (result.vertices.len() / 3) as u32;

        for (i, pos) in positions.iter().enumerate() {
            let p = glam::Vec3::new(pos[0], pos[1], pos[2]);
            let tp = transform.transform_point3(p);
            result.vertices.extend_from_slice(&[tp.x, tp.y, tp.z]);

            // Update bounds
            result.bounds_min[0] = result.bounds_min[0].min(tp.x);
            result.bounds_min[1] = result.bounds_min[1].min(tp.y);
            result.bounds_min[2] = result.bounds_min[2].min(tp.z);
            result.bounds_max[0] = result.bounds_max[0].max(tp.x);
            result.bounds_max[1] = result.bounds_max[1].max(tp.y);
            result.bounds_max[2] = result.bounds_max[2].max(tp.z);

            // Transform normal
            if i < normals.len() {
                let n = glam::Vec3::new(normals[i][0], normals[i][1], normals[i][2]);
                let tn = normal_matrix.transform_vector3(n).normalize_or_zero();
                result.normals.extend_from_slice(&[tn.x, tn.y, tn.z]);
            } else {
                result.normals.extend_from_slice(&[0.0, 1.0, 0.0]);
            }

            // Colors
            if i < colors.len() {
                result
                    .colors
                    .extend_from_slice(&[colors[i][0], colors[i][1], colors[i][2], colors[i][3]]);
            } else {
                result.colors.extend_from_slice(&[1.0, 1.0, 1.0, 1.0]);
            }
        }

        // Append indices with base offset
        for idx in &indices {
            result.indices.push(vertex_base + idx);
        }

        // Update bounds from accessor min/max (if available and no vertices processed yet)
        if let Some(ref min_vals) = accessors[position_accessor_idx].min {
            if min_vals.len() >= 3 {
                for i in 0..3 {
                    result.bounds_min[i] = result.bounds_min[i].min(min_vals[i] as f32);
                }
            }
        }
        if let Some(ref max_vals) = accessors[position_accessor_idx].max {
            if max_vals.len() >= 3 {
                for i in 0..3 {
                    result.bounds_max[i] = result.bounds_max[i].max(max_vals[i] as f32);
                }
            }
        }
    }

    Ok(())
}

/// Read an accessor as Vec3 (e.g., POSITION, NORMAL).
fn read_accessor_vec3(
    accessor: &AccessorDesc,
    buffer_views: &[BufferViewDesc],
    buffers: &[Vec<u8>],
) -> Result<Vec<[f32; 3]>, String> {
    if accessor.accessor_type != "VEC3" {
        return Err(format!(
            "Expected VEC3 accessor, got {}",
            accessor.accessor_type
        ));
    }

    let bv_idx = accessor
        .buffer_view
        .ok_or("Accessor missing bufferView")?;
    let bv = &buffer_views[bv_idx];
    let buf = buffers
        .get(bv.buffer)
        .ok_or(format!("Buffer {} not found", bv.buffer))?;

    let stride = bv.byte_stride.unwrap_or(12); // 3 * sizeof(f32) = 12
    let base_offset = bv.byte_offset + accessor.byte_offset;

    let mut result = Vec::with_capacity(accessor.count);

    for i in 0..accessor.count {
        let offset = base_offset + i * stride;
        match accessor.component_type {
            COMPONENT_FLOAT => {
                if offset + 12 > buf.len() {
                    return Err(format!(
                        "Buffer read out of bounds at offset {} (buf len {})",
                        offset + 12,
                        buf.len()
                    ));
                }
                let x = read_f32_le(buf, offset);
                let y = read_f32_le(buf, offset + 4);
                let z = read_f32_le(buf, offset + 8);
                result.push([x, y, z]);
            }
            _ => {
                return Err(format!(
                    "Unsupported component type {} for VEC3",
                    accessor.component_type
                ));
            }
        }
    }

    Ok(result)
}

/// Read an accessor as scalar indices.
fn read_accessor_scalar(
    accessor: &AccessorDesc,
    buffer_views: &[BufferViewDesc],
    buffers: &[Vec<u8>],
) -> Result<Vec<u32>, String> {
    if accessor.accessor_type != "SCALAR" {
        return Err(format!(
            "Expected SCALAR accessor, got {}",
            accessor.accessor_type
        ));
    }

    let bv_idx = accessor
        .buffer_view
        .ok_or("Accessor missing bufferView")?;
    let bv = &buffer_views[bv_idx];
    let buf = buffers
        .get(bv.buffer)
        .ok_or(format!("Buffer {} not found", bv.buffer))?;

    let component_size = match accessor.component_type {
        COMPONENT_UNSIGNED_BYTE => 1,
        COMPONENT_BYTE => 1,
        COMPONENT_UNSIGNED_SHORT => 2,
        COMPONENT_SHORT => 2,
        COMPONENT_UNSIGNED_INT => 4,
        _ => {
            return Err(format!(
                "Unsupported scalar component type {}",
                accessor.component_type
            ))
        }
    };

    let stride = bv.byte_stride.unwrap_or(component_size);
    let base_offset = bv.byte_offset + accessor.byte_offset;

    let mut result = Vec::with_capacity(accessor.count);

    for i in 0..accessor.count {
        let offset = base_offset + i * stride;
        let value = match accessor.component_type {
            COMPONENT_UNSIGNED_BYTE => {
                if offset >= buf.len() {
                    return Err("Buffer read out of bounds".to_string());
                }
                buf[offset] as u32
            }
            COMPONENT_BYTE => {
                if offset >= buf.len() {
                    return Err("Buffer read out of bounds".to_string());
                }
                buf[offset] as i8 as u32
            }
            COMPONENT_UNSIGNED_SHORT => {
                if offset + 2 > buf.len() {
                    return Err("Buffer read out of bounds".to_string());
                }
                read_u16_le(buf, offset) as u32
            }
            COMPONENT_SHORT => {
                if offset + 2 > buf.len() {
                    return Err("Buffer read out of bounds".to_string());
                }
                read_i16_le(buf, offset) as u32
            }
            COMPONENT_UNSIGNED_INT => {
                if offset + 4 > buf.len() {
                    return Err("Buffer read out of bounds".to_string());
                }
                read_u32_le(buf, offset)
            }
            _ => {
                return Err(format!(
                    "Unsupported scalar type {}",
                    accessor.component_type
                ))
            }
        };
        result.push(value);
    }

    Ok(result)
}

/// Read an accessor as RGBA color. Handles VEC3 (RGB, alpha=1) and VEC4 (RGBA).
/// Supports FLOAT and UNSIGNED_BYTE (normalized) component types.
fn read_accessor_color(
    accessor: &AccessorDesc,
    buffer_views: &[BufferViewDesc],
    buffers: &[Vec<u8>],
) -> Result<Vec<[f32; 4]>, String> {
    let components = match accessor.accessor_type.as_str() {
        "VEC3" => 3,
        "VEC4" => 4,
        _ => {
            return Err(format!(
                "Unsupported color accessor type {}",
                accessor.accessor_type
            ))
        }
    };

    let bv_idx = accessor
        .buffer_view
        .ok_or("Color accessor missing bufferView")?;
    let bv = &buffer_views[bv_idx];
    let buf = buffers
        .get(bv.buffer)
        .ok_or(format!("Buffer {} not found", bv.buffer))?;

    let component_size = match accessor.component_type {
        COMPONENT_FLOAT => 4,
        COMPONENT_UNSIGNED_BYTE => 1,
        COMPONENT_UNSIGNED_SHORT => 2,
        _ => {
            return Err(format!(
                "Unsupported color component type {}",
                accessor.component_type
            ))
        }
    };

    let default_stride = components * component_size;
    let stride = bv.byte_stride.unwrap_or(default_stride);
    let base_offset = bv.byte_offset + accessor.byte_offset;

    let mut result = Vec::with_capacity(accessor.count);

    for i in 0..accessor.count {
        let offset = base_offset + i * stride;
        let mut rgba = [1.0f32; 4];

        for c in 0..components {
            let co = offset + c * component_size;
            rgba[c] = match accessor.component_type {
                COMPONENT_FLOAT => {
                    if co + 4 > buf.len() {
                        return Err("Buffer read out of bounds".to_string());
                    }
                    read_f32_le(buf, co)
                }
                COMPONENT_UNSIGNED_BYTE => {
                    if co >= buf.len() {
                        return Err("Buffer read out of bounds".to_string());
                    }
                    buf[co] as f32 / 255.0
                }
                COMPONENT_UNSIGNED_SHORT => {
                    if co + 2 > buf.len() {
                        return Err("Buffer read out of bounds".to_string());
                    }
                    read_u16_le(buf, co) as f32 / 65535.0
                }
                _ => 1.0,
            };
        }

        result.push(rgba);
    }

    Ok(result)
}

// ========================================================================
// Binary reading helpers
// ========================================================================

fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_i16_le(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_f32_le(data: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

// ========================================================================
// Base64 decoder (minimal, avoids external dependency)
// ========================================================================

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut lookup = [255u8; 256];
    for (i, &c) in TABLE.iter().enumerate() {
        lookup[c as usize] = i as u8;
    }

    // Filter out whitespace
    let clean: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();

    let mut output = Vec::with_capacity(clean.len() * 3 / 4);
    let mut i = 0;

    while i < clean.len() {
        let remaining = clean.len() - i;
        if remaining < 2 {
            break;
        }

        let a = lookup[clean[i] as usize];
        let b = lookup[clean[i + 1] as usize];

        if a == 255 || b == 255 {
            if clean[i] == b'=' {
                break;
            }
            return Err(format!("Invalid base64 character at position {}", i));
        }

        output.push((a << 2) | (b >> 4));

        if i + 2 < clean.len() && clean[i + 2] != b'=' {
            let c = lookup[clean[i + 2] as usize];
            if c == 255 {
                return Err(format!("Invalid base64 character at position {}", i + 2));
            }
            output.push((b << 4) | (c >> 2));

            if i + 3 < clean.len() && clean[i + 3] != b'=' {
                let d = lookup[clean[i + 3] as usize];
                if d == 255 {
                    return Err(format!("Invalid base64 character at position {}", i + 3));
                }
                output.push((c << 6) | d);
            }
        }

        i += 4;
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glb_header_parsing() {
        // Construct a minimal GLB with just a header (no valid chunks)
        let mut data = Vec::new();
        // Magic
        data.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        // Version
        data.extend_from_slice(&2u32.to_le_bytes());
        // Total length (just header)
        data.extend_from_slice(&12u32.to_le_bytes());

        let result = parse_glb(&data);
        // Should fail because no JSON chunk
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing JSON chunk"));
    }

    #[test]
    fn test_glb_invalid_magic() {
        let data = vec![0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00];
        let result = parse_glb(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid GLB magic"));
    }

    #[test]
    fn test_glb_too_short() {
        let data = vec![0x00, 0x01, 0x02];
        let result = parse_glb(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }

    #[test]
    fn test_gltf_version_check() {
        let json = r#"{"asset":{"version":"1.0"}}"#;
        let result = parse_gltf(json, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported glTF version"));
    }

    #[test]
    fn test_gltf_empty_scene() {
        let json = r#"{
            "asset": {"version": "2.0"},
            "scenes": [{"nodes": []}],
            "scene": 0
        }"#;
        let result = parse_gltf(json, None);
        assert!(result.is_ok());
        let mesh = result.unwrap();
        assert!(mesh.vertices.is_empty());
        assert!(mesh.indices.is_empty());
    }

    #[test]
    fn test_gltf_simple_triangle() {
        // Build a minimal glTF with a single triangle using a data URI buffer
        // Triangle: (0,0,0), (1,0,0), (0,1,0)
        let mut buffer = Vec::new();

        // Positions: 3 VEC3 floats
        let positions: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        for p in &positions {
            for &v in p {
                buffer.extend_from_slice(&v.to_le_bytes());
            }
        }
        // Normals: 3 VEC3 floats
        let normals: [[f32; 3]; 3] = [[0.0, 0.0, 1.0]; 3];
        for n in &normals {
            for &v in n {
                buffer.extend_from_slice(&v.to_le_bytes());
            }
        }
        // Indices: 3 UNSIGNED_SHORT
        let indices: [u16; 3] = [0, 1, 2];
        for &idx in &indices {
            buffer.extend_from_slice(&idx.to_le_bytes());
        }
        // Pad to 4-byte alignment
        while buffer.len() % 4 != 0 {
            buffer.push(0);
        }

        // Encode buffer as base64
        let b64 = encode_base64(&buffer);
        let data_uri = format!("data:application/octet-stream;base64,{}", b64);

        let json = format!(
            r#"{{
                "asset": {{"version": "2.0"}},
                "scene": 0,
                "scenes": [{{"nodes": [0]}}],
                "nodes": [{{"mesh": 0}}],
                "meshes": [{{
                    "primitives": [{{
                        "attributes": {{"POSITION": 0, "NORMAL": 1}},
                        "indices": 2
                    }}]
                }}],
                "accessors": [
                    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
                      "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0]}},
                    {{"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3"}},
                    {{"bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR"}}
                ],
                "bufferViews": [
                    {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
                    {{"buffer": 0, "byteOffset": 36, "byteLength": 36}},
                    {{"buffer": 0, "byteOffset": 72, "byteLength": 6}}
                ],
                "buffers": [{{"uri": "{}", "byteLength": {}}}]
            }}"#,
            data_uri,
            buffer.len()
        );

        let result = parse_gltf(&json, None).unwrap();
        assert_eq!(result.indices.len(), 3);
        assert_eq!(result.vertices.len(), 9); // 3 vertices * 3 floats
        assert_eq!(result.normals.len(), 9);
        assert_eq!(result.colors.len(), 12); // 3 vertices * 4 floats (default white)

        // Check bounds
        assert!(result.bounds_min[0] <= 0.0);
        assert!(result.bounds_max[0] >= 1.0);
    }

    #[test]
    fn test_gltf_node_transform_trs() {
        // Test TRS transform parsing
        let node_json = r#"{"translation": [1.0, 2.0, 3.0], "rotation": [0.0, 0.0, 0.0, 1.0], "scale": [2.0, 2.0, 2.0]}"#;
        let node: Value = serde_json::from_str(node_json).unwrap();
        let transform = get_node_transform(&node).unwrap();

        // Verify translation
        let p = transform.transform_point3(glam::Vec3::ZERO);
        assert!((p.x - 1.0).abs() < 1e-6);
        assert!((p.y - 2.0).abs() < 1e-6);
        assert!((p.z - 3.0).abs() < 1e-6);

        // Verify scale
        let p2 = transform.transform_point3(glam::Vec3::ONE);
        assert!((p2.x - 3.0).abs() < 1e-6); // 1*2 + 1 = 3
        assert!((p2.y - 4.0).abs() < 1e-6);
        assert!((p2.z - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_gltf_node_transform_matrix() {
        // Identity matrix
        let node_json = r#"{"matrix": [1,0,0,0, 0,1,0,0, 0,0,1,0, 5,6,7,1]}"#;
        let node: Value = serde_json::from_str(node_json).unwrap();
        let transform = get_node_transform(&node).unwrap();

        let p = transform.transform_point3(glam::Vec3::ZERO);
        assert!((p.x - 5.0).abs() < 1e-6);
        assert!((p.y - 6.0).abs() < 1e-6);
        assert!((p.z - 7.0).abs() < 1e-6);
    }

    #[test]
    fn test_accessor_types() {
        // Test reading UNSIGNED_SHORT indices
        let mut buf = Vec::new();
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes());

        let buffer_views = vec![BufferViewDesc {
            buffer: 0,
            byte_offset: 0,
            byte_length: 6,
            byte_stride: None,
        }];

        let accessor = AccessorDesc {
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: COMPONENT_UNSIGNED_SHORT,
            count: 3,
            accessor_type: "SCALAR".to_string(),
            min: None,
            max: None,
        };

        let indices = read_accessor_scalar(&accessor, &buffer_views, &[buf.clone()]).unwrap();
        assert_eq!(indices, vec![0, 1, 2]);

        // Test UNSIGNED_INT
        let mut buf32 = Vec::new();
        buf32.extend_from_slice(&100u32.to_le_bytes());
        buf32.extend_from_slice(&200u32.to_le_bytes());

        let buffer_views32 = vec![BufferViewDesc {
            buffer: 0,
            byte_offset: 0,
            byte_length: 8,
            byte_stride: None,
        }];

        let accessor32 = AccessorDesc {
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: COMPONENT_UNSIGNED_INT,
            count: 2,
            accessor_type: "SCALAR".to_string(),
            min: None,
            max: None,
        };

        let indices32 =
            read_accessor_scalar(&accessor32, &buffer_views32, &[buf32]).unwrap();
        assert_eq!(indices32, vec![100, 200]);

        // Test UNSIGNED_BYTE
        let buf8 = vec![5u8, 10, 15];
        let buffer_views8 = vec![BufferViewDesc {
            buffer: 0,
            byte_offset: 0,
            byte_length: 3,
            byte_stride: None,
        }];

        let accessor8 = AccessorDesc {
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: COMPONENT_UNSIGNED_BYTE,
            count: 3,
            accessor_type: "SCALAR".to_string(),
            min: None,
            max: None,
        };

        let indices8 =
            read_accessor_scalar(&accessor8, &buffer_views8, &[buf8]).unwrap();
        assert_eq!(indices8, vec![5, 10, 15]);
    }

    #[test]
    fn test_base64_decode() {
        let encoded = "SGVsbG8gV29ybGQ=";
        let decoded = decode_base64(encoded).unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_base64_roundtrip() {
        let data = vec![0u8, 1, 2, 3, 255, 254, 253, 128, 64, 32, 16, 8];
        let encoded = encode_base64(&data);
        let decoded = decode_base64(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_glb_full_roundtrip() {
        // Build a valid GLB with a simple triangle
        let mut buffer = Vec::new();

        // Positions: 3 VEC3 floats (36 bytes)
        for v in &[0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        // Normals: 3 VEC3 floats (36 bytes)
        for v in &[0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        // Indices: 3 UNSIGNED_SHORT (6 bytes)
        for &idx in &[0u16, 1, 2] {
            buffer.extend_from_slice(&idx.to_le_bytes());
        }
        // Pad binary to 4-byte alignment
        while buffer.len() % 4 != 0 {
            buffer.push(0);
        }

        let json_str = format!(
            r#"{{
                "asset": {{"version": "2.0"}},
                "scene": 0,
                "scenes": [{{"nodes": [0]}}],
                "nodes": [{{"mesh": 0}}],
                "meshes": [{{
                    "primitives": [{{
                        "attributes": {{"POSITION": 0, "NORMAL": 1}},
                        "indices": 2
                    }}]
                }}],
                "accessors": [
                    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"}},
                    {{"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3"}},
                    {{"bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR"}}
                ],
                "bufferViews": [
                    {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
                    {{"buffer": 0, "byteOffset": 36, "byteLength": 36}},
                    {{"buffer": 0, "byteOffset": 72, "byteLength": 6}}
                ],
                "buffers": [{{"byteLength": {}}}]
            }}"#,
            buffer.len()
        );

        // Build GLB
        let json_bytes = json_str.as_bytes();
        let json_padded_len = (json_bytes.len() + 3) & !3;
        let bin_padded_len = (buffer.len() + 3) & !3;

        let total_length = 12 + 8 + json_padded_len + 8 + bin_padded_len;

        let mut glb = Vec::with_capacity(total_length);
        // Header
        glb.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total_length as u32).to_le_bytes());

        // JSON chunk
        glb.extend_from_slice(&(json_padded_len as u32).to_le_bytes());
        glb.extend_from_slice(&GLB_JSON_CHUNK.to_le_bytes());
        glb.extend_from_slice(json_bytes);
        // Pad with spaces (0x20 per glTF spec)
        for _ in json_bytes.len()..json_padded_len {
            glb.push(0x20);
        }

        // Binary chunk
        glb.extend_from_slice(&(bin_padded_len as u32).to_le_bytes());
        glb.extend_from_slice(&GLB_BIN_CHUNK.to_le_bytes());
        glb.extend_from_slice(&buffer);
        for _ in buffer.len()..bin_padded_len {
            glb.push(0);
        }

        let result = parse_glb(&glb).unwrap();
        assert_eq!(result.indices.len(), 3);
        assert_eq!(result.vertices.len(), 9);
        assert_eq!(result.normals.len(), 9);
    }

    /// Helper: encode bytes to base64 (for test data generation)
    fn encode_base64(data: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
        for chunk in data.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
            let n = (b0 << 16) | (b1 << 8) | b2;

            result.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
            result.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 {
                result.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
            if chunk.len() > 2 {
                result.push(TABLE[(n & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
        }
        result
    }
}
