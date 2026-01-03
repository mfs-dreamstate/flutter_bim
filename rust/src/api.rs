// Flutter Rust Bridge API
// This file contains all functions exposed to Flutter via FFI

use flutter_rust_bridge::frb;

/// Initialize the BIM viewer engine
/// This should be called once when the app starts
#[frb(sync)]
pub fn initialize() -> String {
    // Initialize logging (ignore error if already initialized - happens on hot restart)
    let _ = tracing_subscriber::fmt::try_init();
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

use crate::bim::{BimModel, ElementInfo, GridLine, IfcFile, ModelInfo, ModelRegistry, RegisteredModelInfo};
use crate::renderer::ray_aabb_intersect;
use glam::Vec3;
use std::sync::{LazyLock, Mutex};

// Global model registry (supports multiple models)
static MODEL_REGISTRY: LazyLock<Mutex<ModelRegistry>> =
    LazyLock::new(|| Mutex::new(ModelRegistry::new()));

// Visibility settings for element types (hidden types are stored here)
static VISIBILITY: LazyLock<Mutex<std::collections::HashSet<String>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashSet::new()));

// Currently selected element ID (for highlighting)
static SELECTED_ELEMENT: Mutex<Option<i32>> = Mutex::new(None);

// Grid visibility flag
static GRID_VISIBLE: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(true));

/// Load an IFC file and parse it (backward compatible - loads as primary)
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

    // Extract name from file path
    let name = std::path::Path::new(&file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    // Store in registry
    let mut registry = MODEL_REGISTRY.lock().unwrap();
    registry.add_model(model, name, Some(file_path));

    tracing::info!("Model loaded successfully");
    Ok(model_info)
}

/// Get information about the currently loaded model (primary model)
#[frb(sync)]
pub fn get_model_info() -> Result<ModelInfo, String> {
    let registry = MODEL_REGISTRY.lock().unwrap();

    match registry.get_primary_model() {
        Some(m) => Ok(m.model.get_info()),
        None => Err("No model loaded".to_string()),
    }
}

/// Check if a model is currently loaded
#[frb(sync)]
pub fn is_model_loaded() -> bool {
    let registry = MODEL_REGISTRY.lock().unwrap();
    !registry.is_empty()
}

/// Unload the current model and free memory (primary model)
#[frb(sync)]
pub fn unload_model() -> Result<(), String> {
    let mut registry = MODEL_REGISTRY.lock().unwrap();

    if registry.is_empty() {
        return Err("No model loaded".to_string());
    }

    // Remove primary model
    if let Some(id) = registry.get_primary_model_id().cloned() {
        registry.remove_model(&id);
        tracing::info!("Model unloaded");
        Ok(())
    } else {
        Err("No primary model to unload".to_string())
    }
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

    // Store in registry
    let mut registry = MODEL_REGISTRY.lock().unwrap();
    registry.add_model(model, "Parsed Model".to_string(), None);

    Ok(model_info)
}

// ============================================================================
// Multi-Model API (Retrofit)
// ============================================================================

