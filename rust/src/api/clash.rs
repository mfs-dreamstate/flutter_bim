//! FFI API for Clash Detection
//!
//! Exposes clash detection to Flutter via flutter_rust_bridge.

use flutter_rust_bridge::frb;
use std::sync::LazyLock;
use std::sync::Mutex;

use super::state::with_state;
use crate::bim::clash::*;

// ============================================================================
// FFI-friendly Data Structures
// ============================================================================

/// Simplified clash result for the Flutter bridge.
#[derive(Debug, Clone)]
pub struct ClashInfo {
    pub id: i32,
    pub element_a_id: i32,
    pub element_a_name: String,
    pub element_a_type: String,
    pub element_b_id: i32,
    pub element_b_name: String,
    pub element_b_type: String,
    /// 0 = Hard, 1 = Soft, 2 = Clearance
    pub clash_type: i32,
    pub distance: f32,
    pub point_x: f32,
    pub point_y: f32,
    pub point_z: f32,
    pub group_name: String,
}

/// Summary of a clash detection run for the Flutter bridge.
#[derive(Debug, Clone)]
pub struct ClashSummary {
    pub total_clashes: i32,
    pub hard_clashes: i32,
    pub soft_clashes: i32,
    pub clearance_clashes: i32,
    pub groups: Vec<ClashGroupInfo>,
}

/// Per-group clash counts.
#[derive(Debug, Clone)]
pub struct ClashGroupInfo {
    pub name: String,
    pub count: i32,
}

// ============================================================================
// Module-level cached results
// ============================================================================

/// Stores the most recent clash detection report so Flutter can query it
/// without re-running the analysis.
static CLASH_REPORT: LazyLock<Mutex<Option<ClashReport>>> = LazyLock::new(|| Mutex::new(None));

fn store_report(report: ClashReport) {
    let mut guard = CLASH_REPORT.lock().unwrap();
    *guard = Some(report);
}

