use flutter_rust_bridge::frb;
use std::collections::HashMap;

use super::state::with_state;

/// Renderer statistics
#[derive(Debug, Clone)]
pub struct RenderStats {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub triangle_count: u32,
    pub vertex_count: u32,
    pub element_count: u32,
}

/// Save current frame as PNG to the given path
pub async fn export_screenshot(path: String) -> Result<(), String> {
    let (image_data, width, height) = with_state(|s| {
        let r = s.renderer()?;
        let data = r.render_frame()?;
        let scene = r
            .scene
            .as_ref()
            .ok_or_else(|| "Scene not initialized".to_string())?;
        Ok::<_, String>((data, scene.width, scene.height))
    })?;

    match image::save_buffer(
        &path,
        &image_data,
        width,
        height,
        image::ColorType::Rgba8,
    ) {
        Ok(_) => {
            tracing::info!("Screenshot saved to: {}", path);
            Ok(())
        }
        Err(e) => Err(format!("Failed to save screenshot: {}", e)),
    }
}

/// Get current frame as RGBA bytes
#[frb(sync)]
pub fn get_current_frame_rgba() -> Result<Vec<u8>, String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.render_frame()
    })
}

/// Get renderer statistics
#[frb(sync)]
pub fn get_render_stats() -> Result<RenderStats, String> {
    with_state(|s| {
        let r = s.renderer.as_ref().ok_or("Renderer not initialized")?;
        let element_count: usize = s
            .registry
            .models()
            .values()
            .map(|reg| reg.model.element_count)
            .sum();

        Ok(RenderStats {
            fps: 60.0,
            frame_time_ms: 16.67,
            triangle_count: r.scene.as_ref().map(|s| s.num_indices / 3).unwrap_or(0),
            vertex_count: 0,
            element_count: element_count as u32,
        })
    })
}

// ========================================================================
// CSV Helpers
// ========================================================================

/// Escape a value for CSV output: wrap in quotes if it contains comma,
/// double-quote, or newline, and double any existing quotes.
fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

// ========================================================================
// Base64 Encoder (no external crate)
// ========================================================================

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

// ========================================================================
// 1. CSV Property Export
// ========================================================================

/// Export all element properties as CSV string.
/// Format: ElementID, GlobalID, Name, Type, Storey, PropertySet, PropertyName, PropertyValue
#[frb(sync)]
pub fn export_properties_csv() -> Result<String, String> {
    with_state(|s| {
        let mut csv = String::from("ElementID,GlobalID,Name,Type,Storey,PropertySet,PropertyName,PropertyValue\n");

        for (_model_id, reg_model) in s.registry.iter_visible() {
            // Build storey lookup: storey entity id -> storey name
            let storey_lookup: HashMap<i32, &str> = reg_model
                .model
                .storeys
                .iter()
                .map(|st| (st.id, st.name.as_str()))
                .collect();

            for elem in reg_model.elements() {
                // Look up the storey name for this element
                let storey_name = reg_model
                    .model
                    .element_to_storey
                    .get(&elem.id)
                    .and_then(|storey_id| storey_lookup.get(storey_id).copied())
                    .unwrap_or("");

                // Get property sets for this element
                if let Some(psets) = reg_model.model.element_property_sets.get(&elem.id) {
                    for pset in psets {
                        for (prop_name, prop_value) in &pset.properties {
                            csv.push_str(&csv_escape(&elem.id.to_string()));
                            csv.push(',');
                            csv.push_str(&csv_escape(&elem.global_id));
                            csv.push(',');
                            csv.push_str(&csv_escape(&elem.name));
                            csv.push(',');
                            csv.push_str(&csv_escape(&elem.element_type));
                            csv.push(',');
                            csv.push_str(&csv_escape(storey_name));
                            csv.push(',');
                            csv.push_str(&csv_escape(&pset.name));
                            csv.push(',');
                            csv.push_str(&csv_escape(prop_name));
                            csv.push(',');
                            csv.push_str(&csv_escape(prop_value));
                            csv.push('\n');
                        }
                    }
                }
            }
        }

        Ok(csv)
    })
}

