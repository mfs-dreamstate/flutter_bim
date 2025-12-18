//! BIM Model Representation
//!
//! High-level API for working with loaded IFC models.

use super::entities::*;
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
    pub walls: Vec<IfcWall>,
    pub slabs: Vec<IfcSlab>,
    pub columns: Vec<IfcColumn>,
    pub beams: Vec<IfcBeam>,
    pub doors: Vec<IfcDoor>,
    pub windows: Vec<IfcWindow>,
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
            walls: Vec::new(),
            slabs: Vec::new(),
            columns: Vec::new(),
            beams: Vec::new(),
            doors: Vec::new(),
            windows: Vec::new(),
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

        // Extract walls
        model.walls = Self::extract_walls(ifc_file);

        // Extract slabs
        model.slabs = Self::extract_slabs(ifc_file);

        // Extract columns
        model.columns = Self::extract_columns(ifc_file);

        // Extract beams
        model.beams = Self::extract_beams(ifc_file);

        // Extract doors
        model.doors = Self::extract_doors(ifc_file);

        // Extract windows
        model.windows = Self::extract_windows(ifc_file);

        model.element_count = model.walls.len()
            + model.slabs.len()
            + model.columns.len()
            + model.beams.len()
            + model.doors.len()
            + model.windows.len();

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
}

impl Default for BimModel {
    fn default() -> Self {
        Self::new()
    }
}
