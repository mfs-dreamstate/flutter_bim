use flutter_rust_bridge::frb;
use std::sync::{LazyLock, Mutex};

use super::state::with_state;

// ========================================================================
// Unit Conversion System
// ========================================================================

/// Unit system for display purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitSystem {
    SI,
    Imperial,
}

/// Internal state for unit conversion.
struct UnitState {
    system: UnitSystem,
    /// Multiply IFC value by this to get meters
    length_factor: f64,
    /// Multiply IFC value by this to get square meters
    area_factor: f64,
    /// Multiply IFC value by this to get cubic meters
    volume_factor: f64,
}

static UNIT_STATE: LazyLock<Mutex<UnitState>> = LazyLock::new(|| {
    Mutex::new(UnitState {
        system: UnitSystem::SI,
        length_factor: 1.0,
        area_factor: 1.0,
        volume_factor: 1.0,
    })
});

// Conversion constants: SI (meters) -> Imperial
const METERS_TO_FEET: f64 = 3.28084;
const SQ_METERS_TO_SQ_FEET: f64 = 10.7639;
const CU_METERS_TO_CU_FEET: f64 = 35.3147;

/// Set the display unit system. 0 = SI, 1 = Imperial.
#[frb(sync)]
pub fn set_unit_system(system: i32) -> Result<(), String> {
    let mut state = UNIT_STATE.lock().map_err(|e| format!("Lock error: {}", e))?;
    state.system = match system {
        0 => UnitSystem::SI,
        1 => UnitSystem::Imperial,
        _ => return Err(format!("Invalid unit system: {}. Use 0 (SI) or 1 (Imperial).", system)),
    };
    Ok(())
}

/// Get the current display unit system. Returns 0 (SI) or 1 (Imperial).
#[frb(sync)]
pub fn get_unit_system() -> i32 {
    let state = UNIT_STATE.lock().unwrap_or_else(|e| e.into_inner());
    match state.system {
        UnitSystem::SI => 0,
        UnitSystem::Imperial => 1,
    }
}

/// Convert a length value from IFC units to the current display units.
///
/// The conversion chain is: IFC value * length_factor -> meters -> display units.
#[frb(sync)]
pub fn convert_length(value: f64) -> f64 {
    let state = UNIT_STATE.lock().unwrap_or_else(|e| e.into_inner());
    let meters = value * state.length_factor;
    match state.system {
        UnitSystem::SI => meters,
        UnitSystem::Imperial => meters * METERS_TO_FEET,
    }
}

/// Convert an area value from IFC units to the current display units.
#[frb(sync)]
pub fn convert_area(value: f64) -> f64 {
    let state = UNIT_STATE.lock().unwrap_or_else(|e| e.into_inner());
    let sq_meters = value * state.area_factor;
    match state.system {
        UnitSystem::SI => sq_meters,
        UnitSystem::Imperial => sq_meters * SQ_METERS_TO_SQ_FEET,
    }
}

/// Convert a volume value from IFC units to the current display units.
#[frb(sync)]
pub fn convert_volume(value: f64) -> f64 {
    let state = UNIT_STATE.lock().unwrap_or_else(|e| e.into_inner());
    let cu_meters = value * state.volume_factor;
    match state.system {
        UnitSystem::SI => cu_meters,
        UnitSystem::Imperial => cu_meters * CU_METERS_TO_CU_FEET,
    }
}

/// Get the label for the current length unit (e.g., "m", "mm", "ft").
#[frb(sync)]
pub fn get_length_unit_label() -> String {
    let state = UNIT_STATE.lock().unwrap_or_else(|e| e.into_inner());
    match state.system {
        UnitSystem::Imperial => "ft".to_string(),
        UnitSystem::SI => {
            // Determine SI label from the length factor
            if (state.length_factor - 0.001).abs() < 1e-9 {
                "mm".to_string()
            } else if (state.length_factor - 0.01).abs() < 1e-9 {
                "cm".to_string()
            } else if (state.length_factor - 0.3048).abs() < 1e-4 {
                "ft".to_string()
            } else if (state.length_factor - 0.0254).abs() < 1e-6 {
                "in".to_string()
            } else {
                "m".to_string()
            }
        }
    }
}

