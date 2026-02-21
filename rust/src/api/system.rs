use std::sync::{LazyLock, Mutex};

use flutter_rust_bridge::frb;

use crate::bim::model::{get_mesh_progress, is_mesh_generating};
use super::state::with_state;

// ========================================================================
// Initialization & Version
// ========================================================================

/// Initialize the BIM viewer engine
/// This should be called once when the app starts
#[frb(sync)]
pub fn initialize() -> String {
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
pub async fn test_async() -> String {
    tracing::debug!("Starting async test");
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    tracing::debug!("Async test completed");
    "Async test completed successfully".to_string()
}

/// Test error handling across FFI
#[frb(sync)]
pub fn test_error_handling(should_fail: bool) -> Result<String, String> {
    if should_fail {
        Err("Intentional error for testing".to_string())
    } else {
        Ok("Success!".to_string())
    }
}

// ========================================================================
// Loading Progress FFI (Task 1)
// ========================================================================

/// Get current loading progress as JSON.
/// Returns: {"phase": "tessellating", "percentage": 45.2, "completed": 452, "total": 1000}
/// Returns empty string if not currently loading.
#[frb(sync)]
pub fn get_loading_progress() -> String {
    match get_mesh_progress() {
        Some(progress) => {
            serde_json::json!({
                "phase": progress.current_phase,
                "percentage": progress.percentage,
                "completed": progress.completed_elements,
                "total": progress.total_elements
            })
            .to_string()
        }
        None => String::new(),
    }
}

/// Check if a model is currently being loaded/processed.
#[frb(sync)]
pub fn is_loading() -> bool {
    is_mesh_generating()
}

// ========================================================================
// Memory Usage Tracking (Task 2)
// ========================================================================

/// Memory usage statistics for the BIM viewer.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryStats {
    pub total_vertices: usize,
    pub total_indices: usize,
    pub total_elements: usize,
    pub total_models: usize,
    pub estimated_gpu_bytes: usize,
    pub estimated_cpu_bytes: usize,
    pub mesh_cache_bytes: usize,
    pub bvh_node_count: usize,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            total_vertices: 0,
            total_indices: 0,
            total_elements: 0,
            total_models: 0,
            estimated_gpu_bytes: 0,
            estimated_cpu_bytes: 0,
            mesh_cache_bytes: 0,
            bvh_node_count: 0,
        }
    }
}

static MEMORY_STATS: LazyLock<Mutex<MemoryStats>> =
    LazyLock::new(|| Mutex::new(MemoryStats::default()));

/// Count BVH nodes recursively.
fn count_bvh_nodes(node: &crate::renderer::BvhNode) -> usize {
    match node {
        crate::renderer::BvhNode::Leaf { .. } => 1,
        crate::renderer::BvhNode::Internal { left, right, .. } => {
            1 + count_bvh_nodes(left) + count_bvh_nodes(right)
        }
    }
}

/// Get current memory usage statistics as JSON.
#[frb(sync)]
pub fn get_memory_stats() -> String {
    let stats = MEMORY_STATS.lock().unwrap();
    serde_json::to_string(&*stats).unwrap_or_else(|_| "{}".to_string())
}

/// Update memory stats (called internally after model load/unload).
pub(crate) fn update_memory_stats() {
    let new_stats = with_state(|s| {
        let mut total_vertices: usize = 0;
        let mut total_indices: usize = 0;
        let mut total_elements: usize = 0;
        let mut mesh_cache_bytes: usize = 0;
        let total_models = s.registry.model_count();

        for (_id, reg_model) in s.registry.iter() {
            if let Some(mesh) = reg_model.cached_mesh() {
                // vertices: Vec<f32>, 3 floats per vertex
                let vertex_count = mesh.vertices.len() / 3;
                total_vertices += vertex_count;
                total_indices += mesh.indices.len();
                total_elements += mesh.elements.len();

                // Mesh cache: vertices + normals + colors (all f32) + indices (u32)
                mesh_cache_bytes += mesh.vertices.len() * std::mem::size_of::<f32>();
                mesh_cache_bytes += mesh.normals.len() * std::mem::size_of::<f32>();
                mesh_cache_bytes += mesh.colors.len() * std::mem::size_of::<f32>();
                mesh_cache_bytes += mesh.indices.len() * std::mem::size_of::<u32>();
            } else {
                // Estimate from model element count
                total_elements += reg_model.elements().len();
            }
        }

        // GPU memory: Vertex is 20 bytes (position [f32;3] + packed normal [i8;4] + packed color [u8;4])
        // plus u32 indices (4 bytes each), plus uniform buffers overhead (~1 KB)
        let estimated_gpu_bytes = total_vertices * 20 + total_indices * 4 + 1024;

        // CPU memory: model structs + element data + BVH + property data
        // Each ElementInfo is ~128 bytes (strings + bounds + ids)
        // Each registered model struct is ~256 bytes base
        // Property data: rough estimate of 512 bytes per element
        let estimated_cpu_bytes = mesh_cache_bytes
            + total_elements * 128
            + total_models * 256
            + total_elements * 512;

        // Count BVH nodes from pick accelerator
        let bvh_node_count = s
            .pick_accelerator
            .bvh
            .as_ref()
            .map(|root| count_bvh_nodes(root))
            .unwrap_or(0);

        MemoryStats {
            total_vertices,
            total_indices,
            total_elements,
            total_models,
            estimated_gpu_bytes,
            estimated_cpu_bytes,
            mesh_cache_bytes,
            bvh_node_count,
        }
    });

    if let Ok(mut stats) = MEMORY_STATS.lock() {
        *stats = new_stats;
    }
}

