use flutter_rust_bridge::frb;

use crate::bim::{ElementInfo, GridLine};
use super::state::with_state;

/// Pick element at screen coordinates (searches all visible models)
#[frb(sync)]
pub fn pick_element(screen_x: f32, screen_y: f32) -> Result<Option<ElementInfo>, String> {
    with_state(|s| {
        let r = s.renderer.as_ref().ok_or("Renderer not initialized")?;
        let (ray_origin, ray_dir) = r.camera.screen_to_ray(screen_x, screen_y);

        // Lazy BVH rebuild — only reconstructs when dirty
        s.ensure_bvh_fresh();

        if let Some(bvh) = &s.pick_accelerator.bvh {
            let mut closest_t = f32::MAX;
            if let Some(idx) = bvh.ray_query(ray_origin, ray_dir, &mut closest_t) {
                return Ok(Some(s.pick_accelerator.elements[idx].clone()));
            }
        }

        Ok(None)
    })
}

/// Get all elements in the model (primary model)
#[frb(sync)]
pub fn get_all_elements() -> Result<Vec<ElementInfo>, String> {
    with_state(|s| {
        let reg_model = s.registry.get_primary_model().ok_or("No model loaded")?;
        Ok(reg_model.elements().to_vec())
    })
}

/// Get all elements from all visible models
#[frb(sync)]
pub fn get_all_elements_from_all_models() -> Result<Vec<ElementInfo>, String> {
    with_state(|s| {
        if s.registry.is_empty() {
            return Err("No models loaded".to_string());
        }
        let mut all_elements = Vec::new();
        for (_model_id, reg_model) in s.registry.iter_visible() {
            all_elements.extend_from_slice(reg_model.elements());
        }
        Ok(all_elements)
    })
}

/// Get element count by type (primary model)
#[frb(sync)]
pub fn get_element_counts() -> Result<std::collections::HashMap<String, usize>, String> {
    with_state(|s| {
        let reg_model = s.registry.get_primary_model().ok_or("No model loaded")?;
        let mut counts = std::collections::HashMap::new();
        for element in reg_model.elements() {
            *counts.entry(element.element_type.clone()).or_insert(0) += 1;
        }
        Ok(counts)
    })
}

/// Set visibility for an element type
#[frb(sync)]
pub fn set_element_type_visible(element_type: String, visible: bool) -> Result<(), String> {
    with_state(|s| {
        if visible {
            s.visibility.remove(&element_type);
        } else {
            s.visibility.insert(element_type);
        }
        Ok(())
    })
}

/// Check if an element type is visible
#[frb(sync)]
pub fn is_element_type_visible(element_type: String) -> bool {
    with_state(|s| !s.visibility.contains(&element_type))
}

/// Get all hidden element types
#[frb(sync)]
pub fn get_hidden_element_types() -> Vec<String> {
    with_state(|s| s.visibility.iter().cloned().collect())
}

// ============================================================================
// Storey Isolation API
// ============================================================================

