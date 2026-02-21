//! FFI API for Model Comparison
//!
//! Compares two loaded BIM models by matching elements via GlobalID.

use flutter_rust_bridge::frb;
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

use super::state::with_state;

// ============================================================================
// Visual Comparison Overlay State
// ============================================================================

/// Visual comparison state: tracks which elements are added/removed/modified/unchanged.
struct ComparisonOverlay {
    added_ids: Vec<i32>,      // green
    removed_ids: Vec<i32>,    // red
    modified_ids: Vec<i32>,   // yellow
    unchanged_ids: Vec<i32>,  // gray
}

static COMPARISON_OVERLAY: LazyLock<Mutex<Option<ComparisonOverlay>>> =
    LazyLock::new(|| Mutex::new(None));

// ============================================================================
// FFI Data Structures
// ============================================================================

/// Result of comparing two models.
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// Elements present in model B but not in model A.
    pub added_elements: Vec<ComparisonElement>,
    /// Elements present in model A but not in model B.
    pub removed_elements: Vec<ComparisonElement>,
    /// Elements present in both models but with differences.
    pub modified_elements: Vec<ComparisonModification>,
    /// Number of elements that are identical in both models.
    pub unchanged_count: i32,
}

/// Basic element info for comparison results.
#[derive(Debug, Clone)]
pub struct ComparisonElement {
    pub element_id: i32,
    pub global_id: String,
    pub name: String,
    pub element_type: String,
}

