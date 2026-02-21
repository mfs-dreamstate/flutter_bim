use flutter_rust_bridge::frb;
use std::sync::{LazyLock, Mutex};

use crate::bim::{ElementInfo, GridLine};
use super::state::with_state;

// ============================================================================
// Smart Group Types & Storage
// ============================================================================

/// A filter criterion for smart groups
#[derive(Debug, Clone)]
pub struct FilterCriterion {
    pub field: String,      // "type", "storey", "material", "name", "property:PropName"
    pub operator: String,   // "equals", "contains", "starts_with", "greater_than", "less_than"
    pub value: String,
}

/// A smart group definition
#[derive(Debug, Clone)]
pub struct SmartGroup {
    pub name: String,
    pub criteria: Vec<FilterCriterion>,
    pub match_all: bool,  // true = AND, false = OR
}

static SMART_GROUPS: LazyLock<Mutex<Vec<SmartGroup>>> = LazyLock::new(|| Mutex::new(Vec::new()));

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
/// 0 = Normal, 1 = By Type, 2 = By Storey, 3 = By Material, 4 = By Property, 5 = Grayscale
/// Call reload_all_models_mesh() after this to apply.
#[frb(sync)]
pub fn set_color_mode(mode: i32) -> Result<(), String> {
    with_state(|s| {
        s.color_mode = match mode {
            0 => super::state::ColorMode::Normal,
            1 => super::state::ColorMode::ByType,
            2 => super::state::ColorMode::ByStorey,
            3 => super::state::ColorMode::ByMaterial,
            4 => super::state::ColorMode::ByProperty,
            5 => super::state::ColorMode::Grayscale,
            _ => return Err(format!("Invalid color mode: {}", mode)),
        };
        Ok(())
    })
}

/// Get the current color mode.
/// Returns: 0 = Normal, 1 = By Type, 2 = By Storey, 3 = By Material, 4 = By Property, 5 = Grayscale
#[frb(sync)]
pub fn get_color_mode() -> i32 {
    with_state(|s| match s.color_mode {
        super::state::ColorMode::Normal => 0,
        super::state::ColorMode::ByType => 1,
        super::state::ColorMode::ByStorey => 2,
        super::state::ColorMode::ByMaterial => 3,
        super::state::ColorMode::ByProperty => 4,
        super::state::ColorMode::Grayscale => 5,
    })
}

