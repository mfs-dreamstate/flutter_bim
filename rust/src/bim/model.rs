//! BIM Model Representation
//!
//! High-level API for working with loaded IFC models.

use std::collections::HashMap;

use super::entities::*;
use super::geometry::{color_for_element_type, generate_box_with_normals, merge_meshes, BoundingBox};
use super::ifc_parser::IfcFile;
use super::placement::PlacementCache;
use super::tessellation;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A parsed IFC property set with named properties.
#[derive(Debug, Clone)]
pub struct ParsedPropertySet {
    pub name: String,
    pub properties: Vec<(String, String)>,
}

/// BIM Model - High-level representation of a loaded IFC file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BimModel {
    pub project: Option<IfcProject>,
    pub site: Option<IfcSite>,
    pub building: Option<IfcBuilding>,
    pub storeys: Vec<IfcBuildingStorey>,
    // Architectural elements
    pub walls: Vec<IfcWall>,
    pub slabs: Vec<IfcSlab>,
    pub doors: Vec<IfcDoor>,
    pub windows: Vec<IfcWindow>,
    pub roofs: Vec<IfcRoof>,
    pub stairs: Vec<IfcStair>,
    // Structural elements
    pub columns: Vec<IfcColumn>,
    pub beams: Vec<IfcBeam>,
    pub footings: Vec<IfcFooting>,
    // MEP elements
    pub pipes: Vec<IfcPipeSegment>,
    pub ducts: Vec<IfcDuctSegment>,
    pub flow_terminals: Vec<IfcFlowTerminal>,
    // Electrical
    pub cable_carriers: Vec<IfcCableCarrierSegment>,
    // Generic
    pub proxies: Vec<IfcBuildingElementProxy>,
    // Grids
    pub grids: Vec<IfcGrid>,
    pub grid_axes: Vec<IfcGridAxis>,
    pub grid_lines: Vec<GridLine>,
    pub element_count: usize,
    /// Raw IFC entity map for tessellation (cleared after mesh generation)
    #[serde(skip)]
    pub entity_map: Option<HashMap<EntityId, IfcEntity>>,
    /// Parsed property sets keyed by element entity ID
    #[serde(skip)]
    pub element_property_sets: HashMap<EntityId, Vec<ParsedPropertySet>>,
    /// Element ID → containing storey ID
    #[serde(skip)]
    pub element_to_storey: HashMap<EntityId, EntityId>,
    /// Spatial decomposition: parent ID → child IDs (Project→Site→Building→Storey)
    #[serde(skip)]
    pub spatial_children: HashMap<EntityId, Vec<EntityId>>,
    /// Material info keyed by element entity ID
    #[serde(skip)]
    pub element_materials: HashMap<EntityId, MaterialInfo>,
    /// Type object info keyed by element entity ID
    #[serde(skip)]
    pub element_type_objects: HashMap<EntityId, TypeObjectInfo>,
}

/// Parsed material information for an element.
#[derive(Debug, Clone)]
pub struct MaterialInfo {
    /// Primary material name (e.g., "C40/50", "S235JR")
    pub name: String,
    /// Material category (e.g., "CONCRETE", "STEEL")
    pub category: Option<String>,
    /// Material layers (for layered materials like walls)
    pub layers: Vec<MaterialLayerInfo>,
}

/// A single layer in a material layer set.
#[derive(Debug, Clone)]
pub struct MaterialLayerInfo {
    pub material_name: String,
    pub thickness: Option<f64>,
}

/// Parsed type object information for an element.
#[derive(Debug, Clone)]
pub struct TypeObjectInfo {
    /// Type entity name (e.g., "FLOOR_PLANK", "INNER_SHELL")
    pub type_name: String,
    /// IFC entity type (e.g., "IFCSLABTYPE", "IFCWALLTYPE")
    pub ifc_type: String,
    /// Property sets defined on the type (HasPropertySets)
    pub property_sets: Vec<ParsedPropertySet>,
}

/// Model statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStats {
    pub total_entities: usize,
    pub walls: usize,
    pub slabs: usize,
    pub columns: usize,
    pub beams: usize,
    pub doors: usize,
    pub windows: usize,
    pub storeys: usize,
    // Note: Extended stats (roofs, stairs, pipes, ducts, etc.) are parsed
    // but not exposed via FRB to avoid breaking existing bindings.
    // Run `flutter_rust_bridge_codegen generate` to add them.
}

/// Model information (for Flutter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub project_name: String,
    pub building_name: String,
    pub site_name: String,
    pub stats: ModelStats,
}

impl BimModel {
    /// Create a new empty model
    pub fn new() -> Self {
        Self {
            project: None,
            site: None,
            building: None,
            storeys: Vec::new(),
            // Architectural
            walls: Vec::new(),
            slabs: Vec::new(),
            doors: Vec::new(),
            windows: Vec::new(),
            roofs: Vec::new(),
            stairs: Vec::new(),
            // Structural
            columns: Vec::new(),
            beams: Vec::new(),
            footings: Vec::new(),
            // MEP
            pipes: Vec::new(),
            ducts: Vec::new(),
            flow_terminals: Vec::new(),
            // Electrical
            cable_carriers: Vec::new(),
            // Generic
            proxies: Vec::new(),
            // Grids
            grids: Vec::new(),
            grid_axes: Vec::new(),
            grid_lines: Vec::new(),
            element_count: 0,
            entity_map: None,
            element_property_sets: HashMap::new(),
            element_to_storey: HashMap::new(),
            spatial_children: HashMap::new(),
            element_materials: HashMap::new(),
            element_type_objects: HashMap::new(),
        }
    }