/// Get the label for the current area unit (e.g., "m\u{b2}", "ft\u{b2}").
#[frb(sync)]
pub fn get_area_unit_label() -> String {
    let state = UNIT_STATE.lock().unwrap_or_else(|e| e.into_inner());
    match state.system {
        UnitSystem::Imperial => "ft\u{b2}".to_string(),
        UnitSystem::SI => {
            if (state.length_factor - 0.001).abs() < 1e-9 {
                "mm\u{b2}".to_string()
            } else if (state.length_factor - 0.01).abs() < 1e-9 {
                "cm\u{b2}".to_string()
            } else {
                "m\u{b2}".to_string()
            }
        }
    }
}

/// Get the label for the current volume unit (e.g., "m\u{b3}", "ft\u{b3}").
#[frb(sync)]
pub fn get_volume_unit_label() -> String {
    let state = UNIT_STATE.lock().unwrap_or_else(|e| e.into_inner());
    match state.system {
        UnitSystem::Imperial => "ft\u{b3}".to_string(),
        UnitSystem::SI => {
            if (state.length_factor - 0.001).abs() < 1e-9 {
                "mm\u{b3}".to_string()
            } else if (state.length_factor - 0.01).abs() < 1e-9 {
                "cm\u{b3}".to_string()
            } else {
                "m\u{b3}".to_string()
            }
        }
    }
}

/// Detect IFC units from the loaded model's entity map and set the conversion factors.
///
/// Reads IFCUNITASSIGNMENT entities to determine the length, area, and volume
/// conversion factors. Returns a human-readable description of the detected units
/// (e.g., "Length: mm, Area: mm\u{b2}, Volume: mm\u{b3}").
#[frb(sync)]
pub fn detect_ifc_units() -> Result<String, String> {
    with_state(|s| {
        // Try to find a model with an entity_map available
        for (_id, reg_model) in s.registry.iter_visible() {
            if let Some(ref entity_map) = reg_model.model.entity_map {
                let factors = crate::bim::ifc_parser::detect_ifc_units(entity_map);

                let mut unit_state = UNIT_STATE
                    .lock()
                    .map_err(|e| format!("Lock error: {}", e))?;
                unit_state.length_factor = factors.length_factor;
                unit_state.area_factor = factors.area_factor;
                unit_state.volume_factor = factors.volume_factor;

                return Ok(factors.description);
            }
        }

        // If no entity map is available, try to infer from coordinate ranges
        let mut max_coord: f64 = 0.0;
        for (_id, reg_model) in s.registry.iter_visible() {
            for elem in reg_model.elements() {
                let coords = [
                    elem.bounds.min[0].abs() as f64,
                    elem.bounds.min[1].abs() as f64,
                    elem.bounds.min[2].abs() as f64,
                    elem.bounds.max[0].abs() as f64,
                    elem.bounds.max[1].abs() as f64,
                    elem.bounds.max[2].abs() as f64,
                ];
                for c in coords {
                    if c > max_coord {
                        max_coord = c;
                    }
                }
            }
        }

        // Heuristic: if coordinates exceed 100, likely millimetres
        let mut unit_state = UNIT_STATE
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        if max_coord > 1000.0 {
            unit_state.length_factor = 0.001;
            unit_state.area_factor = 0.000001;
            unit_state.volume_factor = 0.000000001;
            Ok("Length: mm, Area: mm\u{b2}, Volume: mm\u{b3} (inferred from coordinate range)".to_string())
        } else {
            unit_state.length_factor = 1.0;
            unit_state.area_factor = 1.0;
            unit_state.volume_factor = 1.0;
            Ok("Length: m, Area: m\u{b2}, Volume: m\u{b3} (default)".to_string())
        }
    })
}

/// A property set with name and list of key-value properties.
#[derive(Debug, Clone)]
pub struct PropertySetInfo {
    pub name: String,
    pub properties: Vec<PropertyInfo>,
}