// ========================================================================
// 2. Element Summary CSV
// ========================================================================

/// Export element summary as CSV (one row per element with basic info + bounds).
/// Format: ID, GlobalID, Name, Type, Storey, BoundsMinX, BoundsMinY, BoundsMinZ, BoundsMaxX, BoundsMaxY, BoundsMaxZ
#[frb(sync)]
pub fn export_element_summary_csv() -> Result<String, String> {
    with_state(|s| {
        let mut csv = String::from(
            "ID,GlobalID,Name,Type,Storey,BoundsMinX,BoundsMinY,BoundsMinZ,BoundsMaxX,BoundsMaxY,BoundsMaxZ\n",
        );

        for (_model_id, reg_model) in s.registry.iter_visible() {
            // Build storey lookup: storey entity id -> storey name
            let storey_lookup: HashMap<i32, &str> = reg_model
                .model
                .storeys
                .iter()
                .map(|st| (st.id, st.name.as_str()))
                .collect();

            for elem in reg_model.elements() {
                let storey_name = reg_model
                    .model
                    .element_to_storey
                    .get(&elem.id)
                    .and_then(|storey_id| storey_lookup.get(storey_id).copied())
                    .unwrap_or("");

                csv.push_str(&csv_escape(&elem.id.to_string()));
                csv.push(',');
                csv.push_str(&csv_escape(&elem.global_id));
                csv.push(',');
                csv.push_str(&csv_escape(&elem.name));
                csv.push(',');
                csv.push_str(&csv_escape(&elem.element_type));
                csv.push(',');
                csv.push_str(&csv_escape(storey_name));
                csv.push(',');
                csv.push_str(&format!("{:.6}", elem.bounds.min[0]));
                csv.push(',');
                csv.push_str(&format!("{:.6}", elem.bounds.min[1]));
                csv.push(',');
                csv.push_str(&format!("{:.6}", elem.bounds.min[2]));
                csv.push(',');
                csv.push_str(&format!("{:.6}", elem.bounds.max[0]));
                csv.push(',');
                csv.push_str(&format!("{:.6}", elem.bounds.max[1]));
                csv.push(',');
                csv.push_str(&format!("{:.6}", elem.bounds.max[2]));
                csv.push('\n');
            }
        }

        Ok(csv)
    })
}

// ========================================================================
// 3. glTF Export
// ========================================================================