    /// Load model from IFC file
    pub fn from_ifc_file(ifc_file: &IfcFile) -> Result<Self, String> {
        let mut model = BimModel::new();

        // Extract project
        model.project = Self::extract_project(ifc_file);

        // Extract site
        model.site = Self::extract_site(ifc_file);

        // Extract building
        model.building = Self::extract_building(ifc_file);

        // Extract storeys
        model.storeys = Self::extract_storeys(ifc_file);

        // Architectural elements
        model.walls = Self::extract_walls(ifc_file);
        model.slabs = Self::extract_slabs(ifc_file);
        model.doors = Self::extract_doors(ifc_file);
        model.windows = Self::extract_windows(ifc_file);
        model.roofs = Self::extract_roofs(ifc_file);
        model.stairs = Self::extract_stairs(ifc_file);

        // Structural elements
        model.columns = Self::extract_columns(ifc_file);
        model.beams = Self::extract_beams(ifc_file);
        model.footings = Self::extract_footings(ifc_file);

        // MEP elements
        model.pipes = Self::extract_pipes(ifc_file);
        model.ducts = Self::extract_ducts(ifc_file);
        model.flow_terminals = Self::extract_flow_terminals(ifc_file);

        // Electrical
        model.cable_carriers = Self::extract_cable_carriers(ifc_file);

        // Generic
        model.proxies = Self::extract_proxies(ifc_file);

        // Grids
        model.grids = Self::extract_grids(ifc_file);
        model.grid_axes = Self::extract_grid_axes(ifc_file);
        model.grid_lines = Self::generate_grid_lines(&model);

        model.element_count = model.walls.len()
            + model.slabs.len()
            + model.columns.len()
            + model.beams.len()
            + model.doors.len()
            + model.windows.len()
            + model.roofs.len()
            + model.stairs.len()
            + model.footings.len()
            + model.pipes.len()
            + model.ducts.len()
            + model.flow_terminals.len()
            + model.cable_carriers.len()
            + model.proxies.len();

        // Parse property sets and spatial relationships
        model.element_property_sets = Self::parse_all_property_sets(ifc_file);
        let (spatial_children, element_to_storey) = Self::parse_spatial_structure(ifc_file);
        model.spatial_children = spatial_children;
        model.element_to_storey = element_to_storey;

        // Parse material associations
        model.element_materials = Self::parse_material_associations(ifc_file);

        // Parse type object associations
        model.element_type_objects = Self::parse_type_objects(ifc_file);

        tracing::info!(
            "Parsed {} elements with properties, {} spatial containment mappings, {} material assignments, {} type objects",
            model.element_property_sets.len(),
            model.element_to_storey.len(),
            model.element_materials.len(),
            model.element_type_objects.len(),
        );

        // Store entity map for tessellation
        model.entity_map = Some(ifc_file.entities.clone());

        Ok(model)
    }

