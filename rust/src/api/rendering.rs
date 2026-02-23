use std::collections::HashMap;
use flutter_rust_bridge::frb;

use crate::bim::BoundingBox;
use crate::renderer::{ElementDrawRange, Renderer};
use super::state::{draw_ranges_from_mesh, with_state, ColorMode};

/// Convert IFC Z-up coordinates to Y-up camera coordinates.
/// Transform: [x, y, z] → [x, z, -y]
fn ifc_to_yup_vertices(verts: &mut [f32]) {
    for i in (0..verts.len()).step_by(3) {
        let y = verts[i + 1];
        let z = verts[i + 2];
        verts[i + 1] = z;
        verts[i + 2] = -y;
    }
}

/// Convert IFC Z-up point to Y-up: [x, y, z] → [x, z, -y]
fn ifc_to_yup_point(p: [f32; 3]) -> [f32; 3] {
    [p[0], p[2], -p[1]]
}

/// Convert IFC Z-up bounding box to Y-up.
/// Since -y flips, min/max may swap for the new Z axis.
fn ifc_to_yup_bounds(min: [f32; 3], max: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let new_min = [min[0], min[2], -max[1]];
    let new_max = [max[0], max[2], -min[1]];
    (new_min, new_max)
}

/// Distinct color palette for categorical coloring (10 colors)
const PALETTE: [[f32; 4]; 10] = [
    [0.40, 0.76, 0.65, 1.0], // teal
    [0.99, 0.55, 0.38, 1.0], // coral
    [0.55, 0.63, 0.80, 1.0], // slate blue
    [0.91, 0.76, 0.43, 1.0], // gold
    [0.65, 0.85, 0.33, 1.0], // lime
    [1.00, 0.85, 0.56, 1.0], // warm yellow
    [0.70, 0.50, 0.76, 1.0], // lavender
    [0.99, 0.72, 0.76, 1.0], // pink
    [0.75, 0.75, 0.75, 1.0], // gray
    [0.50, 0.82, 0.90, 1.0], // sky blue
];

/// Get a palette color by hash index
fn palette_color(index: usize) -> [f32; 4] {
    PALETTE[index % PALETTE.len()]
}

/// Test renderer initialization
pub async fn test_renderer_init() -> Result<String, String> {
    tracing::info!("Testing renderer initialization");

    let mut renderer = Renderer::new();
    renderer
        .initialize()
        .await
        .map_err(|e| format!("Renderer init failed: {}", e))?;

    Ok("Renderer initialized successfully! wgpu backend is working.".to_string())
}

/// Initialize the 3D renderer with given dimensions
pub async fn init_renderer(width: u32, height: u32) -> Result<String, String> {
    tracing::info!("Initializing renderer {}x{}", width, height);

    let mut renderer = Renderer::new();
    renderer
        .initialize()
        .await
        .map_err(|e| format!("GPU init failed: {}", e))?;

    renderer
        .init_scene(width, height)
        .map_err(|e| format!("Scene init failed: {}", e))?;

    with_state(|s| {
        s.renderer = Some(renderer);
    });

    Ok(format!("Renderer initialized at {}x{}", width, height))
}

/// Render a frame and return RGBA pixel data
#[frb(sync)]
pub fn render_frame() -> Result<Vec<u8>, String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.render_frame()
    })
}

/// Check if renderer is initialized
#[frb(sync)]
pub fn is_renderer_initialized() -> bool {
    with_state(|s| s.renderer.as_ref().map_or(false, |r| r.initialized))
}

/// Set interaction active state for FastNav adaptive quality.
/// Call with true when camera interaction starts, false when it ends.
/// During interaction, expensive post-processing (SSAO, edge rendering) is skipped.
#[frb(sync)]
pub fn set_interaction_active(active: bool) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.set_interaction_active(active);
        Ok(())
    })
}

/// Get the scene bounding box in world coordinates (Y-up).
/// Returns (min_x, min_y, min_z, max_x, max_y, max_z) or error if no geometry.
#[frb(sync)]
pub fn get_scene_bounds() -> Result<Vec<f32>, String> {
    with_state(|s| {
        let r = s.renderer.as_ref().ok_or("Renderer not initialized")?;
        let (min, max) = r.get_scene_bounds().ok_or("No geometry loaded")?;
        Ok(vec![min[0], min[1], min[2], max[0], max[1], max[2]])
    })
}