fn with_report<T>(f: impl FnOnce(&ClashReport) -> T) -> Result<T, String> {
    let guard = CLASH_REPORT.lock().unwrap();
    match guard.as_ref() {
        Some(r) => Ok(f(r)),
        None => Err("No clash results available. Run clash detection first.".to_string()),
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn clash_result_to_info(c: &ClashResult) -> ClashInfo {
    ClashInfo {
        id: c.id as i32,
        element_a_id: c.element_a_id,
        element_a_name: c.element_a_name.clone(),
        element_a_type: c.element_a_type.clone(),
        element_b_id: c.element_b_id,
        element_b_name: c.element_b_name.clone(),
        element_b_type: c.element_b_type.clone(),
        clash_type: c.clash_type.as_i32(),
        distance: c.distance,
        point_x: c.clash_point[0],
        point_y: c.clash_point[1],
        point_z: c.clash_point[2],
        group_name: c.group_name.clone().unwrap_or_default(),
    }
}

fn build_summary(report: &ClashReport) -> ClashSummary {
    // Aggregate per-group counts
    let mut group_map = std::collections::HashMap::<String, i32>::new();
    for c in &report.clashes {
        let name = c
            .group_name
            .clone()
            .unwrap_or_else(|| "Generic".to_string());
        *group_map.entry(name).or_insert(0) += 1;
    }
    let groups: Vec<ClashGroupInfo> = group_map
        .into_iter()
        .map(|(name, count)| ClashGroupInfo { name, count })
        .collect();

    ClashSummary {
        total_clashes: report.clashes.len() as i32,
        hard_clashes: report.hard_count as i32,
        soft_clashes: report.soft_count as i32,
        clearance_clashes: report.clearance_count as i32,
        groups,
    }
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Run clash detection across all visible models with the given tolerances.
///
/// Stores results internally; query them with `get_clash_results()`.
#[frb(sync)]
pub fn run_clash_detection(
    hard_tolerance: f32,
    soft_tolerance: f32,
    clearance_tolerance: f32,
    check_same_type: bool,
) -> Result<ClashSummary, String> {
    // Gather elements from all visible models
    let elements = with_state(|s| {
        if s.registry.is_empty() {
            return Err("No models loaded".to_string());
        }
        let mut all: Vec<crate::bim::model::ElementInfo> = Vec::new();
        for (_id, reg) in s.registry.iter_visible() {
            all.extend_from_slice(reg.elements());
        }
        Ok(all)
    })?;

    let settings = ClashSettings {
        hard_tolerance,
        soft_tolerance,
        clearance_tolerance,
        check_same_type,
        type_pairs: Vec::new(),
    };

    let report = detect_clashes(&elements, &settings);
    let summary = build_summary(&report);
    store_report(report);
    Ok(summary)
}

/// Run clash detection filtered to a specific pair of element types.
#[frb(sync)]
pub fn run_clash_detection_filtered(
    type_a: String,
    type_b: String,
    tolerance: f32,
) -> Result<ClashSummary, String> {
    let elements = with_state(|s| {
        if s.registry.is_empty() {
            return Err("No models loaded".to_string());
        }
        let mut all: Vec<crate::bim::model::ElementInfo> = Vec::new();
        for (_id, reg) in s.registry.iter_visible() {
            all.extend_from_slice(reg.elements());
        }
        Ok(all)
    })?;

    let settings = ClashSettings {
        hard_tolerance: 0.0,
        soft_tolerance: tolerance,
        clearance_tolerance: tolerance,
        check_same_type: true,
        type_pairs: vec![(type_a, type_b)],
    };

    let report = detect_clashes(&elements, &settings);
    let summary = build_summary(&report);
    store_report(report);
    Ok(summary)
}

/// Return all clash results from the most recent detection run.
#[frb(sync)]
pub fn get_clash_results() -> Vec<ClashInfo> {
    let guard = CLASH_REPORT.lock().unwrap();
    match guard.as_ref() {
        Some(report) => report.clashes.iter().map(clash_result_to_info).collect(),
        None => Vec::new(),
    }
}

/// Return clashes filtered by group name.
#[frb(sync)]
pub fn get_clashes_by_group(group_name: String) -> Vec<ClashInfo> {
    let guard = CLASH_REPORT.lock().unwrap();
    match guard.as_ref() {
        Some(report) => report
            .clashes
            .iter()
            .filter(|c| c.group_name.as_deref() == Some(group_name.as_str()))
            .map(clash_result_to_info)
            .collect(),
        None => Vec::new(),
    }
}

/// Get total clash count from the last run.
#[frb(sync)]
pub fn get_clash_count() -> i32 {
    let guard = CLASH_REPORT.lock().unwrap();
    guard.as_ref().map_or(0, |r| r.clashes.len() as i32)
}

/// Export the last clash report as a CSV string.
#[frb(sync)]
pub fn export_clash_csv() -> Result<String, String> {
    with_report(|r| export_clash_report_csv(r))
}

/// Export the last clash report as an HTML string.
#[frb(sync)]
pub fn export_clash_html() -> Result<String, String> {
    with_report(|r| export_clash_report_html(r))
}

/// Clear stored clash results.
#[frb(sync)]
pub fn clear_clash_results() -> Result<(), String> {
    let mut guard = CLASH_REPORT.lock().unwrap();
    *guard = None;
    Ok(())
}

/// Navigate the camera to focus on a specific clash point.
///
/// Expands a small AABB around the clash point and calls
/// `fit_camera_to_bounds` on the renderer.
#[frb(sync)]
pub fn navigate_to_clash(clash_id: i32) -> Result<(), String> {
    // Find the clash point from the stored report
    let (point, elem_a_bounds, elem_b_bounds) = {
        let guard = CLASH_REPORT.lock().unwrap();
        let report = guard.as_ref().ok_or("No clash results available")?;
        let clash = report
            .clashes
            .iter()
            .find(|c| c.id as i32 == clash_id)
            .ok_or_else(|| format!("Clash ID {} not found", clash_id))?;
        (clash.clash_point, clash.element_a_id, clash.element_b_id)
    };

    // Build a bounding box that encompasses both clashing elements (or a region
    // around the clash point if elements cannot be found).
    with_state(|s| {
        // Try to find the actual element bounds for a better view
        let mut view_min = [point[0] - 1.0, point[1] - 1.0, point[2] - 1.0];
        let mut view_max = [point[0] + 1.0, point[1] + 1.0, point[2] + 1.0];

        for (_id, reg) in s.registry.iter_visible() {
            for elem in reg.elements() {
                if elem.id == elem_a_bounds || elem.id == elem_b_bounds {
                    view_min[0] = view_min[0].min(elem.bounds.min[0]);
                    view_min[1] = view_min[1].min(elem.bounds.min[1]);
                    view_min[2] = view_min[2].min(elem.bounds.min[2]);
                    view_max[0] = view_max[0].max(elem.bounds.max[0]);
                    view_max[1] = view_max[1].max(elem.bounds.max[1]);
                    view_max[2] = view_max[2].max(elem.bounds.max[2]);
                }
            }
        }

        // Ensure a minimum region size so the camera doesn't zoom in too far
        for i in 0..3 {
            if view_max[i] - view_min[i] < 0.5 {
                let mid = (view_min[i] + view_max[i]) / 2.0;
                view_min[i] = mid - 0.5;
                view_max[i] = mid + 0.5;
            }
        }

        let r = s.renderer()?;
        r.fit_camera_to_bounds(view_min, view_max);
        Ok(())
    })
}