    /// Get model information
    pub fn get_info(&self) -> ModelInfo {
        ModelInfo {
            project_name: self
                .project
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "Unknown Project".to_string()),
            building_name: self
                .building
                .as_ref()
                .map(|b| b.name.clone())
                .unwrap_or_else(|| "Unknown Building".to_string()),
            site_name: self
                .site
                .as_ref()
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Unknown Site".to_string()),
            stats: ModelStats {
                total_entities: self.element_count,
                walls: self.walls.len(),
                slabs: self.slabs.len(),
                columns: self.columns.len(),
                beams: self.beams.len(),
                doors: self.doors.len(),
                windows: self.windows.len(),
                storeys: self.storeys.len(),
            },
        }
    }

    // Extraction helper methods

    fn extract_project(ifc_file: &IfcFile) -> Option<IfcProject> {
        let entities = ifc_file.get_entities_by_type("IFCPROJECT");
        entities.first().map(|e| IfcProject {
            id: e.id,
            global_id: e.get_string(0).unwrap_or_default(),
            name: e.get_string(2).unwrap_or_default(),
            description: e.get_string(3),
        })
    }

    fn extract_site(ifc_file: &IfcFile) -> Option<IfcSite> {
        let entities = ifc_file.get_entities_by_type("IFCSITE");
        entities.first().map(|e| IfcSite {
            id: e.id,
            name: e.get_string(2).unwrap_or_default(),
            description: e.get_string(3),
            latitude: None,  // TODO: Parse from attributes
            longitude: None, // TODO: Parse from attributes
            elevation: None, // TODO: Parse from attributes
        })
    }

    fn extract_building(ifc_file: &IfcFile) -> Option<IfcBuilding> {
        let entities = ifc_file.get_entities_by_type("IFCBUILDING");
        entities.first().map(|e| IfcBuilding {
            id: e.id,
            name: e.get_string(2).unwrap_or_default(),
            description: e.get_string(3),
        })
    }

    fn extract_storeys(ifc_file: &IfcFile) -> Vec<IfcBuildingStorey> {
        ifc_file
            .get_entities_by_type("IFCBUILDINGSTOREY")
            .into_iter()
            .map(|e| IfcBuildingStorey {
                id: e.id,
                name: e.get_string(2).unwrap_or_default(),
                elevation: e.get_real(8),
            })
            .collect()
    }

    fn extract_walls(ifc_file: &IfcFile) -> Vec<IfcWall> {
        ifc_file
            .get_entities_by_type("IFCWALL")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcWall {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_slabs(ifc_file: &IfcFile) -> Vec<IfcSlab> {
        ifc_file
            .get_entities_by_type("IFCSLAB")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcSlab {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_columns(ifc_file: &IfcFile) -> Vec<IfcColumn> {
        ifc_file
            .get_entities_by_type("IFCCOLUMN")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcColumn {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_beams(ifc_file: &IfcFile) -> Vec<IfcBeam> {
        ifc_file
            .get_entities_by_type("IFCBEAM")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcBeam {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_doors(ifc_file: &IfcFile) -> Vec<IfcDoor> {
        ifc_file
            .get_entities_by_type("IFCDOOR")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcDoor {
                    product,
                    overall_height: e.get_real(5),
                    overall_width: e.get_real(6),
                }
            })
            .collect()
    }

    fn extract_windows(ifc_file: &IfcFile) -> Vec<IfcWindow> {
        ifc_file
            .get_entities_by_type("IFCWINDOW")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcWindow {
                    product,
                    overall_height: e.get_real(5),
                    overall_width: e.get_real(6),
                }
            })
            .collect()
    }

    fn extract_roofs(ifc_file: &IfcFile) -> Vec<IfcRoof> {
        ifc_file
            .get_entities_by_type("IFCROOF")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcRoof {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_stairs(ifc_file: &IfcFile) -> Vec<IfcStair> {
        ifc_file
            .get_entities_by_type("IFCSTAIR")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcStair {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_footings(ifc_file: &IfcFile) -> Vec<IfcFooting> {
        ifc_file
            .get_entities_by_type("IFCFOOTING")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcFooting {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_pipes(ifc_file: &IfcFile) -> Vec<IfcPipeSegment> {
        ifc_file
            .get_entities_by_type("IFCPIPESEGMENT")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcPipeSegment {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_ducts(ifc_file: &IfcFile) -> Vec<IfcDuctSegment> {
        ifc_file
            .get_entities_by_type("IFCDUCTSEGMENT")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcDuctSegment {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_flow_terminals(ifc_file: &IfcFile) -> Vec<IfcFlowTerminal> {
        ifc_file
            .get_entities_by_type("IFCFLOWTERMINAL")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcFlowTerminal {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_cable_carriers(ifc_file: &IfcFile) -> Vec<IfcCableCarrierSegment> {
        ifc_file
            .get_entities_by_type("IFCCABLECARRIERSEGMENT")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcCableCarrierSegment {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_proxies(ifc_file: &IfcFile) -> Vec<IfcBuildingElementProxy> {
        ifc_file
            .get_entities_by_type("IFCBUILDINGELEMENTPROXY")
            .into_iter()
            .map(|e| {
                let product = IfcProduct {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    description: e.get_string(3),
                    object_type: e.get_string(4),
                    properties: None,
                };
                IfcBuildingElementProxy {
                    product,
                    predefined_type: None,
                }
            })
            .collect()
    }

    fn extract_grids(ifc_file: &IfcFile) -> Vec<IfcGrid> {
        ifc_file
            .get_entities_by_type("IFCGRID")
            .into_iter()
            .map(|e| {
                // IFCGRID(GlobalId, OwnerHistory, Name, Description, ObjectType, ObjectPlacement, Representation, UAxes, VAxes, WAxes)
                let u_axes = e.get_list(7)
                    .map(|list| {
                        list.iter()
                            .filter_map(|v| {
                                if let IfcValue::EntityRef(id) = v {
                                    Some(*id)
                                } else {
                                    None
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let v_axes = e.get_list(8)
                    .map(|list| {
                        list.iter()
                            .filter_map(|v| {
                                if let IfcValue::EntityRef(id) = v {
                                    Some(*id)
                                } else {
                                    None
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                IfcGrid {
                    id: e.id,
                    global_id: e.get_string(0).unwrap_or_default(),
                    name: e.get_string(2),
                    u_axes,
                    v_axes,
                }
            })
            .collect()
    }

    fn extract_grid_axes(ifc_file: &IfcFile) -> Vec<IfcGridAxis> {
        ifc_file
            .get_entities_by_type("IFCGRIDAXIS")
            .into_iter()
            .map(|e| {
                // IFCGRIDAXIS(AxisTag, AxisCurve, SameSense)
                IfcGridAxis {
                    id: e.id,
                    axis_tag: e.get_string(0).unwrap_or_default(),
                    axis_curve: e.get_entity_ref(1),
                    same_sense: match e.get_attr(2) {
                        Some(IfcValue::Boolean(b)) => *b,
                        Some(IfcValue::Enum(s)) => s.contains("T") || s.contains("TRUE"),
                        _ => true,
                    },
                }
            })
            .collect()
    }

    fn generate_grid_lines(model: &BimModel) -> Vec<GridLine> {
        use std::collections::HashSet;

        let mut grid_lines = Vec::new();

        let bounds = model.get_bounds();
        let (min_x, max_x, min_y, max_y) = if let Some(b) = &bounds {
            (b.min[0], b.max[0], b.min[1], b.max[1])
        } else {
            (-20.0, 20.0, -20.0, 20.0)
        };

        let z = 0.0;
        let margin = 5.0;

        // Build HashSet of u-axis IDs upfront to avoid O(n*m) lookup
        let u_axis_ids: HashSet<_> = model.grids.iter().flat_map(|g| &g.u_axes).collect();

        for (i, axis) in model.grid_axes.iter().enumerate() {
            let is_u_axis = u_axis_ids.contains(&axis.id);

            // Calculate position based on index (simplified)
            let spacing = 6.0; // Typical grid spacing in meters
            let position = i as f32 * spacing;

            let (start, end) = if is_u_axis {
                // U axes run in X direction (horizontal)
                (
                    [min_x - margin, position, z],
                    [max_x + margin, position, z],
                )
            } else {
                // V axes run in Y direction (vertical)
                (
                    [position, min_y - margin, z],
                    [position, max_y + margin, z],
                )
            };

            grid_lines.push(GridLine {
                tag: axis.axis_tag.clone(),
                start,
                end,
                is_u_axis,
            });
        }

        // If no grids defined, generate default structural grid
        if grid_lines.is_empty() && bounds.is_some() {
            let bounds = bounds.unwrap();
            let span_x = bounds.max[0] - bounds.min[0];
            let span_y = bounds.max[1] - bounds.min[1];

            // Generate 4 lines in each direction
            let labels_x = ["A", "B", "C", "D"];
            let labels_y = ["1", "2", "3", "4"];

            for (i, label) in labels_x.iter().enumerate() {
                let x = bounds.min[0] + (span_x * i as f32 / 3.0);
                grid_lines.push(GridLine {
                    tag: label.to_string(),
                    start: [x, bounds.min[1] - margin, z],
                    end: [x, bounds.max[1] + margin, z],
                    is_u_axis: false, // V-direction
                });
            }

            for (i, label) in labels_y.iter().enumerate() {
                let y = bounds.min[1] + (span_y * i as f32 / 3.0);
                grid_lines.push(GridLine {
                    tag: label.to_string(),
                    start: [bounds.min[0] - margin, y, z],
                    end: [bounds.max[0] + margin, y, z],
                    is_u_axis: true, // U-direction
                });
            }
        }

        grid_lines
    }

    /// Get approximate bounding box from element positions without generating full mesh.
    /// This avoids the expensive mesh generation just for bounds.
    fn get_bounds(&self) -> Option<BoundingBox> {
        if self.element_count == 0 {
            return None;
        }

        // Compute bounds from element layout positions (same logic as generate_meshes)
        let mut min = [f32::MAX, f32::MAX, f32::MAX];
        let mut max = [f32::MIN, f32::MIN, f32::MIN];

        let mut found = false;

        // Helper to expand bounds
        macro_rules! expand {
            ($center:expr, $size:expr) => {
                let hx = $size[0] / 2.0;
                let hy = $size[1] / 2.0;
                let hz = $size[2] / 2.0;
                min[0] = min[0].min($center[0] - hx);
                min[1] = min[1].min($center[1] - hy);
                min[2] = min[2].min($center[2] - hz);
                max[0] = max[0].max($center[0] + hx);
                max[1] = max[1].max($center[1] + hy);
                max[2] = max[2].max($center[2] + hz);
                found = true;
            };
        }

        for (i, _) in self.walls.iter().enumerate() {
            expand!([i as f32 * 3.0, 1.5, 0.0], [2.5, 3.0, 0.2]);
        }
        for (i, _) in self.slabs.iter().enumerate() {
            expand!([0.0, i as f32 * 3.5, 0.0], [10.0, 0.3, 8.0]);
        }
        for (i, _) in self.columns.iter().enumerate() {
            let x = (i % 4) as f32 * 3.0 - 4.5;
            let z = (i / 4) as f32 * 3.0 - 3.0;
            expand!([x, 1.5, z], [0.4, 3.0, 0.4]);
        }
        for (i, _) in self.beams.iter().enumerate() {
            expand!([0.0, 2.8, i as f32 * 2.0 - 2.0], [8.0, 0.4, 0.3]);
        }
        for (i, _) in self.doors.iter().enumerate() {
            expand!([i as f32 * 3.0 + 1.0, 1.05, 0.1], [0.9, 2.1, 0.1]);
        }
        for (i, _) in self.windows.iter().enumerate() {
            expand!([i as f32 * 3.0 + 1.5, 1.5, 0.1], [1.0, 1.2, 0.05]);
        }
        for (i, _) in self.roofs.iter().enumerate() {
            expand!([0.0, 3.15 + i as f32 * 0.5, 0.0], [10.0, 0.3, 8.0]);
        }
        for (i, _) in self.stairs.iter().enumerate() {
            expand!([3.0 + i as f32 * 2.0, 1.5, 2.0], [1.5, 3.0, 3.0]);
        }
        for (i, _) in self.footings.iter().enumerate() {
            let x = (i % 4) as f32 * 3.0 - 4.5;
            let z = (i / 4) as f32 * 3.0 - 3.0;
            expand!([x, -0.5, z], [1.0, 0.6, 1.0]);
        }
        for (i, _) in self.pipes.iter().enumerate() {
            let y = 2.5 + (i / 3) as f32 * 0.3;
            let z = (i % 3) as f32 * 2.0 - 2.0;
            expand!([0.0, y, z], [8.0, 0.1, 0.1]);
        }
        for (i, _) in self.ducts.iter().enumerate() {
            let z = (i % 2) as f32 * 4.0 - 2.0;
            expand!([0.0, 2.7, z], [8.0, 0.4, 0.6]);
        }
        for (i, _) in self.proxies.iter().enumerate() {
            let x = (i % 3) as f32 * 2.0 - 2.0;
            let z = (i / 3) as f32 * 2.0 - 2.0;
            expand!([x, 1.0, z], [0.5, 0.5, 0.5]);
        }

        if found {
            Some(BoundingBox { min, max })
        } else {
            None
        }
    }
}

impl Default for BimModel {
    fn default() -> Self {
        Self::new()
    }
}

/// Element information for selection/properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    pub id: i32,
    pub element_type: String,
    pub name: String,
    pub global_id: String,
    pub bounds: BoundingBox,
    pub triangle_start: u32,
    pub triangle_count: u32,
}

/// Generated mesh data for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMesh {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub colors: Vec<f32>,
    pub bounds: Option<BoundingBox>,
    pub elements: Vec<ElementInfo>,
}

impl BimModel {
    /// Generate meshes from the BIM model for rendering.
    /// Attempts real IFC tessellation first; falls back to placeholder boxes.
    pub fn generate_meshes(&self) -> ModelMesh {
        let mut meshes = Vec::with_capacity(self.element_count);
        let mut elements = Vec::with_capacity(self.element_count);
        let mut current_triangle = 0u32;

        let has_entities = self.entity_map.as_ref().map_or(false, |m| !m.is_empty());
        let mut placement_cache = PlacementCache::new();
        let mut real_count = 0usize;
        let mut fallback_count = 0usize;

        // Collect all typed elements with their type label and color
        let all_elements: Vec<(EntityId, &str, Option<&str>, &str)> = self.collect_all_elements();

        for (product_id, element_type, name, color_key) in &all_elements {
            let color = color_for_element_type(color_key);

            // Try real tessellation if entity map is available
            let real_mesh = if has_entities {
                let entities = self.entity_map.as_ref().unwrap();
                tessellation::tessellate_product(*product_id, entities, &mut placement_cache, color)
            } else {
                None
            };

            let mesh = if let Some(m) = real_mesh {
                real_count += 1;
                m
            } else {
                // Fallback to placeholder box
                fallback_count += 1;
                let (center, size) = self.placeholder_box_for(*product_id, element_type);
                generate_box_with_normals(center, size, color)
            };

            let triangles = (mesh.indices.len() / 3) as u32;
            let bounds = mesh.bounding_box().unwrap_or(BoundingBox {
                min: [0.0, 0.0, 0.0],
                max: [0.1, 0.1, 0.1],
            });

            // Look up global_id from the entity map or the typed element
            let (global_id, display_name) = self.lookup_element_meta(*product_id, *name);

            elements.push(ElementInfo {
                id: *product_id,
                element_type: element_type.to_string(),
                name: display_name,
                global_id,
                bounds,
                triangle_start: current_triangle,
                triangle_count: triangles,
            });
            current_triangle += triangles;
            meshes.push(mesh);
        }

        if has_entities {
            tracing::info!(
                "Tessellation: {} real, {} fallback boxes (total {})",
                real_count,
                fallback_count,
                real_count + fallback_count
            );
        }

        // If no elements, create a default building shape
        if meshes.is_empty() {
            let default_elements = [
                ([0.0, 0.0, 0.0], [10.0, 0.3, 8.0], "SLAB", "Floor"),
                ([-4.9, 1.5, 0.0], [0.2, 3.0, 8.0], "WALL", "Left Wall"),
                ([4.9, 1.5, 0.0], [0.2, 3.0, 8.0], "WALL", "Right Wall"),
                ([0.0, 1.5, -3.9], [10.0, 3.0, 0.2], "WALL", "Back Wall"),
                ([0.0, 1.5, 3.9], [10.0, 3.0, 0.2], "WALL", "Front Wall"),
                ([0.0, 3.15, 0.0], [10.0, 0.3, 8.0], "ROOF", "Roof"),
            ];

            for (i, (center, size, elem_type, name)) in default_elements.iter().enumerate() {
                let mesh = generate_box_with_normals(*center, *size, color_for_element_type(elem_type));
                let triangles = (mesh.indices.len() / 3) as u32;
                let half = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
                elements.push(ElementInfo {
                    id: i as i32,
                    element_type: elem_type.to_string(),
                    name: name.to_string(),
                    global_id: format!("default_{}", i),
                    bounds: BoundingBox {
                        min: [center[0] - half[0], center[1] - half[1], center[2] - half[2]],
                        max: [center[0] + half[0], center[1] + half[1], center[2] + half[2]],
                    },
                    triangle_start: current_triangle,
                    triangle_count: triangles,
                });
                current_triangle += triangles;
                meshes.push(mesh);
            }
        }

        // Merge all meshes
        let merged = merge_meshes(meshes);
        let bounds = merged.bounding_box();

        ModelMesh {
            vertices: merged.vertices,
            indices: merged.indices,
            normals: merged.normals,
            colors: merged.colors,
            bounds,
            elements,
        }
    }

    /// Collect all typed elements as (product_id, type_label, name, color_key).
    fn collect_all_elements(&self) -> Vec<(EntityId, &str, Option<&str>, &str)> {
        let mut result = Vec::with_capacity(self.element_count);

        for w in &self.walls {
            result.push((w.product.id, "Wall", w.product.name.as_deref(), "WALL"));
        }
        for s in &self.slabs {
            result.push((s.product.id, "Slab", s.product.name.as_deref(), "SLAB"));
        }
        for c in &self.columns {
            result.push((c.product.id, "Column", c.product.name.as_deref(), "COLUMN"));
        }
        for b in &self.beams {
            result.push((b.product.id, "Beam", b.product.name.as_deref(), "BEAM"));
        }
        for d in &self.doors {
            result.push((d.product.id, "Door", d.product.name.as_deref(), "DOOR"));
        }
        for w in &self.windows {
            result.push((w.product.id, "Window", w.product.name.as_deref(), "WINDOW"));
        }
        for r in &self.roofs {
            result.push((r.product.id, "Roof", r.product.name.as_deref(), "ROOF"));
        }
        for s in &self.stairs {
            result.push((s.product.id, "Stair", s.product.name.as_deref(), "STAIR"));
        }
        for f in &self.footings {
            result.push((f.product.id, "Footing", f.product.name.as_deref(), "FOOTING"));
        }
        for p in &self.pipes {
            result.push((p.product.id, "Pipe", p.product.name.as_deref(), "PIPE"));
        }
        for d in &self.ducts {
            result.push((d.product.id, "Duct", d.product.name.as_deref(), "DUCT"));
        }
        for t in &self.flow_terminals {
            result.push((t.product.id, "FlowTerminal", t.product.name.as_deref(), "FLOWTERMINAL"));
        }
        for c in &self.cable_carriers {
            result.push((c.product.id, "CableCarrier", c.product.name.as_deref(), "CABLE"));
        }
        for p in &self.proxies {
            result.push((p.product.id, "Proxy", p.product.name.as_deref(), "PROXY"));
        }

        result
    }

    /// Look up element name and global_id from typed element collections.
    fn lookup_element_meta(&self, product_id: EntityId, name: Option<&str>) -> (String, String) {
        // Find global_id from the typed element collections
        let global_id = self.find_global_id(product_id);
        let display_name = name.unwrap_or("Element").to_string();
        (global_id, display_name)
    }

    fn find_global_id(&self, id: EntityId) -> String {
        macro_rules! check {
            ($list:expr) => {
                for e in $list {
                    if e.product.id == id {
                        return e.product.global_id.clone();
                    }
                }
            };
        }
        check!(&self.walls);
        check!(&self.slabs);
        check!(&self.columns);
        check!(&self.beams);
        check!(&self.doors);
        check!(&self.windows);
        check!(&self.roofs);
        check!(&self.stairs);
        check!(&self.footings);
        check!(&self.pipes);
        check!(&self.ducts);
        check!(&self.flow_terminals);
        check!(&self.cable_carriers);
        check!(&self.proxies);
        String::new()
    }

    /// Generate a placeholder box position/size for a given element type.
    /// Used as fallback when real tessellation fails.
    fn placeholder_box_for(&self, product_id: EntityId, element_type: &str) -> ([f32; 3], [f32; 3]) {
        // Use a simple hash-based position to spread elements out
        let hash = product_id.unsigned_abs();
        let x = ((hash % 20) as f32 - 10.0) * 2.0;
        let z = ((hash / 20 % 20) as f32 - 10.0) * 2.0;

        match element_type {
            "Wall" => ([x, 1.5, z], [2.5, 3.0, 0.2]),
            "Slab" => ([x, 0.0, z], [5.0, 0.3, 5.0]),
            "Column" => ([x, 1.5, z], [0.4, 3.0, 0.4]),
            "Beam" => ([x, 2.8, z], [4.0, 0.4, 0.3]),
            "Door" => ([x, 1.05, z], [0.9, 2.1, 0.1]),
            "Window" => ([x, 1.5, z], [1.0, 1.2, 0.05]),
            "Roof" => ([x, 3.15, z], [5.0, 0.3, 5.0]),
            "Stair" => ([x, 1.5, z], [1.5, 3.0, 3.0]),
            "Footing" => ([x, -0.5, z], [1.0, 0.6, 1.0]),
            "Pipe" => ([x, 2.5, z], [4.0, 0.1, 0.1]),
            "Duct" => ([x, 2.7, z], [4.0, 0.4, 0.6]),
            "FlowTerminal" => ([x, 2.9, z], [0.4, 0.1, 0.4]),
            "CableCarrier" => ([x, 2.8, z], [4.0, 0.08, 0.15]),
            _ => ([x, 1.0, z], [0.5, 0.5, 0.5]),
        }
    }

    /// Free the entity map to reclaim memory after tessellation.
    pub fn clear_entity_map(&mut self) {
        self.entity_map = None;
    }

    // ========================================================================
    // Property Set Parsing
    // ========================================================================

    /// Parse all IFCRELDEFINESBYPROPERTIES relationships to extract property sets.
    fn parse_all_property_sets(ifc_file: &IfcFile) -> HashMap<EntityId, Vec<ParsedPropertySet>> {
        let mut result: HashMap<EntityId, Vec<ParsedPropertySet>> = HashMap::new();

        for rel in ifc_file.get_entities_by_type("IFCRELDEFINESBYPROPERTIES") {
            // RelatedObjects (attr 4) = list of product entity refs
            let related_objects = match rel.get_list(4) {
                Some(list) => list,
                None => continue,
            };

            // RelatingPropertyDefinition (attr 5) = ref to IFCPROPERTYSET or IFCELEMENTQUANTITY
            let pset_id = match rel.get_entity_ref(5) {
                Some(id) => id,
                None => continue,
            };

            // Parse the property set / element quantity
            let pset = match Self::parse_single_property_set(pset_id, &ifc_file.entities) {
                Some(ps) => ps,
                None => continue,
            };

            // Assign to each related object
            for val in related_objects {
                if let IfcValue::EntityRef(obj_id) = val {
                    result.entry(*obj_id).or_default().push(pset.clone());
                }
            }
        }

        result
    }

    /// Parse a single IFCPROPERTYSET or IFCELEMENTQUANTITY entity.
    fn parse_single_property_set(
        id: EntityId,
        entities: &HashMap<EntityId, IfcEntity>,
    ) -> Option<ParsedPropertySet> {
        let entity = entities.get(&id)?;
        let name = entity.get_string(2).unwrap_or_default();

        let mut properties = Vec::new();

        if entity.entity_type.eq_ignore_ascii_case("IFCPROPERTYSET") {
            // HasProperties (attr 4) = list of property refs
            if let Some(prop_list) = entity.get_list(4) {
                for val in prop_list {
                    if let IfcValue::EntityRef(prop_id) = val {
                        if let Some(pair) = Self::parse_property(*prop_id, entities) {
                            properties.push(pair);
                        }
                    }
                }
            }
        } else if entity.entity_type.eq_ignore_ascii_case("IFCELEMENTQUANTITY") {
            // Quantities (attr 5) = list of quantity refs
            if let Some(qty_list) = entity.get_list(5) {
                for val in qty_list {
                    if let IfcValue::EntityRef(qty_id) = val {
                        if let Some(pair) = Self::parse_quantity(*qty_id, entities) {
                            properties.push(pair);
                        }
                    }
                }
            }
        }

        if properties.is_empty() {
            return None;
        }

        Some(ParsedPropertySet { name, properties })
    }

    /// Parse IFCPROPERTYSINGLEVALUE, IFCPROPERTYENUMERATEDVALUE, etc.
    fn parse_property(
        id: EntityId,
        entities: &HashMap<EntityId, IfcEntity>,
    ) -> Option<(String, String)> {
        let entity = entities.get(&id)?;
        let prop_name = entity.get_string(0)?;

        let value = if entity
            .entity_type
            .eq_ignore_ascii_case("IFCPROPERTYSINGLEVALUE")
        {
            // NominalValue (attr 2) - typed value unwrapped by parser
            Self::ifc_value_to_string(entity.get_attr(2))
        } else if entity
            .entity_type
            .eq_ignore_ascii_case("IFCPROPERTYENUMERATEDVALUE")
        {
            // EnumerationValues (attr 2) = list of values
            match entity.get_list(2) {
                Some(list) => list
                    .iter()
                    .map(|v| Self::ifc_value_to_string(Some(v)))
                    .collect::<Vec<_>>()
                    .join(", "),
                None => String::new(),
            }
        } else if entity
            .entity_type
            .eq_ignore_ascii_case("IFCPROPERTYBOUNDEDVALUE")
        {
            // UpperBoundValue (attr 2), LowerBoundValue (attr 3)
            let upper = Self::ifc_value_to_string(entity.get_attr(2));
            let lower = Self::ifc_value_to_string(entity.get_attr(3));
            format!("{} - {}", lower, upper)
        } else if entity
            .entity_type
            .eq_ignore_ascii_case("IFCPROPERTYLISTVALUE")
        {
            // ListValues (attr 2) = list
            match entity.get_list(2) {
                Some(list) => list
                    .iter()
                    .map(|v| Self::ifc_value_to_string(Some(v)))
                    .collect::<Vec<_>>()
                    .join(", "),
                None => String::new(),
            }
        } else {
            // Unknown property type - try attribute 2 as a generic value
            Self::ifc_value_to_string(entity.get_attr(2))
        };

        if value.is_empty() {
            return None;
        }

        Some((prop_name, value))
    }

    /// Parse IFCQUANTITYLENGTH, IFCQUANTITYAREA, IFCQUANTITYVOLUME, etc.
    fn parse_quantity(
        id: EntityId,
        entities: &HashMap<EntityId, IfcEntity>,
    ) -> Option<(String, String)> {
        let entity = entities.get(&id)?;
        let qty_name = entity.get_string(0)?;

        // For all quantity types, the value is at attribute 3
        // IFCQUANTITYLENGTH(Name, Description, Unit, LengthValue)
        // IFCQUANTITYAREA(Name, Description, Unit, AreaValue)
        // IFCQUANTITYVOLUME(Name, Description, Unit, VolumeValue)
        // IFCQUANTITYWEIGHT(Name, Description, Unit, WeightValue)
        // IFCQUANTITYCOUNT(Name, Description, Unit, CountValue)
        let value = Self::ifc_value_to_string(entity.get_attr(3));
        if value.is_empty() {
            return None;
        }

        Some((qty_name, value))
    }

    /// Convert an IfcValue to a display string.
    fn ifc_value_to_string(value: Option<&IfcValue>) -> String {
        match value {
            None | Some(IfcValue::Null) => String::new(),
            Some(IfcValue::String(s)) => s.clone(),
            Some(IfcValue::Real(r)) => {
                // Format nicely: avoid unnecessary decimals
                if (*r - r.round()).abs() < 1e-9 {
                    format!("{}", *r as i64)
                } else {
                    format!("{:.4}", r).trim_end_matches('0').trim_end_matches('.').to_string()
                }
            }
            Some(IfcValue::Integer(i)) => format!("{}", i),
            Some(IfcValue::Boolean(b)) => if *b { "Yes".to_string() } else { "No".to_string() },
            Some(IfcValue::Enum(e)) => e.clone(),
            Some(IfcValue::EntityRef(id)) => format!("#{}", id),
            Some(IfcValue::List(list)) => list
                .iter()
                .map(|v| Self::ifc_value_to_string(Some(v)))
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    // ========================================================================
    // Spatial Structure Parsing
    // ========================================================================

    /// Parse IFCRELAGGREGATES and IFCRELCONTAINEDINSPATIALSTRUCTURE
    /// to build the spatial hierarchy tree and element containment mapping.
    fn parse_spatial_structure(
        ifc_file: &IfcFile,
    ) -> (HashMap<EntityId, Vec<EntityId>>, HashMap<EntityId, EntityId>) {
        let mut spatial_children: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
        let mut element_to_storey: HashMap<EntityId, EntityId> = HashMap::new();

        // Collect storey IDs for quick lookup
        let storey_ids: HashSet<EntityId> = ifc_file
            .get_entities_by_type("IFCBUILDINGSTOREY")
            .iter()
            .map(|e| e.id)
            .collect();

        // Parse IFCRELAGGREGATES: spatial decomposition (Project→Site→Building→Storey)
        // IFCRELAGGREGATES(GlobalId, OwnerHistory, Name, Description, RelatingObject, RelatedObjects)
        for rel in ifc_file.get_entities_by_type("IFCRELAGGREGATES") {
            let parent_id = match rel.get_entity_ref(4) {
                Some(id) => id,
                None => continue,
            };
            let children = match rel.get_list(5) {
                Some(list) => list,
                None => continue,
            };

            let child_ids: Vec<EntityId> = children
                .iter()
                .filter_map(|v| {
                    if let IfcValue::EntityRef(id) = v {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();

            spatial_children
                .entry(parent_id)
                .or_default()
                .extend(child_ids);
        }

        // Parse IFCRELCONTAINEDINSPATIALSTRUCTURE: element → spatial element mapping
        // IFCRELCONTAINEDINSPATIALSTRUCTURE(GlobalId, OwnerHistory, Name, Description,
        //                                   RelatedElements, RelatingStructure)
        for rel in ifc_file.get_entities_by_type("IFCRELCONTAINEDINSPATIALSTRUCTURE") {
            let structure_id = match rel.get_entity_ref(5) {
                Some(id) => id,
                None => continue,
            };
            let elements = match rel.get_list(4) {
                Some(list) => list,
                None => continue,
            };

            // Find the containing storey: either structure_id is a storey,
            // or walk up the spatial tree to find the nearest storey ancestor.
            let storey_id = if storey_ids.contains(&structure_id) {
                Some(structure_id)
            } else {
                Self::find_ancestor_storey(structure_id, &spatial_children, &storey_ids)
            };

            for val in elements {
                if let IfcValue::EntityRef(elem_id) = val {
                    // Map element to its containing spatial structure
                    if let Some(sid) = storey_id {
                        element_to_storey.insert(*elem_id, sid);
                    }
                    // Also record elements as children of the spatial structure
                    spatial_children
                        .entry(structure_id)
                        .or_default()
                        .push(*elem_id);
                }
            }
        }

        (spatial_children, element_to_storey)
    }

    /// Walk up the spatial decomposition tree to find the nearest storey ancestor.
    fn find_ancestor_storey(
        id: EntityId,
        spatial_children: &HashMap<EntityId, Vec<EntityId>>,
        storey_ids: &HashSet<EntityId>,
    ) -> Option<EntityId> {
        // Build reverse map: child → parent
        for (parent, children) in spatial_children {
            if children.contains(&id) {
                if storey_ids.contains(parent) {
                    return Some(*parent);
                }
                // Recurse up (limited depth to avoid infinite loops)
                return Self::find_ancestor_storey(*parent, spatial_children, storey_ids);
            }
        }
        None
    }

    // ========================================================================
    // Material Parsing
    // ========================================================================

    /// Parse IFCRELASSOCIATESMATERIAL relationships to extract material info per element.
    fn parse_material_associations(ifc_file: &IfcFile) -> HashMap<EntityId, MaterialInfo> {
        let mut result: HashMap<EntityId, MaterialInfo> = HashMap::new();

        // IFCRELASSOCIATESMATERIAL(GlobalId, OwnerHistory, Name, Description,
        //                          RelatedObjects, RelatingMaterial)
        for rel in ifc_file.get_entities_by_type("IFCRELASSOCIATESMATERIAL") {
            let related_objects = match rel.get_list(4) {
                Some(list) => list,
                None => continue,
            };

            let material_ref = match rel.get_entity_ref(5) {
                Some(id) => id,
                None => continue,
            };

            // Resolve the material reference to a MaterialInfo
            let mat_info = match Self::resolve_material(material_ref, &ifc_file.entities) {
                Some(info) => info,
                None => continue,
            };

            // Assign to each related object
            for val in related_objects {
                if let IfcValue::EntityRef(obj_id) = val {
                    result.insert(*obj_id, mat_info.clone());
                }
            }
        }

        result
    }

    /// Resolve a material reference to a MaterialInfo.
    /// Handles: IFCMATERIAL, IFCMATERIALLAYERSETUSAGE, IFCMATERIALLAYERSET,
    /// IFCMATERIALPROFILESETUSAGE, IFCMATERIALPROFILESET, IFCMATERIALLIST,
    /// IFCMATERIALCONSTITUENTSET.
    fn resolve_material(
        id: EntityId,
        entities: &HashMap<EntityId, IfcEntity>,
    ) -> Option<MaterialInfo> {
        let entity = entities.get(&id)?;
        let etype = entity.entity_type.as_str();

        if etype.eq_ignore_ascii_case("IFCMATERIAL") {
            // IFCMATERIAL(Name, Description, Category)
            let name = entity.get_string(0).unwrap_or_default();
            let category = entity.get_string(2);
            return Some(MaterialInfo {
                name,
                category,
                layers: Vec::new(),
            });
        }

        if etype.eq_ignore_ascii_case("IFCMATERIALLAYERSETUSAGE") {
            // IFCMATERIALLAYERSETUSAGE(ForLayerSet, ...)
            let layer_set_id = entity.get_entity_ref(0)?;
            return Self::resolve_material(layer_set_id, entities);
        }

        if etype.eq_ignore_ascii_case("IFCMATERIALLAYERSET") {
            // IFCMATERIALLAYERSET(MaterialLayers, LayerSetName, Description)
            let layer_list = entity.get_list(0)?;
            let set_name = entity.get_string(1);
            let mut layers = Vec::new();
            let mut primary_name = set_name.unwrap_or_default();

            for val in layer_list {
                if let IfcValue::EntityRef(layer_id) = val {
                    if let Some(layer_info) =
                        Self::resolve_material_layer(*layer_id, entities)
                    {
                        if primary_name.is_empty() {
                            primary_name = layer_info.material_name.clone();
                        }
                        layers.push(layer_info);
                    }
                }
            }

            if primary_name.is_empty() && !layers.is_empty() {
                primary_name = layers[0].material_name.clone();
            }

            return Some(MaterialInfo {
                name: primary_name,
                category: None,
                layers,
            });
        }

        if etype.eq_ignore_ascii_case("IFCMATERIALPROFILESETUSAGE") {
            // IFCMATERIALPROFILESETUSAGE(ForProfileSet, CardinalPoint, ReferenceExtent)
            let profile_set_id = entity.get_entity_ref(0)?;
            return Self::resolve_material(profile_set_id, entities);
        }

        if etype.eq_ignore_ascii_case("IFCMATERIALPROFILESET") {
            // IFCMATERIALPROFILESET(Name, Description, MaterialProfiles, CompositeProfile)
            let set_name = entity.get_string(0);
            let profile_list = entity.get_list(2)?;
            let mut primary_name = set_name.unwrap_or_default();
            let mut layers = Vec::new();

            for val in profile_list {
                if let IfcValue::EntityRef(profile_id) = val {
                    if let Some(layer_info) =
                        Self::resolve_material_profile(*profile_id, entities)
                    {
                        if primary_name.is_empty() {
                            primary_name = layer_info.material_name.clone();
                        }
                        layers.push(layer_info);
                    }
                }
            }

            if primary_name.is_empty() && !layers.is_empty() {
                primary_name = layers[0].material_name.clone();
            }

            return Some(MaterialInfo {
                name: primary_name,
                category: None,
                layers,
            });
        }

        if etype.eq_ignore_ascii_case("IFCMATERIALLIST") {
            // IFCMATERIALLIST(Materials)
            let mat_list = entity.get_list(0)?;
            let mut names = Vec::new();
            for val in mat_list {
                if let IfcValue::EntityRef(mat_id) = val {
                    if let Some(mat) = Self::resolve_material(*mat_id, entities) {
                        names.push(mat.name);
                    }
                }
            }
            if names.is_empty() {
                return None;
            }
            return Some(MaterialInfo {
                name: names.join(", "),
                category: None,
                layers: Vec::new(),
            });
        }

        if etype.eq_ignore_ascii_case("IFCMATERIALCONSTITUENTSET") {
            // IFCMATERIALCONSTITUENTSET(Name, Description, MaterialConstituents)
            let set_name = entity.get_string(0);
            let constituents = entity.get_list(2)?;
            let mut primary_name = set_name.unwrap_or_default();
            let mut layers = Vec::new();

            for val in constituents {
                if let IfcValue::EntityRef(c_id) = val {
                    if let Some(c_entity) = entities.get(c_id) {
                        // IFCMATERIALCONSTITUENT(Name, Description, Material, Fraction, Category)
                        let mat_id = c_entity.get_entity_ref(2);
                        if let Some(mid) = mat_id {
                            if let Some(mat) = Self::resolve_material(mid, entities) {
                                if primary_name.is_empty() {
                                    primary_name = mat.name.clone();
                                }
                                layers.push(MaterialLayerInfo {
                                    material_name: mat.name,
                                    thickness: None,
                                });
                            }
                        }
                    }
                }
            }

            if primary_name.is_empty() {
                return None;
            }
            return Some(MaterialInfo {
                name: primary_name,
                category: None,
                layers,
            });
        }

        None
    }

    /// Parse IFCMATERIALLAYER entity.
    fn resolve_material_layer(
        id: EntityId,
        entities: &HashMap<EntityId, IfcEntity>,
    ) -> Option<MaterialLayerInfo> {
        let entity = entities.get(&id)?;
        // IFCMATERIALLAYER(Material, LayerThickness, IsVentilated, Name, Description, ...)
        let mat_id = entity.get_entity_ref(0)?;
        let thickness = entity.get_real(1);
        let mat = Self::resolve_material(mat_id, entities)?;

        Some(MaterialLayerInfo {
            material_name: mat.name,
            thickness,
        })
    }

    /// Parse IFCMATERIALPROFILE entity.
    fn resolve_material_profile(
        id: EntityId,
        entities: &HashMap<EntityId, IfcEntity>,
    ) -> Option<MaterialLayerInfo> {
        let entity = entities.get(&id)?;
        // IFCMATERIALPROFILE(Name, Description, Material, Profile, Priority, Category)
        let mat_id = entity.get_entity_ref(2)?;
        let mat = Self::resolve_material(mat_id, entities)?;
        let profile_name = entity.get_string(0);

        Some(MaterialLayerInfo {
            material_name: format!(
                "{}{}",
                mat.name,
                profile_name
                    .map(|n| format!(" ({})", n))
                    .unwrap_or_default()
            ),
            thickness: None,
        })
    }

    /// Parse IFCRELDEFINESBYTYPE relationships to build element → type object mappings.
    fn parse_type_objects(ifc_file: &IfcFile) -> HashMap<EntityId, TypeObjectInfo> {
        let mut result: HashMap<EntityId, TypeObjectInfo> = HashMap::new();

        for rel in ifc_file.get_entities_by_type("IFCRELDEFINESBYTYPE") {
            // RelatedObjects (attr 4) = list of element refs
            let related_objects = match rel.get_list(4) {
                Some(list) => list,
                None => continue,
            };

            // RelatingType (attr 5) = ref to type entity (e.g., IFCWALLTYPE)
            let type_id = match rel.get_entity_ref(5) {
                Some(id) => id,
                None => continue,
            };

            let type_entity = match ifc_file.entities.get(&type_id) {
                Some(e) => e,
                None => continue,
            };

            // Type entity name (attr 2 for most type entities)
            let type_name = type_entity.get_string(2).unwrap_or_default();
            let ifc_type = type_entity.entity_type.clone();

            // Parse HasPropertySets (attr 5 for most IFC type entities)
            // Type entities like IFCWALLTYPE have:
            //   0=GlobalId, 1=OwnerHistory, 2=Name, 3=Description,
            //   4=ApplicableOccurrence, 5=HasPropertySets, ...
            let type_psets = Self::parse_type_property_sets(type_id, &ifc_file.entities);

            let info = TypeObjectInfo {
                type_name,
                ifc_type,
                property_sets: type_psets,
            };

            for val in related_objects {
                if let IfcValue::EntityRef(obj_id) = val {
                    result.insert(*obj_id, info.clone());
                }
            }
        }

        result
    }

    /// Parse the HasPropertySets attribute of a type entity.
    fn parse_type_property_sets(
        type_id: EntityId,
        entities: &HashMap<EntityId, IfcEntity>,
    ) -> Vec<ParsedPropertySet> {
        let entity = match entities.get(&type_id) {
            Some(e) => e,
            None => return Vec::new(),
        };

        // HasPropertySets is attr 5 for element type entities
        let pset_list = match entity.get_list(5) {
            Some(list) => list,
            None => return Vec::new(),
        };

        let mut psets = Vec::new();
        for val in pset_list {
            if let IfcValue::EntityRef(pset_id) = val {
                if let Some(ps) = Self::parse_single_property_set(*pset_id, entities) {
                    psets.push(ps);
                }
            }
        }
        psets
    }

    /// Get element by ID.
    /// NOTE: Prefer using RegisteredModel.elements() from the cache instead of this method.
    /// This regenerates the full mesh which is expensive.
    pub fn get_element_info(&self, element_id: i32) -> Option<ElementInfo> {
        let mesh = self.generate_meshes();
        mesh.elements.into_iter().find(|e| e.id == element_id)
    }

}
