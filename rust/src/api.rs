// Flutter Rust Bridge API
// This file contains all functions exposed to Flutter via FFI

use flutter_rust_bridge::frb;

/// Initialize the BIM viewer engine
/// This should be called once when the app starts
#[frb(sync)]
pub fn initialize() -> String {
    // Initialize logging
    tracing_subscriber::fmt::init();
    tracing::info!("BIM Viewer initialized");

    "BIM Viewer initialized successfully".to_string()
}

/// Get the library version
#[frb(sync)]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get system information for debugging
#[frb(sync)]
pub fn get_system_info() -> String {
    format!(
        "Rust Version: {}\nTarget: {}\nOS: {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        std::env::consts::OS
    )
}

/// Test async functionality
/// This demonstrates that async Rust functions work correctly across FFI
pub async fn test_async() -> String {
    tracing::debug!("Starting async test");

    // Simulate some async work
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    tracing::debug!("Async test completed");
    "Async test completed successfully".to_string()
}

/// Test error handling across FFI
/// Returns a Result to demonstrate error propagation
#[frb(sync)]
pub fn test_error_handling(should_fail: bool) -> Result<String, String> {
    if should_fail {
        Err("Intentional error for testing".to_string())
    } else {
        Ok("Success!".to_string())
    }
}

// ============================================================================
// Phase 1 API is now complete
// Additional API functions will be added in subsequent phases:
//
// Phase 2: BIM File Parsing
// - load_model(path: String) -> Result<ModelInfo>
// - get_element_count() -> u32
// - get_element_info(id: String) -> Option<ElementInfo>
//
// Phase 3: 3D Rendering
// - create_renderer(config: RendererConfig) -> Result<()>
// - render_frame() -> Result<()>
// - update_camera(transform: CameraTransform) -> Result<()>
//
// Phase 4: Materials & Lighting
// - set_material(element_id: String, material: Material) -> Result<()>
// - set_lighting(config: LightingConfig) -> Result<()>
//
// Phase 5: Interaction
// - ray_cast(ray: Ray) -> Option<String>
// - select_element(id: String) -> Result<ElementInfo>
//
// Phase 6: GIS Integration
// - get_geo_location() -> Option<GeoLocation>
// - get_building_footprint() -> Option<Vec<GeoCoordinate>>
// ============================================================================
