use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use flutter_rust_bridge::frb;

use crate::bim::{BimModel, IfcFile, ModelInfo, ModelStats, RegisteredModelInfo};
use crate::bim::ifc_parser::detect_ifc_schema;
use crate::bim::obj_import;
use crate::bim::gltf_import;
use super::state::with_state;
use super::system::update_memory_stats;

// ========================================================================
// Async Loading State Management (Task 3)
// ========================================================================

/// State of an asynchronous loading task.
#[derive(Debug, Clone)]
pub enum LoadingState {
    Reading,
    Parsing,
    Tessellating,
    Uploading,
    Complete(ModelInfo),
    Failed(String),
}

impl LoadingState {
    /// Serialize the loading state to JSON.
    fn to_json(&self) -> String {
        match self {
            LoadingState::Reading => {
                serde_json::json!({"state": "reading"}).to_string()
            }
            LoadingState::Parsing => {
                serde_json::json!({"state": "parsing"}).to_string()
            }
            LoadingState::Tessellating => {
                serde_json::json!({"state": "tessellating"}).to_string()
            }
            LoadingState::Uploading => {
                serde_json::json!({"state": "uploading"}).to_string()
            }
            LoadingState::Complete(info) => {
                serde_json::json!({
                    "state": "complete",
                    "model_info": {
                        "project_name": info.project_name,
                        "building_name": info.building_name,
                        "site_name": info.site_name,
                        "stats": {
                            "total_entities": info.stats.total_entities,
                            "walls": info.stats.walls,
                            "slabs": info.stats.slabs,
                            "columns": info.stats.columns,
                            "beams": info.stats.beams,
                            "doors": info.stats.doors,
                            "windows": info.stats.windows,
                            "storeys": info.stats.storeys,
                        }
                    }
                })
                .to_string()
            }
            LoadingState::Failed(err) => {
                serde_json::json!({"state": "failed", "error": err}).to_string()
            }
        }
    }
}

static LOADING_TASKS: LazyLock<Mutex<HashMap<String, LoadingState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Counter for generating unique loading IDs.
static LOADING_ID_COUNTER: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(0));

/// Generate a unique loading task ID.
fn next_loading_id() -> String {
    let mut counter = LOADING_ID_COUNTER.lock().unwrap();
    *counter += 1;
    format!("load_{}", *counter)
}

/// Update the state of a loading task.
fn set_loading_state(id: &str, state: LoadingState) {
    if let Ok(mut tasks) = LOADING_TASKS.lock() {
        tasks.insert(id.to_string(), state);
    }
}

/// Check the status of an async loading task.
/// Returns JSON describing the current state.
#[frb(sync)]
pub fn get_loading_state(loading_id: String) -> String {
    let tasks = LOADING_TASKS.lock().unwrap();
    match tasks.get(&loading_id) {
        Some(state) => state.to_json(),
        None => serde_json::json!({"state": "not_found"}).to_string(),
    }
}

/// Cancel an async loading task (if possible).
/// Currently marks the task as failed; actual cancellation of in-flight I/O
/// is not supported by the synchronous parsing pipeline.
#[frb(sync)]
pub fn cancel_loading(loading_id: String) -> Result<(), String> {
    let mut tasks = LOADING_TASKS.lock().unwrap();
    if tasks.contains_key(&loading_id) {
        tasks.insert(
            loading_id,
            LoadingState::Failed("Cancelled by user".to_string()),
        );
        Ok(())
    } else {
        Err(format!("Loading task '{}' not found", loading_id))
    }
}

/// List all active loading tasks as JSON array.
#[frb(sync)]
pub fn list_loading_tasks() -> String {
    let tasks = LOADING_TASKS.lock().unwrap();
    let entries: Vec<serde_json::Value> = tasks
        .iter()
        .map(|(id, state)| {
            let state_str = match state {
                LoadingState::Reading => "reading",
                LoadingState::Parsing => "parsing",
                LoadingState::Tessellating => "tessellating",
                LoadingState::Uploading => "uploading",
                LoadingState::Complete(_) => "complete",
                LoadingState::Failed(_) => "failed",
            };
            serde_json::json!({
                "id": id,
                "state": state_str,
            })
        })
        .collect();
    serde_json::Value::Array(entries).to_string()
}

