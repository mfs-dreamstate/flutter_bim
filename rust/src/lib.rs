mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */
// BIM Viewer Rust Library
// This library handles:
// - IFC file parsing
// - 3D rendering with wgpu
// - GIS/georeferencing
// - High-performance geometry processing

pub mod api;

// Module declarations (will be implemented in phases)
// pub mod bim;      // Phase 2: IFC parsing
// pub mod renderer; // Phase 3: 3D rendering
// pub mod gis;      // Phase 6: GIS integration

// Re-export API for Flutter Rust Bridge
pub use api::*;
