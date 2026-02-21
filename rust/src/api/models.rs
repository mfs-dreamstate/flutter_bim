use flutter_rust_bridge::frb;

use crate::bim::{BimModel, IfcFile, ModelInfo, RegisteredModelInfo};
use super::state::with_state;

/// Load an IFC file and parse it (backward compatible - loads as primary)
pub async fn load_ifc_file(file_path: String) -> Result<ModelInfo, String> {
    tracing::info!("Loading IFC file: {}", file_path);

    let model = {
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let ifc_file = IfcFile::parse(&content)?;

        tracing::info!("Parsed IFC file: {} entities", ifc_file.entity_count());

        BimModel::from_ifc_file(&ifc_file)?
    };

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

    tracing::info!("Model loaded successfully");
    Ok(model_info)
}

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
    })
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
    })
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
}
