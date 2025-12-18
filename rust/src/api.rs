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
// Phase 2 API: BIM File Parsing
// ============================================================================

use crate::bim::{BimModel, IfcFile, ModelInfo};
use std::sync::Mutex;

// Global model storage (simple approach for Phase 2)
static CURRENT_MODEL: Mutex<Option<BimModel>> = Mutex::new(None);

/// Load an IFC file and parse it
/// This is async because file I/O can be slow
pub async fn load_ifc_file(file_path: String) -> Result<ModelInfo, String> {
    tracing::info!("Loading IFC file: {}", file_path);

    // Read file contents
    let content = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Parse IFC file
    let ifc_file = IfcFile::parse(&content)?;

    tracing::info!(
        "Parsed IFC file: {} entities",
        ifc_file.entity_count()
    );

    // Build BIM model from IFC
    let model = BimModel::from_ifc_file(&ifc_file)?;

    // Get model info before storing
    let model_info = model.get_info();

    // Store the model
    let mut current = CURRENT_MODEL.lock().unwrap();
    *current = Some(model);

    tracing::info!("Model loaded successfully");
    Ok(model_info)
}

/// Get information about the currently loaded model
#[frb(sync)]
pub fn get_model_info() -> Result<ModelInfo, String> {
    let model = CURRENT_MODEL.lock().unwrap();

    match model.as_ref() {
        Some(m) => Ok(m.get_info()),
        None => Err("No model loaded".to_string()),
    }
}

/// Check if a model is currently loaded
#[frb(sync)]
pub fn is_model_loaded() -> bool {
    let model = CURRENT_MODEL.lock().unwrap();
    model.is_some()
}

/// Unload the current model and free memory
#[frb(sync)]
pub fn unload_model() -> Result<(), String> {
    let mut model = CURRENT_MODEL.lock().unwrap();

    if model.is_none() {
        return Err("No model loaded".to_string());
    }

    *model = None;
    tracing::info!("Model unloaded");
    Ok(())
}

/// Parse IFC file content (for testing - takes content string instead of file path)
pub async fn parse_ifc_content(content: String) -> Result<ModelInfo, String> {
    tracing::info!("Parsing IFC content ({} bytes)", content.len());

    // Parse IFC file
    let ifc_file = IfcFile::parse(&content)?;

    tracing::info!(
        "Parsed IFC file: {} entities",
        ifc_file.entity_count()
    );

    // Build BIM model from IFC
    let model = BimModel::from_ifc_file(&ifc_file)?;

    // Get model info before storing
    let model_info = model.get_info();

    // Store the model
    let mut current = CURRENT_MODEL.lock().unwrap();
    *current = Some(model);

    Ok(model_info)
}

// ============================================================================
// Future Phases:
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