/// A single property with name and value.
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub name: String,
    pub value: String,
}

/// Material information for an element.
#[derive(Debug, Clone)]
pub struct MaterialData {
    pub name: String,
    pub category: Option<String>,
    pub layers: Vec<MaterialLayerData>,
}

/// A single material layer.
#[derive(Debug, Clone)]
pub struct MaterialLayerData {
    pub material_name: String,
    pub thickness: Option<f64>,
}

/// Type object information for an element.
#[derive(Debug, Clone)]
pub struct TypeObjectData {
    /// Type name (e.g., "FLOOR_PLANK", "INNER_SHELL")
    pub type_name: String,
    /// IFC entity type (e.g., "IFCSLABTYPE", "IFCWALLTYPE")
    pub ifc_type: String,
    /// Property sets defined on the type
    pub property_sets: Vec<PropertySetInfo>,
}

/// A node in the spatial hierarchy tree.
#[derive(Debug, Clone)]
pub struct SpatialNode {
    pub id: i32,
    pub name: String,
    /// "Project", "Site", "Building", "Storey"
    pub node_type: String,
    pub parent_id: Option<i32>,
    pub element_ids: Vec<i32>,
}

/// Get property sets for a given element ID.
/// Returns a list of property sets, each containing key-value pairs.
#[frb(sync)]
pub fn get_element_properties(element_id: i32) -> Result<Vec<PropertySetInfo>, String> {
    with_state(|s| {
        let reg_model = s.registry.get_primary_model().ok_or("No model loaded")?;
        let psets = reg_model
            .model
            .element_property_sets
            .get(&element_id);

        match psets {
            Some(sets) => Ok(sets
                .iter()
                .map(|ps| PropertySetInfo {
                    name: ps.name.clone(),
                    properties: ps
                        .properties
                        .iter()
                        .map(|(k, v)| PropertyInfo {
                            name: k.clone(),
                            value: v.clone(),
                        })
                        .collect(),
                })
                .collect()),
            None => Ok(Vec::new()),
        }
    })
}

/// Get property sets for an element across all visible models.
#[frb(sync)]
pub fn get_element_properties_all_models(element_id: i32) -> Result<Vec<PropertySetInfo>, String> {
    with_state(|s| {
        for (_id, reg_model) in s.registry.iter_visible() {
            if let Some(sets) = reg_model.model.element_property_sets.get(&element_id) {
                return Ok(sets
                    .iter()
                    .map(|ps| PropertySetInfo {
                        name: ps.name.clone(),
                        properties: ps
                            .properties
                            .iter()
                            .map(|(k, v)| PropertyInfo {
                                name: k.clone(),
                                value: v.clone(),
                            })
                            .collect(),
                    })
                    .collect());
            }
        }
        Ok(Vec::new())
    })
}

/// Get the containing storey name for an element.
#[frb(sync)]
pub fn get_element_storey(element_id: i32) -> Result<Option<String>, String> {
    with_state(|s| {
        for (_id, reg_model) in s.registry.iter_visible() {
            if let Some(storey_id) = reg_model.model.element_to_storey.get(&element_id) {
                // Find the storey name
                for storey in &reg_model.model.storeys {
                    if storey.id == *storey_id {
                        return Ok(Some(storey.name.clone()));
                    }
                }
                return Ok(Some(format!("Storey #{}", storey_id)));
            }
        }
        Ok(None)
    })
}