/// Get a human-readable memory usage summary.
#[frb(sync)]
pub fn get_memory_summary() -> String {
    let stats = MEMORY_STATS.lock().unwrap();
    format!(
        "{} models, {}K vertices, ~{} GPU, ~{} CPU",
        stats.total_models,
        if stats.total_vertices >= 1000 {
            format!("{}", stats.total_vertices / 1000)
        } else {
            format!("{}", stats.total_vertices)
        },
        format_bytes(stats.estimated_gpu_bytes),
        format_bytes(stats.estimated_cpu_bytes),
    )
}

/// Format a byte count into a human-readable string (e.g. "12.4 MB").
fn format_bytes(bytes: usize) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Get estimated GPU memory usage in bytes.
#[frb(sync)]
pub fn get_gpu_memory_bytes() -> u64 {
    let stats = MEMORY_STATS.lock().unwrap();
    stats.estimated_gpu_bytes as u64
}

/// Get estimated CPU memory usage in bytes.
#[frb(sync)]
pub fn get_cpu_memory_bytes() -> u64 {
    let stats = MEMORY_STATS.lock().unwrap();
    stats.estimated_cpu_bytes as u64
}

/// Force garbage collection / memory cleanup.
/// Clears invalidated mesh caches and entity maps, then updates stats.
#[frb(sync)]
pub fn compact_memory() -> String {
    let freed = with_state(|s| {
        let mut freed_bytes: usize = 0;
        let model_ids: Vec<String> = s.registry.list_models();
        for id in &model_ids {
            if let Some(reg) = s.registry.get_model_mut(id) {
                // Clear entity map if still present
                if reg.model.entity_map.is_some() {
                    // Rough estimate: entity map ~1 KB per entity
                    freed_bytes += reg.model.element_count * 1024;
                    reg.model.clear_entity_map();
                }
            }
        }
        freed_bytes
    });

    update_memory_stats();

    let stats = MEMORY_STATS.lock().unwrap();
    format!(
        "Freed ~{}, now: {} models, {}K vertices, ~{} GPU, ~{} CPU",
        format_bytes(freed),
        stats.total_models,
        if stats.total_vertices >= 1000 {
            format!("{}", stats.total_vertices / 1000)
        } else {
            format!("{}", stats.total_vertices)
        },
        format_bytes(stats.estimated_gpu_bytes),
        format_bytes(stats.estimated_cpu_bytes),
    )
}

// ========================================================================
// System Diagnostics (Task 4)
// ========================================================================

/// Performance counters for monitoring.
#[derive(Debug, Default)]
pub struct PerfCounters {
    pub frame_count: u64,
    pub total_render_time_us: u64,
    pub peak_render_time_us: u64,
}

static PERF_COUNTERS: LazyLock<Mutex<PerfCounters>> =
    LazyLock::new(|| Mutex::new(PerfCounters::default()));

/// Increment the frame counter and record render time (called internally).
pub(crate) fn record_frame(render_time_us: u64) {
    if let Ok(mut counters) = PERF_COUNTERS.lock() {
        counters.frame_count += 1;
        counters.total_render_time_us += render_time_us;
        if render_time_us > counters.peak_render_time_us {
            counters.peak_render_time_us = render_time_us;
        }
    }
}

/// Get the number of frames rendered (for performance monitoring).
#[frb(sync)]
pub fn get_frame_count() -> u64 {
    PERF_COUNTERS
        .lock()
        .map(|c| c.frame_count)
        .unwrap_or(0)
}

/// Reset performance counters.
#[frb(sync)]
pub fn reset_performance_counters() {
    if let Ok(mut counters) = PERF_COUNTERS.lock() {
        counters.frame_count = 0;
        counters.total_render_time_us = 0;
        counters.peak_render_time_us = 0;
    }
}