// ========================================================================
// Model Loading
// ========================================================================

/// Load an IFC file and parse it (backward compatible - loads as primary).
/// Updates loading task state as it progresses.
pub async fn load_ifc_file(file_path: String) -> Result<ModelInfo, String> {
    tracing::info!("Loading IFC file: {}", file_path);

    let loading_id = next_loading_id();
    set_loading_state(&loading_id, LoadingState::Reading);

    let model = {
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| {
                set_loading_state(&loading_id, LoadingState::Failed(format!("Read failed: {}", e)));
                format!("Failed to read file: {}", e)
            })?;

        set_loading_state(&loading_id, LoadingState::Parsing);
        let ifc_file = IfcFile::parse(&content).map_err(|e| {
            set_loading_state(&loading_id, LoadingState::Failed(format!("Parse failed: {}", e)));
            e
        })?;

        tracing::info!("Parsed IFC file: {} entities", ifc_file.entity_count());

        set_loading_state(&loading_id, LoadingState::Tessellating);
        BimModel::from_ifc_file(&ifc_file).map_err(|e| {
            set_loading_state(&loading_id, LoadingState::Failed(format!("Model creation failed: {}", e)));
            e
        })?
    };

    set_loading_state(&loading_id, LoadingState::Uploading);

    let model_info = model.get_info();

    let name = std::path::Path::new(&file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    with_state(|s| {
        s.registry.add_model(model, name, Some(file_path));
        s.mark_bvh_dirty();
    });

    update_memory_stats();

    set_loading_state(&loading_id, LoadingState::Complete(model_info.clone()));
    tracing::info!("Model loaded successfully (task {})", loading_id);
    Ok(model_info)
}

/// Start loading an IFC file in the background.
/// Returns a loading ID that can be used to check progress via `get_loading_state`.
pub async fn load_ifc_file_async(file_path: String) -> Result<String, String> {
    let loading_id = next_loading_id();
    set_loading_state(&loading_id, LoadingState::Reading);

    let id = loading_id.clone();

    tokio::spawn(async move {
        // Read file
        let content = match tokio::fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) => {
                set_loading_state(&id, LoadingState::Failed(format!("Failed to read file: {}", e)));
                return;
            }
        };

        // Parse
        set_loading_state(&id, LoadingState::Parsing);
        let ifc_file = match IfcFile::parse(&content) {
            Ok(f) => f,
            Err(e) => {
                set_loading_state(&id, LoadingState::Failed(format!("Parse error: {}", e)));
                return;
            }
        };
        drop(content);

        tracing::info!("Async: Parsed IFC file: {} entities", ifc_file.entity_count());

        // Tessellate
        set_loading_state(&id, LoadingState::Tessellating);
        let model = match BimModel::from_ifc_file(&ifc_file) {
            Ok(m) => m,
            Err(e) => {
                set_loading_state(&id, LoadingState::Failed(format!("Model error: {}", e)));
                return;
            }
        };

        set_loading_state(&id, LoadingState::Uploading);

        let model_info = model.get_info();

        let name = std::path::Path::new(&file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();

        with_state(|s| {
            s.registry.add_model(model, name, Some(file_path));
            s.mark_bvh_dirty();
        });

        update_memory_stats();

        set_loading_state(&id, LoadingState::Complete(model_info));
        tracing::info!("Async model load complete (task {})", id);
    });

    Ok(loading_id)
}

// ========================================================================
// Existing Model API
// ========================================================================

/// Get information about the currently loaded model (primary model)
#[frb(sync)]
pub fn get_model_info() -> Result<ModelInfo, String> {
    with_state(|s| {
        s.registry
            .get_primary_model()
            .map(|m| m.model.get_info())
            .ok_or_else(|| "No model loaded".to_string())
    })
}

/// Check if a model is currently loaded
#[frb(sync)]
pub fn is_model_loaded() -> bool {
    with_state(|s| !s.registry.is_empty())
}

