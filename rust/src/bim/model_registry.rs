//! Model Registry - Multi-Model Management
//!
//! Manages multiple BIM models for federated model support.
//! Enables loading, unloading, and visibility control of multiple IFC files.

use super::model::{BimModel, ModelInfo};
use super::geometry::BoundingBox;
use std::collections::HashMap;

/// Unique identifier for a loaded model
pub type ModelId = String;

/// Information about a loaded model in the registry
#[derive(Debug, Clone)]
pub struct RegisteredModel {
    /// The model data
    pub model: BimModel,
    /// User-friendly display name
    pub name: String,
    /// Source file path (if loaded from file)
    pub file_path: Option<String>,
    /// Whether this model is visible
    pub visible: bool,
    /// Transform matrix (4x4, column-major) for model positioning
    pub transform: [f32; 16],
    /// Cached bounding box
    pub bounds: Option<BoundingBox>,
}

impl RegisteredModel {
    /// Create a new registered model with default settings
    pub fn new(model: BimModel, name: String, file_path: Option<String>) -> Self {
        Self {
            model,
            name,
            file_path,
            visible: true,
            transform: Self::identity_matrix(),
            bounds: None,
        }
    }

    /// Identity transform matrix
    fn identity_matrix() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]
    }
}

/// Registry for managing multiple BIM models
#[derive(Debug, Default)]
pub struct ModelRegistry {
    /// Map of model ID to registered model
    models: HashMap<ModelId, RegisteredModel>,
    /// The primary/active model (for operations that need a default)
    primary_model: Option<ModelId>,
    /// Counter for generating unique IDs
    next_id: u32,
}

impl ModelRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            primary_model: None,
            next_id: 1,
        }
    }

    /// Generate a unique model ID
    fn generate_id(&mut self) -> ModelId {
        let id = format!("model_{}", self.next_id);
        self.next_id += 1;
        id
    }

    /// Add a model to the registry
    /// Returns the assigned model ID
    pub fn add_model(&mut self, model: BimModel, name: String, file_path: Option<String>) -> ModelId {
        let id = self.generate_id();
        let registered = RegisteredModel::new(model, name, file_path);

        // If this is the first model, make it primary
        if self.models.is_empty() {
            self.primary_model = Some(id.clone());
        }

        self.models.insert(id.clone(), registered);
        id
    }

    /// Add a model with a specific ID (for backward compatibility)
    pub fn add_model_with_id(&mut self, id: ModelId, model: BimModel, name: String, file_path: Option<String>) -> ModelId {
        let registered = RegisteredModel::new(model, name, file_path);

        // If this is the first model, make it primary
        if self.models.is_empty() {
            self.primary_model = Some(id.clone());
        }

        self.models.insert(id.clone(), registered);
        id
    }

    /// Remove a model from the registry
    pub fn remove_model(&mut self, id: &ModelId) -> Option<RegisteredModel> {
        let removed = self.models.remove(id);

        // If we removed the primary model, assign a new one
        if self.primary_model.as_ref() == Some(id) {
            self.primary_model = self.models.keys().next().cloned();
        }

        removed
    }

    /// Get a reference to a model
    pub fn get_model(&self, id: &ModelId) -> Option<&RegisteredModel> {
        self.models.get(id)
    }

    /// Get a mutable reference to a model
    pub fn get_model_mut(&mut self, id: &ModelId) -> Option<&mut RegisteredModel> {
        self.models.get_mut(id)
    }

    /// Get the primary model
    pub fn get_primary_model(&self) -> Option<&RegisteredModel> {
        self.primary_model.as_ref().and_then(|id| self.models.get(id))
    }

    /// Get the primary model mutably
    pub fn get_primary_model_mut(&mut self) -> Option<&mut RegisteredModel> {
        if let Some(id) = self.primary_model.clone() {
            self.models.get_mut(&id)
        } else {
            None
        }
    }

    /// Get the primary model ID
    pub fn get_primary_model_id(&self) -> Option<&ModelId> {
        self.primary_model.as_ref()
    }

    /// Set the primary model
    pub fn set_primary_model(&mut self, id: &ModelId) -> Result<(), String> {
        if self.models.contains_key(id) {
            self.primary_model = Some(id.clone());
            Ok(())
        } else {
            Err(format!("Model '{}' not found", id))
        }
    }

    /// Set model visibility
    pub fn set_model_visible(&mut self, id: &ModelId, visible: bool) -> Result<(), String> {
        match self.models.get_mut(id) {
            Some(model) => {
                model.visible = visible;
                Ok(())
            }
            None => Err(format!("Model '{}' not found", id)),
        }
    }

    /// Get model visibility
    pub fn is_model_visible(&self, id: &ModelId) -> Option<bool> {
        self.models.get(id).map(|m| m.visible)
    }

    /// Set model transform
    pub fn set_model_transform(&mut self, id: &ModelId, transform: [f32; 16]) -> Result<(), String> {
        match self.models.get_mut(id) {
            Some(model) => {
                model.transform = transform;
                Ok(())
            }
            None => Err(format!("Model '{}' not found", id)),
        }
    }

    /// Get all model IDs
    pub fn list_models(&self) -> Vec<ModelId> {
        self.models.keys().cloned().collect()
    }

    /// Get all visible model IDs
    pub fn list_visible_models(&self) -> Vec<ModelId> {
        self.models
            .iter()
            .filter(|(_, m)| m.visible)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get number of loaded models
    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Check if a model exists
    pub fn has_model(&self, id: &ModelId) -> bool {
        self.models.contains_key(id)
    }

    /// Clear all models
    pub fn clear(&mut self) {
        self.models.clear();
        self.primary_model = None;
    }

    /// Get combined bounding box of all visible models
    pub fn get_combined_bounds(&self) -> Option<BoundingBox> {
        let mut combined: Option<BoundingBox> = None;

        for model in self.models.values() {
            if !model.visible {
                continue;
            }

            if let Some(bounds) = &model.bounds {
                combined = Some(match combined {
                    None => bounds.clone(),
                    Some(existing) => existing.union(bounds),
                });
            }
        }

        combined
    }

    /// Iterate over all registered models
    pub fn iter(&self) -> impl Iterator<Item = (&ModelId, &RegisteredModel)> {
        self.models.iter()
    }

    /// Iterate over all visible models
    pub fn iter_visible(&self) -> impl Iterator<Item = (&ModelId, &RegisteredModel)> {
        self.models.iter().filter(|(_, m)| m.visible)
    }

    /// Get all models (for iteration)
    pub fn models(&self) -> &HashMap<ModelId, RegisteredModel> {
        &self.models
    }
}