/// Set the property name for color-by-property mode.
/// Also sets the color mode to ByProperty.
#[frb(sync)]
pub fn set_color_property(property_name: String) -> Result<(), String> {
    with_state(|s| {
        s.color_property_name = Some(property_name);
        s.color_mode = super::state::ColorMode::ByProperty;
        Ok(())
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
// Multi-Select API
// ============================================================================

/// Select an element (add to selection)
#[frb(sync)]
pub fn select_element(element_id: i32) -> Result<(), String> {
    with_state(|s| {
        s.selected_elements.insert(element_id);
        s.selected_element = Some(element_id); // keep backward compat
        Ok(())
    })
}

/// Deselect an element (remove from selection)
#[frb(sync)]
pub fn deselect_element(element_id: i32) -> Result<(), String> {
    with_state(|s| {
        s.selected_elements.remove(&element_id);
        if s.selected_element == Some(element_id) {
            s.selected_element = s.selected_elements.iter().next().copied();
        }
        Ok(())
    })
}

/// Clear all selection
#[frb(sync)]
pub fn clear_selection() -> Result<(), String> {
    with_state(|s| {
        s.selected_elements.clear();
        s.selected_element = None;
        Ok(())
    })
}

/// Get all selected element IDs
#[frb(sync)]
pub fn get_selected_elements() -> Vec<i32> {
    with_state(|s| s.selected_elements.iter().copied().collect())
}

/// Get selected element count
#[frb(sync)]
pub fn get_selection_count() -> usize {
    with_state(|s| s.selected_elements.len())
}

/// Toggle element selection
#[frb(sync)]
pub fn toggle_element_selection(element_id: i32) -> bool {
    with_state(|s| {
        if s.selected_elements.contains(&element_id) {
            s.selected_elements.remove(&element_id);
            if s.selected_element == Some(element_id) {
                s.selected_element = s.selected_elements.iter().next().copied();
            }
            false
        } else {
            s.selected_elements.insert(element_id);
            s.selected_element = Some(element_id);
            true
        }
    })
}

// ============================================================================
// Hide / Show / Isolate API
// ============================================================================

/// Hide all currently selected elements
#[frb(sync)]
pub fn hide_selected() -> Result<i32, String> {
    with_state(|s| {
        let count = s.selected_elements.len() as i32;
        s.hidden_elements.extend(s.selected_elements.iter());
        s.selected_elements.clear();
        s.selected_element = None;
        Ok(count)
    })
}

/// Show all hidden elements (clear hidden set)
#[frb(sync)]
pub fn show_all_elements() -> Result<(), String> {
    with_state(|s| {
        s.hidden_elements.clear();
        s.isolated_storey = None;
        Ok(())
    })
}

/// Isolate selected elements (hide everything except selection)
#[frb(sync)]
pub fn isolate_selected() -> Result<i32, String> {
    with_state(|s| {
        if s.selected_elements.is_empty() {
            return Err("No elements selected".to_string());
        }

        let selected = s.selected_elements.clone();
        s.hidden_elements.clear();

        // Hide all elements that are NOT in the selection
        for (_id, reg_model) in s.registry.iter_visible() {
            for elem in reg_model.elements() {
                if !selected.contains(&elem.id) {
                    s.hidden_elements.insert(elem.id);
                }
            }
        }

        Ok(selected.len() as i32)
    })
}

/// Hide a specific element by ID
#[frb(sync)]
pub fn hide_element(element_id: i32) -> Result<(), String> {
    with_state(|s| {
        s.hidden_elements.insert(element_id);
        s.selected_elements.remove(&element_id);
        if s.selected_element == Some(element_id) {
            s.selected_element = s.selected_elements.iter().next().copied();
        }
        Ok(())
    })
}

/// Show a specific hidden element
#[frb(sync)]
pub fn show_element(element_id: i32) -> Result<(), String> {
    with_state(|s| {
        s.hidden_elements.remove(&element_id);
        Ok(())
    })
}

/// Get count of hidden elements
#[frb(sync)]
pub fn get_hidden_element_count() -> usize {
    with_state(|s| s.hidden_elements.len())
}

// ============================================================================
// Selection Sets (Named Groups)
// ============================================================================

/// Save current selection as a named set
#[frb(sync)]
pub fn save_selection_set(name: String) -> Result<usize, String> {
    with_state(|s| {
        let elements: Vec<i32> = s.selected_elements.iter().copied().collect();
        let count = elements.len();
        s.selection_sets.insert(name, elements);
        Ok(count)
    })
}

/// Restore a named selection set
#[frb(sync)]
pub fn restore_selection_set(name: String) -> Result<usize, String> {
    with_state(|s| {
        let elements = s.selection_sets.get(&name).ok_or("Selection set not found")?;
        s.selected_elements = elements.iter().copied().collect();
        s.selected_element = s.selected_elements.iter().next().copied();
        Ok(s.selected_elements.len())
    })
}

/// Get all selection set names
#[frb(sync)]
pub fn get_selection_set_names() -> Vec<String> {
    with_state(|s| s.selection_sets.keys().cloned().collect())
}

/// Delete a selection set
#[frb(sync)]
pub fn delete_selection_set(name: String) -> Result<(), String> {
    with_state(|s| {
        s.selection_sets.remove(&name);
        Ok(())
    })
}

// ============================================================================
// Search & Filter API
// ============================================================================

/// Search elements by name (case-insensitive substring match)
#[frb(sync)]
pub fn search_elements(query: String) -> Vec<ElementInfo> {
    with_state(|s| {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        for (_id, reg_model) in s.registry.iter_visible() {
            for elem in reg_model.elements() {
                if elem.name.to_lowercase().contains(&query_lower)
                    || elem.element_type.to_lowercase().contains(&query_lower)
                    || elem.global_id.to_lowercase().contains(&query_lower)
                {
                    results.push(elem.clone());
                }
            }
        }
        results
    })
}

/// Filter elements by type
#[frb(sync)]
pub fn filter_elements_by_type(element_type: String) -> Vec<ElementInfo> {
    with_state(|s| {
        let mut results = Vec::new();
        for (_id, reg_model) in s.registry.iter_visible() {
            for elem in reg_model.elements() {
                if elem.element_type.eq_ignore_ascii_case(&element_type) {
                    results.push(elem.clone());
                }
            }
        }
        results
    })
}

/// Select all elements matching a search query
#[frb(sync)]
pub fn select_by_search(query: String) -> usize {
    with_state(|s| {
        let query_lower = query.to_lowercase();
        s.selected_elements.clear();
        for (_id, reg_model) in s.registry.iter_visible() {
            for elem in reg_model.elements() {
                if elem.name.to_lowercase().contains(&query_lower)
                    || elem.element_type.to_lowercase().contains(&query_lower)
                {
                    s.selected_elements.insert(elem.id);
                }
            }
        }
        s.selected_element = s.selected_elements.iter().next().copied();
        s.selected_elements.len()
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

// ============================================================================
// Smart Groups API
// ============================================================================

/// Create a smart group with filter criteria parsed from JSON.
///
/// `criteria_json` format: `[{"field": "type", "operator": "equals", "value": "IfcWall"}, ...]`
#[frb(sync)]
pub fn create_smart_group(name: String, criteria_json: String, match_all: bool) -> Result<(), String> {
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&criteria_json)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let mut criteria = Vec::new();
    for item in parsed {
        let field = item.get("field")
            .and_then(|v| v.as_str())
            .ok_or("Each criterion must have a 'field' string")?
            .to_string();
        let operator = item.get("operator")
            .and_then(|v| v.as_str())
            .ok_or("Each criterion must have an 'operator' string")?
            .to_string();
        let value = item.get("value")
            .and_then(|v| v.as_str())
            .ok_or("Each criterion must have a 'value' string")?
            .to_string();
        criteria.push(FilterCriterion { field, operator, value });
    }

    if criteria.is_empty() {
        return Err("Smart group must have at least one criterion".to_string());
    }

    let group = SmartGroup { name: name.clone(), criteria, match_all };
    let mut groups = SMART_GROUPS.lock().unwrap();
    // Replace if same name exists
    groups.retain(|g| g.name != name);
    groups.push(group);
    Ok(())
}

/// Apply a smart group: find matching elements across all visible models and select them.
/// Returns count of matched elements.
#[frb(sync)]
pub fn apply_smart_group(name: String) -> Result<usize, String> {
    let group = {
        let groups = SMART_GROUPS.lock().unwrap();
        groups.iter().find(|g| g.name == name).cloned()
            .ok_or_else(|| format!("Smart group '{}' not found", name))?
    };

    with_state(|s| {
        let matched = evaluate_smart_group_on_state(s, &group);
        for id in &matched {
            s.selected_elements.insert(*id);
        }
        s.selected_element = s.selected_elements.iter().next().copied();
        Ok(matched.len())
    })
}

/// Get all smart group names.
#[frb(sync)]
pub fn get_smart_group_names() -> Vec<String> {
    let groups = SMART_GROUPS.lock().unwrap();
    groups.iter().map(|g| g.name.clone()).collect()
}

/// Delete a smart group by name.
#[frb(sync)]
pub fn delete_smart_group(name: String) -> Result<(), String> {
    let mut groups = SMART_GROUPS.lock().unwrap();
    let before = groups.len();
    groups.retain(|g| g.name != name);
    if groups.len() == before {
        Err(format!("Smart group '{}' not found", name))
    } else {
        Ok(())
    }
}

/// Preview the element count for a smart group without selecting.
#[frb(sync)]
pub fn get_smart_group_element_count(name: String) -> Result<usize, String> {
    let group = {
        let groups = SMART_GROUPS.lock().unwrap();
        groups.iter().find(|g| g.name == name).cloned()
            .ok_or_else(|| format!("Smart group '{}' not found", name))?
    };

    with_state(|s| {
        let matched = evaluate_smart_group_on_state(s, &group);
        Ok(matched.len())
    })
}

/// Get the criteria JSON for a smart group.
#[frb(sync)]
pub fn get_smart_group_criteria_json(name: String) -> Result<String, String> {
    let groups = SMART_GROUPS.lock().unwrap();
    let group = groups.iter().find(|g| g.name == name)
        .ok_or_else(|| format!("Smart group '{}' not found", name))?;

    let criteria_values: Vec<serde_json::Value> = group.criteria.iter().map(|c| {
        serde_json::json!({
            "field": c.field,
            "operator": c.operator,
            "value": c.value,
        })
    }).collect();

    serde_json::to_string(&criteria_values)
        .map_err(|e| format!("Serialization error: {}", e))
}

// ============================================================================
// Smart Group Evaluation Helpers (internal)
// ============================================================================

/// Evaluate a smart group against state, returning matching element IDs.
fn evaluate_smart_group_on_state(
    s: &super::state::AppState,
    group: &SmartGroup,
) -> Vec<i32> {
    let mut matched = Vec::new();

    for (_model_id, reg_model) in s.registry.iter_visible() {
        let model = &reg_model.model;

        // Pre-build storey name lookup
        let storey_names: std::collections::HashMap<i32, String> = model
            .storeys
            .iter()
            .map(|st| (st.id, st.name.clone()))
            .collect();

        for elem in reg_model.elements() {
            let passes = if group.match_all {
                // AND: all criteria must match
                group.criteria.iter().all(|c| {
                    criterion_matches(c, elem, model, &storey_names)
                })
            } else {
                // OR: any criterion must match
                group.criteria.iter().any(|c| {
                    criterion_matches(c, elem, model, &storey_names)
                })
            };

            if passes {
                matched.push(elem.id);
            }
        }
    }

    matched
}

/// Check if a single criterion matches a given element.
fn criterion_matches(
    criterion: &FilterCriterion,
    elem: &ElementInfo,
    model: &crate::bim::model::BimModel,
    storey_names: &std::collections::HashMap<i32, String>,
) -> bool {
    let field_value = resolve_field_value(&criterion.field, elem, model, storey_names);
    match field_value {
        Some(val) => apply_operator(&criterion.operator, &val, &criterion.value),
        None => false,
    }
}

/// Resolve the value of a field for a given element.
fn resolve_field_value(
    field: &str,
    elem: &ElementInfo,
    model: &crate::bim::model::BimModel,
    storey_names: &std::collections::HashMap<i32, String>,
) -> Option<String> {
    match field {
        "type" => Some(elem.element_type.clone()),
        "name" => Some(elem.name.clone()),
        "storey" => {
            let storey_id = model.element_to_storey.get(&elem.id)?;
            storey_names.get(storey_id).cloned()
        }
        "material" => {
            let mat = model.element_materials.get(&elem.id)?;
            Some(mat.name.clone())
        }
        _ if field.starts_with("property:") => {
            let prop_name = &field["property:".len()..];
            let psets = model.element_property_sets.get(&elem.id)?;
            for pset in psets {
                for (name, value) in &pset.properties {
                    if name.eq_ignore_ascii_case(prop_name) {
                        return Some(value.clone());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Apply an operator to compare a field value against a criterion value.
fn apply_operator(operator: &str, field_value: &str, criterion_value: &str) -> bool {
    match operator {
        "equals" => field_value.eq_ignore_ascii_case(criterion_value),
        "contains" => field_value.to_lowercase().contains(&criterion_value.to_lowercase()),
        "starts_with" => field_value.to_lowercase().starts_with(&criterion_value.to_lowercase()),
        "greater_than" => {
            match (field_value.parse::<f64>(), criterion_value.parse::<f64>()) {
                (Ok(a), Ok(b)) => a > b,
                _ => field_value > criterion_value,
            }
        }
        "less_than" => {
            match (field_value.parse::<f64>(), criterion_value.parse::<f64>()) {
                (Ok(a), Ok(b)) => a < b,
                _ => field_value < criterion_value,
            }
        }
        _ => false,
    }
}

// ============================================================================
// Search Results Highlight API
// ============================================================================

/// Search across all visible models and select matches.
/// Searches name, type, and global_id with case-insensitive substring matching.
/// Marks BVH as dirty so mesh can be reloaded with highlights.
/// Returns the count of matched elements.
#[frb(sync)]
pub fn search_and_select(query: String) -> Result<usize, String> {
    with_state(|s| {
        if s.registry.is_empty() {
            return Err("No models loaded".to_string());
        }

        let query_lower = query.to_lowercase();
        let mut count = 0usize;

        for (_id, reg_model) in s.registry.iter_visible() {
            for elem in reg_model.elements() {
                if elem.name.to_lowercase().contains(&query_lower)
                    || elem.element_type.to_lowercase().contains(&query_lower)
                    || elem.global_id.to_lowercase().contains(&query_lower)
                {
                    s.selected_elements.insert(elem.id);
                    count += 1;
                }
            }
        }

        s.selected_element = s.selected_elements.iter().next().copied();
        s.mark_bvh_dirty();
        Ok(count)
    })
}

/// Search for elements where a specific property matches (case-insensitive substring).
/// Returns matching ElementInfo objects.
#[frb(sync)]
pub fn search_elements_by_property(property_name: String, property_value: String) -> Vec<ElementInfo> {
    with_state(|s| {
        let value_lower = property_value.to_lowercase();
        let mut results = Vec::new();

        for (_model_id, reg_model) in s.registry.iter_visible() {
            let model = &reg_model.model;
            for elem in reg_model.elements() {
                if let Some(psets) = model.element_property_sets.get(&elem.id) {
                    let found = psets.iter().any(|pset| {
                        pset.properties.iter().any(|(name, val)| {
                            name.eq_ignore_ascii_case(&property_name)
                                && val.to_lowercase().contains(&value_lower)
                        })
                    });
                    if found {
                        results.push(elem.clone());
                    }
                }
            }
        }

        results
    })
}

/// Select all elements matching a property value (case-insensitive substring).
/// Returns the count of selected elements.
#[frb(sync)]
pub fn select_by_property(property_name: String, property_value: String) -> usize {
    with_state(|s| {
        let value_lower = property_value.to_lowercase();
        let mut count = 0usize;

        for (_model_id, reg_model) in s.registry.iter_visible() {
            let model = &reg_model.model;
            for elem in reg_model.elements() {
                if let Some(psets) = model.element_property_sets.get(&elem.id) {
                    let found = psets.iter().any(|pset| {
                        pset.properties.iter().any(|(name, val)| {
                            name.eq_ignore_ascii_case(&property_name)
                                && val.to_lowercase().contains(&value_lower)
                        })
                    });
                    if found {
                        s.selected_elements.insert(elem.id);
                        count += 1;
                    }
                }
            }
        }

        s.selected_element = s.selected_elements.iter().next().copied();
        count
    })
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_operator_equals() {
        assert!(apply_operator("equals", "IfcWall", "ifcwall"));
        assert!(apply_operator("equals", "IfcWall", "IfcWall"));
        assert!(!apply_operator("equals", "IfcWall", "IfcSlab"));
    }

    #[test]
    fn test_apply_operator_contains() {
        assert!(apply_operator("contains", "Basic Wall", "wall"));
        assert!(apply_operator("contains", "Basic Wall", "BASIC"));
        assert!(!apply_operator("contains", "Basic Wall", "slab"));
    }

    #[test]
    fn test_apply_operator_starts_with() {
        assert!(apply_operator("starts_with", "IfcWall", "ifc"));
        assert!(apply_operator("starts_with", "IfcWall", "Ifc"));
        assert!(!apply_operator("starts_with", "IfcWall", "Wall"));
    }

    #[test]
    fn test_apply_operator_greater_than() {
        assert!(apply_operator("greater_than", "10.5", "5.0"));
        assert!(!apply_operator("greater_than", "3.0", "5.0"));
    }

    #[test]
    fn test_apply_operator_less_than() {
        assert!(apply_operator("less_than", "3.0", "5.0"));
        assert!(!apply_operator("less_than", "10.5", "5.0"));
    }

    #[test]
    fn test_apply_operator_unknown() {
        assert!(!apply_operator("regex", "value", "pattern"));
    }

    #[test]
    fn test_smart_group_storage() {
        // Clean up from previous test runs
        {
            let mut groups = SMART_GROUPS.lock().unwrap();
            groups.clear();
        }

        // Create a smart group
        let json = r#"[{"field": "type", "operator": "equals", "value": "Wall"}]"#;
        create_smart_group("walls".to_string(), json.to_string(), true).unwrap();

        let names = get_smart_group_names();
        assert!(names.contains(&"walls".to_string()));

        // Get criteria JSON back
        let criteria = get_smart_group_criteria_json("walls".to_string()).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&criteria).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["field"], "type");

        // Delete
        delete_smart_group("walls".to_string()).unwrap();
        let names = get_smart_group_names();
        assert!(!names.contains(&"walls".to_string()));
    }

    #[test]
    fn test_create_smart_group_invalid_json() {
        let result = create_smart_group("bad".to_string(), "not json".to_string(), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_smart_group_empty_criteria() {
        let result = create_smart_group("empty".to_string(), "[]".to_string(), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_nonexistent_smart_group() {
        let result = delete_smart_group("nonexistent_group_xyz".to_string());
        assert!(result.is_err());
    }
}