/// An element that exists in both models but has changed.
#[derive(Debug, Clone)]
pub struct ComparisonModification {
    pub global_id: String,
    pub name: String,
    pub element_type: String,
    pub old_element_id: i32,
    pub new_element_id: i32,
    /// Human-readable descriptions of what changed.
    pub changes: Vec<String>,
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Compare two bounding boxes and report differences.
fn compare_bounds(
    old_min: [f32; 3],
    old_max: [f32; 3],
    new_min: [f32; 3],
    new_max: [f32; 3],
) -> Vec<String> {
    let mut changes = Vec::new();

    // Position change (center)
    let old_center = [
        (old_min[0] + old_max[0]) / 2.0,
        (old_min[1] + old_max[1]) / 2.0,
        (old_min[2] + old_max[2]) / 2.0,
    ];
    let new_center = [
        (new_min[0] + new_max[0]) / 2.0,
        (new_min[1] + new_max[1]) / 2.0,
        (new_min[2] + new_max[2]) / 2.0,
    ];
    let dx = new_center[0] - old_center[0];
    let dy = new_center[1] - old_center[1];
    let dz = new_center[2] - old_center[2];
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    if dist > 0.001 {
        changes.push(format!("Position moved by {:.3}m", dist));
    }

    // Size change
    let old_size = [
        old_max[0] - old_min[0],
        old_max[1] - old_min[1],
        old_max[2] - old_min[2],
    ];
    let new_size = [
        new_max[0] - new_min[0],
        new_max[1] - new_min[1],
        new_max[2] - new_min[2],
    ];
    let sx = (new_size[0] - old_size[0]).abs();
    let sy = (new_size[1] - old_size[1]).abs();
    let sz = (new_size[2] - old_size[2]).abs();
    if sx > 0.001 || sy > 0.001 || sz > 0.001 {
        changes.push(format!(
            "Size changed (dx={:.3}, dy={:.3}, dz={:.3})",
            sx, sy, sz
        ));
    }

    changes
}

/// Core comparison logic: compare elements from two models.
fn compare_elements(model_a_id: &str, model_b_id: &str) -> Result<ComparisonResult, String> {
    with_state(|s| {
        let reg_a = s
            .registry
            .get_model(&model_a_id.to_string())
            .ok_or_else(|| format!("Model '{}' not found", model_a_id))?;
        let reg_b = s
            .registry
            .get_model(&model_b_id.to_string())
            .ok_or_else(|| format!("Model '{}' not found", model_b_id))?;

        // Build lookup maps keyed by global_id
        let map_a: HashMap<&str, &crate::bim::model::ElementInfo> = reg_a
            .elements()
            .iter()
            .filter(|e| !e.global_id.is_empty())
            .map(|e| (e.global_id.as_str(), e))
            .collect();

        let map_b: HashMap<&str, &crate::bim::model::ElementInfo> = reg_b
            .elements()
            .iter()
            .filter(|e| !e.global_id.is_empty())
            .map(|e| (e.global_id.as_str(), e))
            .collect();

        let keys_a: HashSet<&str> = map_a.keys().copied().collect();
        let keys_b: HashSet<&str> = map_b.keys().copied().collect();

        // Added: in B but not A
        let added_elements: Vec<ComparisonElement> = keys_b
            .difference(&keys_a)
            .filter_map(|gid| {
                map_b.get(gid).map(|e| ComparisonElement {
                    element_id: e.id,
                    global_id: e.global_id.clone(),
                    name: e.name.clone(),
                    element_type: e.element_type.clone(),
                })
            })
            .collect();

        // Removed: in A but not B
        let removed_elements: Vec<ComparisonElement> = keys_a
            .difference(&keys_b)
            .filter_map(|gid| {
                map_a.get(gid).map(|e| ComparisonElement {
                    element_id: e.id,
                    global_id: e.global_id.clone(),
                    name: e.name.clone(),
                    element_type: e.element_type.clone(),
                })
            })
            .collect();

        // Modified & Unchanged: in both
        let mut modified_elements = Vec::new();
        let mut unchanged_count = 0i32;

        for gid in keys_a.intersection(&keys_b) {
            let ea = map_a[gid];
            let eb = map_b[gid];

            let mut changes = Vec::new();

            // Check name change
            if ea.name != eb.name {
                changes.push(format!("Name changed from '{}' to '{}'", ea.name, eb.name));
            }

            // Check type change
            if ea.element_type != eb.element_type {
                changes.push(format!(
                    "Type changed from '{}' to '{}'",
                    ea.element_type, eb.element_type
                ));
            }

            // Check bounds (position / size)
            let bound_changes =
                compare_bounds(ea.bounds.min, ea.bounds.max, eb.bounds.min, eb.bounds.max);
            changes.extend(bound_changes);

            if changes.is_empty() {
                unchanged_count += 1;
            } else {
                modified_elements.push(ComparisonModification {
                    global_id: gid.to_string(),
                    name: eb.name.clone(),
                    element_type: eb.element_type.clone(),
                    old_element_id: ea.id,
                    new_element_id: eb.id,
                    changes,
                });
            }
        }

        Ok(ComparisonResult {
            added_elements,
            removed_elements,
            modified_elements,
            unchanged_count,
        })
    })
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Compare two loaded models by matching elements via GlobalID.
///
/// Model A is treated as the "old" / baseline model.
/// Model B is treated as the "new" / updated model.
#[frb(sync)]
pub fn compare_models(model_a_id: String, model_b_id: String) -> Result<ComparisonResult, String> {
    compare_elements(&model_a_id, &model_b_id)
}

/// Get just the element IDs of elements added in model B.
#[frb(sync)]
pub fn get_added_element_ids(model_a_id: String, model_b_id: String) -> Result<Vec<i32>, String> {
    let result = compare_elements(&model_a_id, &model_b_id)?;
    Ok(result.added_elements.iter().map(|e| e.element_id).collect())
}

/// Get just the element IDs of elements removed from model A.
#[frb(sync)]
pub fn get_removed_element_ids(model_a_id: String, model_b_id: String) -> Result<Vec<i32>, String> {
    let result = compare_elements(&model_a_id, &model_b_id)?;
    Ok(result
        .removed_elements
        .iter()
        .map(|e| e.element_id)
        .collect())
}

/// Get just the new element IDs of modified elements.
#[frb(sync)]
pub fn get_modified_element_ids(
    model_a_id: String,
    model_b_id: String,
) -> Result<Vec<i32>, String> {
    let result = compare_elements(&model_a_id, &model_b_id)?;
    Ok(result
        .modified_elements
        .iter()
        .map(|e| e.new_element_id)
        .collect())
}

/// Export a CSV change report comparing two models.
#[frb(sync)]
pub fn export_comparison_csv(model_a_id: String, model_b_id: String) -> Result<String, String> {
    let result = compare_elements(&model_a_id, &model_b_id)?;

    let mut csv = String::from("Status,GlobalID,Name,Type,Changes\n");

    for e in &result.added_elements {
        csv.push_str(&format!(
            "Added,{},{},{},\n",
            e.global_id, e.name, e.element_type,
        ));
    }

    for e in &result.removed_elements {
        csv.push_str(&format!(
            "Removed,{},{},{},\n",
            e.global_id, e.name, e.element_type,
        ));
    }

    for m in &result.modified_elements {
        let changes_str = m.changes.join("; ");
        csv.push_str(&format!(
            "Modified,{},{},{},{}\n",
            m.global_id, m.name, m.element_type, changes_str,
        ));
    }

    Ok(csv)
}

// ============================================================================
// Visual Diff Overlay FFI Functions
// ============================================================================

/// Run comparison between two models and store results in the overlay state.
/// Returns JSON summary: `{"added": N, "removed": N, "modified": N, "unchanged": N}`
#[frb(sync)]
pub fn set_comparison_overlay(model_id_a: String, model_id_b: String) -> Result<String, String> {
    let result = compare_elements(&model_id_a, &model_id_b)?;

    let added_ids: Vec<i32> = result.added_elements.iter().map(|e| e.element_id).collect();
    let removed_ids: Vec<i32> = result.removed_elements.iter().map(|e| e.element_id).collect();
    let modified_ids: Vec<i32> = result.modified_elements.iter().map(|e| e.new_element_id).collect();

    // Build unchanged IDs: all element IDs from model B that are not added or modified
    let unchanged_count = result.unchanged_count as usize;
    // We need to collect the unchanged element IDs from model B
    let unchanged_ids: Vec<i32> = with_state(|s| {
        let reg_b = match s.registry.get_model(&model_id_b) {
            Some(m) => m,
            None => return Vec::new(),
        };

        let added_set: HashSet<i32> = added_ids.iter().copied().collect();
        let modified_new_set: HashSet<i32> = modified_ids.iter().copied().collect();

        reg_b.elements().iter()
            .filter(|e| !added_set.contains(&e.id) && !modified_new_set.contains(&e.id))
            .map(|e| e.id)
            .collect()
    });

    let summary = serde_json::json!({
        "added": added_ids.len(),
        "removed": removed_ids.len(),
        "modified": modified_ids.len(),
        "unchanged": unchanged_count,
    });

    let overlay = ComparisonOverlay {
        added_ids,
        removed_ids,
        modified_ids,
        unchanged_ids,
    };

    let mut lock = COMPARISON_OVERLAY.lock().unwrap();
    *lock = Some(overlay);

    serde_json::to_string(&summary).map_err(|e| format!("JSON error: {}", e))
}

/// Clear the comparison overlay state.
#[frb(sync)]
pub fn clear_comparison_overlay() -> Result<(), String> {
    let mut lock = COMPARISON_OVERLAY.lock().unwrap();
    *lock = None;
    Ok(())
}

/// Check if a comparison overlay is currently active.
#[frb(sync)]
pub fn is_comparison_overlay_active() -> bool {
    let lock = COMPARISON_OVERLAY.lock().unwrap();
    lock.is_some()
}

/// Get the comparison status of a specific element.
/// Returns "added", "removed", "modified", "unchanged", or "unknown".
#[frb(sync)]
pub fn get_comparison_element_status(element_id: i32) -> String {
    let lock = COMPARISON_OVERLAY.lock().unwrap();
    match lock.as_ref() {
        None => "unknown".to_string(),
        Some(overlay) => {
            if overlay.added_ids.contains(&element_id) {
                "added".to_string()
            } else if overlay.removed_ids.contains(&element_id) {
                "removed".to_string()
            } else if overlay.modified_ids.contains(&element_id) {
                "modified".to_string()
            } else if overlay.unchanged_ids.contains(&element_id) {
                "unchanged".to_string()
            } else {
                "unknown".to_string()
            }
        }
    }
}

/// Get the comparison overlay colors as JSON for Flutter to apply.
/// Returns: `{"added_ids": [1,2,3], "removed_ids": [4,5], "modified_ids": [6,7], "unchanged_ids": [8,9]}`
#[frb(sync)]
pub fn get_comparison_overlay_colors() -> Result<String, String> {
    let lock = COMPARISON_OVERLAY.lock().unwrap();
    let overlay = lock.as_ref().ok_or("No comparison overlay active")?;

    let json = serde_json::json!({
        "added_ids": overlay.added_ids,
        "removed_ids": overlay.removed_ids,
        "modified_ids": overlay.modified_ids,
        "unchanged_ids": overlay.unchanged_ids,
    });

    serde_json::to_string(&json).map_err(|e| format!("JSON error: {}", e))
}

/// Select all elements with a given comparison status ("added", "removed", "modified").
/// Returns the count of selected elements.
#[frb(sync)]
pub fn navigate_to_comparison_group(status: String) -> Result<usize, String> {
    let ids = {
        let lock = COMPARISON_OVERLAY.lock().unwrap();
        let overlay = lock.as_ref().ok_or("No comparison overlay active")?;

        match status.as_str() {
            "added" => overlay.added_ids.clone(),
            "removed" => overlay.removed_ids.clone(),
            "modified" => overlay.modified_ids.clone(),
            "unchanged" => overlay.unchanged_ids.clone(),
            _ => return Err(format!("Invalid status '{}'. Use: added, removed, modified, unchanged", status)),
        }
    };

    with_state(|s| {
        s.selected_elements.clear();
        for id in &ids {
            s.selected_elements.insert(*id);
        }
        s.selected_element = s.selected_elements.iter().next().copied();
        Ok(s.selected_elements.len())
    })
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    // Serialize all comparison tests to prevent global state races
    static TEST_LOCK: StdMutex<()> = StdMutex::new(());

    #[test]
    fn test_comparison_overlay_lifecycle() {
        let _guard = TEST_LOCK.lock().unwrap();

        // Start clean
        { *COMPARISON_OVERLAY.lock().unwrap() = None; }

        assert!(!is_comparison_overlay_active());
        assert_eq!(get_comparison_element_status(1), "unknown");

        {
            let mut lock = COMPARISON_OVERLAY.lock().unwrap();
            *lock = Some(ComparisonOverlay {
                added_ids: vec![1, 2],
                removed_ids: vec![3],
                modified_ids: vec![4, 5],
                unchanged_ids: vec![6, 7, 8],
            });
        }

        assert!(is_comparison_overlay_active());
        assert_eq!(get_comparison_element_status(1), "added");
        assert_eq!(get_comparison_element_status(3), "removed");
        assert_eq!(get_comparison_element_status(4), "modified");
        assert_eq!(get_comparison_element_status(6), "unchanged");
        assert_eq!(get_comparison_element_status(99), "unknown");

        let colors_json = get_comparison_overlay_colors().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&colors_json).unwrap();
        assert_eq!(parsed["added_ids"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["removed_ids"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["modified_ids"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["unchanged_ids"].as_array().unwrap().len(), 3);

        clear_comparison_overlay().unwrap();
        assert!(!is_comparison_overlay_active());
    }

    #[test]
    fn test_navigate_to_comparison_group_no_overlay() {
        let _guard = TEST_LOCK.lock().unwrap();

        { *COMPARISON_OVERLAY.lock().unwrap() = None; }

        let result = navigate_to_comparison_group("added".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_navigate_to_comparison_group_invalid_status() {
        let _guard = TEST_LOCK.lock().unwrap();

        {
            let mut lock = COMPARISON_OVERLAY.lock().unwrap();
            *lock = Some(ComparisonOverlay {
                added_ids: vec![],
                removed_ids: vec![],
                modified_ids: vec![],
                unchanged_ids: vec![],
            });
        }
        let result = navigate_to_comparison_group("invalid".to_string());
        assert!(result.is_err());

        { *COMPARISON_OVERLAY.lock().unwrap() = None; }
    }
}