/// Get the spatial hierarchy tree (Project → Site → Building → Storey → elements).
/// Returns a flat list of nodes that can be assembled into a tree on the Dart side.
#[frb(sync)]
pub fn get_spatial_tree() -> Result<Vec<SpatialNode>, String> {
    with_state(|s| {
        let mut nodes = Vec::new();

        for (_model_id, reg_model) in s.registry.iter_visible() {
            let model = &reg_model.model;

            // Project node
            if let Some(project) = &model.project {
                let element_ids = model
                    .spatial_children
                    .get(&project.id)
                    .map(|c| c.clone())
                    .unwrap_or_default();
                nodes.push(SpatialNode {
                    id: project.id,
                    name: project.name.clone(),
                    node_type: "Project".to_string(),
                    parent_id: None,
                    element_ids,
                });
            }

            // Site node
            if let Some(site) = &model.site {
                let parent_id = model.project.as_ref().map(|p| p.id);
                let element_ids = model
                    .spatial_children
                    .get(&site.id)
                    .map(|c| c.clone())
                    .unwrap_or_default();
                nodes.push(SpatialNode {
                    id: site.id,
                    name: site.name.clone(),
                    node_type: "Site".to_string(),
                    parent_id,
                    element_ids,
                });
            }

            // Building node
            if let Some(building) = &model.building {
                let parent_id = model.site.as_ref().map(|s| s.id);
                let element_ids = model
                    .spatial_children
                    .get(&building.id)
                    .map(|c| c.clone())
                    .unwrap_or_default();
                nodes.push(SpatialNode {
                    id: building.id,
                    name: building.name.clone(),
                    node_type: "Building".to_string(),
                    parent_id,
                    element_ids,
                });
            }

            // Storey nodes
            let building_id = model.building.as_ref().map(|b| b.id);
            for storey in &model.storeys {
                // Elements contained in this storey
                let element_ids: Vec<i32> = model
                    .element_to_storey
                    .iter()
                    .filter(|(_, sid)| **sid == storey.id)
                    .map(|(eid, _)| *eid)
                    .collect();

                nodes.push(SpatialNode {
                    id: storey.id,
                    name: storey.name.clone(),
                    node_type: "Storey".to_string(),
                    parent_id: building_id,
                    element_ids,
                });
            }
        }

        Ok(nodes)
    })
}

/// Get material information for an element across all visible models.
#[frb(sync)]
pub fn get_element_material(element_id: i32) -> Result<Option<MaterialData>, String> {
    with_state(|s| {
        for (_id, reg_model) in s.registry.iter_visible() {
            if let Some(mat) = reg_model.model.element_materials.get(&element_id) {
                return Ok(Some(MaterialData {
                    name: mat.name.clone(),
                    category: mat.category.clone(),
                    layers: mat
                        .layers
                        .iter()
                        .map(|l| MaterialLayerData {
                            material_name: l.material_name.clone(),
                            thickness: l.thickness,
                        })
                        .collect(),
                }));
            }
        }
        Ok(None)
    })
}

/// Get type object information for an element across all visible models.
#[frb(sync)]
pub fn get_element_type_info(element_id: i32) -> Result<Option<TypeObjectData>, String> {
    with_state(|s| {
        for (_id, reg_model) in s.registry.iter_visible() {
            if let Some(type_obj) = reg_model.model.element_type_objects.get(&element_id) {
                return Ok(Some(TypeObjectData {
                    type_name: type_obj.type_name.clone(),
                    ifc_type: type_obj.ifc_type.clone(),
                    property_sets: type_obj
                        .property_sets
                        .iter()
                        .map(|ps| PropertySetInfo {
                            name: ps.name.clone(),
                            properties: ps
                                .properties
                                .iter()
                                .map(|(k, v)| PropertyInfo {
                                    name: k.clone(),
                                    value: v.clone(),
                                })
                                .collect(),
                        })
                        .collect(),
                }));
            }
        }
        Ok(None)
    })
}

/// Quantity summary entry (aggregated by type or storey)
#[derive(Debug, Clone)]
pub struct QuantitySummary {
    pub group_name: String,
    pub element_count: i32,
    pub total_area: f64,
    pub total_volume: f64,
    pub total_length: f64,
    pub total_weight: f64,
}

/// Try to parse a string property value as f64.
fn try_parse_quantity_value(value: &str) -> Option<f64> {
    value.trim().parse::<f64>().ok()
}

/// Check if a property name is an area quantity.
pub(crate) fn is_area_quantity(name: &str) -> bool {
    matches!(
        name,
        "Area" | "NetFloorArea" | "NetSideArea" | "GrossArea" | "NetArea"
            | "CrossSectionArea" | "OuterSurfaceArea" | "GrossSideArea"
            | "GrossFloorArea" | "NetSurfaceArea" | "GrossCeilingArea"
            | "NetCeilingArea" | "TotalSurfaceArea"
    )
}

