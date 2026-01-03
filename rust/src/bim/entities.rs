//! IFC Entity Definitions
//!
//! Core IFC entity types for BIM data representation.
//! Based on IFC 2x3 and IFC 4 specifications.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for IFC entities (e.g., #123)
pub type EntityId = i32;

/// IFC Entity - Generic container for any IFC object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcEntity {
    pub id: EntityId,
    pub entity_type: String,
    pub attributes: Vec<IfcValue>,
}

/// IFC Value - Represents any value in IFC files
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IfcValue {
    Null,
    Integer(i64),
    Real(f64),
    String(String),
    Enum(String), // Enumeration values like .ELEMENT., .TRUE., .FALSE.
    Boolean(bool),
    EntityRef(EntityId),
    List(Vec<IfcValue>),
}

/// IFC Product - Base class for physical objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcProduct {
    pub id: EntityId,
    pub global_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub object_type: Option<String>,
    pub properties: HashMap<String, String>,
}

/// IFC Wall
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcWall {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Slab (floor/ceiling)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcSlab {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcColumn {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Beam
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcBeam {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Door
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcDoor {
    pub product: IfcProduct,
    pub overall_height: Option<f64>,
    pub overall_width: Option<f64>,
}

/// IFC Window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcWindow {
    pub product: IfcProduct,
    pub overall_height: Option<f64>,
    pub overall_width: Option<f64>,
}

/// IFC Roof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcRoof {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Stair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcStair {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Pipe Segment (MEP)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcPipeSegment {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Duct Segment (MEP)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcDuctSegment {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Cable Carrier Segment (Electrical)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcCableCarrierSegment {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Flow Terminal (MEP - outlets, fixtures)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcFlowTerminal {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Building Element Proxy (generic elements)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcBuildingElementProxy {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Footing (foundation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcFooting {
    pub product: IfcProduct,
    pub predefined_type: Option<String>,
}

/// IFC Grid - Structural grid system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcGrid {
    pub id: EntityId,
    pub global_id: String,
    pub name: Option<String>,
    pub u_axes: Vec<EntityId>, // References to IfcGridAxis entities
    pub v_axes: Vec<EntityId>, // References to IfcGridAxis entities
}

/// IFC Grid Axis - Individual axis line in a grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcGridAxis {
    pub id: EntityId,
    pub axis_tag: String,          // Label like "A", "B", "1", "2"
    pub axis_curve: Option<EntityId>, // Reference to curve geometry
    pub same_sense: bool,          // Direction of axis
}

/// Represents a parsed grid line for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridLine {
    pub tag: String,              // Label ("A", "1", etc.)
    pub start: [f32; 3],          // Start point
    pub end: [f32; 3],            // End point
    pub is_u_axis: bool,          // True for U axis, false for V axis
}

/// IFC Building Storey (floor level)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcBuildingStorey {
    pub id: EntityId,
    pub name: String,
    pub elevation: Option<f64>,
}

/// IFC Building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcBuilding {
    pub id: EntityId,
    pub name: String,
    pub description: Option<String>,
}

/// IFC Site
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcSite {
    pub id: EntityId,
    pub name: String,
    pub description: Option<String>,
    pub latitude: Option<Vec<i32>>,  // [degrees, minutes, seconds, microseconds]
    pub longitude: Option<Vec<i32>>, // [degrees, minutes, seconds, microseconds]
    pub elevation: Option<f64>,
}

/// IFC Project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcProject {
    pub id: EntityId,
    pub global_id: String,
    pub name: String,
    pub description: Option<String>,
}

impl IfcEntity {
    /// Create a new IFC entity
    pub fn new(id: EntityId, entity_type: String) -> Self {
        Self {
            id,
            entity_type,
            attributes: Vec::new(),
        }
    }

    /// Get attribute at index
    pub fn get_attr(&self, index: usize) -> Option<&IfcValue> {
        self.attributes.get(index)
    }

    /// Get string attribute
    pub fn get_string(&self, index: usize) -> Option<String> {
        match self.get_attr(index)? {
            IfcValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Get integer attribute
    pub fn get_int(&self, index: usize) -> Option<i64> {
        match self.get_attr(index)? {
            IfcValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Get real attribute
    pub fn get_real(&self, index: usize) -> Option<f64> {
        match self.get_attr(index)? {
            IfcValue::Real(r) => Some(*r),
            IfcValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get entity reference attribute
    pub fn get_entity_ref(&self, index: usize) -> Option<EntityId> {
        match self.get_attr(index)? {
            IfcValue::EntityRef(id) => Some(*id),
            _ => None,
        }
    }

    /// Get list attribute
    pub fn get_list(&self, index: usize) -> Option<&Vec<IfcValue>> {
        match self.get_attr(index)? {
            IfcValue::List(list) => Some(list),
            _ => None,
        }
    }
}

impl Default for IfcValue {
    fn default() -> Self {
        IfcValue::Null
    }
}