/// Export all visible models as glTF 2.0 JSON string.
/// This produces a complete self-contained glTF with embedded base64 buffers.
/// Can be saved as .gltf file and opened in any 3D viewer.
#[frb(sync)]
pub fn export_gltf_json() -> Result<String, String> {
    with_state(|s| {
        // Gather all vertex data from visible models
        let mut all_positions: Vec<f32> = Vec::new();
        let mut all_normals: Vec<f32> = Vec::new();
        let mut all_colors: Vec<f32> = Vec::new();
        let mut all_indices: Vec<u32> = Vec::new();

        for (_model_id, reg_model) in s.registry.iter_visible() {
            if let Some(mesh) = reg_model.cached_mesh() {
                let vertex_offset = (all_positions.len() / 3) as u32;

                // Positions (Vec3 f32)
                all_positions.extend_from_slice(&mesh.vertices);

                // Normals (Vec3 f32)
                all_normals.extend_from_slice(&mesh.normals);

                // Colors (Vec4 f32) - mesh.colors is rgba
                all_colors.extend_from_slice(&mesh.colors);

                // Indices (u32), offset by current vertex count
                for idx in &mesh.indices {
                    all_indices.push(idx + vertex_offset);
                }
            }
        }

        if all_positions.is_empty() {
            return Err("No visible mesh data to export".to_string());
        }

        let vertex_count = all_positions.len() / 3;
        let index_count = all_indices.len();

        // Compute min/max for position accessor
        let mut pos_min = [f32::MAX; 3];
        let mut pos_max = [f32::MIN; 3];
        for i in 0..vertex_count {
            let base = i * 3;
            for c in 0..3 {
                pos_min[c] = pos_min[c].min(all_positions[base + c]);
                pos_max[c] = pos_max[c].max(all_positions[base + c]);
            }
        }

        // Build binary buffer: positions | normals | colors | indices
        let positions_bytes = vertex_count * 3 * 4; // Vec3 * f32
        let normals_bytes = vertex_count * 3 * 4; // Vec3 * f32
        let colors_bytes = vertex_count * 4 * 4; // Vec4 * f32
        let indices_bytes = index_count * 4; // u32
        let total_bytes = positions_bytes + normals_bytes + colors_bytes + indices_bytes;

        let mut buffer = Vec::with_capacity(total_bytes);

        // Positions
        for v in &all_positions {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        // Normals
        for v in &all_normals {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        // Colors
        for v in &all_colors {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        // Indices
        for idx in &all_indices {
            buffer.extend_from_slice(&idx.to_le_bytes());
        }

        let b64 = base64_encode(&buffer);
        let data_uri = format!("data:application/octet-stream;base64,{}", b64);

        // Build glTF JSON
        let gltf = serde_json::json!({
            "asset": {
                "version": "2.0",
                "generator": "Flutter BIM Viewer"
            },
            "scene": 0,
            "scenes": [{ "nodes": [0] }],
            "nodes": [{ "mesh": 0 }],
            "meshes": [{
                "primitives": [{
                    "attributes": {
                        "POSITION": 0,
                        "NORMAL": 1,
                        "COLOR_0": 2
                    },
                    "indices": 3,
                    "mode": 4
                }]
            }],
            "accessors": [
                {
                    "bufferView": 0,
                    "componentType": 5126,
                    "type": "VEC3",
                    "count": vertex_count,
                    "min": [pos_min[0], pos_min[1], pos_min[2]],
                    "max": [pos_max[0], pos_max[1], pos_max[2]]
                },
                {
                    "bufferView": 1,
                    "componentType": 5126,
                    "type": "VEC3",
                    "count": vertex_count
                },
                {
                    "bufferView": 2,
                    "componentType": 5126,
                    "type": "VEC4",
                    "count": vertex_count
                },
                {
                    "bufferView": 3,
                    "componentType": 5125,
                    "type": "SCALAR",
                    "count": index_count
                }
            ],
            "bufferViews": [
                {
                    "buffer": 0,
                    "byteOffset": 0,
                    "byteLength": positions_bytes,
                    "target": 34962
                },
                {
                    "buffer": 0,
                    "byteOffset": positions_bytes,
                    "byteLength": normals_bytes,
                    "target": 34962
                },
                {
                    "buffer": 0,
                    "byteOffset": positions_bytes + normals_bytes,
                    "byteLength": colors_bytes,
                    "target": 34962
                },
                {
                    "buffer": 0,
                    "byteOffset": positions_bytes + normals_bytes + colors_bytes,
                    "byteLength": indices_bytes,
                    "target": 34963
                }
            ],
            "buffers": [{
                "uri": data_uri,
                "byteLength": total_bytes
            }]
        });

        serde_json::to_string_pretty(&gltf).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

// ========================================================================
// 4. Enhanced Screenshot with Metadata
// ========================================================================

/// Get screenshot as PNG bytes with model info metadata.
/// Returns (png_bytes, metadata_json).
#[frb(sync)]
pub fn get_screenshot_with_info() -> Result<(Vec<u8>, String), String> {
    with_state(|s| {
        let r = s.renderer()?;
        let rgba_data = r.render_frame()?;
        let scene = r
            .scene
            .as_ref()
            .ok_or_else(|| "Scene not initialized".to_string())?;
        let width = scene.width;
        let height = scene.height;

        // Encode RGBA pixels as PNG
        let png_bytes = {
            let img = image::RgbaImage::from_raw(width, height, rgba_data)
                .ok_or_else(|| "Failed to create image from pixel data".to_string())?;
            let mut png_buf: Vec<u8> = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(&mut png_buf);
            image::ImageEncoder::write_image(
                encoder,
                img.as_raw(),
                width,
                height,
                image::ColorType::Rgba8,
            )
            .map_err(|e| format!("PNG encoding failed: {}", e))?;
            png_buf
        };

        // Gather metadata
        let cam_pos = r.camera.position();
        let cam_target = r.camera.target();
        let model_count = s.registry.model_count();
        let element_count: usize = s
            .registry
            .models()
            .values()
            .map(|reg| reg.model.element_count)
            .sum();

        let metadata = serde_json::json!({
            "model_count": model_count,
            "element_count": element_count,
            "camera": {
                "position": [cam_pos[0], cam_pos[1], cam_pos[2]],
                "target": [cam_target[0], cam_target[1], cam_target[2]]
            },
            "dimensions": {
                "width": width,
                "height": height
            },
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        });

        let metadata_json = serde_json::to_string(&metadata)
            .map_err(|e| format!("Metadata serialization failed: {}", e))?;

        Ok((png_bytes, metadata_json))
    })
}

// ========================================================================
// 5. Share Viewpoint Data
// ========================================================================

/// Get current viewpoint as a shareable JSON string.
/// Contains camera state, selection, visibility -- can be used as a deep link payload.
#[frb(sync)]
pub fn get_viewpoint_share_data() -> Result<String, String> {
    with_state(|s| {
        let r = s.renderer.as_ref().ok_or("Renderer not initialized")?;
        let cam_pos = r.camera.position();
        let cam_target = r.camera.target();

        // Use save_viewpoint to get fov and up (private fields)
        let vp = r.camera.save_viewpoint("__share__".to_string());

        let selected_elements: Vec<i32> = s.selected_elements.iter().copied().collect();
        let hidden_elements: Vec<i32> = s.hidden_elements.iter().copied().collect();

        let color_mode_val = match s.color_mode {
            super::state::ColorMode::Normal => 0,
            super::state::ColorMode::ByType => 1,
            super::state::ColorMode::ByStorey => 2,
            super::state::ColorMode::ByMaterial => 3,
            super::state::ColorMode::ByProperty => 4,
            super::state::ColorMode::Grayscale => 5,
        };

        let data = serde_json::json!({
            "camera": {
                "position": [cam_pos[0], cam_pos[1], cam_pos[2]],
                "target": [cam_target[0], cam_target[1], cam_target[2]],
                "up": [vp.up.x, vp.up.y, vp.up.z],
                "fov": vp.fov
            },
            "selected_elements": selected_elements,
            "hidden_elements": hidden_elements,
            "render_mode": 0,
            "color_mode": color_mode_val
        });

        serde_json::to_string(&data).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

// ========================================================================
// 6. Apply Shared Viewpoint
// ========================================================================

/// Apply a viewpoint from shared JSON data.
#[frb(sync)]
pub fn apply_viewpoint_share_data(json: String) -> Result<(), String> {
    with_state(|s| {
        let data: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| format!("Invalid JSON: {}", e))?;

        // Apply camera state
        if let Some(camera) = data.get("camera") {
            let r = s.renderer()?;

            if let Some(pos) = camera.get("position").and_then(|v| v.as_array()) {
                if pos.len() == 3 {
                    let x = pos[0].as_f64().unwrap_or(0.0) as f32;
                    let y = pos[1].as_f64().unwrap_or(0.0) as f32;
                    let z = pos[2].as_f64().unwrap_or(0.0) as f32;
                    r.camera.set_position([x, y, z]);
                }
            }

            if let Some(target) = camera.get("target").and_then(|v| v.as_array()) {
                if target.len() == 3 {
                    let x = target[0].as_f64().unwrap_or(0.0) as f32;
                    let y = target[1].as_f64().unwrap_or(0.0) as f32;
                    let z = target[2].as_f64().unwrap_or(0.0) as f32;
                    r.camera.set_target([x, y, z]);
                }
            }
        }

        // Apply selected elements
        if let Some(selected) = data.get("selected_elements").and_then(|v| v.as_array()) {
            s.selected_elements.clear();
            for val in selected {
                if let Some(id) = val.as_i64() {
                    s.selected_elements.insert(id as i32);
                }
            }
            // Also update single selection to last element if any
            s.selected_element = s.selected_elements.iter().next().copied();
        }

        // Apply hidden elements
        if let Some(hidden) = data.get("hidden_elements").and_then(|v| v.as_array()) {
            s.hidden_elements.clear();
            for val in hidden {
                if let Some(id) = val.as_i64() {
                    s.hidden_elements.insert(id as i32);
                }
            }
        }

        // Apply color mode
        if let Some(cm) = data.get("color_mode").and_then(|v| v.as_u64()) {
            s.color_mode = match cm {
                0 => super::state::ColorMode::Normal,
                1 => super::state::ColorMode::ByType,
                2 => super::state::ColorMode::ByStorey,
                3 => super::state::ColorMode::ByMaterial,
                4 => super::state::ColorMode::ByProperty,
                5 => super::state::ColorMode::Grayscale,
                _ => super::state::ColorMode::Normal,
            };
        }

        Ok(())
    })
}

// ========================================================================
// 7. Model Info Export
// ========================================================================

/// Export a professional HTML report for the loaded BIM model.
///
/// Generates a standalone HTML document with:
/// - Model summary (project name, site, building, storey count, element count)
/// - Element counts by type (table)
/// - Storey summary (table with name, elevation, element count)
/// - Quantity takeoffs summary (total area, volume per type)
/// - Material summary (material names and element counts)
///
/// Uses embedded CSS styling with professional layout, headers, tables,
/// and alternating row colors.
#[frb(sync)]
pub fn export_html_report() -> Result<String, String> {
    with_state(|s| {
        // ---- Gather data ----
        let mut project_name = "Unknown Project".to_string();
        let mut site_name = "Unknown Site".to_string();
        let mut building_name = "Unknown Building".to_string();
        let mut total_elements: usize = 0;
        let mut total_storeys: usize = 0;
        let mut type_counts: HashMap<String, usize> = HashMap::new();

        // Storey info: (name, elevation, element_count)
        let mut storey_info: Vec<(String, Option<f64>, usize)> = Vec::new();

        // Material summary: material_name -> element_count
        let mut material_counts: HashMap<String, usize> = HashMap::new();

        // Quantity takeoffs by type
        let mut quantity_by_type: HashMap<String, (usize, f64, f64, f64, f64)> = HashMap::new();

        for (_model_id, reg_model) in s.registry.iter_visible() {
            let model = &reg_model.model;

            if let Some(ref p) = model.project {
                project_name = p.name.clone();
            }
            if let Some(ref st) = model.site {
                site_name = st.name.clone();
            }
            if let Some(ref b) = model.building {
                building_name = b.name.clone();
            }

            total_elements += model.element_count;
            total_storeys += model.storeys.len();

            // Build storey element counts
            let storey_ids: HashMap<i32, usize> = {
                let mut sc: HashMap<i32, usize> = HashMap::new();
                for (_elem_id, storey_id) in &model.element_to_storey {
                    *sc.entry(*storey_id).or_default() += 1;
                }
                sc
            };

            for st in &model.storeys {
                let elem_count = storey_ids.get(&st.id).copied().unwrap_or(0);
                storey_info.push((st.name.clone(), st.elevation, elem_count));
            }

            // Count by type and gather quantities
            for elem in reg_model.elements() {
                *type_counts.entry(elem.element_type.clone()).or_default() += 1;

                let entry = quantity_by_type
                    .entry(elem.element_type.clone())
                    .or_insert((0, 0.0, 0.0, 0.0, 0.0));
                entry.0 += 1;

                if let Some(psets) = model.element_property_sets.get(&elem.id) {
                    for pset in psets {
                        for (prop_name, prop_value) in &pset.properties {
                            if let Ok(val) = prop_value.trim().parse::<f64>() {
                                if super::properties::is_area_quantity(prop_name) {
                                    entry.1 += val;
                                } else if super::properties::is_volume_quantity(prop_name) {
                                    entry.2 += val;
                                } else if super::properties::is_length_quantity(prop_name) {
                                    entry.3 += val;
                                } else if super::properties::is_weight_quantity(prop_name) {
                                    entry.4 += val;
                                }
                            }
                        }
                    }
                }
            }

            // Material counts
            for (_elem_id, mat) in &model.element_materials {
                *material_counts.entry(mat.name.clone()).or_default() += 1;
            }
        }

        // ---- Build HTML ----
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| {
                let secs = d.as_secs();
                // Simple UTC date formatting
                let days = secs / 86400;
                let time_of_day = secs % 86400;
                let hours = time_of_day / 3600;
                let minutes = (time_of_day % 3600) / 60;

                // Approximate date calculation (from epoch)
                let mut y = 1970i64;
                let mut remaining_days = days as i64;
                loop {
                    let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
                    if remaining_days < days_in_year {
                        break;
                    }
                    remaining_days -= days_in_year;
                    y += 1;
                }
                let month_days = [31, if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
                let mut m = 0;
                for (i, &md) in month_days.iter().enumerate() {
                    if remaining_days < md {
                        m = i + 1;
                        break;
                    }
                    remaining_days -= md;
                }
                if m == 0 { m = 12; }
                let d = remaining_days + 1;

                format!("{:04}-{:02}-{:02} {:02}:{:02} UTC", y, m, d, hours, minutes)
            })
            .unwrap_or_else(|_| "Unknown".to_string());

        let mut html = String::with_capacity(8192);

        // DOCTYPE and head
        html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
        html.push_str("<title>BIM Model Report</title>\n");
        html.push_str("<style>\n");
        html.push_str(r#"
body { font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; color: #333; }
.container { max-width: 960px; margin: 0 auto; background: #fff; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); padding: 40px; }
h1 { color: #1a237e; border-bottom: 3px solid #1a237e; padding-bottom: 10px; margin-top: 0; }
h2 { color: #283593; margin-top: 30px; border-bottom: 1px solid #e0e0e0; padding-bottom: 8px; }
.summary-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px; margin: 20px 0; }
.summary-card { background: #e8eaf6; padding: 16px; border-radius: 6px; text-align: center; }
.summary-card .label { font-size: 0.85em; color: #5c6bc0; text-transform: uppercase; letter-spacing: 1px; }
.summary-card .value { font-size: 1.8em; font-weight: 700; color: #1a237e; margin-top: 4px; }
table { width: 100%; border-collapse: collapse; margin: 16px 0; }
th { background: #283593; color: #fff; padding: 10px 12px; text-align: left; font-weight: 600; }
td { padding: 8px 12px; border-bottom: 1px solid #e0e0e0; }
tr:nth-child(even) td { background: #f5f5f5; }
tr:hover td { background: #e8eaf6; }
.footer { margin-top: 40px; padding-top: 16px; border-top: 1px solid #e0e0e0; font-size: 0.85em; color: #9e9e9e; text-align: center; }
.num { text-align: right; font-variant-numeric: tabular-nums; }
"#);
        html.push_str("</style>\n</head>\n<body>\n<div class=\"container\">\n");

        // Title and summary
        html.push_str(&format!("<h1>{}</h1>\n", html_escape(&project_name)));
        html.push_str("<div class=\"summary-grid\">\n");
        html.push_str(&format!(
            "<div class=\"summary-card\"><div class=\"label\">Site</div><div class=\"value\">{}</div></div>\n",
            html_escape(&site_name)
        ));
        html.push_str(&format!(
            "<div class=\"summary-card\"><div class=\"label\">Building</div><div class=\"value\">{}</div></div>\n",
            html_escape(&building_name)
        ));
        html.push_str(&format!(
            "<div class=\"summary-card\"><div class=\"label\">Storeys</div><div class=\"value\">{}</div></div>\n",
            total_storeys
        ));
        html.push_str(&format!(
            "<div class=\"summary-card\"><div class=\"label\">Elements</div><div class=\"value\">{}</div></div>\n",
            total_elements
        ));
        html.push_str("</div>\n");

        // Element Counts by Type
        html.push_str("<h2>Elements by Type</h2>\n");
        html.push_str("<table><thead><tr><th>Type</th><th class=\"num\">Count</th></tr></thead><tbody>\n");
        let mut type_vec: Vec<_> = type_counts.iter().collect();
        type_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (etype, count) in &type_vec {
            html.push_str(&format!(
                "<tr><td>{}</td><td class=\"num\">{}</td></tr>\n",
                html_escape(etype),
                count
            ));
        }
        html.push_str("</tbody></table>\n");

        // Storey Summary
        if !storey_info.is_empty() {
            html.push_str("<h2>Storeys</h2>\n");
            html.push_str("<table><thead><tr><th>Name</th><th class=\"num\">Elevation</th><th class=\"num\">Elements</th></tr></thead><tbody>\n");
            storey_info.sort_by(|a, b| {
                a.1.unwrap_or(0.0)
                    .partial_cmp(&b.1.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for (name, elev, count) in &storey_info {
                let elev_str = elev
                    .map(|e| format!("{:.2}", e))
                    .unwrap_or_else(|| "-".to_string());
                html.push_str(&format!(
                    "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>\n",
                    html_escape(name),
                    elev_str,
                    count
                ));
            }
            html.push_str("</tbody></table>\n");
        }

        // Quantity Takeoffs
        let has_quantities = quantity_by_type.values().any(|v| v.1 > 0.0 || v.2 > 0.0 || v.3 > 0.0 || v.4 > 0.0);
        if has_quantities {
            html.push_str("<h2>Quantity Takeoffs by Type</h2>\n");
            html.push_str("<table><thead><tr><th>Type</th><th class=\"num\">Count</th><th class=\"num\">Total Area</th><th class=\"num\">Total Volume</th><th class=\"num\">Total Length</th><th class=\"num\">Total Weight</th></tr></thead><tbody>\n");
            let mut qty_vec: Vec<_> = quantity_by_type.iter().collect();
            qty_vec.sort_by_key(|a| a.0.clone());
            for (etype, (count, area, volume, length, weight)) in &qty_vec {
                html.push_str(&format!(
                    "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{:.2}</td><td class=\"num\">{:.4}</td><td class=\"num\">{:.2}</td><td class=\"num\">{:.2}</td></tr>\n",
                    html_escape(etype),
                    count,
                    area,
                    volume,
                    length,
                    weight,
                ));
            }
            html.push_str("</tbody></table>\n");
        }

        // Material Summary
        if !material_counts.is_empty() {
            html.push_str("<h2>Materials</h2>\n");
            html.push_str("<table><thead><tr><th>Material</th><th class=\"num\">Element Count</th></tr></thead><tbody>\n");
            let mut mat_vec: Vec<_> = material_counts.iter().collect();
            mat_vec.sort_by(|a, b| b.1.cmp(a.1));
            for (mat_name, count) in &mat_vec {
                html.push_str(&format!(
                    "<tr><td>{}</td><td class=\"num\">{}</td></tr>\n",
                    html_escape(mat_name),
                    count
                ));
            }
            html.push_str("</tbody></table>\n");
        }

        // Footer
        html.push_str(&format!(
            "<div class=\"footer\">Generated by Flutter BIM Viewer &mdash; {}</div>\n",
            html_escape(&timestamp)
        ));

        html.push_str("</div>\n</body>\n</html>");

        Ok(html)
    })
}

/// Export a detailed quantity report as CSV.
///
/// One row per element type with columns:
/// Type, Count, Total Area, Total Volume, Total Length, Total Weight
#[frb(sync)]
pub fn export_quantity_report_csv() -> Result<String, String> {
    with_state(|s| {
        let mut quantity_by_type: HashMap<String, (usize, f64, f64, f64, f64)> = HashMap::new();

        for (_model_id, reg_model) in s.registry.iter_visible() {
            let model = &reg_model.model;
            for elem in reg_model.elements() {
                let entry = quantity_by_type
                    .entry(elem.element_type.clone())
                    .or_insert((0, 0.0, 0.0, 0.0, 0.0));
                entry.0 += 1;

                if let Some(psets) = model.element_property_sets.get(&elem.id) {
                    for pset in psets {
                        for (prop_name, prop_value) in &pset.properties {
                            if let Ok(val) = prop_value.trim().parse::<f64>() {
                                if super::properties::is_area_quantity(prop_name) {
                                    entry.1 += val;
                                } else if super::properties::is_volume_quantity(prop_name) {
                                    entry.2 += val;
                                } else if super::properties::is_length_quantity(prop_name) {
                                    entry.3 += val;
                                } else if super::properties::is_weight_quantity(prop_name) {
                                    entry.4 += val;
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut csv = String::from("Type,Count,TotalArea,TotalVolume,TotalLength,TotalWeight\n");

        let mut rows: Vec<_> = quantity_by_type.into_iter().collect();
        rows.sort_by_key(|r| r.0.clone());

        for (etype, (count, area, volume, length, weight)) in &rows {
            csv.push_str(&csv_escape(etype));
            csv.push(',');
            csv.push_str(&count.to_string());
            csv.push(',');
            csv.push_str(&format!("{:.4}", area));
            csv.push(',');
            csv.push_str(&format!("{:.6}", volume));
            csv.push(',');
            csv.push_str(&format!("{:.4}", length));
            csv.push(',');
            csv.push_str(&format!("{:.4}", weight));
            csv.push('\n');
        }

        Ok(csv)
    })
}

/// Escape a string for safe inclusion in HTML.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Export comprehensive model information as JSON.
#[frb(sync)]
pub fn export_model_info_json() -> Result<String, String> {
    with_state(|s| {
        let mut models_info = Vec::new();

        for (model_id, reg_model) in s.registry.iter_visible() {
            let model = &reg_model.model;

            // Count elements by type
            let mut type_counts: HashMap<String, usize> = HashMap::new();
            for elem in reg_model.elements() {
                *type_counts.entry(elem.element_type.clone()).or_insert(0) += 1;
            }

            // Build storey info
            let storeys: Vec<serde_json::Value> = model
                .storeys
                .iter()
                .map(|st| {
                    serde_json::json!({
                        "id": st.id,
                        "name": st.name,
                        "elevation": st.elevation
                    })
                })
                .collect();

            let info = serde_json::json!({
                "model_id": model_id,
                "name": reg_model.name,
                "file_path": reg_model.file_path,
                "project_name": model.project.as_ref().map(|p| p.name.as_str()).unwrap_or("Unknown"),
                "site_name": model.site.as_ref().map(|s| s.name.as_str()).unwrap_or("Unknown"),
                "building_name": model.building.as_ref().map(|b| b.name.as_str()).unwrap_or("Unknown"),
                "storeys": storeys,
                "element_count": model.element_count,
                "element_counts_by_type": type_counts,
                "bounds": reg_model.bounds.map(|b| serde_json::json!({
                    "min": b.min,
                    "max": b.max
                }))
            });

            models_info.push(info);
        }

        let result = serde_json::json!({
            "total_models": s.registry.model_count(),
            "visible_models": models_info.len(),
            "models": models_info
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| format!("JSON serialization failed: {}", e))
    })
}