/// Isolate a storey — hide all elements NOT on the given storey.
/// Call reload_all_models_mesh() after this to apply.
#[frb(sync)]
pub fn isolate_storey(storey_name: String) -> Result<i32, String> {
    with_state(|s| {
        s.hidden_elements.clear();
        let mut visible_count = 0i32;

        for (_id, reg_model) in s.registry.iter_visible() {
            let model = &reg_model.model;

            // Build set of element IDs in the target storey
            let storey_names: std::collections::HashMap<i32, String> = model
                .storeys
                .iter()
                .map(|st| (st.id, st.name.clone()))
                .collect();

            let mut storey_element_ids: std::collections::HashSet<i32> =
                std::collections::HashSet::new();
            for (elem_id, storey_id) in &model.element_to_storey {
                let sname = storey_names
                    .get(storey_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Storey #{}", storey_id));
                if sname == storey_name {
                    storey_element_ids.insert(*elem_id);
                }
            }

            // Hide elements that are NOT in the target storey
            for elem in reg_model.elements() {
                if storey_element_ids.contains(&elem.id) {
                    visible_count += 1;
                } else {
                    s.hidden_elements.insert(elem.id);
                }
            }
        }

        s.isolated_storey = Some(storey_name);
        Ok(visible_count)
    })
}

/// Show all storeys (clear storey isolation).
/// Call reload_all_models_mesh() after this to apply.
#[frb(sync)]
pub fn show_all_storeys() -> Result<(), String> {
    with_state(|s| {
        s.hidden_elements.clear();
        s.isolated_storey = None;
        Ok(())
    })
}

/// Get the currently isolated storey name, or null if all visible.
#[frb(sync)]
pub fn get_isolated_storey() -> Option<String> {
    with_state(|s| s.isolated_storey.clone())
}

// ============================================================================
// Color Mode API
// ============================================================================

/// Set the color visualization mode.
/// 0 = Normal, 1 = By Type, 2 = By Storey, 3 = By Material
/// Call reload_all_models_mesh() after this to apply.
#[frb(sync)]
pub fn set_color_mode(mode: i32) -> Result<(), String> {
    with_state(|s| {
        s.color_mode = match mode {
            0 => super::state::ColorMode::Normal,
            1 => super::state::ColorMode::ByType,
            2 => super::state::ColorMode::ByStorey,
            3 => super::state::ColorMode::ByMaterial,
            _ => return Err(format!("Invalid color mode: {}", mode)),
        };
        Ok(())
    })
}

/// Get the current color mode.
/// Returns: 0 = Normal, 1 = By Type, 2 = By Storey, 3 = By Material
#[frb(sync)]
pub fn get_color_mode() -> i32 {
    with_state(|s| match s.color_mode {
        super::state::ColorMode::Normal => 0,
        super::state::ColorMode::ByType => 1,
        super::state::ColorMode::ByStorey => 2,
        super::state::ColorMode::ByMaterial => 3,
    })
}

/// Set the selected element for highlighting
#[frb(sync)]
pub fn set_selected_element(element_id: Option<i32>) -> Result<(), String> {
    with_state(|s| {
        s.selected_element = element_id;
        Ok(())
    })
}

// ============================================================================
// Grid API
// ============================================================================

/// Get all grid lines from all visible models
#[frb(sync)]
pub fn get_grid_lines() -> Result<Vec<GridLine>, String> {
    with_state(|s| {
        let mut all_grid_lines = Vec::new();
        for (_model_id, reg_model) in s.registry.iter_visible() {
            all_grid_lines.extend(reg_model.model.grid_lines.clone());
        }
        Ok(all_grid_lines)
    })
}

/// Check if grid is visible
#[frb(sync)]
pub fn is_grid_visible() -> bool {
    with_state(|s| s.grid_visible)
}

/// Set grid visibility
#[frb(sync)]
pub fn set_grid_visible(visible: bool) -> Result<(), String> {
    with_state(|s| {
        s.grid_visible = visible;
        Ok(())
    })
}

/// Toggle grid visibility
#[frb(sync)]
pub fn toggle_grid_visible() -> bool {
    with_state(|s| {
        s.grid_visible = !s.grid_visible;
        s.grid_visible
    })
}

/// Get grid line count
#[frb(sync)]
pub fn get_grid_line_count() -> Result<usize, String> {
    with_state(|s| {
        let count: usize = s
            .registry
            .iter_visible()
            .map(|(_, reg_model)| reg_model.model.grid_lines.len())
            .sum();
        Ok(count)
    })
}

// ============================================================================
// GIS / Georeferencing API
// ============================================================================

/// Georeferencing data from IFC site
#[derive(Debug, Clone)]
pub struct GeoReference {
    pub latitude: f64,
    pub longitude: f64,
    pub rotation: f64,
    pub width: f64,
    pub depth: f64,
    pub site_name: Option<String>,
}

/// Get georeferencing data from the primary model's site
#[frb(sync)]
pub fn get_geo_reference() -> Option<GeoReference> {
    with_state(|s| {
        let reg_model = s.registry.get_primary_model()?;

        let site = reg_model.model.site.as_ref()?;
        let (lat_parts, lng_parts) = match (&site.latitude, &site.longitude) {
            (Some(lat), Some(lng)) if lat.len() >= 3 && lng.len() >= 3 => (lat, lng),
            _ => return None,
        };

        let latitude = dms_to_decimal(lat_parts);
        let longitude = dms_to_decimal(lng_parts);

        let (width, depth) = if let Some(bounds) = reg_model.bounds {
            (
                (bounds.max[0] - bounds.min[0]) as f64,
                (bounds.max[1] - bounds.min[1]) as f64,
            )
        } else {
            (30.0, 20.0)
        };

        Some(GeoReference {
            latitude,
            longitude,
            rotation: 0.0,
            width,
            depth,
            site_name: Some(site.name.clone()),
        })
    })
}

/// Convert degrees, minutes, seconds to decimal degrees
fn dms_to_decimal(dms: &[i32]) -> f64 {
    if dms.len() < 3 {
        return 0.0;
    }
    let degrees = dms[0] as f64;
    let minutes = dms[1] as f64;
    let seconds = dms[2] as f64;
    let microseconds = if dms.len() > 3 { dms[3] as f64 } else { 0.0 };

    let sign = if degrees < 0.0 { -1.0 } else { 1.0 };
    sign * (degrees.abs() + minutes / 60.0 + seconds / 3600.0 + microseconds / 3600000000.0)
}
