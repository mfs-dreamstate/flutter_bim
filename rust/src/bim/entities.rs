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