/// Get the effective height range (Y axis) where most geometry lives.
/// Returns [minY, maxY] using percentile-based outlier exclusion.
/// Better than full scene bounds for level-cut sliders.
#[frb(sync)]
pub fn get_effective_height_range() -> Result<Vec<f32>, String> {
    with_state(|s| {
        let r = s.renderer.as_ref().ok_or("Renderer not initialized")?;
        let (min_y, max_y) = r
            .get_effective_height_range()
            .ok_or("No geometry loaded")?;
        Ok(vec![min_y, max_y])
    })
}

/// Load the currently loaded BIM model into the renderer (primary model)
#[frb(sync)]
pub fn load_model_into_renderer() -> Result<String, String> {
    with_state(|s| {
        let (mut vertices, mut normals, colors, indices, mut draw_ranges, bounds) = {
            let reg_model = s.registry.get_primary_model_mut().ok_or("No model loaded")?;
            let mesh = reg_model.get_mesh();
            let ranges = draw_ranges_from_mesh(mesh, None);
            (
                mesh.vertices.clone(),
                mesh.normals.clone(),
                mesh.colors.clone(),
                mesh.indices.clone(),
                ranges,
                mesh.bounds,
            )
        };
        let element_count = draw_ranges.len();

        // Convert IFC Z-up to Y-up
        ifc_to_yup_vertices(&mut vertices);
        ifc_to_yup_vertices(&mut normals);
        for range in &mut draw_ranges {
            let (new_min, new_max) = ifc_to_yup_bounds(range.aabb_min, range.aabb_max);
            range.aabb_min = new_min;
            range.aabb_max = new_max;
        }

        s.mark_bvh_dirty();

        let r = s.renderer()?;
        // Upload real geometry (non-instanced)
        r.load_mesh(&vertices, &normals, &colors, &indices)?;
        r.set_element_draw_ranges(draw_ranges)?;
        // Clear instances so render_frame takes the non-instanced branch
        r.set_instances(vec![])?;

        if let Some(bounds) = bounds {
            let (new_min, new_max) = ifc_to_yup_bounds(bounds.min, bounds.max);
            r.fit_camera_to_bounds(new_min, new_max);
        }

        tracing::info!("Loaded model (non-instanced): {} elements", element_count);
        Ok(format!(
            "Model loaded: {} elements",
            element_count
        ))
    })
}

/// Load all visible models into the renderer
#[frb(sync)]
pub fn load_all_models_into_renderer() -> Result<String, String> {
    with_state(|s| {
        if s.registry.is_empty() {
            return Err("No models loaded".to_string());
        }

        let visible_ids: Vec<String> =
            s.registry.iter_visible().map(|(id, _)| id.clone()).collect();

        // Merge all visible model meshes
        let mut all_vertices = Vec::new();
        let mut all_normals = Vec::new();
        let mut all_colors = Vec::new();
        let mut all_indices = Vec::new();
        let mut all_draw_ranges = Vec::new();
        let mut combined_bounds: Option<BoundingBox> = None;

        for id in &visible_ids {
            if let Some(reg_model) = s.registry.get_model_mut(id) {
                let mesh = reg_model.get_mesh();
                let vertex_offset = (all_vertices.len() / 3) as u32;
                let index_offset = all_indices.len() as u32;

                // Merge vertices, normals, colors
                all_vertices.extend_from_slice(&mesh.vertices);
                all_normals.extend_from_slice(&mesh.normals);
                all_colors.extend_from_slice(&mesh.colors);

                // Merge indices with offset
                all_indices.extend(mesh.indices.iter().map(|i| i + vertex_offset));

                // Merge draw ranges with offset
                for e in &mesh.elements {
                    all_draw_ranges.push(ElementDrawRange {
                        index_start: e.triangle_start * 3 + index_offset,
                        index_count: e.triangle_count * 3,
                        aabb_min: e.bounds.min,
                        aabb_max: e.bounds.max,
                    });
                }

                if let Some(bounds) = mesh.bounds {
                    combined_bounds = Some(match combined_bounds {
                        None => bounds,
                        Some(existing) => existing.union(&bounds),
                    });
                }
            }
        }

        let element_count = all_draw_ranges.len();
        let model_count = s.registry.model_count();

        // Convert IFC Z-up coordinates to Y-up for the camera system
        ifc_to_yup_vertices(&mut all_vertices);
        ifc_to_yup_vertices(&mut all_normals);

        // Transform element draw range bounds
        for range in &mut all_draw_ranges {
            let (new_min, new_max) = ifc_to_yup_bounds(range.aabb_min, range.aabb_max);
            range.aabb_min = new_min;
            range.aabb_max = new_max;
        }

        // Transform combined bounds
        combined_bounds = combined_bounds.map(|b| {
            let (new_min, new_max) = ifc_to_yup_bounds(b.min, b.max);
            BoundingBox { min: new_min, max: new_max }
        });

        tracing::info!(
            "Uploading mesh: {} vertices, {} normals, {} colors, {} indices, {} elements",
            all_vertices.len() / 3,
            all_normals.len() / 3,
            all_colors.len() / 3,
            all_indices.len(),
            element_count
        );

        s.mark_bvh_dirty();

        let r = s.renderer()?;
        r.load_mesh(&all_vertices, &all_normals, &all_colors, &all_indices)?;
        r.set_element_draw_ranges(all_draw_ranges)?;
        r.set_instances(vec![])?;

        // Compute vertex centroid (average of all vertex positions).
        // Already in Y-up coordinates after the transform above.
        let centroid = if all_vertices.len() >= 3 {
            let vert_count = all_vertices.len() / 3;
            let mut cx = 0.0_f64;
            let mut cy = 0.0_f64;
            let mut cz = 0.0_f64;
            for i in 0..vert_count {
                cx += all_vertices[i * 3] as f64;
                cy += all_vertices[i * 3 + 1] as f64;
                cz += all_vertices[i * 3 + 2] as f64;
            }
            let n = vert_count as f64;
            Some([(cx / n) as f32, (cy / n) as f32, (cz / n) as f32])
        } else {
            None
        };

        let bounds_info = if let Some(bounds) = combined_bounds {
            r.fit_camera_to_bounds_with_centroid(bounds.min, bounds.max, centroid);
            format!("bounds=[{:.1},{:.1},{:.1}]-[{:.1},{:.1},{:.1}]",
                bounds.min[0], bounds.min[1], bounds.min[2],
                bounds.max[0], bounds.max[1], bounds.max[2])
        } else {
            "NO BOUNDS".to_string()
        };

        Ok(format!(
            "Loaded {} models: {} elements, {} verts, {} idx, {}",
            model_count,
            element_count,
            all_vertices.len() / 3,
            all_indices.len(),
            bounds_info,
        ))
    })
}