/// Check if a property name is a volume quantity.
pub(crate) fn is_volume_quantity(name: &str) -> bool {
    matches!(name, "Volume" | "NetVolume" | "GrossVolume")
}

/// Check if a property name is a length quantity.
pub(crate) fn is_length_quantity(name: &str) -> bool {
    matches!(
        name,
        "Length" | "Height" | "Width" | "Depth" | "Perimeter" | "NominalLength"
            | "NominalWidth" | "NominalHeight"
    )
}

/// Check if a property name is a weight quantity.
pub(crate) fn is_weight_quantity(name: &str) -> bool {
    matches!(name, "Weight" | "GrossWeight" | "NetWeight")
}

/// Get quantity summary grouped by element type.
#[frb(sync)]
pub fn get_quantity_summary_by_type() -> Vec<QuantitySummary> {
    with_state(|s| {
        let mut summaries: std::collections::HashMap<String, QuantitySummary> =
            std::collections::HashMap::new();

        for (_id, reg_model) in s.registry.iter_visible() {
            for elem in reg_model.elements() {
                let entry = summaries
                    .entry(elem.element_type.clone())
                    .or_insert(QuantitySummary {
                        group_name: elem.element_type.clone(),
                        element_count: 0,
                        total_area: 0.0,
                        total_volume: 0.0,
                        total_length: 0.0,
                        total_weight: 0.0,
                    });
                entry.element_count += 1;

                // Sum quantities from element property sets
                if let Some(psets) = reg_model.model.element_property_sets.get(&elem.id) {
                    for pset in psets {
                        for (prop_name, prop_value) in &pset.properties {
                            if let Some(val) = try_parse_quantity_value(prop_value) {
                                if is_area_quantity(prop_name) {
                                    entry.total_area += val;
                                } else if is_volume_quantity(prop_name) {
                                    entry.total_volume += val;
                                } else if is_length_quantity(prop_name) {
                                    entry.total_length += val;
                                } else if is_weight_quantity(prop_name) {
                                    entry.total_weight += val;
                                }
                            }
                        }
                    }
                }
            }
        }

        summaries.into_values().collect()
    })
}

/// Get quantity summary grouped by storey.
#[frb(sync)]
pub fn get_quantity_summary_by_storey() -> Vec<QuantitySummary> {
    with_state(|s| {
        let mut summaries: std::collections::HashMap<String, QuantitySummary> =
            std::collections::HashMap::new();

        for (_id, reg_model) in s.registry.iter_visible() {
            let model = &reg_model.model;
            let storey_names: std::collections::HashMap<i32, String> = model
                .storeys
                .iter()
                .map(|st| (st.id, st.name.clone()))
                .collect();

            for elem in reg_model.elements() {
                let storey_name = model
                    .element_to_storey
                    .get(&elem.id)
                    .and_then(|sid| storey_names.get(sid))
                    .cloned()
                    .unwrap_or_else(|| "Unassigned".to_string());

                let entry = summaries
                    .entry(storey_name.clone())
                    .or_insert(QuantitySummary {
                        group_name: storey_name,
                        element_count: 0,
                        total_area: 0.0,
                        total_volume: 0.0,
                        total_length: 0.0,
                        total_weight: 0.0,
                    });
                entry.element_count += 1;

                // Sum quantities from element property sets
                if let Some(psets) = reg_model.model.element_property_sets.get(&elem.id) {
                    for pset in psets {
                        for (prop_name, prop_value) in &pset.properties {
                            if let Some(val) = try_parse_quantity_value(prop_value) {
                                if is_area_quantity(prop_name) {
                                    entry.total_area += val;
                                } else if is_volume_quantity(prop_name) {
                                    entry.total_volume += val;
                                } else if is_length_quantity(prop_name) {
                                    entry.total_length += val;
                                } else if is_weight_quantity(prop_name) {
                                    entry.total_weight += val;
                                }
                            }
                        }
                    }
                }
            }
        }

        summaries.into_values().collect()
    })
}