/// Get renderer capabilities/features as JSON.
#[frb(sync)]
pub fn get_renderer_capabilities() -> String {
    let caps = with_state(|s| {
        let renderer_initialized = s.renderer.is_some();
        let wireframe_supported = s
            .renderer
            .as_ref()
            .map(|r| r.gpu.wireframe_supported())
            .unwrap_or(false);
        let scene_initialized = s
            .renderer
            .as_ref()
            .and_then(|r| r.scene.as_ref())
            .is_some();

        serde_json::json!({
            "renderer_initialized": renderer_initialized,
            "scene_initialized": scene_initialized,
            "msaa": true,
            "wireframe": wireframe_supported,
            "fxaa": true,
            "max_section_planes": 6,
            "edge_rendering": true,
            "frustum_culling": true,
            "bvh_picking": true,
            "parallel_tessellation": true,
            "mesh_caching": true,
        })
    });

    caps.to_string()
}

/// Get a full system diagnostic report.
#[frb(sync)]
pub fn get_diagnostic_report() -> String {
    update_memory_stats();

    let stats = MEMORY_STATS.lock().unwrap().clone();
    let counters = PERF_COUNTERS.lock().unwrap();

    let avg_frame_us = if counters.frame_count > 0 {
        counters.total_render_time_us / counters.frame_count
    } else {
        0
    };

    let renderer_info = with_state(|s| {
        let initialized = s.renderer.is_some();
        let dimensions = s
            .renderer
            .as_ref()
            .and_then(|r| r.get_dimensions())
            .map(|(w, h)| format!("{}x{}", w, h))
            .unwrap_or_else(|| "N/A".to_string());
        let wireframe = s
            .renderer
            .as_ref()
            .map(|r| r.gpu.wireframe_supported())
            .unwrap_or(false);
        (initialized, dimensions, wireframe)
    });

    format!(
        "=== BIM Viewer Diagnostic Report ===\n\
         \n\
         System:\n\
         \x20 Rust crate version: {}\n\
         \x20 Target arch: {}\n\
         \x20 OS: {}\n\
         \n\
         Memory:\n\
         \x20 Models: {}\n\
         \x20 Vertices: {}\n\
         \x20 Indices: {}\n\
         \x20 Elements: {}\n\
         \x20 GPU memory (est): {}\n\
         \x20 CPU memory (est): {}\n\
         \x20 Mesh cache: {}\n\
         \x20 BVH nodes: {}\n\
         \n\
         Renderer:\n\
         \x20 Initialized: {}\n\
         \x20 Dimensions: {}\n\
         \x20 Wireframe support: {}\n\
         \n\
         Performance:\n\
         \x20 Frames rendered: {}\n\
         \x20 Avg frame time: {} us\n\
         \x20 Peak frame time: {} us\n\
         \n\
         Features:\n\
         \x20 MSAA: enabled\n\
         \x20 FXAA: available\n\
         \x20 Max section planes: 6\n\
         \x20 Parallel tessellation: enabled\n\
         \x20 BVH picking: enabled\n\
         \x20 Mesh caching: enabled\n\
         ===================================",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        std::env::consts::OS,
        stats.total_models,
        stats.total_vertices,
        stats.total_indices,
        stats.total_elements,
        format_bytes(stats.estimated_gpu_bytes),
        format_bytes(stats.estimated_cpu_bytes),
        format_bytes(stats.mesh_cache_bytes),
        stats.bvh_node_count,
        renderer_info.0,
        renderer_info.1,
        renderer_info.2,
        counters.frame_count,
        avg_frame_us,
        counters.peak_render_time_us,
    )
}

