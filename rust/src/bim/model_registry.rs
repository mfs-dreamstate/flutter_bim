//! Model Registry - Multi-Model Management
//!
//! Manages multiple BIM models for federated model support.
//! Enables loading, unloading, and visibility control of multiple IFC files.

use super::model::{BimModel, ElementInfo, ModelInfo, ModelMesh};
use super::geometry::BoundingBox;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
    /// Cached bounding box (computed once from mesh)
    pub bounds: Option<BoundingBox>,
    /// Cached element info list (computed once on load, avoids mesh regeneration)
    pub cached_elements: Vec<ElementInfo>,
    /// Cached mesh data (computed once on load)
    pub cached_mesh: Option<ModelMesh>,
    /// Path to the on-disk mesh cache file (if any)
    pub cache_path: Option<String>,
}

impl RegisteredModel {
    /// Create a new registered model with default settings.
    /// Generates and caches mesh data immediately to avoid repeated regeneration.
    /// If a disk cache file exists (newer than the source file), it is loaded instead
    /// of re-tessellating.
    pub fn new(mut model: BimModel, name: String, file_path: Option<String>) -> Self {
        // Determine the cache file path from the source file
        let cache_file = file_path.as_ref().map(|fp| format!("{}.mesh.json", fp));

        // Try loading from disk cache first
        let mesh = if let Some(ref cache_path) = cache_file {
            if let Some(ref source_path) = file_path {
                if Self::is_cache_valid(source_path, cache_path) {
                    if let Some(cached) = Self::try_load_cached(cache_path) {
                        tracing::info!("Loaded mesh from cache: {}", cache_path);
                        Some(cached)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let mesh = match mesh {
            Some(m) => m,
            None => {
                let m = model.generate_meshes();
                // Save to cache if we have a cache path
                if let Some(ref cache_path) = cache_file {
                    if let Err(e) = Self::save_mesh_cache_static(&m, cache_path) {
                        tracing::warn!("Failed to save mesh cache: {}", e);
                    } else {
                        tracing::info!("Saved mesh cache: {}", cache_path);
                    }
                }
                m
            }
        };

        let bounds = mesh.bounds;
        let cached_elements = mesh.elements.clone();
        // Free entity map memory after mesh generation
        model.clear_entity_map();

        Self {
            model,
            name,
            file_path,
            visible: true,
            transform: Self::identity_matrix(),
            bounds,
            cached_elements,
            cached_mesh: Some(mesh),
            cache_path: cache_file,
        }
    }

    /// Check whether the cache file is newer than the source file.
    fn is_cache_valid(source_path: &str, cache_path: &str) -> bool {
        let source_meta = match fs::metadata(source_path) {
            Ok(m) => m,
            Err(_) => return false,
        };
        let cache_meta = match fs::metadata(cache_path) {
            Ok(m) => m,
            Err(_) => return false,
        };
        let source_mtime = source_meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let cache_mtime = cache_meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        cache_mtime >= source_mtime
    }

    /// Save the current mesh to a JSON file on disk.
    pub fn save_mesh_cache(&self, path: &str) -> Result<(), String> {
        let mesh = match &self.cached_mesh {
            Some(m) => m,
            None => return Err("No cached mesh to save".to_string()),
        };
        Self::save_mesh_cache_static(mesh, path)
    }

    /// Static helper: serialise a `ModelMesh` to a JSON file.
    fn save_mesh_cache_static(mesh: &ModelMesh, path: &str) -> Result<(), String> {
        let json = serde_json::to_string(mesh).map_err(|e| format!("Serialize error: {}", e))?;
        fs::write(path, json).map_err(|e| format!("Write error: {}", e))
    }

    /// Load a `ModelMesh` from a JSON cache file.
    pub fn load_mesh_cache(path: &str) -> Result<ModelMesh, String> {
        let data = fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
        serde_json::from_str(&data).map_err(|e| format!("Deserialize error: {}", e))
    }

    /// Try to load a cached mesh from disk. Returns `None` on any error.
    pub fn try_load_cached(path: &str) -> Option<ModelMesh> {
        Self::load_mesh_cache(path).ok()
    }

    /// Invalidate cached mesh data (call after visibility/selection changes)
    pub fn invalidate_cache(&mut self) {
        self.cached_mesh = None;
    }

    /// Get or regenerate the cached mesh
    pub fn get_mesh(&mut self) -> &ModelMesh {
        if self.cached_mesh.is_none() {
            let mesh = self.model.generate_meshes();
            self.bounds = mesh.bounds;
            self.cached_elements = mesh.elements.clone();
            self.cached_mesh = Some(mesh);
        }
        self.cached_mesh.as_ref().unwrap()
    }

    /// Get cached mesh without regenerating (returns None if invalidated)
    pub fn cached_mesh(&self) -> Option<&ModelMesh> {
        self.cached_mesh.as_ref()
    }

    /// Get cached elements (always available, even if mesh is invalidated)
    pub fn elements(&self) -> &[ElementInfo] {
        &self.cached_elements
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

/// Delete a mesh cache file from disk. Returns `true` if the file was deleted.
pub fn clear_mesh_cache(path: &str) -> bool {
    let cache_path = if path.ends_with(".mesh.json") {
        path.to_string()
    } else {
        format!("{}.mesh.json", path)
    };
    Path::new(&cache_path).exists() && fs::remove_file(&cache_path).is_ok()
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

// ---------------------------------------------------------------------------
// Indirect Draw Preparation
// ---------------------------------------------------------------------------

/// Indirect draw command for `wgpu::DrawIndexedIndirect`.
///
/// The layout matches wgpu's `DrawIndexedIndirectArgs`:
///   - index_count: u32
///   - instance_count: u32
///   - first_index: u32
///   - base_vertex: i32
///   - first_instance: u32
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IndirectDrawCommand {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

// Safety: IndirectDrawCommand is #[repr(C)] with only primitive fields,
// and all bit patterns are valid for u32/i32.
unsafe impl bytemuck::Zeroable for IndirectDrawCommand {}
unsafe impl bytemuck::Pod for IndirectDrawCommand {}

/// Prepare indirect draw commands from element draw ranges.
///
/// Each visible element produces one `IndirectDrawCommand`. Hidden elements
/// are skipped. `triangle_start` is treated as an index-buffer offset
/// (first_index = triangle_start * 3) and `triangle_count` gives the
/// number of triangles (index_count = triangle_count * 3).
pub fn prepare_indirect_draws(
    elements: &[ElementInfo],
    visible_mask: &[bool],
) -> Vec<IndirectDrawCommand> {
    let mut commands = Vec::new();
    for (i, elem) in elements.iter().enumerate() {
        let visible = visible_mask.get(i).copied().unwrap_or(true);
        if !visible || elem.triangle_count == 0 {
            continue;
        }
        commands.push(IndirectDrawCommand {
            index_count: elem.triangle_count * 3,
            instance_count: 1,
            first_index: elem.triangle_start * 3,
            base_vertex: 0,
            first_instance: 0,
        });
    }
    commands
}

/// Prepare a multi-draw-indirect buffer containing all visible elements.
///
/// Returns the list of commands and the number of visible elements.
pub fn prepare_multi_draw_buffer(
    elements: &[ElementInfo],
    visible_mask: &[bool],
) -> (Vec<IndirectDrawCommand>, usize) {
    let commands = prepare_indirect_draws(elements, visible_mask);
    let count = commands.len();
    (commands, count)
}

/// Compact draw commands by merging adjacent index ranges into fewer draw calls.
///
/// Two consecutive commands can be merged when `cmd_a.first_index + cmd_a.index_count == cmd_b.first_index`
/// and both have `base_vertex == 0` and `instance_count == 1`.
pub fn compact_draw_commands(commands: &[IndirectDrawCommand]) -> Vec<IndirectDrawCommand> {
    if commands.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<IndirectDrawCommand> = Vec::with_capacity(commands.len());
    result.push(commands[0]);

    for cmd in &commands[1..] {
        let last = result.last_mut().unwrap();
        // Can merge if the new command's first_index is contiguous with the last
        // command's end, and both have compatible base_vertex and instance_count.
        let last_end = last.first_index + last.index_count;
        if cmd.first_index == last_end
            && last.base_vertex == cmd.base_vertex
            && last.instance_count == 1
            && cmd.instance_count == 1
        {
            last.index_count += cmd.index_count;
        } else {
            result.push(*cmd);
        }
    }

    result
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

// ========================================================================
// Out-of-Core (Disk-Backed) Geometry Storage
// ========================================================================

use serde::{Deserialize, Serialize};

/// Configuration for out-of-core (disk-backed) geometry storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutOfCoreConfig {
    pub cache_dir: String,            // directory for geometry files
    pub max_memory_bytes: usize,      // max CPU memory for loaded geometry
    pub page_size_triangles: usize,   // triangles per disk page (default 10000)
    pub prefetch_pages: usize,        // pages to prefetch ahead (default 2)
    pub compression: bool,            // compress pages on disk
}

impl Default for OutOfCoreConfig {
    fn default() -> Self {
        Self {
            cache_dir: std::env::temp_dir()
                .join("bim_ooc_cache")
                .to_string_lossy()
                .to_string(),
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB
            page_size_triangles: 10_000,
            prefetch_pages: 2,
            compression: false,
        }
    }
}

/// A page of geometry data that can be paged to/from disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryPage {
    pub page_id: usize,
    pub model_id: String,
    pub element_ids: Vec<usize>,
    pub vertex_data: Vec<f32>,   // positions flat
    pub normal_data: Vec<f32>,   // normals flat
    pub index_data: Vec<u32>,
    pub triangle_count: usize,
    pub memory_bytes: usize,
    pub last_access_frame: u64,
}

/// Lightweight metadata about a page (always in memory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryPageInfo {
    pub page_id: usize,
    pub element_ids: Vec<usize>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub triangle_count: usize,
    pub memory_bytes: usize,
    pub on_disk: bool,
    pub in_memory: bool,
    pub disk_path: Option<String>,
    pub last_access_frame: u64,
}

/// Statistics about out-of-core memory usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutOfCoreStats {
    pub total_pages: usize,
    pub loaded_pages: usize,
    pub memory_used_bytes: usize,
    pub memory_budget_bytes: usize,
    pub utilization: f32,
    pub cache_hit_rate: f32,
    pub total_triangles: usize,
    pub loaded_triangles: usize,
}

/// Manages paging geometry to/from disk for large models.
#[derive(Debug)]
pub struct OutOfCoreManager {
    pub config: OutOfCoreConfig,
    pub pages: Vec<GeometryPageInfo>,   // metadata for all pages
    pub loaded_pages: Vec<usize>,        // indices of pages currently in memory
    pub total_memory_used: usize,
    pub frame_counter: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl OutOfCoreManager {
    /// Create a new OutOfCoreManager with the given configuration.
    pub fn new(config: OutOfCoreConfig) -> Self {
        // Ensure cache directory exists
        let _ = fs::create_dir_all(&config.cache_dir);
        Self {
            config,
            pages: Vec::new(),
            loaded_pages: Vec::new(),
            total_memory_used: 0,
            frame_counter: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Partition elements (id, triangle_count, bounds_min, bounds_max) into pages.
    /// Returns the number of pages created.
    pub fn partition_into_pages(
        &mut self,
        elements: &[(usize, usize, [f32; 3], [f32; 3])],
    ) -> usize {
        self.pages.clear();
        self.loaded_pages.clear();
        self.total_memory_used = 0;

        if elements.is_empty() {
            return 0;
        }

        let page_budget = self.config.page_size_triangles;
        let mut current_page_elements: Vec<usize> = Vec::new();
        let mut current_tri_count: usize = 0;
        let mut current_bounds_min = [f32::MAX; 3];
        let mut current_bounds_max = [f32::MIN; 3];
        let mut page_id = 0usize;

        for (elem_id, tri_count, bmin, bmax) in elements {
            // If adding this element would exceed the page budget and we already have elements,
            // finalize the current page.
            if current_tri_count + tri_count > page_budget && !current_page_elements.is_empty() {
                let mem_bytes = current_tri_count * 3 * (3 * 4 + 3 * 4 + 4); // verts + normals + index per triangle vertex
                self.pages.push(GeometryPageInfo {
                    page_id,
                    element_ids: current_page_elements.clone(),
                    bounds_min: current_bounds_min,
                    bounds_max: current_bounds_max,
                    triangle_count: current_tri_count,
                    memory_bytes: mem_bytes,
                    on_disk: false,
                    in_memory: false,
                    disk_path: None,
                    last_access_frame: 0,
                });
                page_id += 1;
                current_page_elements.clear();
                current_tri_count = 0;
                current_bounds_min = [f32::MAX; 3];
                current_bounds_max = [f32::MIN; 3];
            }

            current_page_elements.push(*elem_id);
            current_tri_count += tri_count;
            for k in 0..3 {
                current_bounds_min[k] = current_bounds_min[k].min(bmin[k]);
                current_bounds_max[k] = current_bounds_max[k].max(bmax[k]);
            }
        }

        // Finalize last page
        if !current_page_elements.is_empty() {
            let mem_bytes = current_tri_count * 3 * (3 * 4 + 3 * 4 + 4);
            self.pages.push(GeometryPageInfo {
                page_id,
                element_ids: current_page_elements,
                bounds_min: current_bounds_min,
                bounds_max: current_bounds_max,
                triangle_count: current_tri_count,
                memory_bytes: mem_bytes,
                on_disk: false,
                in_memory: false,
                disk_path: None,
                last_access_frame: 0,
            });
        }

        self.pages.len()
    }

    /// Serialize page to JSON file in cache_dir. Returns file path.
    pub fn save_page_to_disk(&mut self, page: &GeometryPage) -> Result<String, String> {
        let file_name = format!("page_{}.json", page.page_id);
        let file_path = Path::new(&self.config.cache_dir).join(&file_name);
        let file_path_str = file_path.to_string_lossy().to_string();

        let json =
            serde_json::to_string(page).map_err(|e| format!("Serialize error: {}", e))?;
        fs::write(&file_path, &json).map_err(|e| format!("Write error: {}", e))?;

        // Update page info
        if let Some(info) = self.pages.iter_mut().find(|p| p.page_id == page.page_id) {
            info.on_disk = true;
            info.disk_path = Some(file_path_str.clone());
        }

        Ok(file_path_str)
    }

    /// Load page from disk by reading the JSON file.
    pub fn load_page_from_disk(&mut self, page_id: usize) -> Result<GeometryPage, String> {
        let info = self
            .pages
            .iter()
            .find(|p| p.page_id == page_id)
            .ok_or_else(|| format!("Page {} not found", page_id))?;

        let disk_path = info
            .disk_path
            .as_ref()
            .ok_or_else(|| format!("Page {} has no disk path", page_id))?
            .clone();

        let data =
            fs::read_to_string(&disk_path).map_err(|e| format!("Read error: {}", e))?;
        let page: GeometryPage =
            serde_json::from_str(&data).map_err(|e| format!("Deserialize error: {}", e))?;

        // Mark as in memory
        if let Some(info) = self.pages.iter_mut().find(|p| p.page_id == page_id) {
            info.in_memory = true;
            info.last_access_frame = self.frame_counter;
            self.total_memory_used += info.memory_bytes;
        }

        if !self.loaded_pages.contains(&page_id) {
            self.loaded_pages.push(page_id);
        }

        Ok(page)
    }

    /// Evict least-recently-used pages until needed_bytes are free.
    /// Returns evicted page IDs.
    pub fn evict_lru_pages(&mut self, needed_bytes: usize) -> Vec<usize> {
        let mut evicted = Vec::new();
        let budget = self.config.max_memory_bytes;

        // Calculate how much we need to free
        let available = if budget > self.total_memory_used {
            budget - self.total_memory_used
        } else {
            0
        };

        if available >= needed_bytes {
            return evicted;
        }

        let mut to_free = needed_bytes - available;

        // Sort loaded pages by last_access_frame (oldest first)
        let mut candidates: Vec<(usize, u64, usize)> = self
            .loaded_pages
            .iter()
            .filter_map(|&pid| {
                self.pages.iter().find(|p| p.page_id == pid).map(|info| {
                    (pid, info.last_access_frame, info.memory_bytes)
                })
            })
            .collect();

        candidates.sort_by_key(|c| c.1); // oldest first

        for (pid, _, mem) in candidates {
            if to_free == 0 {
                break;
            }
            // Evict this page
            if let Some(info) = self.pages.iter_mut().find(|p| p.page_id == pid) {
                info.in_memory = false;
            }
            self.loaded_pages.retain(|&p| p != pid);
            self.total_memory_used = self.total_memory_used.saturating_sub(mem);
            to_free = to_free.saturating_sub(mem);
            evicted.push(pid);
        }

        evicted
    }

    /// Mark pages as needed. Returns pages that need loading from disk.
    pub fn request_pages(&mut self, page_ids: &[usize]) -> Vec<usize> {
        let mut need_load = Vec::new();
        for &pid in page_ids {
            if let Some(info) = self.pages.iter().find(|p| p.page_id == pid) {
                if info.in_memory {
                    self.cache_hits += 1;
                } else if info.on_disk {
                    self.cache_misses += 1;
                    need_load.push(pid);
                }
            }
        }
        need_load
    }

    /// Update last_access_frame for loaded pages.
    pub fn update_access(&mut self, page_ids: &[usize]) {
        for &pid in page_ids {
            if let Some(info) = self.pages.iter_mut().find(|p| p.page_id == pid) {
                if info.in_memory {
                    info.last_access_frame = self.frame_counter;
                }
            }
        }
    }

    /// Increment frame counter.
    pub fn advance_frame(&mut self) {
        self.frame_counter += 1;
    }

    /// Get memory usage statistics.
    pub fn get_memory_stats(&self) -> OutOfCoreStats {
        let total_triangles: usize = self.pages.iter().map(|p| p.triangle_count).sum();
        let loaded_triangles: usize = self
            .loaded_pages
            .iter()
            .filter_map(|&pid| self.pages.iter().find(|p| p.page_id == pid))
            .map(|p| p.triangle_count)
            .sum();

        let total_accesses = self.cache_hits + self.cache_misses;
        let cache_hit_rate = if total_accesses > 0 {
            self.cache_hits as f32 / total_accesses as f32
        } else {
            0.0
        };

        let utilization = if self.config.max_memory_bytes > 0 {
            self.total_memory_used as f32 / self.config.max_memory_bytes as f32
        } else {
            0.0
        };

        OutOfCoreStats {
            total_pages: self.pages.len(),
            loaded_pages: self.loaded_pages.len(),
            memory_used_bytes: self.total_memory_used,
            memory_budget_bytes: self.config.max_memory_bytes,
            utilization: utilization.min(1.0),
            cache_hit_rate,
            total_triangles,
            loaded_triangles,
        }
    }

    /// Return page IDs whose bounds intersect the frustum, sorted by distance to camera.
    pub fn get_visible_pages(
        &self,
        camera_pos: [f32; 3],
        frustum_planes: &[[f32; 4]; 6],
    ) -> Vec<usize> {
        let mut visible: Vec<(usize, f32)> = Vec::new();

        for page in &self.pages {
            if Self::aabb_intersects_frustum(
                &page.bounds_min,
                &page.bounds_max,
                frustum_planes,
            ) {
                // Compute distance from camera to AABB center
                let cx = (page.bounds_min[0] + page.bounds_max[0]) * 0.5;
                let cy = (page.bounds_min[1] + page.bounds_max[1]) * 0.5;
                let cz = (page.bounds_min[2] + page.bounds_max[2]) * 0.5;
                let dx = cx - camera_pos[0];
                let dy = cy - camera_pos[1];
                let dz = cz - camera_pos[2];
                let dist = dx * dx + dy * dy + dz * dz; // squared distance for sorting
                visible.push((page.page_id, dist));
            }
        }

        // Sort by distance (nearest first)
        visible.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        visible.into_iter().map(|(id, _)| id).collect()
    }

    /// Test whether an AABB intersects a frustum defined by 6 planes.
    /// Each plane is [a, b, c, d] where ax + by + cz + d >= 0 is inside.
    fn aabb_intersects_frustum(
        bounds_min: &[f32; 3],
        bounds_max: &[f32; 3],
        planes: &[[f32; 4]; 6],
    ) -> bool {
        for plane in planes {
            let a = plane[0];
            let b = plane[1];
            let c = plane[2];
            let d = plane[3];

            // Find the "positive vertex" (the vertex most in the direction of the plane normal)
            let px = if a >= 0.0 { bounds_max[0] } else { bounds_min[0] };
            let py = if b >= 0.0 { bounds_max[1] } else { bounds_min[1] };
            let pz = if c >= 0.0 { bounds_max[2] } else { bounds_min[2] };

            // If the positive vertex is outside this plane, the AABB is fully outside
            if a * px + b * py + c * pz + d < 0.0 {
                return false;
            }
        }
        true
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

    #[test]
    fn test_mesh_cache_save_load_roundtrip() {
        // Generate a mesh from an empty model (will produce default building elements)
        let model = BimModel::new();
        let registered = RegisteredModel::new(model, "CacheTest".to_string(), None);

        let original_mesh = registered.cached_mesh().expect("mesh should exist");

        // Use a temporary file for the cache
        let cache_path = std::env::temp_dir().join("test_mesh_cache.mesh.json");
        let cache_str = cache_path.to_str().unwrap();

        // Save
        registered.save_mesh_cache(cache_str).expect("save should succeed");

        // Load
        let loaded = RegisteredModel::load_mesh_cache(cache_str).expect("load should succeed");

        // Verify round-trip equality
        assert_eq!(original_mesh.vertices.len(), loaded.vertices.len());
        assert_eq!(original_mesh.indices.len(), loaded.indices.len());
        assert_eq!(original_mesh.normals.len(), loaded.normals.len());
        assert_eq!(original_mesh.colors.len(), loaded.colors.len());
        assert_eq!(original_mesh.elements.len(), loaded.elements.len());

        // Clean up
        let _ = std::fs::remove_file(cache_str);
    }

    #[test]
    fn test_try_load_cached_returns_none_for_missing_file() {
        let result = RegisteredModel::try_load_cached("/nonexistent/path/cache.mesh.json");
        assert!(result.is_none());
    }

    #[test]
    fn test_clear_mesh_cache() {
        // Create a temporary cache file
        let cache_path = std::env::temp_dir().join("test_clear_cache.ifc.mesh.json");
        let cache_str = cache_path.to_str().unwrap();
        std::fs::write(cache_str, "{}").expect("create temp file");

        // Clear using the source file path (without .mesh.json suffix)
        let source_path = std::env::temp_dir().join("test_clear_cache.ifc");
        let source_str = source_path.to_str().unwrap();
        assert!(clear_mesh_cache(source_str));

        // File should be gone
        assert!(!cache_path.exists());

        // Clearing again should return false
        assert!(!clear_mesh_cache(source_str));
    }

    // -------------------------------------------------------------------
    // Indirect draw tests
    // -------------------------------------------------------------------

    fn make_test_element(id: i32, tri_start: u32, tri_count: u32) -> ElementInfo {
        ElementInfo {
            id,
            element_type: "IfcWall".to_string(),
            name: format!("Wall {}", id),
            global_id: format!("guid-{}", id),
            bounds: BoundingBox {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 1.0, 1.0],
            },
            triangle_start: tri_start,
            triangle_count: tri_count,
        }
    }

    #[test]
    fn test_indirect_draw_command_layout() {
        // Verify the struct size matches wgpu's DrawIndexedIndirectArgs (5 * 4 = 20 bytes)
        assert_eq!(
            std::mem::size_of::<IndirectDrawCommand>(),
            20,
            "IndirectDrawCommand must be exactly 20 bytes to match wgpu layout"
        );

        // Verify bytemuck Pod works
        let cmd = IndirectDrawCommand {
            index_count: 12,
            instance_count: 1,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        };
        let bytes: &[u8] = bytemuck::bytes_of(&cmd);
        assert_eq!(bytes.len(), 20);

        // Verify Zeroable
        let zeroed: IndirectDrawCommand = bytemuck::Zeroable::zeroed();
        assert_eq!(zeroed.index_count, 0);
        assert_eq!(zeroed.instance_count, 0);
    }

    #[test]
    fn test_prepare_indirect_draws_basic() {
        let elements = vec![
            make_test_element(1, 0, 10),
            make_test_element(2, 10, 20),
            make_test_element(3, 30, 5),
        ];
        let visible = vec![true, true, true];

        let commands = prepare_indirect_draws(&elements, &visible);
        assert_eq!(commands.len(), 3);

        // First element: 10 triangles = 30 indices, starting at index 0
        assert_eq!(commands[0].index_count, 30);
        assert_eq!(commands[0].first_index, 0);
        assert_eq!(commands[0].instance_count, 1);

        // Second element: 20 triangles = 60 indices, starting at triangle 10 = index 30
        assert_eq!(commands[1].index_count, 60);
        assert_eq!(commands[1].first_index, 30);

        // Third element: 5 triangles = 15 indices, starting at triangle 30 = index 90
        assert_eq!(commands[2].index_count, 15);
        assert_eq!(commands[2].first_index, 90);
    }

    #[test]
    fn test_prepare_indirect_draws_with_hidden_elements() {
        let elements = vec![
            make_test_element(1, 0, 10),
            make_test_element(2, 10, 20),
            make_test_element(3, 30, 5),
        ];
        let visible = vec![true, false, true];

        let commands = prepare_indirect_draws(&elements, &visible);
        assert_eq!(commands.len(), 2, "Hidden elements should be excluded");
        assert_eq!(commands[0].first_index, 0);  // Element 1
        assert_eq!(commands[1].first_index, 90); // Element 3
    }

    #[test]
    fn test_prepare_multi_draw_buffer() {
        let elements = vec![
            make_test_element(1, 0, 10),
            make_test_element(2, 10, 0), // zero triangles
            make_test_element(3, 10, 5),
        ];
        let visible = vec![true, true, true];

        let (commands, count) = prepare_multi_draw_buffer(&elements, &visible);
        assert_eq!(count, 2, "Zero-triangle elements should be excluded");
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn test_compact_draw_commands_merges_adjacent() {
        let commands = vec![
            IndirectDrawCommand {
                index_count: 30,
                instance_count: 1,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            },
            IndirectDrawCommand {
                index_count: 60,
                instance_count: 1,
                first_index: 30,  // contiguous with previous
                base_vertex: 0,
                first_instance: 0,
            },
            IndirectDrawCommand {
                index_count: 15,
                instance_count: 1,
                first_index: 90,  // contiguous
                base_vertex: 0,
                first_instance: 0,
            },
        ];

        let compacted = compact_draw_commands(&commands);
        assert_eq!(compacted.len(), 1, "All 3 contiguous commands should merge into 1");
        assert_eq!(compacted[0].index_count, 105);
        assert_eq!(compacted[0].first_index, 0);
    }

    #[test]
    fn test_compact_draw_commands_gap() {
        let commands = vec![
            IndirectDrawCommand {
                index_count: 30,
                instance_count: 1,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            },
            IndirectDrawCommand {
                index_count: 15,
                instance_count: 1,
                first_index: 60,  // NOT contiguous (gap from 30 to 60)
                base_vertex: 0,
                first_instance: 0,
            },
        ];

        let compacted = compact_draw_commands(&commands);
        assert_eq!(compacted.len(), 2, "Non-contiguous commands should not merge");
    }

    #[test]
    fn test_compact_draw_commands_empty() {
        let compacted = compact_draw_commands(&[]);
        assert!(compacted.is_empty());
    }

    // ================================================================
    // Out-of-Core Tests
    // ================================================================

    #[test]
    fn test_ooc_config_default() {
        let config = OutOfCoreConfig::default();
        assert_eq!(config.max_memory_bytes, 512 * 1024 * 1024);
        assert_eq!(config.page_size_triangles, 10_000);
        assert_eq!(config.prefetch_pages, 2);
        assert!(!config.compression);
        assert!(!config.cache_dir.is_empty());
    }

    #[test]
    fn test_ooc_manager_new() {
        let config = OutOfCoreConfig::default();
        let mgr = OutOfCoreManager::new(config.clone());
        assert!(mgr.pages.is_empty());
        assert!(mgr.loaded_pages.is_empty());
        assert_eq!(mgr.total_memory_used, 0);
        assert_eq!(mgr.frame_counter, 0);
        assert_eq!(mgr.cache_hits, 0);
        assert_eq!(mgr.cache_misses, 0);
        assert_eq!(mgr.config.max_memory_bytes, config.max_memory_bytes);
    }

    #[test]
    fn test_ooc_partition_pages() {
        let mut config = OutOfCoreConfig::default();
        config.page_size_triangles = 100;
        let mut mgr = OutOfCoreManager::new(config);

        let elements = vec![
            (0, 50, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
            (1, 60, [1.0, 0.0, 0.0], [2.0, 1.0, 1.0]),
            (2, 30, [2.0, 0.0, 0.0], [3.0, 1.0, 1.0]),
            (3, 80, [3.0, 0.0, 0.0], [4.0, 1.0, 1.0]),
        ];

        let page_count = mgr.partition_into_pages(&elements);

        // With budget=100: elem0(50) fits, elem1(60) would push to 110, so page 0 = [elem0],
        // page 1 starts with elem1(60), elem2(30) fits (90), elem3(80) would push to 170,
        // so page 1 = [elem1, elem2], page 2 = [elem3]
        assert!(page_count >= 2, "Should create multiple pages, got {}", page_count);

        // Total triangles across all pages should match input
        let total_tri: usize = mgr.pages.iter().map(|p| p.triangle_count).sum();
        assert_eq!(total_tri, 220);

        // All elements should be assigned
        let all_elems: Vec<usize> = mgr.pages.iter().flat_map(|p| p.element_ids.clone()).collect();
        assert_eq!(all_elems.len(), 4);
    }

    #[test]
    fn test_ooc_save_load_page() {
        let temp_dir = std::env::temp_dir().join("bim_ooc_test_save_load");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);

        let config = OutOfCoreConfig {
            cache_dir: temp_dir.to_string_lossy().to_string(),
            max_memory_bytes: 1024 * 1024,
            page_size_triangles: 1000,
            prefetch_pages: 0,
            compression: false,
        };

        let mut mgr = OutOfCoreManager::new(config);

        // Create a page info entry
        mgr.pages.push(GeometryPageInfo {
            page_id: 0,
            element_ids: vec![10, 20],
            bounds_min: [0.0, 0.0, 0.0],
            bounds_max: [1.0, 1.0, 1.0],
            triangle_count: 4,
            memory_bytes: 256,
            on_disk: false,
            in_memory: false,
            disk_path: None,
            last_access_frame: 0,
        });

        let page = GeometryPage {
            page_id: 0,
            model_id: "test_model".to_string(),
            element_ids: vec![10, 20],
            vertex_data: vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
            normal_data: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            index_data: vec![0, 1, 2, 0, 2, 3],
            triangle_count: 2,
            memory_bytes: 256,
            last_access_frame: 0,
        };

        // Save
        let path = mgr.save_page_to_disk(&page).expect("save should succeed");
        assert!(!path.is_empty());
        assert!(mgr.pages[0].on_disk);

        // Load
        let loaded = mgr.load_page_from_disk(0).expect("load should succeed");
        assert_eq!(loaded.page_id, 0);
        assert_eq!(loaded.model_id, "test_model");
        assert_eq!(loaded.vertex_data, page.vertex_data);
        assert_eq!(loaded.normal_data, page.normal_data);
        assert_eq!(loaded.index_data, page.index_data);
        assert_eq!(loaded.triangle_count, 2);

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ooc_evict_lru() {
        let config = OutOfCoreConfig {
            cache_dir: std::env::temp_dir().to_string_lossy().to_string(),
            max_memory_bytes: 1000,
            page_size_triangles: 100,
            prefetch_pages: 0,
            compression: false,
        };

        let mut mgr = OutOfCoreManager::new(config);

        // Simulate 3 loaded pages with different access times
        mgr.pages.push(GeometryPageInfo {
            page_id: 0,
            element_ids: vec![0],
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            triangle_count: 10,
            memory_bytes: 400,
            on_disk: true,
            in_memory: true,
            disk_path: Some("page_0.json".to_string()),
            last_access_frame: 1, // oldest
        });
        mgr.pages.push(GeometryPageInfo {
            page_id: 1,
            element_ids: vec![1],
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            triangle_count: 10,
            memory_bytes: 400,
            on_disk: true,
            in_memory: true,
            disk_path: Some("page_1.json".to_string()),
            last_access_frame: 5, // newest
        });
        mgr.pages.push(GeometryPageInfo {
            page_id: 2,
            element_ids: vec![2],
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            triangle_count: 10,
            memory_bytes: 400,
            on_disk: true,
            in_memory: true,
            disk_path: Some("page_2.json".to_string()),
            last_access_frame: 3, // middle
        });
        mgr.loaded_pages = vec![0, 1, 2];
        mgr.total_memory_used = 1200;

        // Need 500 bytes free. Budget is 1000, used is 1200, so need to free 700.
        let evicted = mgr.evict_lru_pages(500);

        // Should evict oldest first: page 0 (frame 1), then page 2 (frame 3)
        assert!(!evicted.is_empty());
        assert!(evicted.contains(&0), "Oldest page (frame 1) should be evicted first");
    }

    #[test]
    fn test_ooc_request_pages() {
        let config = OutOfCoreConfig {
            cache_dir: std::env::temp_dir().to_string_lossy().to_string(),
            max_memory_bytes: 10000,
            page_size_triangles: 100,
            prefetch_pages: 0,
            compression: false,
        };

        let mut mgr = OutOfCoreManager::new(config);

        mgr.pages.push(GeometryPageInfo {
            page_id: 0,
            element_ids: vec![0],
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            triangle_count: 10,
            memory_bytes: 100,
            on_disk: true,
            in_memory: true, // already loaded
            disk_path: Some("p0.json".to_string()),
            last_access_frame: 0,
        });
        mgr.pages.push(GeometryPageInfo {
            page_id: 1,
            element_ids: vec![1],
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            triangle_count: 10,
            memory_bytes: 100,
            on_disk: true,
            in_memory: false, // needs loading
            disk_path: Some("p1.json".to_string()),
            last_access_frame: 0,
        });
        mgr.loaded_pages = vec![0];

        let need_load = mgr.request_pages(&[0, 1]);
        assert_eq!(need_load, vec![1], "Only page 1 should need loading");
        assert_eq!(mgr.cache_hits, 1);
        assert_eq!(mgr.cache_misses, 1);
    }

    #[test]
    fn test_ooc_memory_stats() {
        let config = OutOfCoreConfig {
            cache_dir: std::env::temp_dir().to_string_lossy().to_string(),
            max_memory_bytes: 2000,
            page_size_triangles: 100,
            prefetch_pages: 0,
            compression: false,
        };

        let mut mgr = OutOfCoreManager::new(config);

        mgr.pages.push(GeometryPageInfo {
            page_id: 0,
            element_ids: vec![0],
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            triangle_count: 50,
            memory_bytes: 500,
            on_disk: true,
            in_memory: true,
            disk_path: None,
            last_access_frame: 0,
        });
        mgr.pages.push(GeometryPageInfo {
            page_id: 1,
            element_ids: vec![1],
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            triangle_count: 30,
            memory_bytes: 300,
            on_disk: true,
            in_memory: false,
            disk_path: None,
            last_access_frame: 0,
        });
        mgr.loaded_pages = vec![0];
        mgr.total_memory_used = 500;
        mgr.cache_hits = 8;
        mgr.cache_misses = 2;

        let stats = mgr.get_memory_stats();
        assert_eq!(stats.total_pages, 2);
        assert_eq!(stats.loaded_pages, 1);
        assert_eq!(stats.memory_used_bytes, 500);
        assert_eq!(stats.memory_budget_bytes, 2000);
        assert!((stats.utilization - 0.25).abs() < 1e-4);
        assert!((stats.cache_hit_rate - 0.8).abs() < 1e-4);
        assert_eq!(stats.total_triangles, 80);
        assert_eq!(stats.loaded_triangles, 50);
    }

    #[test]
    fn test_ooc_visible_pages() {
        let config = OutOfCoreConfig::default();
        let mut mgr = OutOfCoreManager::new(config);

        // Page 0: near the camera
        mgr.pages.push(GeometryPageInfo {
            page_id: 0,
            element_ids: vec![0],
            bounds_min: [-1.0, -1.0, -1.0],
            bounds_max: [1.0, 1.0, 1.0],
            triangle_count: 10,
            memory_bytes: 100,
            on_disk: false,
            in_memory: false,
            disk_path: None,
            last_access_frame: 0,
        });

        // Page 1: far away
        mgr.pages.push(GeometryPageInfo {
            page_id: 1,
            element_ids: vec![1],
            bounds_min: [100.0, 100.0, 100.0],
            bounds_max: [101.0, 101.0, 101.0],
            triangle_count: 10,
            memory_bytes: 100,
            on_disk: false,
            in_memory: false,
            disk_path: None,
            last_access_frame: 0,
        });

        // Page 2: behind the camera (should be culled by back plane)
        mgr.pages.push(GeometryPageInfo {
            page_id: 2,
            element_ids: vec![2],
            bounds_min: [-200.0, -1.0, -1.0],
            bounds_max: [-199.0, 1.0, 1.0],
            triangle_count: 10,
            memory_bytes: 100,
            on_disk: false,
            in_memory: false,
            disk_path: None,
            last_access_frame: 0,
        });

        let camera_pos = [0.0, 0.0, 0.0];

        // Frustum planes: create a large frustum that includes pages 0 and 1
        // but excludes page 2 (x < -100).
        // Using simple planes: each plane is [a, b, c, d] where ax+by+cz+d >= 0 is inside
        let frustum_planes: [[f32; 4]; 6] = [
            [1.0, 0.0, 0.0, 150.0],   // left: x >= -150
            [-1.0, 0.0, 0.0, 150.0],  // right: x <= 150
            [0.0, 1.0, 0.0, 150.0],   // bottom: y >= -150
            [0.0, -1.0, 0.0, 150.0],  // top: y <= 150
            [0.0, 0.0, 1.0, 150.0],   // near: z >= -150
            [0.0, 0.0, -1.0, 150.0],  // far: z <= 150
        ];

        let visible = mgr.get_visible_pages(camera_pos, &frustum_planes);

        // Pages 0 and 1 should be visible, page 2 is at x=-200 which is outside x >= -150
        assert!(visible.contains(&0), "Page 0 should be visible");
        assert!(visible.contains(&1), "Page 1 should be visible");
        assert!(!visible.contains(&2), "Page 2 should be culled");

        // Page 0 is closer, so should come first
        if visible.len() >= 2 {
            assert_eq!(visible[0], 0, "Closer page should be first");
        }
    }
}