/// Reload model mesh with current visibility and highlight settings (primary model)
#[frb(sync)]
pub fn reload_model_mesh() -> Result<String, String> {
    with_state(|s| {
        let highlight_color = [0.2f32, 0.9, 0.9, 1.0];
        let selected = s.selected_element;

        let (mut vertices, mut normals, mut colors, indices, mut draw_ranges) = {
            let reg_model = s.registry.get_primary_model().ok_or("No model loaded")?;
            let mesh = reg_model.cached_mesh().ok_or("Mesh not cached")?;
            let ranges = draw_ranges_from_mesh(mesh, Some(&s.visibility));
            (
                mesh.vertices.clone(),
                mesh.normals.clone(),
                mesh.colors.clone(),
                mesh.indices.clone(),
                ranges,
            )
        };

        // Apply selection highlight by overriding colors for the selected element
        if let Some(sel_id) = selected {
            let reg_model = s.registry.get_primary_model().unwrap();
            let mesh = reg_model.cached_mesh().unwrap();
            for e in &mesh.elements {
                if e.id == sel_id {
                    // Override vertex colors for this element's vertex range
                    // triangle_start is in triangle units, each triangle = 3 indices
                    let idx_start = (e.triangle_start * 3) as usize;
                    let idx_end = idx_start + (e.triangle_count * 3) as usize;
                    for idx_pos in idx_start..idx_end.min(indices.len()) {
                        let vi = indices[idx_pos] as usize;
                        let ci = vi * 4;
                        if ci + 3 < colors.len() {
                            colors[ci] = highlight_color[0];
                            colors[ci + 1] = highlight_color[1];
                            colors[ci + 2] = highlight_color[2];
                            colors[ci + 3] = highlight_color[3];
                        }
                    }
                    break;
                }
            }
        }

        let element_count = draw_ranges.len();

        // Convert IFC Z-up to Y-up
        ifc_to_yup_vertices(&mut vertices);
        ifc_to_yup_vertices(&mut normals);
        for range in &mut draw_ranges {
            let (new_min, new_max) = ifc_to_yup_bounds(range.aabb_min, range.aabb_max);
            range.aabb_min = new_min;
            range.aabb_max = new_max;
        }

        s.mark_bvh_dirty();

        let r = s.renderer()?;
        r.load_mesh(&vertices, &normals, &colors, &indices)?;
        r.set_element_draw_ranges(draw_ranges)?;
        r.set_instances(vec![])?;

        Ok(format!(
            "Mesh reloaded: {} elements",
            element_count
        ))
    })
}

