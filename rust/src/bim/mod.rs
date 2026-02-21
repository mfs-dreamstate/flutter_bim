//! BIM Module - IFC file parsing and processing
//!
//! This module handles loading and parsing IFC (Industry Foundation Classes) files.
//! IFC files use the STEP format (ISO 10303-21) for data representation.

pub mod entities;
pub mod geometry;
pub mod gltf_import;
pub mod ifc_parser;
pub mod model;
pub mod model_registry;
pub mod obj_import;
pub mod placement;
pub mod tessellation;
pub mod triangulate;
pub mod bcf;
pub mod clash;
pub mod dxf_import;
pub mod point_cloud;
pub mod pdf_import;

pub use entities::*;
pub use geometry::*;
pub use ifc_parser::*;
pub use model::*;
pub use model_registry::*;