/// Unload the current model and free memory (primary model)
#[frb(sync)]
pub fn unload_model() -> Result<(), String> {
    with_state(|s| {
        if s.registry.is_empty() {
            return Err("No model loaded".to_string());
        }
        if let Some(id) = s.registry.get_primary_model_id().cloned() {
            s.registry.remove_model(&id);
            s.mark_bvh_dirty();
            tracing::info!("Model unloaded");
            Ok(())
        } else {
            Err("No primary model to unload".to_string())
        }
    })?;
    update_memory_stats();
    Ok(())
}

/// Parse IFC file content (for testing - takes content string instead of file path)
pub async fn parse_ifc_content(content: String) -> Result<ModelInfo, String> {
    tracing::info!("Parsing IFC content ({} bytes)", content.len());

    let model = {
        let ifc_file = IfcFile::parse(&content)?;

        tracing::info!("Parsed IFC file: {} entities", ifc_file.entity_count());

        BimModel::from_ifc_file(&ifc_file)?
    };
    drop(content);

    let model_info = model.get_info();

    with_state(|s| {
        s.registry
            .add_model(model, "Parsed Model".to_string(), None);
        s.mark_bvh_dirty();
    });

    update_memory_stats();

    Ok(model_info)
}

/// Load a model with a specific ID
pub async fn load_model(model_id: String, file_path: String) -> Result<ModelInfo, String> {
    tracing::info!("Loading model '{}' from: {}", model_id, file_path);

    let model = {
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let ifc_file = IfcFile::parse(&content)?;

        BimModel::from_ifc_file(&ifc_file)?
    };

    let model_info = model.get_info();

    let name = std::path::Path::new(&file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    with_state(|s| {
        s.registry
            .add_model_with_id(model_id.clone(), model, name, Some(file_path));
        s.mark_bvh_dirty();
    });

    update_memory_stats();

    tracing::info!("Model '{}' loaded successfully", model_id);
    Ok(model_info)
}

/// Unload a specific model by ID
#[frb(sync)]
pub fn unload_model_by_id(model_id: String) -> Result<(), String> {
    with_state(|s| {
        if s.registry.remove_model(&model_id).is_some() {
            s.mark_bvh_dirty();
            tracing::info!("Model '{}' unloaded", model_id);
            Ok(())
        } else {
            Err(format!("Model '{}' not found", model_id))
        }
    })?;
    update_memory_stats();
    Ok(())
}

/// List all loaded models
#[frb(sync)]
pub fn list_loaded_models() -> Vec<RegisteredModelInfo> {
    with_state(|s| s.registry.get_all_model_info())
}

/// Get number of loaded models
#[frb(sync)]
pub fn get_model_count() -> usize {
    with_state(|s| s.registry.model_count())
}

/// Set model visibility
#[frb(sync)]
pub fn set_model_visible(model_id: String, visible: bool) -> Result<(), String> {
    with_state(|s| s.registry.set_model_visible(&model_id, visible))
}

/// Set the primary model
#[frb(sync)]
pub fn set_primary_model(model_id: String) -> Result<(), String> {
    with_state(|s| s.registry.set_primary_model(&model_id))
}

/// Clear all models
#[frb(sync)]
pub fn clear_all_models() {
    with_state(|s| {
        s.registry.clear();
        s.mark_bvh_dirty();
        tracing::info!("All models cleared");
    });
    update_memory_stats();
}

/// Import an OBJ file and register it as a model.
pub async fn import_obj_file(file_path: String) -> Result<ModelInfo, String> {
    tracing::info!("Importing OBJ file: {}", file_path);

    let content = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("Failed to read OBJ file: {}", e))?;

    let obj_mesh = obj_import::parse_obj(&content)?;
    drop(content);

    let name = std::path::Path::new(&file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("OBJ Import")
        .to_string();

    let element_count = obj_mesh.objects.len().max(1);

    let model_info = ModelInfo {
        project_name: name.clone(),
        building_name: "OBJ Import".to_string(),
        site_name: "Imported".to_string(),
        stats: ModelStats {
            total_entities: element_count,
            walls: 0,
            slabs: 0,
            columns: 0,
            beams: 0,
            doors: 0,
            windows: 0,
            storeys: 0,
        },
    };

    // Create a BimModel with the imported mesh data stored in entity_map = None
    // and proxy elements for each OBJ object
    let mut model = BimModel::new();
    model.element_count = element_count;

    // Add proxy elements for each OBJ object so generate_meshes produces output
    use crate::bim::entities::{IfcProduct, IfcBuildingElementProxy};
    for (i, obj) in obj_mesh.objects.iter().enumerate() {
        model.proxies.push(IfcBuildingElementProxy {
            product: IfcProduct {
                id: i as i32 + 1,
                global_id: format!("obj_{}", i),
                name: Some(obj.name.clone()),
                description: None,
                object_type: Some("OBJ Import".to_string()),
                properties: None,
            },
            predefined_type: None,
        });
    }

    // Register in model registry - the RegisteredModel::new will call generate_meshes,
    // but since we have no entity_map, it'll produce placeholder boxes.
    // Instead, we need to store the OBJ mesh data directly.
    // Let's create a custom RegisteredModel with pre-built mesh data.
    use crate::bim::model::{ModelMesh, ElementInfo};
    use crate::bim::geometry::BoundingBox;
    use crate::bim::model_registry::RegisteredModel;

    // Generate colors (default gray for OBJ)
    let vertex_count = obj_mesh.vertices.len() / 3;
    let colors: Vec<f32> = (0..vertex_count)
        .flat_map(|_| [0.7f32, 0.7, 0.7, 1.0])
        .collect();

    let bounds = if obj_mesh.vertices.is_empty() {
        None
    } else {
        Some(BoundingBox {
            min: obj_mesh.bounds_min,
            max: obj_mesh.bounds_max,
        })
    };

    let elements: Vec<ElementInfo> = obj_mesh
        .objects
        .iter()
        .enumerate()
        .map(|(i, obj)| {
            let tri_start = obj.index_start / 3;
            let tri_count = obj.index_count / 3;
            ElementInfo {
                id: i as i32 + 1,
                element_type: "Proxy".to_string(),
                name: obj.name.clone(),
                global_id: format!("obj_{}", i),
                bounds: bounds.unwrap_or(BoundingBox {
                    min: [0.0; 3],
                    max: [0.1; 3],
                }),
                triangle_start: tri_start,
                triangle_count: tri_count,
            }
        })
        .collect();

    let mesh = ModelMesh {
        vertices: obj_mesh.vertices,
        indices: obj_mesh.indices,
        normals: obj_mesh.normals,
        colors,
        bounds,
        elements: elements.clone(),
    };

    // Directly construct RegisteredModel with cached mesh
    let registered = RegisteredModel {
        model,
        name: name.clone(),
        file_path: Some(file_path),
        visible: true,
        transform: [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ],
        bounds,
        cached_elements: elements,
        cached_mesh: Some(mesh),
        cache_path: None,
    };

    with_state(|s| {
        let id = format!("model_{}", s.registry.model_count() + 1);
        let model_placeholder = BimModel::new();
        let added_id = s.registry.add_model_with_id(id, model_placeholder, name.clone(), None);
        if let Some(reg) = s.registry.get_model_mut(&added_id) {
            *reg = registered;
        }
        s.mark_bvh_dirty();
    });

    update_memory_stats();

    tracing::info!("OBJ file imported successfully");
    Ok(model_info)
}

/// Import a glTF JSON file and register it as a model.
pub async fn import_gltf_file(file_path: String) -> Result<ModelInfo, String> {
    tracing::info!("Importing glTF file: {}", file_path);

    let content = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("Failed to read glTF file: {}", e))?;

    let gltf_mesh = gltf_import::parse_gltf(&content, None)?;
    drop(content);

    build_model_from_gltf(gltf_mesh, &file_path)
}

