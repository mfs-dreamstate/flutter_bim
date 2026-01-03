//! BIM Module - IFC file parsing and processing
//!
//! This module handles loading and parsing IFC (Industry Foundation Classes) files.
//! IFC files use the STEP format (ISO 10303-21) for data representation.

pub mod entities;
pub mod geometry;
pub mod ifc_parser;
pub mod model;
pub mod model_registry;

pub use entities::*;
pub use geometry::*;
pub use ifc_parser::*;
pub use model::*;
pub use model_registry::*;