/// Get elements grouped by storey, as a flat map of storey_name → element_ids.
/// Elements not assigned to any storey are grouped under "Unassigned".
#[frb(sync)]
pub fn get_elements_by_storey() -> Result<std::collections::HashMap<String, Vec<i32>>, String> {
    with_state(|s| {
        let mut by_storey: std::collections::HashMap<String, Vec<i32>> =
            std::collections::HashMap::new();

        for (_model_id, reg_model) in s.registry.iter_visible() {
            let model = &reg_model.model;

            // Build storey ID → name lookup
            let storey_names: std::collections::HashMap<i32, String> = model
                .storeys
                .iter()
                .map(|st| (st.id, st.name.clone()))
                .collect();

            // Assign elements to their storey
            let mut assigned: std::collections::HashSet<i32> = std::collections::HashSet::new();
            for (elem_id, storey_id) in &model.element_to_storey {
                let storey_name = storey_names
                    .get(storey_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Storey #{}", storey_id));
                by_storey.entry(storey_name).or_default().push(*elem_id);
                assigned.insert(*elem_id);
            }

            // Collect unassigned elements
            for elem in reg_model.elements() {
                if !assigned.contains(&elem.id) {
                    by_storey
                        .entry("Unassigned".to_string())
                        .or_default()
                        .push(elem.id);
                }
            }
        }

        Ok(by_storey)
    })
}

// ========================================================================
// Group Elements by Type
// ========================================================================

/// Get elements grouped by element type, as a flat map of type_name -> element_ids.
///
/// Similar to `get_elements_by_storey()` but groups by element type instead of storey.
/// Returns a map like: { "Wall": [1, 5, 9], "Slab": [2, 6], "Door": [3, 7] }
#[frb(sync)]
pub fn get_elements_grouped_by_type() -> Result<std::collections::HashMap<String, Vec<i32>>, String> {
    with_state(|s| {
        let mut by_type: std::collections::HashMap<String, Vec<i32>> =
            std::collections::HashMap::new();

        for (_model_id, reg_model) in s.registry.iter_visible() {
            for elem in reg_model.elements() {
                by_type
                    .entry(elem.element_type.clone())
                    .or_default()
                    .push(elem.id);
            }
        }

        Ok(by_type)
    })
}

// ========================================================================
// Map Conversion (Georeferencing)
// ========================================================================

/// Get the IFC map conversion (georeferencing) data as JSON, if present.
///
/// Returns `Ok(Some(json))` if IFCMAPCONVERSION data was found in the model,
/// or `Ok(None)` if no georeferencing data is present.
///
/// The JSON includes: eastings, northings, orthogonal_height, x_axis_abscissa,
/// x_axis_ordinate, scale, crs_name, crs_description, geodetic_datum,
/// vertical_datum, map_projection, map_zone.
#[frb(sync)]
pub fn get_map_conversion() -> Result<Option<String>, String> {
    with_state(|s| {
        for (_id, reg_model) in s.registry.iter_visible() {
            if let Some(ref entity_map) = reg_model.model.entity_map {
                if let Some(mc) = crate::bim::ifc_parser::extract_map_conversion(entity_map) {
                    let json = serde_json::json!({
                        "eastings": mc.eastings,
                        "northings": mc.northings,
                        "orthogonal_height": mc.orthogonal_height,
                        "x_axis_abscissa": mc.x_axis_abscissa,
                        "x_axis_ordinate": mc.x_axis_ordinate,
                        "scale": mc.scale,
                        "crs_name": mc.crs_name,
                        "crs_description": mc.crs_description,
                        "geodetic_datum": mc.geodetic_datum,
                        "vertical_datum": mc.vertical_datum,
                        "map_projection": mc.map_projection,
                        "map_zone": mc.map_zone,
                    });
                    let json_str = serde_json::to_string_pretty(&json)
                        .map_err(|e| format!("JSON serialization failed: {}", e))?;
                    return Ok(Some(json_str));
                }
            }
        }
        Ok(None)
    })
}