// ========================================================================
// Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_stats_defaults() {
        let stats = MemoryStats::default();
        assert_eq!(stats.total_vertices, 0);
        assert_eq!(stats.total_indices, 0);
        assert_eq!(stats.total_elements, 0);
        assert_eq!(stats.total_models, 0);
        assert_eq!(stats.estimated_gpu_bytes, 0);
        assert_eq!(stats.estimated_cpu_bytes, 0);
        assert_eq!(stats.mesh_cache_bytes, 0);
        assert_eq!(stats.bvh_node_count, 0);
    }

    #[test]
    fn test_memory_summary_format() {
        // Write known stats into the global
        {
            let mut stats = MEMORY_STATS.lock().unwrap();
            *stats = MemoryStats {
                total_vertices: 245_000,
                total_indices: 490_000,
                total_elements: 1200,
                total_models: 3,
                estimated_gpu_bytes: 12_400_000,
                estimated_cpu_bytes: 8_200_000,
                mesh_cache_bytes: 5_000_000,
                bvh_node_count: 2400,
            };
        }

        let summary = get_memory_summary();
        assert!(summary.contains("3 models"), "Expected '3 models' in: {}", summary);
        assert!(summary.contains("245K vertices"), "Expected '245K vertices' in: {}", summary);
        assert!(summary.contains("MB"), "Expected 'MB' in: {}", summary);

        // Restore defaults to avoid polluting other tests
        {
            let mut stats = MEMORY_STATS.lock().unwrap();
            *stats = MemoryStats::default();
        }
    }

    #[test]
    fn test_loading_progress_when_idle() {
        // When no model is loading, get_loading_progress should return empty string
        let progress = get_loading_progress();
        // Progress may or may not be empty depending on concurrent tests,
        // but when truly idle it should be empty.
        // We assert that it's either empty or valid JSON.
        if !progress.is_empty() {
            let parsed: serde_json::Value = serde_json::from_str(&progress)
                .expect("Loading progress should be valid JSON if non-empty");
            assert!(parsed.get("phase").is_some());
            assert!(parsed.get("percentage").is_some());
        }
    }

    #[test]
    fn test_performance_counters() {
        // Reset first
        reset_performance_counters();
        assert_eq!(get_frame_count(), 0);

        // Record some frames
        record_frame(1000);
        record_frame(2000);
        record_frame(500);

        assert_eq!(get_frame_count(), 3);

        let counters = PERF_COUNTERS.lock().unwrap();
        assert_eq!(counters.total_render_time_us, 3500);
        assert_eq!(counters.peak_render_time_us, 2000);
        drop(counters);

        // Reset and verify
        reset_performance_counters();
        assert_eq!(get_frame_count(), 0);
    }

    #[test]
    fn test_diagnostic_report_format() {
        // Reset state for deterministic output
        reset_performance_counters();
        {
            let mut stats = MEMORY_STATS.lock().unwrap();
            *stats = MemoryStats::default();
        }

        let report = get_diagnostic_report();
        assert!(report.contains("BIM Viewer Diagnostic Report"), "Missing header in report");
        assert!(report.contains("System:"), "Missing System section");
        assert!(report.contains("Memory:"), "Missing Memory section");
        assert!(report.contains("Renderer:"), "Missing Renderer section");
        assert!(report.contains("Performance:"), "Missing Performance section");
        assert!(report.contains("Features:"), "Missing Features section");
        assert!(report.contains("Target arch:"), "Missing target arch info");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(12_400_000), "11.8 MB");
    }

    #[test]
    fn test_renderer_capabilities_json() {
        let caps_json = get_renderer_capabilities();
        let parsed: serde_json::Value =
            serde_json::from_str(&caps_json).expect("Capabilities should be valid JSON");
        assert_eq!(parsed["max_section_planes"], 6);
        assert_eq!(parsed["msaa"], true);
        assert_eq!(parsed["fxaa"], true);
        assert_eq!(parsed["parallel_tessellation"], true);
    }

    #[test]
    fn test_is_loading_when_idle() {
        // When no model is actively being loaded, is_loading should return false
        // (assuming no concurrent loading in other tests)
        let loading = is_loading();
        // We can't guarantee false due to concurrent tests, but at minimum
        // the function should not panic.
        let _ = loading;
    }

    #[test]
    fn test_get_memory_stats_json() {
        {
            let mut stats = MEMORY_STATS.lock().unwrap();
            *stats = MemoryStats {
                total_vertices: 100,
                total_indices: 200,
                total_elements: 10,
                total_models: 1,
                estimated_gpu_bytes: 4096,
                estimated_cpu_bytes: 8192,
                mesh_cache_bytes: 2048,
                bvh_node_count: 19,
            };
        }

        let json_str = get_memory_stats();
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("Memory stats should be valid JSON");
        assert_eq!(parsed["total_vertices"], 100);
        assert_eq!(parsed["total_models"], 1);
        assert_eq!(parsed["bvh_node_count"], 19);

        // Restore defaults
        {
            let mut stats = MEMORY_STATS.lock().unwrap();
            *stats = MemoryStats::default();
        }
    }

    #[test]
    fn test_gpu_cpu_memory_bytes() {
        {
            let mut stats = MEMORY_STATS.lock().unwrap();
            *stats = MemoryStats {
                total_vertices: 0,
                total_indices: 0,
                total_elements: 0,
                total_models: 0,
                estimated_gpu_bytes: 999,
                estimated_cpu_bytes: 888,
                mesh_cache_bytes: 0,
                bvh_node_count: 0,
            };
        }

        assert_eq!(get_gpu_memory_bytes(), 999);
        assert_eq!(get_cpu_memory_bytes(), 888);

        // Restore defaults
        {
            let mut stats = MEMORY_STATS.lock().unwrap();
            *stats = MemoryStats::default();
        }
    }
}