/// Information about a model in the registry (for Flutter)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegisteredModelInfo {
    pub id: String,
    pub name: String,
    pub file_path: Option<String>,
    pub visible: bool,
    pub is_primary: bool,
    pub model_info: ModelInfo,
}

impl ModelRegistry {
    /// Get info about all registered models (for Flutter)
    pub fn get_all_model_info(&self) -> Vec<RegisteredModelInfo> {
        self.models
            .iter()
            .map(|(id, reg)| RegisteredModelInfo {
                id: id.clone(),
                name: reg.name.clone(),
                file_path: reg.file_path.clone(),
                visible: reg.visible,
                is_primary: self.primary_model.as_ref() == Some(id),
                model_info: reg.model.get_info(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_remove_model() {
        let mut registry = ModelRegistry::new();

        let model = BimModel::new();
        let id = registry.add_model(model, "Test Model".to_string(), None);

        assert_eq!(registry.model_count(), 1);
        assert!(registry.has_model(&id));

        registry.remove_model(&id);
        assert_eq!(registry.model_count(), 0);
        assert!(!registry.has_model(&id));
    }

    #[test]
    fn test_primary_model() {
        let mut registry = ModelRegistry::new();

        let model1 = BimModel::new();
        let id1 = registry.add_model(model1, "Model 1".to_string(), None);

        // First model should be primary
        assert_eq!(registry.get_primary_model_id(), Some(&id1));

        let model2 = BimModel::new();
        let id2 = registry.add_model(model2, "Model 2".to_string(), None);

        // First model should still be primary
        assert_eq!(registry.get_primary_model_id(), Some(&id1));

        // Change primary
        registry.set_primary_model(&id2).unwrap();
        assert_eq!(registry.get_primary_model_id(), Some(&id2));
    }

    #[test]
    fn test_visibility() {
        let mut registry = ModelRegistry::new();

        let model = BimModel::new();
        let id = registry.add_model(model, "Test".to_string(), None);

        assert_eq!(registry.is_model_visible(&id), Some(true));

        registry.set_model_visible(&id, false).unwrap();
        assert_eq!(registry.is_model_visible(&id), Some(false));

        assert_eq!(registry.list_visible_models().len(), 0);
    }
}