/// Import a GLB binary file and register it as a model.
pub async fn import_glb_file(file_path: String) -> Result<ModelInfo, String> {
    tracing::info!("Importing GLB file: {}", file_path);

    let data = tokio::fs::read(&file_path)
        .await
        .map_err(|e| format!("Failed to read GLB file: {}", e))?;

    let gltf_mesh = gltf_import::parse_glb(&data)?;
    drop(data);

    build_model_from_gltf(gltf_mesh, &file_path)
}

/// Detect the IFC schema version of a file from its content.
#[frb(sync)]
pub fn detect_file_schema(file_content: String) -> String {
    let version = detect_ifc_schema(&file_content);
    match version {
        crate::bim::entities::IfcSchemaVersion::Ifc2x3 => "IFC2X3".to_string(),
        crate::bim::entities::IfcSchemaVersion::Ifc4 => "IFC4".to_string(),
        crate::bim::entities::IfcSchemaVersion::Ifc4x3 => "IFC4X3".to_string(),
        crate::bim::entities::IfcSchemaVersion::Unknown(s) => {
            if s.is_empty() {
                "UNKNOWN".to_string()
            } else {
                format!("UNKNOWN({})", s)
            }
        }
    }
}

/// Helper: build a model from parsed glTF mesh data and register it.
fn build_model_from_gltf(
    gltf_mesh: gltf_import::GltfMesh,
    file_path: &str,
) -> Result<ModelInfo, String> {
    use crate::bim::model::{ModelMesh, ElementInfo};
    use crate::bim::geometry::BoundingBox;
    use crate::bim::model_registry::RegisteredModel;
    use crate::bim::entities::{IfcProduct, IfcBuildingElementProxy};

    let name = std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("glTF Import")
        .to_string();

    let element_count = gltf_mesh.nodes.len().max(1);

    let model_info = ModelInfo {
        project_name: name.clone(),
        building_name: "glTF Import".to_string(),
        site_name: "Imported".to_string(),
        stats: ModelStats {
            total_entities: element_count,
            walls: 0,
            slabs: 0,
            columns: 0,
            beams: 0,
            doors: 0,
            windows: 0,
            storeys: 0,
        },
    };

    let bounds = if gltf_mesh.vertices.is_empty() {
        None
    } else {
        Some(BoundingBox {
            min: gltf_mesh.bounds_min,
            max: gltf_mesh.bounds_max,
        })
    };

    let mut model = BimModel::new();
    model.element_count = element_count;

    for (i, node) in gltf_mesh.nodes.iter().enumerate() {
        model.proxies.push(IfcBuildingElementProxy {
            product: IfcProduct {
                id: i as i32 + 1,
                global_id: format!("gltf_{}", i),
                name: Some(node.name.clone()),
                description: None,
                object_type: Some("glTF Import".to_string()),
                properties: None,
            },
            predefined_type: None,
        });
    }

    let elements: Vec<ElementInfo> = gltf_mesh
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let tri_start = node.index_start / 3;
            let tri_count = node.index_count / 3;
            ElementInfo {
                id: i as i32 + 1,
                element_type: "Proxy".to_string(),
                name: node.name.clone(),
                global_id: format!("gltf_{}", i),
                bounds: bounds.unwrap_or(BoundingBox {
                    min: [0.0; 3],
                    max: [0.1; 3],
                }),
                triangle_start: tri_start,
                triangle_count: tri_count,
            }
        })
        .collect();

    let mesh = ModelMesh {
        vertices: gltf_mesh.vertices,
        indices: gltf_mesh.indices,
        normals: gltf_mesh.normals,
        colors: gltf_mesh.colors,
        bounds,
        elements: elements.clone(),
    };

    let registered = RegisteredModel {
        model,
        name: name.clone(),
        file_path: Some(file_path.to_string()),
        visible: true,
        transform: [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ],
        bounds,
        cached_elements: elements,
        cached_mesh: Some(mesh),
        cache_path: None,
    };

    with_state(|s| {
        let id = format!("model_{}", s.registry.model_count() + 1);
        let model_placeholder = BimModel::new();
        let added_id = s.registry.add_model_with_id(id, model_placeholder, name.clone(), None);
        if let Some(reg) = s.registry.get_model_mut(&added_id) {
            *reg = registered;
        }
        s.mark_bvh_dirty();
    });

    update_memory_stats();

    tracing::info!("glTF file imported successfully");
    Ok(model_info)
}