/// Load a model with a specific ID
pub async fn load_model(model_id: String, file_path: String) -> Result<ModelInfo, String> {
    tracing::info!("Loading model '{}' from: {}", model_id, file_path);

    // Read file contents
    let content = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Parse IFC file
    let ifc_file = IfcFile::parse(&content)?;

    // Build BIM model from IFC
    let model = BimModel::from_ifc_file(&ifc_file)?;
    let model_info = model.get_info();

    // Extract name from file path
    let name = std::path::Path::new(&file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    // Store in registry with specified ID
    let mut registry = MODEL_REGISTRY.lock().unwrap();
    registry.add_model_with_id(model_id.clone(), model, name, Some(file_path));

    tracing::info!("Model '{}' loaded successfully", model_id);
    Ok(model_info)
}

/// Unload a specific model by ID
#[frb(sync)]
pub fn unload_model_by_id(model_id: String) -> Result<(), String> {
    let mut registry = MODEL_REGISTRY.lock().unwrap();

    if registry.remove_model(&model_id).is_some() {
        tracing::info!("Model '{}' unloaded", model_id);
        Ok(())
    } else {
        Err(format!("Model '{}' not found", model_id))
    }
}

/// List all loaded models
#[frb(sync)]
pub fn list_loaded_models() -> Vec<RegisteredModelInfo> {
    let registry = MODEL_REGISTRY.lock().unwrap();
    registry.get_all_model_info()
}

/// Get number of loaded models
#[frb(sync)]
pub fn get_model_count() -> usize {
    let registry = MODEL_REGISTRY.lock().unwrap();
    registry.model_count()
}

/// Set model visibility
#[frb(sync)]
pub fn set_model_visible(model_id: String, visible: bool) -> Result<(), String> {
    let mut registry = MODEL_REGISTRY.lock().unwrap();
    registry.set_model_visible(&model_id, visible)
}

/// Set the primary model
#[frb(sync)]
pub fn set_primary_model(model_id: String) -> Result<(), String> {
    let mut registry = MODEL_REGISTRY.lock().unwrap();
    registry.set_primary_model(&model_id)
}

/// Clear all models
#[frb(sync)]
pub fn clear_all_models() {
    let mut registry = MODEL_REGISTRY.lock().unwrap();
    registry.clear();
    tracing::info!("All models cleared");
}

// ============================================================================
// Phase 3 API: 3D Rendering
// ============================================================================

use crate::renderer::Renderer;

// Global renderer instance
static RENDERER: Mutex<Option<Renderer>> = Mutex::new(None);

/// Test renderer initialization
/// This initializes the wgpu graphics backend (headless for now)
pub async fn test_renderer_init() -> Result<String, String> {
    tracing::info!("Testing renderer initialization");

    let mut renderer = Renderer::new();
    renderer
        .initialize()
        .await
        .map_err(|e| format!("Renderer init failed: {}", e))?;

    Ok("Renderer initialized successfully! wgpu backend is working.".to_string())
}

/// Initialize the 3D renderer with given dimensions
pub async fn init_renderer(width: u32, height: u32) -> Result<String, String> {
    tracing::info!("Initializing renderer {}x{}", width, height);

    let mut renderer = Renderer::new();
    renderer
        .initialize()
        .await
        .map_err(|e| format!("GPU init failed: {}", e))?;

    renderer
        .init_scene(width, height)
        .map_err(|e| format!("Scene init failed: {}", e))?;

    // Store renderer globally
    let mut global = RENDERER.lock().unwrap();
    *global = Some(renderer);

    Ok(format!("Renderer initialized at {}x{}", width, height))
}

/// Render a frame and return RGBA pixel data
#[frb(sync)]
pub fn render_frame() -> Result<Vec<u8>, String> {
    let renderer = RENDERER.lock().unwrap();
    let r = renderer.as_ref().ok_or("Renderer not initialized")?;
    r.render_frame()
}

/// Orbit the camera around the target
#[frb(sync)]
pub fn orbit_camera(delta_x: f32, delta_y: f32) -> Result<(), String> {
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;
    r.orbit_camera(delta_x, delta_y);
    Ok(())
}

/// Zoom the camera in/out
#[frb(sync)]
pub fn zoom_camera(delta: f32) -> Result<(), String> {
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;
    r.zoom_camera(delta);
    Ok(())
}

/// Check if renderer is initialized
#[frb(sync)]
pub fn is_renderer_initialized() -> bool {
    let renderer = RENDERER.lock().unwrap();
    renderer.as_ref().map_or(false, |r| r.initialized)
}

/// Load the currently loaded BIM model into the renderer (primary model)
#[frb(sync)]
pub fn load_model_into_renderer() -> Result<String, String> {
    // Get model mesh data from primary model
    let registry = MODEL_REGISTRY.lock().unwrap();
    let reg_model = registry.get_primary_model().ok_or("No model loaded")?;

    let mesh = reg_model.model.generate_meshes();
    let vertex_count = mesh.vertices.len() / 3;
    let triangle_count = mesh.indices.len() / 3;

    // Upload to renderer
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;

    r.load_mesh(&mesh.vertices, &mesh.normals, &mesh.colors, &mesh.indices)?;

    // Fit camera to bounds if available
    if let Some(bounds) = mesh.bounds {
        r.fit_camera_to_bounds(bounds.min, bounds.max);
    }

    tracing::info!(
        "Loaded model: {} vertices, {} triangles",
        vertex_count,
        triangle_count
    );

    Ok(format!(
        "Model loaded: {} vertices, {} triangles",
        vertex_count, triangle_count
    ))
}

/// Load all visible models into the renderer
#[frb(sync)]
pub fn load_all_models_into_renderer() -> Result<String, String> {
    let registry = MODEL_REGISTRY.lock().unwrap();

    if registry.is_empty() {
        return Err("No models loaded".to_string());
    }

    // Collect mesh data from all visible models
    let mut all_vertices = Vec::new();
    let mut all_normals = Vec::new();
    let mut all_colors = Vec::new();
    let mut all_indices = Vec::new();
    let mut combined_bounds: Option<crate::bim::BoundingBox> = None;

    for (_id, reg_model) in registry.iter_visible() {
        let mesh = reg_model.model.generate_meshes();

        // Offset indices by current vertex count
        let vertex_offset = (all_vertices.len() / 3) as u32;
        for idx in &mesh.indices {
            all_indices.push(idx + vertex_offset);
        }

        all_vertices.extend(&mesh.vertices);
        all_normals.extend(&mesh.normals);
        all_colors.extend(&mesh.colors);

        // Update combined bounds
        if let Some(bounds) = mesh.bounds {
            combined_bounds = Some(match combined_bounds {
                None => bounds,
                Some(existing) => existing.union(&bounds),
            });
        }
    }

    let vertex_count = all_vertices.len() / 3;
    let triangle_count = all_indices.len() / 3;

    // Upload to renderer
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;

    r.load_mesh(&all_vertices, &all_normals, &all_colors, &all_indices)?;

    // Fit camera to combined bounds
    if let Some(bounds) = combined_bounds {
        r.fit_camera_to_bounds(bounds.min, bounds.max);
    }

    tracing::info!(
        "Loaded {} models: {} vertices, {} triangles",
        registry.model_count(),
        vertex_count,
        triangle_count
    );

    Ok(format!(
        "Loaded {} models: {} vertices, {} triangles",
        registry.model_count(),
        vertex_count,
        triangle_count
    ))
}

/// Fit camera to current model bounds (primary model)
#[frb(sync)]
pub fn fit_camera_to_model() -> Result<(), String> {
    // Get model mesh bounds from primary model
    let registry = MODEL_REGISTRY.lock().unwrap();
    let reg_model = registry.get_primary_model().ok_or("No model loaded")?;

    let mesh = reg_model.model.generate_meshes();
    let bounds = mesh.bounds.ok_or("Model has no bounds")?;

    // Update renderer camera
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;

    r.fit_camera_to_bounds(bounds.min, bounds.max);

    Ok(())
}

/// Fit camera to all visible models
#[frb(sync)]
pub fn fit_camera_to_all_models() -> Result<(), String> {
    let registry = MODEL_REGISTRY.lock().unwrap();

    if registry.is_empty() {
        return Err("No models loaded".to_string());
    }

    // Calculate combined bounds
    let mut combined_bounds: Option<crate::bim::BoundingBox> = None;

    for (_id, reg_model) in registry.iter_visible() {
        let mesh = reg_model.model.generate_meshes();
        if let Some(bounds) = mesh.bounds {
            combined_bounds = Some(match combined_bounds {
                None => bounds,
                Some(existing) => existing.union(&bounds),
            });
        }
    }

    let bounds = combined_bounds.ok_or("No visible models with bounds")?;

    // Update renderer camera
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;

    r.fit_camera_to_bounds(bounds.min, bounds.max);

    Ok(())
}

// ============================================================================
// Phase 5 API: Element Selection
// ============================================================================

/// Pick element at screen coordinates (searches all visible models)
/// screen_x and screen_y are normalized (0-1) with origin at top-left
#[frb(sync)]
pub fn pick_element(screen_x: f32, screen_y: f32) -> Result<Option<ElementInfo>, String> {
    let registry = MODEL_REGISTRY.lock().unwrap();

    if registry.is_empty() {
        return Err("No model loaded".to_string());
    }

    // Get camera for ray casting
    let renderer = RENDERER.lock().unwrap();
    let r = renderer.as_ref().ok_or("Renderer not initialized")?;

    // Cast ray from screen position
    let (ray_origin, ray_dir) = r.camera.screen_to_ray(screen_x, screen_y);

    // Find closest intersecting element across all visible models
    let mut closest: Option<(f32, ElementInfo)> = None;

    for (_model_id, reg_model) in registry.iter_visible() {
        let mesh = reg_model.model.generate_meshes();

        for element in &mesh.elements {
            let box_min = Vec3::from_array(element.bounds.min);
            let box_max = Vec3::from_array(element.bounds.max);

            if let Some(t) = ray_aabb_intersect(ray_origin, ray_dir, box_min, box_max) {
                match &closest {
                    None => closest = Some((t, element.clone())),
                    Some((closest_t, _)) if t < *closest_t => closest = Some((t, element.clone())),
                    _ => {}
                }
            }
        }
    }

    Ok(closest.map(|(_, e)| e))
}

/// Get all elements in the model (primary model)
#[frb(sync)]
pub fn get_all_elements() -> Result<Vec<ElementInfo>, String> {
    let registry = MODEL_REGISTRY.lock().unwrap();
    let reg_model = registry.get_primary_model().ok_or("No model loaded")?;
    let mesh = reg_model.model.generate_meshes();
    Ok(mesh.elements)
}

/// Get all elements from all visible models
#[frb(sync)]
pub fn get_all_elements_from_all_models() -> Result<Vec<ElementInfo>, String> {
    let registry = MODEL_REGISTRY.lock().unwrap();

    if registry.is_empty() {
        return Err("No models loaded".to_string());
    }

    let mut all_elements = Vec::new();

    for (_model_id, reg_model) in registry.iter_visible() {
        let mesh = reg_model.model.generate_meshes();
        all_elements.extend(mesh.elements);
    }

    Ok(all_elements)
}

/// Get element count by type (primary model)
#[frb(sync)]
pub fn get_element_counts() -> Result<std::collections::HashMap<String, usize>, String> {
    let registry = MODEL_REGISTRY.lock().unwrap();
    let reg_model = registry.get_primary_model().ok_or("No model loaded")?;
    let mesh = reg_model.model.generate_meshes();

    let mut counts = std::collections::HashMap::new();
    for element in &mesh.elements {
        *counts.entry(element.element_type.clone()).or_insert(0) += 1;
    }
    Ok(counts)
}

/// Set visibility for an element type
#[frb(sync)]
pub fn set_element_type_visible(element_type: String, visible: bool) -> Result<(), String> {
    let mut visibility = VISIBILITY.lock().unwrap();
    if visible {
        visibility.remove(&element_type);
    } else {
        visibility.insert(element_type);
    }
    Ok(())
}

/// Check if an element type is visible
#[frb(sync)]
pub fn is_element_type_visible(element_type: String) -> bool {
    let visibility = VISIBILITY.lock().unwrap();
    !visibility.contains(&element_type)
}

/// Get all hidden element types
#[frb(sync)]
pub fn get_hidden_element_types() -> Vec<String> {
    let visibility = VISIBILITY.lock().unwrap();
    visibility.iter().cloned().collect()
}

// ============================================================================
// Grid API
// ============================================================================

/// Get all grid lines from all visible models
#[frb(sync)]
pub fn get_grid_lines() -> Result<Vec<GridLine>, String> {
    let registry = MODEL_REGISTRY.lock().unwrap();

    let mut all_grid_lines = Vec::new();

    for (_model_id, reg_model) in registry.iter_visible() {
        all_grid_lines.extend(reg_model.model.grid_lines.clone());
    }

    Ok(all_grid_lines)
}

/// Check if grid is visible
#[frb(sync)]
pub fn is_grid_visible() -> bool {
    *GRID_VISIBLE.lock().unwrap()
}

/// Set grid visibility
#[frb(sync)]
pub fn set_grid_visible(visible: bool) -> Result<(), String> {
    let mut grid_visible = GRID_VISIBLE.lock().unwrap();
    *grid_visible = visible;
    Ok(())
}

/// Toggle grid visibility
#[frb(sync)]
pub fn toggle_grid_visible() -> bool {
    let mut grid_visible = GRID_VISIBLE.lock().unwrap();
    *grid_visible = !*grid_visible;
    *grid_visible
}

/// Get grid line count
#[frb(sync)]
pub fn get_grid_line_count() -> Result<usize, String> {
    let registry = MODEL_REGISTRY.lock().unwrap();

    let count: usize = registry
        .iter_visible()
        .map(|(_, reg_model)| reg_model.model.grid_lines.len())
        .sum();

    Ok(count)
}

// ============================================================================
// GIS / Georeferencing API
// ============================================================================

/// Georeferencing data from IFC site
#[derive(Debug, Clone)]
pub struct GeoReference {
    pub latitude: f64,
    pub longitude: f64,
    pub rotation: f64,
    pub width: f64,
    pub depth: f64,
    pub site_name: Option<String>,
}

/// Get georeferencing data from the primary model's site
#[frb(sync)]
pub fn get_geo_reference() -> Option<GeoReference> {
    let registry = MODEL_REGISTRY.lock().unwrap();
    let reg_model = registry.get_primary_model()?;

    // Try to extract georeferencing from IfcSite
    if let Some(site) = &reg_model.model.site {
        // Check if we have latitude/longitude data
        if let (Some(lat_parts), Some(lng_parts)) = (&site.latitude, &site.longitude) {
            if lat_parts.len() >= 3 && lng_parts.len() >= 3 {
                // Convert DMS (degrees, minutes, seconds) to decimal
                let latitude = dms_to_decimal(lat_parts);
                let longitude = dms_to_decimal(lng_parts);

                // Get model bounds for width/depth estimation
                let mesh = reg_model.model.generate_meshes();
                let (width, depth) = if let Some(bounds) = mesh.bounds {
                    (
                        (bounds.max[0] - bounds.min[0]) as f64,
                        (bounds.max[1] - bounds.min[1]) as f64,
                    )
                } else {
                    (30.0, 20.0) // Default size
                };

                return Some(GeoReference {
                    latitude,
                    longitude,
                    rotation: 0.0, // TODO: Extract from IfcMapConversion if available
                    width,
                    depth,
                    site_name: Some(site.name.clone()),
                });
            }
        }
    }

    None
}

/// Convert degrees, minutes, seconds to decimal degrees
fn dms_to_decimal(dms: &[i32]) -> f64 {
    if dms.len() < 3 {
        return 0.0;
    }
    let degrees = dms[0] as f64;
    let minutes = dms[1] as f64;
    let seconds = dms[2] as f64;
    let microseconds = if dms.len() > 3 { dms[3] as f64 } else { 0.0 };

    let sign = if degrees < 0.0 { -1.0 } else { 1.0 };
    sign * (degrees.abs() + minutes / 60.0 + seconds / 3600.0 + microseconds / 3600000000.0)
}

/// Set the selected element for highlighting
#[frb(sync)]
pub fn set_selected_element(element_id: Option<i32>) -> Result<(), String> {
    let mut selected = SELECTED_ELEMENT.lock().unwrap();
    *selected = element_id;
    Ok(())
}

/// Reload model mesh with current visibility and highlight settings (primary model)
#[frb(sync)]
pub fn reload_model_mesh() -> Result<String, String> {
    let registry = MODEL_REGISTRY.lock().unwrap();
    let reg_model = registry.get_primary_model().ok_or("No model loaded")?;

    let visibility = VISIBILITY.lock().unwrap();
    let selected = SELECTED_ELEMENT.lock().unwrap();

    // Generate mesh with visibility filter and highlight
    let mesh = reg_model.model.generate_meshes_filtered(&visibility, *selected);
    let vertex_count = mesh.vertices.len() / 3;
    let triangle_count = mesh.indices.len() / 3;

    // Upload to renderer
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;

    r.load_mesh(&mesh.vertices, &mesh.normals, &mesh.colors, &mesh.indices)?;

    Ok(format!(
        "Mesh reloaded: {} vertices, {} triangles",
        vertex_count, triangle_count
    ))
}

/// Reload all visible models with current visibility and highlight settings
#[frb(sync)]
pub fn reload_all_models_mesh() -> Result<String, String> {
    let registry = MODEL_REGISTRY.lock().unwrap();

    if registry.is_empty() {
        return Err("No models loaded".to_string());
    }

    let visibility = VISIBILITY.lock().unwrap();
    let selected = SELECTED_ELEMENT.lock().unwrap();

    // Collect mesh data from all visible models
    let mut all_vertices = Vec::new();
    let mut all_normals = Vec::new();
    let mut all_colors = Vec::new();
    let mut all_indices = Vec::new();

    for (_id, reg_model) in registry.iter_visible() {
        let mesh = reg_model.model.generate_meshes_filtered(&visibility, *selected);

        // Offset indices by current vertex count
        let vertex_offset = (all_vertices.len() / 3) as u32;
        for idx in &mesh.indices {
            all_indices.push(idx + vertex_offset);
        }

        all_vertices.extend(&mesh.vertices);
        all_normals.extend(&mesh.normals);
        all_colors.extend(&mesh.colors);
    }

    let vertex_count = all_vertices.len() / 3;
    let triangle_count = all_indices.len() / 3;

    // Upload to renderer
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;

    r.load_mesh(&all_vertices, &all_normals, &all_colors, &all_indices)?;

    Ok(format!(
        "Reloaded {} models: {} vertices, {} triangles",
        registry.model_count(),
        vertex_count,
        triangle_count
    ))
}

// ============================================================================
// Phase 4 API: Materials & Lighting
// ============================================================================

/// Set the directional light direction (will be normalized)
/// Default is (0.5, 0.8, 0.3) - upper right front
#[frb(sync)]
pub fn set_light_direction(x: f32, y: f32, z: f32) -> Result<(), String> {
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;
    r.set_light_direction(x, y, z)
}

/// Set the directional light color (RGB, 0.0-1.0)
/// Default is warm white (1.0, 0.98, 0.95)
#[frb(sync)]
pub fn set_light_color(r: f32, g: f32, b: f32) -> Result<(), String> {
    let mut renderer = RENDERER.lock().unwrap();
    let renderer = renderer.as_mut().ok_or("Renderer not initialized")?;
    renderer.set_light_color(r, g, b)
}

/// Set the directional light intensity (0.0+)
/// Default is 1.0
#[frb(sync)]
pub fn set_light_intensity(intensity: f32) -> Result<(), String> {
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;
    r.set_light_intensity(intensity)
}

/// Set the ambient light color (RGB, 0.0-1.0)
/// Default is soft blue (0.15, 0.17, 0.2)
#[frb(sync)]
pub fn set_ambient_color(r: f32, g: f32, b: f32) -> Result<(), String> {
    let mut renderer = RENDERER.lock().unwrap();
    let renderer = renderer.as_mut().ok_or("Renderer not initialized")?;
    renderer.set_ambient_color(r, g, b)
}

/// Set the render mode
/// 0 = Shaded (default), 1 = Wireframe
#[frb(sync)]
pub fn set_render_mode(mode: i32) -> Result<(), String> {
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;
    let render_mode = match mode {
        0 => crate::renderer::RenderMode::Shaded,
        1 => crate::renderer::RenderMode::Wireframe,
        _ => return Err(format!("Invalid render mode: {}", mode)),
    };
    r.set_render_mode(render_mode)
}

/// Get the current render mode
/// Returns: 0 = Shaded, 1 = Wireframe
#[frb(sync)]
pub fn get_render_mode() -> Result<i32, String> {
    let renderer = RENDERER.lock().unwrap();
    let r = renderer.as_ref().ok_or("Renderer not initialized")?;
    Ok(match r.get_render_mode()? {
        crate::renderer::RenderMode::Shaded => 0,
        crate::renderer::RenderMode::Wireframe => 1,
    })
}

/// Check if wireframe rendering is supported on this device
#[frb(sync)]
pub fn is_wireframe_supported() -> bool {
    let renderer = RENDERER.lock().unwrap();
    renderer
        .as_ref()
        .map(|r| r.gpu.wireframe_supported())
        .unwrap_or(false)
}

// ============================================================================
// Phase 7: Measurements
// ============================================================================

/// Measurement type
#[derive(Debug, Clone)]
pub enum MeasurementType {
    Distance,
    Area,
    Volume,
}

/// Measurement point in 3D space
#[derive(Debug, Clone)]
pub struct MeasurementPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Measurement result
#[derive(Debug, Clone)]
pub struct MeasurementResult {
    pub measurement_type: String,
    pub value: f64,
    pub unit: String,
    pub points: Vec<MeasurementPoint>,
}

/// Global measurement state
static MEASUREMENT_POINTS: Mutex<Vec<MeasurementPoint>> = Mutex::new(Vec::new());
static MEASUREMENT_TYPE: Mutex<Option<MeasurementType>> = Mutex::new(None);

/// Start a new measurement
#[frb(sync)]
pub fn start_measurement(measurement_type: String) -> Result<(), String> {
    let mut points = MEASUREMENT_POINTS.lock().unwrap();
    let mut mtype = MEASUREMENT_TYPE.lock().unwrap();

    points.clear();
    *mtype = Some(match measurement_type.as_str() {
        "distance" => MeasurementType::Distance,
        "area" => MeasurementType::Area,
        "volume" => MeasurementType::Volume,
        _ => return Err(format!("Invalid measurement type: {}", measurement_type)),
    });

    Ok(())
}

/// Add a measurement point
/// Returns the current number of points
#[frb(sync)]
pub fn add_measurement_point(x: f32, y: f32, z: f32) -> Result<i32, String> {
    let mut points = MEASUREMENT_POINTS.lock().unwrap();
    points.push(MeasurementPoint { x, y, z });
    Ok(points.len() as i32)
}

/// Get the current measurement result
#[frb(sync)]
pub fn get_measurement_result() -> Result<MeasurementResult, String> {
    let points = MEASUREMENT_POINTS.lock().unwrap();
    let mtype = MEASUREMENT_TYPE.lock().unwrap();

    let measurement_type = mtype.as_ref().ok_or("No measurement in progress")?;

    match measurement_type {
        MeasurementType::Distance => {
            if points.len() < 2 {
                return Err("Need at least 2 points for distance measurement".to_string());
            }

            let mut total_distance = 0.0;
            for i in 0..points.len() - 1 {
                let p1 = &points[i];
                let p2 = &points[i + 1];
                let dx = p2.x - p1.x;
                let dy = p2.y - p1.y;
                let dz = p2.z - p1.z;
                total_distance += ((dx * dx + dy * dy + dz * dz) as f64).sqrt();
            }

            Ok(MeasurementResult {
                measurement_type: "distance".to_string(),
                value: total_distance,
                unit: "m".to_string(),
                points: points.clone(),
            })
        }
        MeasurementType::Area => {
            if points.len() < 3 {
                return Err("Need at least 3 points for area measurement".to_string());
            }

            // Calculate polygon area using shoelace formula (projected to XY plane)
            let mut area = 0.0;
            for i in 0..points.len() {
                let j = (i + 1) % points.len();
                area += (points[i].x * points[j].y - points[j].x * points[i].y) as f64;
            }
            area = (area / 2.0).abs();

            Ok(MeasurementResult {
                measurement_type: "area".to_string(),
                value: area,
                unit: "m²".to_string(),
                points: points.clone(),
            })
        }
        MeasurementType::Volume => {
            if points.len() < 4 {
                return Err("Need at least 4 points for volume measurement".to_string());
            }

            // Calculate bounding box volume
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            let mut min_z = f32::MAX;
            let mut max_z = f32::MIN;

            for p in points.iter() {
                min_x = min_x.min(p.x);
                max_x = max_x.max(p.x);
                min_y = min_y.min(p.y);
                max_y = max_y.max(p.y);
                min_z = min_z.min(p.z);
                max_z = max_z.max(p.z);
            }

            let width = (max_x - min_x) as f64;
            let depth = (max_y - min_y) as f64;
            let height = (max_z - min_z) as f64;
            let volume = width * depth * height;

            Ok(MeasurementResult {
                measurement_type: "volume".to_string(),
                value: volume,
                unit: "m³".to_string(),
                points: points.clone(),
            })
        }
    }
}

/// Clear the current measurement
#[frb(sync)]
pub fn clear_measurement() {
    let mut points = MEASUREMENT_POINTS.lock().unwrap();
    let mut mtype = MEASUREMENT_TYPE.lock().unwrap();
    points.clear();
    *mtype = None;
}

/// Get the number of measurement points
#[frb(sync)]
pub fn get_measurement_point_count() -> i32 {
    let points = MEASUREMENT_POINTS.lock().unwrap();
    points.len() as i32
}

// ============================================================================
// Phase 7: Section Planes
// ============================================================================

/// Section plane definition
#[derive(Debug, Clone)]
pub struct SectionPlane {
    /// Plane origin (point on plane)
    pub origin: [f32; 3],
    /// Plane normal vector (normalized)
    pub normal: [f32; 3],
    /// Whether the plane is enabled
    pub enabled: bool,
}

/// Global section plane state
static SECTION_PLANE: Mutex<Option<SectionPlane>> = Mutex::new(None);

/// Set the section plane
/// Origin: point on the plane
/// Normal: direction the plane faces (normalized)
#[frb(sync)]
pub fn set_section_plane(
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    normal_x: f32,
    normal_y: f32,
    normal_z: f32,
) -> Result<(), String> {
    // Normalize the normal vector
    let length = (normal_x * normal_x + normal_y * normal_y + normal_z * normal_z).sqrt();
    if length < 0.0001 {
        return Err("Normal vector cannot be zero".to_string());
    }

    let normalized_normal = [
        normal_x / length,
        normal_y / length,
        normal_z / length,
    ];

    let mut plane = SECTION_PLANE.lock().unwrap();
    *plane = Some(SectionPlane {
        origin: [origin_x, origin_y, origin_z],
        normal: normalized_normal,
        enabled: true,
    });

    // Update renderer if initialized
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(r) = renderer.as_mut() {
        r.set_section_plane(Some((
            [origin_x, origin_y, origin_z],
            normalized_normal,
        )))?;
    }

    Ok(())
}

/// Enable or disable the section plane
#[frb(sync)]
pub fn set_section_plane_enabled(enabled: bool) -> Result<(), String> {
    let mut plane = SECTION_PLANE.lock().unwrap();

    if let Some(ref mut p) = *plane {
        p.enabled = enabled;

        // Update renderer
        let mut renderer = RENDERER.lock().unwrap();
        if let Some(r) = renderer.as_mut() {
            if enabled {
                r.set_section_plane(Some((p.origin, p.normal)))?;
            } else {
                r.set_section_plane(None)?;
            }
        }
    } else {
        return Err("No section plane defined".to_string());
    }

    Ok(())
}

/// Clear the section plane
#[frb(sync)]
pub fn clear_section_plane() -> Result<(), String> {
    let mut plane = SECTION_PLANE.lock().unwrap();
    *plane = None;

    // Update renderer
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(r) = renderer.as_mut() {
        r.set_section_plane(None)?;
    }

    Ok(())
}

/// Check if section plane is active
#[frb(sync)]
pub fn is_section_plane_active() -> bool {
    let plane = SECTION_PLANE.lock().unwrap();
    plane.as_ref().map(|p| p.enabled).unwrap_or(false)
}

/// Set section plane from axis (X=0, Y=1, Z=2) and position
#[frb(sync)]
pub fn set_section_plane_from_axis(axis: i32, position: f32) -> Result<(), String> {
    let (normal_x, normal_y, normal_z) = match axis {
        0 => (1.0, 0.0, 0.0), // X axis
        1 => (0.0, 1.0, 0.0), // Y axis
        2 => (0.0, 0.0, 1.0), // Z axis
        _ => return Err(format!("Invalid axis: {}", axis)),
    };

    let (origin_x, origin_y, origin_z) = match axis {
        0 => (position, 0.0, 0.0),
        1 => (0.0, position, 0.0),
        2 => (0.0, 0.0, position),
        _ => unreachable!(),
    };

    set_section_plane(origin_x, origin_y, origin_z, normal_x, normal_y, normal_z)
}

// ============================================================================
// Phase 7: Color Coding by Properties
// ============================================================================

/// Set element color by ID
#[frb(sync)]
pub fn set_element_color(element_id: i32, r: u8, g: u8, b: u8) -> Result<(), String> {
    let mut renderer = RENDERER.lock().unwrap();
    let renderer_ref = renderer.as_mut().ok_or("Renderer not initialized")?;
    renderer_ref.set_element_color(
        element_id as usize,
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0
    )
}

/// Reset all element colors to defaults
#[frb(sync)]
pub fn reset_element_colors() -> Result<(), String> {
    let mut renderer = RENDERER.lock().unwrap();
    let r = renderer.as_mut().ok_or("Renderer not initialized")?;
    r.reset_element_colors()
}

/// Color elements by type
/// Automatically assigns different colors to different element types
#[frb(sync)]
pub fn color_by_type() -> Result<(), String> {
    let model = MODEL_REGISTRY.lock().unwrap();
    if model.is_empty() {
        return Err("No model loaded".to_string());
    }

    // Predefined color palette for different types (for future implementation)
    let _type_colors: std::collections::HashMap<&str, [u8; 3]> = [
        ("IfcWall", [200, 200, 200]),          // Light gray
        ("IfcSlab", [150, 150, 150]),          // Medium gray
        ("IfcBeam", [139, 69, 19]),            // Brown
        ("IfcColumn", [160, 82, 45]),          // Sienna
        ("IfcDoor", [210, 180, 140]),          // Tan
        ("IfcWindow", [173, 216, 230]),        // Light blue
        ("IfcStair", [188, 143, 143]),         // Rosy brown
        ("IfcRoof", [178, 34, 34]),            // Firebrick
        ("IfcSpace", [240, 255, 240]),         // Honeydew
        ("IfcBuildingElementProxy", [192, 192, 192]), // Silver
    ].iter().cloned().collect();

    let mut renderer = RENDERER.lock().unwrap();
    let _r = renderer.as_mut().ok_or("Renderer not initialized")?;

    // TODO: Implement per-element coloring by iterating over all element types
    // For now, just log that the operation would be applied
    let total_elements: usize = model.models().values().map(|reg| reg.model.element_count).sum();

    tracing::info!("Color-by-type requested for {} total elements (stub implementation)", total_elements);
    Ok(())
}

// ============================================================================
// Phase 8: Export & Settings
// ============================================================================

/// Save current frame as PNG to the given path
pub async fn export_screenshot(path: String) -> Result<(), String> {
    let renderer = RENDERER.lock().unwrap();
    let r = renderer.as_ref().ok_or("Renderer not initialized")?;

    // Render current frame and get image data
    let image_data = r.render_frame()?;

    // Save to file
    match image::save_buffer(
        &path,
        &image_data,
        r.scene.as_ref().ok_or("Scene not initialized")?.width,
        r.scene.as_ref().ok_or("Scene not initialized")?.height,
        image::ColorType::Rgba8,
    ) {
        Ok(_) => {
            tracing::info!("Screenshot saved to: {}", path);
            Ok(())
        }
        Err(e) => Err(format!("Failed to save screenshot: {}", e)),
    }
}

/// Get current frame as RGBA bytes
/// Returns width, height, and pixel data
#[frb(sync)]
pub fn get_current_frame_rgba() -> Result<Vec<u8>, String> {
    let renderer = RENDERER.lock().unwrap();
    let r = renderer.as_ref().ok_or("Renderer not initialized")?;
    r.render_frame()
}

/// Get renderer statistics
#[derive(Debug, Clone)]
pub struct RenderStats {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub triangle_count: u32,
    pub vertex_count: u32,
    pub element_count: u32,
}

#[frb(sync)]
pub fn get_render_stats() -> Result<RenderStats, String> {
    let renderer = RENDERER.lock().unwrap();
    let r = renderer.as_ref().ok_or("Renderer not initialized")?;

    let model = MODEL_REGISTRY.lock().unwrap();
    let element_count = model.models().values().map(|reg| reg.model.element_count).sum::<usize>();

    Ok(RenderStats {
        fps: 60.0, // Placeholder - would need frame timing tracking
        frame_time_ms: 16.67,
        triangle_count: r.scene.as_ref().map(|s| s.num_indices / 3).unwrap_or(0),
        vertex_count: 0, // Would need to track this
        element_count: element_count as u32,
    })
}

// ============================================================================
// Phase 6/7: 2D Drawing Overlay
// ============================================================================

/// View mode for the renderer
#[derive(Debug, Clone)]
pub enum ViewMode {
    /// Normal 3D view
    ThreeD,
    /// 2D overlay view (looking straight down)
    TwoD,
    /// 3D model with overlay visible
    Overlay,
}

/// Current view mode
static VIEW_MODE: Mutex<ViewMode> = Mutex::new(ViewMode::ThreeD);

/// Upload a 2D drawing/floor plan as an overlay texture
/// id: Unique identifier for this overlay
/// width, height: Image dimensions
/// rgba_pixels: RGBA pixel data (width * height * 4 bytes)
pub async fn upload_drawing_overlay(
    id: String,
    width: u32,
    height: u32,
    _rgba_pixels: Vec<u8>,
) -> Result<(), String> {
    tracing::info!("Uploading drawing overlay: {} ({}x{})", id, width, height);

    // TODO: Store overlay in renderer
    // This would require extending the Renderer struct to manage overlays
    // For now, return success to generate the API binding

    Ok(())
}

/// Set overlay transform (position, scale, rotation)
#[frb(sync)]
pub fn set_overlay_transform(
    id: String,
    _position_x: f32,
    _position_y: f32,
    _position_z: f32,
    _scale_x: f32,
    _scale_y: f32,
    _rotation: f32,
) -> Result<(), String> {
    tracing::info!("Set overlay transform: {}", id);
    // TODO: Update overlay transform
    Ok(())
}

/// Set overlay opacity (0.0 to 1.0)
#[frb(sync)]
pub fn set_overlay_opacity(id: String, opacity: f32) -> Result<(), String> {
    let opacity = opacity.clamp(0.0, 1.0);
    tracing::info!("Set overlay opacity: {} = {}", id, opacity);
    // TODO: Update overlay opacity
    Ok(())
}

/// Set overlay visibility
#[frb(sync)]
pub fn set_overlay_visible(id: String, visible: bool) -> Result<(), String> {
    tracing::info!("Set overlay visible: {} = {}", id, visible);
    // TODO: Update overlay visibility
    Ok(())
}

/// Remove an overlay
#[frb(sync)]
pub fn remove_overlay(id: String) -> Result<(), String> {
    tracing::info!("Remove overlay: {}", id);
    // TODO: Remove overlay from renderer
    Ok(())
}

/// Set view mode
#[frb(sync)]
pub fn set_view_mode(mode: String) -> Result<(), String> {
    let mut view_mode = VIEW_MODE.lock().unwrap();
    *view_mode = match mode.as_str() {
        "3d" => ViewMode::ThreeD,
        "2d" => ViewMode::TwoD,
        "overlay" => ViewMode::Overlay,
        _ => return Err(format!("Invalid view mode: {}", mode)),
    };

    tracing::info!("Set view mode: {}", mode);

    // TODO: Update camera position based on view mode
    // For 2D mode: position camera looking straight down
    // For overlay mode: keep current 3D view but show overlay

    Ok(())
}

/// Get current view mode
#[frb(sync)]
pub fn get_view_mode() -> String {
    let view_mode = VIEW_MODE.lock().unwrap();
    match *view_mode {
        ViewMode::ThreeD => "3d".to_string(),
        ViewMode::TwoD => "2d".to_string(),
        ViewMode::Overlay => "overlay".to_string(),
    }
}

// ============================================================================
// Future Phases
// ============================================================================
