use flutter_rust_bridge::frb;

use super::state::with_state;

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