// ========================================================================
// Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading_state_lifecycle() {
        let id = next_loading_id();

        // Initially not found
        let state = get_loading_state(id.clone());
        assert!(state.contains("not_found"), "Expected not_found, got: {}", state);

        // Set to Reading
        set_loading_state(&id, LoadingState::Reading);
        let state = get_loading_state(id.clone());
        assert!(state.contains("reading"), "Expected reading, got: {}", state);

        // Set to Parsing
        set_loading_state(&id, LoadingState::Parsing);
        let state = get_loading_state(id.clone());
        assert!(state.contains("parsing"), "Expected parsing, got: {}", state);

        // Set to Tessellating
        set_loading_state(&id, LoadingState::Tessellating);
        let state = get_loading_state(id.clone());
        assert!(state.contains("tessellating"), "Expected tessellating, got: {}", state);

        // Set to Uploading
        set_loading_state(&id, LoadingState::Uploading);
        let state = get_loading_state(id.clone());
        assert!(state.contains("uploading"), "Expected uploading, got: {}", state);

        // Set to Complete
        let info = ModelInfo {
            project_name: "Test".to_string(),
            building_name: "Building".to_string(),
            site_name: "Site".to_string(),
            stats: ModelStats {
                total_entities: 10,
                walls: 2,
                slabs: 1,
                columns: 3,
                beams: 0,
                doors: 1,
                windows: 2,
                storeys: 1,
            },
        };
        set_loading_state(&id, LoadingState::Complete(info));
        let state = get_loading_state(id.clone());
        assert!(state.contains("complete"), "Expected complete, got: {}", state);
        assert!(state.contains("Test"), "Expected project name in: {}", state);

        // Set to Failed
        set_loading_state(&id, LoadingState::Failed("test error".to_string()));
        let state = get_loading_state(id.clone());
        assert!(state.contains("failed"), "Expected failed, got: {}", state);
        assert!(state.contains("test error"), "Expected error msg in: {}", state);
    }

    #[test]
    fn test_cancel_loading() {
        let id = next_loading_id();
        set_loading_state(&id, LoadingState::Reading);

        // Cancel should succeed
        cancel_loading(id.clone()).expect("Cancel should succeed");
        let state = get_loading_state(id.clone());
        assert!(state.contains("failed"), "Expected failed after cancel, got: {}", state);
        assert!(state.contains("Cancelled"), "Expected cancellation msg in: {}", state);

        // Cancel non-existent should fail
        let result = cancel_loading("nonexistent_task".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_list_loading_tasks() {
        // Create a couple of tasks
        let id1 = next_loading_id();
        let id2 = next_loading_id();
        set_loading_state(&id1, LoadingState::Parsing);
        set_loading_state(&id2, LoadingState::Tessellating);

        let json_str = list_loading_tasks();
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("Should be valid JSON array");
        assert!(parsed.is_array(), "Expected JSON array");

        // Clean up
        let mut tasks = LOADING_TASKS.lock().unwrap();
        tasks.remove(&id1);
        tasks.remove(&id2);
    }

    #[test]
    fn test_loading_state_json_format() {
        // Test each state variant produces valid JSON
        let states = vec![
            LoadingState::Reading,
            LoadingState::Parsing,
            LoadingState::Tessellating,
            LoadingState::Uploading,
            LoadingState::Failed("some error".to_string()),
            LoadingState::Complete(ModelInfo {
                project_name: "P".to_string(),
                building_name: "B".to_string(),
                site_name: "S".to_string(),
                stats: ModelStats {
                    total_entities: 0,
                    walls: 0,
                    slabs: 0,
                    columns: 0,
                    beams: 0,
                    doors: 0,
                    windows: 0,
                    storeys: 0,
                },
            }),
        ];

        for state in states {
            let json = state.to_json();
            let parsed: serde_json::Value =
                serde_json::from_str(&json).expect("Each state should produce valid JSON");
            assert!(parsed.get("state").is_some(), "Missing 'state' key in: {}", json);
        }
    }
}
