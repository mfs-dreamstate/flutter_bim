//! BIM Model Representation
//!
//! High-level API for working with loaded IFC models.

use super::entities::*;
use super::geometry::{color_for_element_type, generate_box_with_normals, merge_meshes, BoundingBox};
use super::ifc_parser::IfcFile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
                    properties: HashMap::new(),
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
        // Generate grid lines based on model bounds
        // Since we may not have full geometry, we generate lines based on axis labels
        // and use the model's bounding box to determine extents

        let mut grid_lines = Vec::new();

        // Get model bounds for extent calculation
        let bounds = model.get_bounds();
        let (min_x, max_x, min_y, max_y) = if let Some(b) = &bounds {
            (b.min[0], b.max[0], b.min[1], b.max[1])
        } else {
            // Default bounds if no geometry
            (-20.0, 20.0, -20.0, 20.0)
        };

        let z = 0.0; // Grid at ground level
        let margin = 5.0; // Extend lines beyond bounds

        // Generate lines for each grid axis
        for (i, axis) in model.grid_axes.iter().enumerate() {
            // Check if this is a U or V axis
            let is_u_axis = model.grids.iter().any(|g| g.u_axes.contains(&axis.id));

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

    /// Get the bounding box of all elements in the model
    fn get_bounds(&self) -> Option<BoundingBox> {
        let mesh = self.generate_meshes();
        mesh.bounds
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
    /// Generate meshes from the BIM model for rendering
    /// This creates placeholder box geometry for each element
    pub fn generate_meshes(&self) -> ModelMesh {
        let mut meshes = Vec::new();
        let mut elements = Vec::new();
        let mut current_triangle = 0u32;
        let y_offset = 0.0f32;

        // Helper to add element info
        fn add_element(
            elements: &mut Vec<ElementInfo>,
            current_triangle: &mut u32,
            mesh_triangles: u32,
            id: i32,
            element_type: &str,
            name: &str,
            global_id: &str,
            center: [f32; 3],
            size: [f32; 3],
        ) {
            let half = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
            elements.push(ElementInfo {
                id,
                element_type: element_type.to_string(),
                name: name.to_string(),
                global_id: global_id.to_string(),
                bounds: BoundingBox {
                    min: [center[0] - half[0], center[1] - half[1], center[2] - half[2]],
                    max: [center[0] + half[0], center[1] + half[1], center[2] + half[2]],
                },
                triangle_start: *current_triangle,
                triangle_count: mesh_triangles,
            });
            *current_triangle += mesh_triangles;
        }

        // Generate wall meshes
        for (i, wall) in self.walls.iter().enumerate() {
            let color = color_for_element_type("WALL");
            let center = [i as f32 * 3.0, 1.5 + y_offset, 0.0];
            let size = [2.5, 3.0, 0.2];
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                wall.product.id, "Wall",
                wall.product.name.as_deref().unwrap_or("Wall"),
                &wall.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate slab meshes (floors)
        for (i, slab) in self.slabs.iter().enumerate() {
            let color = color_for_element_type("SLAB");
            let center = [0.0, y_offset + i as f32 * 3.5, 0.0];
            let size = [10.0, 0.3, 8.0];
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                slab.product.id, "Slab",
                slab.product.name.as_deref().unwrap_or("Slab"),
                &slab.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate column meshes
        for (i, column) in self.columns.iter().enumerate() {
            let color = color_for_element_type("COLUMN");
            let x = (i % 4) as f32 * 3.0 - 4.5;
            let z = (i / 4) as f32 * 3.0 - 3.0;
            let center = [x, 1.5 + y_offset, z];
            let size = [0.4, 3.0, 0.4];
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                column.product.id, "Column",
                column.product.name.as_deref().unwrap_or("Column"),
                &column.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate beam meshes
        for (i, beam) in self.beams.iter().enumerate() {
            let color = color_for_element_type("BEAM");
            let center = [0.0, 2.8 + y_offset, i as f32 * 2.0 - 2.0];
            let size = [8.0, 0.4, 0.3];
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                beam.product.id, "Beam",
                beam.product.name.as_deref().unwrap_or("Beam"),
                &beam.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate door meshes
        for (i, door) in self.doors.iter().enumerate() {
            let color = color_for_element_type("DOOR");
            let height = door.overall_height.unwrap_or(2.1) as f32;
            let width = door.overall_width.unwrap_or(0.9) as f32;
            let center = [i as f32 * 3.0 + 1.0, height / 2.0 + y_offset, 0.1];
            let size = [width, height, 0.1];
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                door.product.id, "Door",
                door.product.name.as_deref().unwrap_or("Door"),
                &door.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate window meshes
        for (i, window) in self.windows.iter().enumerate() {
            let color = color_for_element_type("WINDOW");
            let height = window.overall_height.unwrap_or(1.2) as f32;
            let width = window.overall_width.unwrap_or(1.0) as f32;
            let center = [i as f32 * 3.0 + 1.5, 1.5 + y_offset, 0.1];
            let size = [width, height, 0.05];
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                window.product.id, "Window",
                window.product.name.as_deref().unwrap_or("Window"),
                &window.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate roof meshes
        for (i, roof) in self.roofs.iter().enumerate() {
            let color = color_for_element_type("ROOF");
            let center = [0.0, 3.15 + y_offset + i as f32 * 0.5, 0.0];
            let size = [10.0, 0.3, 8.0];
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                roof.product.id, "Roof",
                roof.product.name.as_deref().unwrap_or("Roof"),
                &roof.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate stair meshes
        for (i, stair) in self.stairs.iter().enumerate() {
            let color = color_for_element_type("STAIR");
            let center = [3.0 + i as f32 * 2.0, 1.5 + y_offset, 2.0];
            let size = [1.5, 3.0, 3.0];
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                stair.product.id, "Stair",
                stair.product.name.as_deref().unwrap_or("Stair"),
                &stair.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate footing meshes (foundations)
        for (i, footing) in self.footings.iter().enumerate() {
            let color = color_for_element_type("FOOTING");
            let x = (i % 4) as f32 * 3.0 - 4.5;
            let z = (i / 4) as f32 * 3.0 - 3.0;
            let center = [x, -0.5 + y_offset, z];
            let size = [1.0, 0.6, 1.0];
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                footing.product.id, "Footing",
                footing.product.name.as_deref().unwrap_or("Footing"),
                &footing.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate pipe meshes (MEP - shown as thin horizontal boxes)
        for (i, pipe) in self.pipes.iter().enumerate() {
            let color = color_for_element_type("PIPE");
            let y_pos = 2.5 + (i / 3) as f32 * 0.3;
            let z_pos = (i % 3) as f32 * 2.0 - 2.0;
            let center = [0.0, y_pos + y_offset, z_pos];
            let size = [8.0, 0.1, 0.1]; // Thin horizontal pipe
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                pipe.product.id, "Pipe",
                pipe.product.name.as_deref().unwrap_or("Pipe"),
                &pipe.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate duct meshes (MEP - shown as rectangular boxes)
        for (i, duct) in self.ducts.iter().enumerate() {
            let color = color_for_element_type("DUCT");
            let z_pos = (i % 2) as f32 * 4.0 - 2.0;
            let center = [0.0, 2.7 + y_offset, z_pos];
            let size = [8.0, 0.4, 0.6]; // Rectangular duct
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                duct.product.id, "Duct",
                duct.product.name.as_deref().unwrap_or("Duct"),
                &duct.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate flow terminal meshes (vents, outlets)
        for (i, terminal) in self.flow_terminals.iter().enumerate() {
            let color = color_for_element_type("FLOWTERMINAL");
            let x = (i % 4) as f32 * 2.5 - 3.75;
            let z = (i / 4) as f32 * 3.0 - 1.5;
            let center = [x, 2.9 + y_offset, z];
            let size = [0.4, 0.1, 0.4]; // Small square vent
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                terminal.product.id, "FlowTerminal",
                terminal.product.name.as_deref().unwrap_or("Vent"),
                &terminal.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate cable carrier meshes (electrical)
        for (i, carrier) in self.cable_carriers.iter().enumerate() {
            let color = color_for_element_type("CABLE");
            let y_pos = 2.8 + (i / 2) as f32 * 0.2;
            let z_pos = (i % 2) as f32 * 6.0 - 3.0;
            let center = [0.0, y_pos + y_offset, z_pos];
            let size = [8.0, 0.08, 0.15]; // Cable tray
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                carrier.product.id, "CableCarrier",
                carrier.product.name.as_deref().unwrap_or("Cable Tray"),
                &carrier.product.global_id,
                center, size,
            );
            meshes.push(mesh);
        }

        // Generate proxy meshes (generic elements)
        for (i, proxy) in self.proxies.iter().enumerate() {
            let color = color_for_element_type("PROXY");
            let x = (i % 3) as f32 * 2.0 - 2.0;
            let z = (i / 3) as f32 * 2.0 - 2.0;
            let center = [x, 1.0 + y_offset, z];
            let size = [0.5, 0.5, 0.5];
            let mesh = generate_box_with_normals(center, size, color);
            let triangles = (mesh.indices.len() / 3) as u32;
            add_element(
                &mut elements, &mut current_triangle, triangles,
                proxy.product.id, "Proxy",
                proxy.product.name.as_deref().unwrap_or("Element"),
                &proxy.product.global_id,
                center, size,
            );
            meshes.push(mesh);
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
                add_element(
                    &mut elements, &mut current_triangle, triangles,
                    i as i32, elem_type, name, &format!("default_{}", i),
                    *center, *size,
                );
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

    /// Get element by ID
    pub fn get_element_info(&self, element_id: i32) -> Option<ElementInfo> {
        let mesh = self.generate_meshes();
        mesh.elements.into_iter().find(|e| e.id == element_id)
    }

    /// Generate meshes with visibility filter and highlight support
    pub fn generate_meshes_filtered(
        &self,
        hidden_types: &std::collections::HashSet<String>,
        selected_id: Option<i32>,
    ) -> ModelMesh {
        use super::geometry::Mesh;

        let mut meshes = Vec::new();
        let mut elements = Vec::new();
        let mut current_triangle = 0u32;
        let y_offset = 0.0f32;

        // Highlight color (bright cyan/teal)
        let highlight_color: [f32; 4] = [0.2, 0.9, 0.9, 1.0];

        // Helper to add element info
        fn add_element(
            elements: &mut Vec<ElementInfo>,
            current_triangle: &mut u32,
            mesh_triangles: u32,
            id: i32,
            element_type: &str,
            name: &str,
            global_id: &str,
            center: [f32; 3],
            size: [f32; 3],
        ) {
            let half = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
            elements.push(ElementInfo {
                id,
                element_type: element_type.to_string(),
                name: name.to_string(),
                global_id: global_id.to_string(),
                bounds: BoundingBox {
                    min: [center[0] - half[0], center[1] - half[1], center[2] - half[2]],
                    max: [center[0] + half[0], center[1] + half[1], center[2] + half[2]],
                },
                triangle_start: *current_triangle,
                triangle_count: mesh_triangles,
            });
            *current_triangle += mesh_triangles;
        }

        // Helper to apply highlight color to mesh
        fn apply_highlight(mesh: &mut Mesh, highlight: [f32; 4]) {
            for i in (0..mesh.colors.len()).step_by(4) {
                mesh.colors[i] = highlight[0];
                mesh.colors[i + 1] = highlight[1];
                mesh.colors[i + 2] = highlight[2];
                mesh.colors[i + 3] = highlight[3];
            }
        }

        // Generate wall meshes
        if !hidden_types.contains("Wall") {
            for (i, wall) in self.walls.iter().enumerate() {
                let color = color_for_element_type("WALL");
                let center = [i as f32 * 3.0, 1.5 + y_offset, 0.0];
                let size = [2.5, 3.0, 0.2];
                let mut mesh = generate_box_with_normals(center, size, color);

                if selected_id == Some(wall.product.id) {
                    apply_highlight(&mut mesh, highlight_color);
                }

                let triangles = (mesh.indices.len() / 3) as u32;
                add_element(
                    &mut elements, &mut current_triangle, triangles,
                    wall.product.id, "Wall",
                    wall.product.name.as_deref().unwrap_or("Wall"),
                    &wall.product.global_id,
                    center, size,
                );
                meshes.push(mesh);
            }
        }

        // Generate slab meshes (floors)
        if !hidden_types.contains("Slab") {
            for (i, slab) in self.slabs.iter().enumerate() {
                let color = color_for_element_type("SLAB");
                let center = [0.0, y_offset + i as f32 * 3.5, 0.0];
                let size = [10.0, 0.3, 8.0];
                let mut mesh = generate_box_with_normals(center, size, color);

                if selected_id == Some(slab.product.id) {
                    apply_highlight(&mut mesh, highlight_color);
                }

                let triangles = (mesh.indices.len() / 3) as u32;
                add_element(
                    &mut elements, &mut current_triangle, triangles,
                    slab.product.id, "Slab",
                    slab.product.name.as_deref().unwrap_or("Slab"),
                    &slab.product.global_id,
                    center, size,
                );
                meshes.push(mesh);
            }
        }

        // Generate column meshes
        if !hidden_types.contains("Column") {
            for (i, column) in self.columns.iter().enumerate() {
                let color = color_for_element_type("COLUMN");
                let x = (i % 4) as f32 * 3.0 - 4.5;
                let z = (i / 4) as f32 * 3.0 - 3.0;
                let center = [x, 1.5 + y_offset, z];
                let size = [0.4, 3.0, 0.4];
                let mut mesh = generate_box_with_normals(center, size, color);

                if selected_id == Some(column.product.id) {
                    apply_highlight(&mut mesh, highlight_color);
                }

                let triangles = (mesh.indices.len() / 3) as u32;
                add_element(
                    &mut elements, &mut current_triangle, triangles,
                    column.product.id, "Column",
                    column.product.name.as_deref().unwrap_or("Column"),
                    &column.product.global_id,
                    center, size,
                );
                meshes.push(mesh);
            }
        }

        // Generate beam meshes
        if !hidden_types.contains("Beam") {
            for (i, beam) in self.beams.iter().enumerate() {
                let color = color_for_element_type("BEAM");
                let center = [0.0, 2.8 + y_offset, i as f32 * 2.0 - 2.0];
                let size = [8.0, 0.4, 0.3];
                let mut mesh = generate_box_with_normals(center, size, color);

                if selected_id == Some(beam.product.id) {
                    apply_highlight(&mut mesh, highlight_color);
                }

                let triangles = (mesh.indices.len() / 3) as u32;
                add_element(
                    &mut elements, &mut current_triangle, triangles,
                    beam.product.id, "Beam",
                    beam.product.name.as_deref().unwrap_or("Beam"),
                    &beam.product.global_id,
                    center, size,
                );
                meshes.push(mesh);
            }
        }

        // Generate door meshes
        if !hidden_types.contains("Door") {
            for (i, door) in self.doors.iter().enumerate() {
                let color = color_for_element_type("DOOR");
                let height = door.overall_height.unwrap_or(2.1) as f32;
                let width = door.overall_width.unwrap_or(0.9) as f32;
                let center = [i as f32 * 3.0 + 1.0, height / 2.0 + y_offset, 0.1];
                let size = [width, height, 0.1];
                let mut mesh = generate_box_with_normals(center, size, color);

                if selected_id == Some(door.product.id) {
                    apply_highlight(&mut mesh, highlight_color);
                }

                let triangles = (mesh.indices.len() / 3) as u32;
                add_element(
                    &mut elements, &mut current_triangle, triangles,
                    door.product.id, "Door",
                    door.product.name.as_deref().unwrap_or("Door"),
                    &door.product.global_id,
                    center, size,
                );
                meshes.push(mesh);
            }
        }

        // Generate window meshes
        if !hidden_types.contains("Window") {
            for (i, window) in self.windows.iter().enumerate() {
                let color = color_for_element_type("WINDOW");
                let height = window.overall_height.unwrap_or(1.2) as f32;
                let width = window.overall_width.unwrap_or(1.0) as f32;
                let center = [i as f32 * 3.0 + 1.5, 1.5 + y_offset, 0.1];
                let size = [width, height, 0.05];
                let mut mesh = generate_box_with_normals(center, size, color);

                if selected_id == Some(window.product.id) {
                    apply_highlight(&mut mesh, highlight_color);
                }

                let triangles = (mesh.indices.len() / 3) as u32;
                add_element(
                    &mut elements, &mut current_triangle, triangles,
                    window.product.id, "Window",
                    window.product.name.as_deref().unwrap_or("Window"),
                    &window.product.global_id,
                    center, size,
                );
                meshes.push(mesh);
            }
        }

        // If no elements, create a default building shape
        if meshes.is_empty() {
            let default_elements = [
                ([0.0, 0.0, 0.0], [10.0, 0.3, 8.0], "SLAB", "Floor", "Slab"),
                ([-4.9, 1.5, 0.0], [0.2, 3.0, 8.0], "WALL", "Left Wall", "Wall"),
                ([4.9, 1.5, 0.0], [0.2, 3.0, 8.0], "WALL", "Right Wall", "Wall"),
                ([0.0, 1.5, -3.9], [10.0, 3.0, 0.2], "WALL", "Back Wall", "Wall"),
                ([0.0, 1.5, 3.9], [10.0, 3.0, 0.2], "WALL", "Front Wall", "Wall"),
                ([0.0, 3.15, 0.0], [10.0, 0.3, 8.0], "ROOF", "Roof", "Roof"),
            ];

            for (i, (center, size, elem_type, name, type_name)) in default_elements.iter().enumerate() {
                if hidden_types.contains(*type_name) {
                    continue;
                }
                let mut mesh = generate_box_with_normals(*center, *size, color_for_element_type(elem_type));

                if selected_id == Some(i as i32) {
                    apply_highlight(&mut mesh, highlight_color);
                }

                let triangles = (mesh.indices.len() / 3) as u32;
                add_element(
                    &mut elements, &mut current_triangle, triangles,
                    i as i32, type_name, name, &format!("default_{}", i),
                    *center, *size,
                );
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
}