/// Reload all visible models with current visibility and highlight settings
#[frb(sync)]
pub fn reload_all_models_mesh() -> Result<String, String> {
    with_state(|s| {
        if s.registry.is_empty() {
            return Err("No models loaded".to_string());
        }

        let highlight_color = [0.2f32, 0.9, 0.9, 1.0];
        let selected = s.selected_element;
        let selected_set = s.selected_elements.clone();
        let color_mode = s.color_mode;

        // Build color-by maps if needed
        let mut type_color_map: HashMap<String, [f32; 4]> = HashMap::new();
        let mut storey_color_map: HashMap<i32, [f32; 4]> = HashMap::new();
        let mut material_color_map: HashMap<String, [f32; 4]> = HashMap::new();
        let mut property_color_map: HashMap<String, [f32; 4]> = HashMap::new();
        let mut element_storey: HashMap<i32, i32> = HashMap::new();
        let mut element_material: HashMap<i32, String> = HashMap::new();
        let mut element_property_value: HashMap<i32, String> = HashMap::new();

        if color_mode != ColorMode::Normal && color_mode != ColorMode::Grayscale {
            let mut type_index = 0usize;
            let mut storey_index = 0usize;
            let mut material_index = 0usize;
            let mut property_index = 0usize;

            for (_id, reg_model) in s.registry.iter_visible() {
                let model = &reg_model.model;

                // Build type color map
                if color_mode == ColorMode::ByType {
                    for e in reg_model.elements() {
                        if !type_color_map.contains_key(&e.element_type) {
                            type_color_map.insert(e.element_type.clone(), palette_color(type_index));
                            type_index += 1;
                        }
                    }
                }

                // Build storey color map and element→storey mapping
                if color_mode == ColorMode::ByStorey {
                    for storey in &model.storeys {
                        if !storey_color_map.contains_key(&storey.id) {
                            storey_color_map.insert(storey.id, palette_color(storey_index));
                            storey_index += 1;
                        }
                    }
                    for (elem_id, storey_id) in &model.element_to_storey {
                        element_storey.insert(*elem_id, *storey_id);
                    }
                }

                // Build material color map and element→material mapping
                if color_mode == ColorMode::ByMaterial {
                    for (elem_id, mat) in &model.element_materials {
                        if !material_color_map.contains_key(&mat.name) {
                            material_color_map.insert(mat.name.clone(), palette_color(material_index));
                            material_index += 1;
                        }
                        element_material.insert(*elem_id, mat.name.clone());
                    }
                }

                // Build property color map — maps distinct property values to palette colors
                if color_mode == ColorMode::ByProperty {
                    if let Some(ref prop_name) = s.color_property_name {
                        let prop_lower = prop_name.to_lowercase();
                        for (elem_id, psets) in &model.element_property_sets {
                            for pset in psets {
                                for (key, value) in &pset.properties {
                                    if key.to_lowercase() == prop_lower {
                                        element_property_value.insert(*elem_id, value.clone());
                                        if !property_color_map.contains_key(value) {
                                            property_color_map.insert(value.clone(), palette_color(property_index));
                                            property_index += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut all_vertices = Vec::new();
        let mut all_normals = Vec::new();
        let mut all_colors = Vec::new();
        let mut all_indices = Vec::new();
        let mut all_draw_ranges = Vec::new();

        for (_id, reg_model) in s.registry.iter_visible() {
            if let Some(mesh) = reg_model.cached_mesh() {
                let vertex_offset = (all_vertices.len() / 3) as u32;
                let index_offset = all_indices.len() as u32;

                all_vertices.extend_from_slice(&mesh.vertices);
                all_normals.extend_from_slice(&mesh.normals);
                all_colors.extend_from_slice(&mesh.colors);
                all_indices.extend(mesh.indices.iter().map(|i| i + vertex_offset));

                for e in &mesh.elements {
                    if s.visibility.contains(&e.element_type) {
                        continue; // hidden type
                    }
                    if s.hidden_elements.contains(&e.id) {
                        continue; // individually hidden element
                    }

                    let is_selected = selected == Some(e.id) || selected_set.contains(&e.id);

                    // Determine override color for this element
                    let override_color = if is_selected {
                        Some(highlight_color)
                    } else {
                        match color_mode {
                            ColorMode::Normal | ColorMode::Grayscale => None,
                            ColorMode::ByType => type_color_map.get(&e.element_type).copied(),
                            ColorMode::ByStorey => {
                                element_storey
                                    .get(&e.id)
                                    .and_then(|sid| storey_color_map.get(sid))
                                    .copied()
                                    .or(Some([0.5, 0.5, 0.5, 1.0])) // gray for unassigned
                            }
                            ColorMode::ByMaterial => {
                                element_material
                                    .get(&e.id)
                                    .and_then(|name| material_color_map.get(name))
                                    .copied()
                                    .or(Some([0.5, 0.5, 0.5, 1.0])) // gray for no material
                            }
                            ColorMode::ByProperty => {
                                element_property_value
                                    .get(&e.id)
                                    .and_then(|val| property_color_map.get(val))
                                    .copied()
                                    .or(Some([0.5, 0.5, 0.5, 1.0])) // gray for no property value
                            }
                        }
                    };

                    // Apply color override to vertices
                    if let Some(color) = override_color {
                        let idx_start = (e.triangle_start * 3) as usize;
                        let idx_end = idx_start + (e.triangle_count * 3) as usize;
                        for idx_pos in idx_start..idx_end.min(mesh.indices.len()) {
                            let vi = mesh.indices[idx_pos] as usize;
                            let ci = (vi as u32 + vertex_offset) as usize * 4;
                            if ci + 3 < all_colors.len() {
                                all_colors[ci] = color[0];
                                all_colors[ci + 1] = color[1];
                                all_colors[ci + 2] = color[2];
                                all_colors[ci + 3] = color[3];
                            }
                        }
                    }

                    all_draw_ranges.push(ElementDrawRange {
                        index_start: e.triangle_start * 3 + index_offset,
                        index_count: e.triangle_count * 3,
                        aabb_min: e.bounds.min,
                        aabb_max: e.bounds.max,
                    });
                }
            }
        }

        // Apply grayscale post-processing (but preserve highlight color for selected elements)
        if color_mode == ColorMode::Grayscale {
            for i in (0..all_colors.len()).step_by(4) {
                let r_val = all_colors[i];
                let g_val = all_colors[i + 1];
                let b_val = all_colors[i + 2];
                // Skip vertices that have the highlight color (selected elements)
                let is_highlight = (r_val - highlight_color[0]).abs() < 0.05
                    && (g_val - highlight_color[1]).abs() < 0.05
                    && (b_val - highlight_color[2]).abs() < 0.05;
                if !is_highlight {
                    let luma = r_val * 0.299 + g_val * 0.587 + b_val * 0.114;
                    all_colors[i] = luma;
                    all_colors[i + 1] = luma;
                    all_colors[i + 2] = luma;
                    // Keep alpha unchanged
                }
            }
        }

        let element_count = all_draw_ranges.len();
        let model_count = s.registry.model_count();

        // Check X-ray mode before borrowing renderer mutably
        let is_xray = s.renderer
            .as_ref()
            .and_then(|r| r.get_render_mode().ok())
            == Some(crate::renderer::RenderMode::XRay);

        // Apply X-ray alpha if in X-ray mode
        if is_xray {
            for i in (3..all_colors.len()).step_by(4) {
                // Check if this is a highlight color (selected element) — keep full alpha
                let r_val = all_colors[i - 3];
                let g_val = all_colors[i - 2];
                let b_val = all_colors[i - 1];
                let is_highlight = (r_val - 0.2).abs() < 0.05
                    && (g_val - 0.9).abs() < 0.05
                    && (b_val - 0.9).abs() < 0.05;
                if !is_highlight {
                    all_colors[i] = 0.2;
                }
            }
        }

        // Convert IFC Z-up to Y-up
        ifc_to_yup_vertices(&mut all_vertices);
        ifc_to_yup_vertices(&mut all_normals);
        for range in &mut all_draw_ranges {
            let (new_min, new_max) = ifc_to_yup_bounds(range.aabb_min, range.aabb_max);
            range.aabb_min = new_min;
            range.aabb_max = new_max;
        }

        s.mark_bvh_dirty();

        let r = s.renderer()?;
        r.load_mesh(&all_vertices, &all_normals, &all_colors, &all_indices)?;
        r.set_element_draw_ranges(all_draw_ranges)?;
        r.set_instances(vec![])?;

        Ok(format!(
            "Reloaded {} models: {} elements",
            model_count,
            element_count
        ))
    })
}
