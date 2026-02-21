use flutter_rust_bridge::frb;

use super::state::with_state;

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

/// Upload a 2D drawing/floor plan as an overlay texture
pub async fn upload_drawing_overlay(
    id: String,
    width: u32,
    height: u32,
    _rgba_pixels: Vec<u8>,
) -> Result<(), String> {
    tracing::info!("Uploading drawing overlay: {} ({}x{})", id, width, height);
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
    Ok(())
}

/// Set overlay opacity (0.0 to 1.0)
#[frb(sync)]
pub fn set_overlay_opacity(id: String, opacity: f32) -> Result<(), String> {
    let opacity = opacity.clamp(0.0, 1.0);
    tracing::info!("Set overlay opacity: {} = {}", id, opacity);
    Ok(())
}

/// Set overlay visibility
#[frb(sync)]
pub fn set_overlay_visible(id: String, visible: bool) -> Result<(), String> {
    tracing::info!("Set overlay visible: {} = {}", id, visible);
    Ok(())
}

/// Remove an overlay
#[frb(sync)]
pub fn remove_overlay(id: String) -> Result<(), String> {
    tracing::info!("Remove overlay: {}", id);
    Ok(())
}

/// Set view mode
#[frb(sync)]
pub fn set_view_mode(mode: String) -> Result<(), String> {
    with_state(|s| {
        s.view_mode = match mode.as_str() {
            "3d" => ViewMode::ThreeD,
            "2d" => ViewMode::TwoD,
            "overlay" => ViewMode::Overlay,
            _ => return Err(format!("Invalid view mode: {}", mode)),
        };
        tracing::info!("Set view mode: {}", mode);
        Ok(())
    })
}

/// Get current view mode
#[frb(sync)]
pub fn get_view_mode() -> String {
    with_state(|s| match s.view_mode {
        ViewMode::ThreeD => "3d".to_string(),
        ViewMode::TwoD => "2d".to_string(),
        ViewMode::Overlay => "overlay".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Annotation & Markup FFI
// ---------------------------------------------------------------------------

use std::sync::{LazyLock, Mutex};
use crate::renderer::overlay::{Annotation, AnnotationType, MarkupStore};

static MARKUP: LazyLock<Mutex<MarkupStore>> = LazyLock::new(|| Mutex::new(MarkupStore::new()));

fn with_markup<T>(f: impl FnOnce(&mut MarkupStore) -> T) -> T {
    let mut store = MARKUP.lock().unwrap();
    f(&mut store)
}

/// Helper: get the active set ID or return an error.
fn active_set_id() -> Result<String, String> {
    with_markup(|store| {
        store
            .active_set_id
            .clone()
            .ok_or_else(|| "No active annotation set".to_string())
    })
}

/// Helper: current unix timestamp in milliseconds.
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// -- Annotation set management --

/// Create a new annotation set. Returns the set ID.
#[frb(sync)]
pub fn create_annotation_set(name: String) -> String {
    with_markup(|store| store.create_set(name))
}

/// Delete an annotation set by ID.
#[frb(sync)]
pub fn delete_annotation_set(set_id: String) -> Result<(), String> {
    with_markup(|store| {
        if store.delete_set(&set_id) {
            Ok(())
        } else {
            Err(format!("Annotation set not found: {}", set_id))
        }
    })
}

/// List all annotation sets as a JSON array of {id, name, count, visible}.
#[frb(sync)]
pub fn list_annotation_sets() -> String {
    with_markup(|store| {
        let entries: Vec<serde_json::Value> = store
            .sets
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "name": s.name,
                    "count": s.annotations.len(),
                    "visible": s.visible,
                })
            })
            .collect();
        serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
    })
}

/// Set an annotation set's visibility.
#[frb(sync)]
pub fn set_annotation_set_visible(set_id: String, visible: bool) -> Result<(), String> {
    with_markup(|store| {
        let set = store
            .get_set_mut(&set_id)
            .ok_or_else(|| format!("Annotation set not found: {}", set_id))?;
        set.visible = visible;
        Ok(())
    })
}

/// Set the active annotation set for subsequent CRUD operations.
#[frb(sync)]
pub fn set_active_annotation_set(set_id: String) -> Result<(), String> {
    with_markup(|store| {
        if store.get_set(&set_id).is_none() {
            return Err(format!("Annotation set not found: {}", set_id));
        }
        store.active_set_id = Some(set_id);
        Ok(())
    })
}

/// Get the active annotation set ID, or None.
#[frb(sync)]
pub fn get_active_annotation_set() -> Option<String> {
    with_markup(|store| store.active_set_id.clone())
}

// -- Annotation CRUD (operates on active set) --

/// Add a pin annotation to the active set.
#[frb(sync)]
pub fn add_pin_annotation(
    id: String,
    x: f32,
    y: f32,
    z: f32,
    label: String,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
) -> Result<(), String> {
    let set_id = active_set_id()?;
    let annotation = Annotation::new(
        id,
        [x, y, z],
        AnnotationType::Pin {
            label,
            color: [r, g, b, a],
        },
    );
    with_markup(|store| store.add_annotation(&set_id, annotation))
}

/// Add a callout annotation to the active set.
#[frb(sync)]
pub fn add_callout_annotation(
    id: String,
    x: f32,
    y: f32,
    z: f32,
    text: String,
) -> Result<(), String> {
    let set_id = active_set_id()?;
    let annotation = Annotation::new(
        id,
        [x, y, z],
        AnnotationType::Callout {
            text,
            background_color: [1.0, 1.0, 0.8, 0.9],
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
    );
    with_markup(|store| store.add_annotation(&set_id, annotation))
}

/// Add a redline annotation to the active set. Points are passed as a JSON array of [x,y,z].
#[frb(sync)]
pub fn add_redline_annotation(
    id: String,
    points_json: String,
    r: f32,
    g: f32,
    b: f32,
    width: f32,
) -> Result<(), String> {
    let points: Vec<[f32; 3]> = serde_json::from_str(&points_json)
        .map_err(|e| format!("Invalid points JSON: {}", e))?;
    let set_id = active_set_id()?;
    let annotation = Annotation::new(
        id,
        points.first().copied().unwrap_or([0.0, 0.0, 0.0]),
        AnnotationType::Redline {
            points,
            color: [r, g, b, 1.0],
            width,
        },
    );
    with_markup(|store| store.add_annotation(&set_id, annotation))
}

/// Add a dimension line between two 3D points.
#[frb(sync)]
pub fn add_dimension_line(
    id: String,
    x1: f32,
    y1: f32,
    z1: f32,
    x2: f32,
    y2: f32,
    z2: f32,
    offset: f32,
) -> Result<(), String> {
    let start = [x1, y1, z1];
    let end = [x2, y2, z2];
    let set_id = active_set_id()?;
    let midpoint = [
        (x1 + x2) * 0.5,
        (y1 + y2) * 0.5,
        (z1 + z2) * 0.5,
    ];
    let annotation = Annotation::new(
        id,
        midpoint,
        AnnotationType::DimensionLine {
            start,
            end,
            offset,
            text_override: None,
        },
    );
    with_markup(|store| store.add_annotation(&set_id, annotation))
}

/// Add a leader line from an anchor point to a label position.
#[frb(sync)]
pub fn add_leader_line(
    id: String,
    anchor_x: f32,
    anchor_y: f32,
    anchor_z: f32,
    label_x: f32,
    label_y: f32,
    label_z: f32,
    text: String,
) -> Result<(), String> {
    let set_id = active_set_id()?;
    let annotation = Annotation::new(
        id,
        [anchor_x, anchor_y, anchor_z],
        AnnotationType::LeaderLine {
            anchor: [anchor_x, anchor_y, anchor_z],
            label_position: [label_x, label_y, label_z],
            text,
        },
    );
    with_markup(|store| store.add_annotation(&set_id, annotation))
}

/// Remove an annotation by ID from the active set.
#[frb(sync)]
pub fn remove_annotation(annotation_id: String) -> Result<(), String> {
    let set_id = active_set_id()?;
    with_markup(|store| {
        if store.remove_annotation(&set_id, &annotation_id) {
            Ok(())
        } else {
            Err(format!("Annotation not found: {}", annotation_id))
        }
    })
}

/// Get all annotations in the active set as JSON.
#[frb(sync)]
pub fn get_annotations_json() -> String {
    let set_id = match with_markup(|store| store.active_set_id.clone()) {
        Some(id) => id,
        None => return "[]".to_string(),
    };
    with_markup(|store| {
        match store.get_set(&set_id) {
            Some(set) => {
                serde_json::to_string(&set.annotations).unwrap_or_else(|_| "[]".to_string())
            }
            None => "[]".to_string(),
        }
    })
}

// -- Persistence --

/// Export the full markup store as JSON.
#[frb(sync)]
pub fn export_markup_json() -> String {
    with_markup(|store| store.to_json())
}

/// Import markup data from JSON, replacing the current store.
#[frb(sync)]
pub fn import_markup_json(json: String) -> Result<(), String> {
    let imported = MarkupStore::from_json(&json)?;
    with_markup(|store| {
        *store = imported;
        Ok(())
    })
}

// -- Dimension helpers --

/// Compute the 3D Euclidean distance between two points.
#[frb(sync)]
pub fn compute_distance(x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let dz = z2 - z1;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Compute the area of a polygon defined by a JSON array of [x,y,z] points (projected to XY).
/// Uses the shoelace formula on the XY projection.
#[frb(sync)]
pub fn compute_area_from_points(points_json: String) -> Result<f32, String> {
    let points: Vec<[f32; 3]> = serde_json::from_str(&points_json)
        .map_err(|e| format!("Invalid points JSON: {}", e))?;
    if points.len() < 3 {
        return Err("At least 3 points are required to compute area".to_string());
    }
    // Shoelace formula on XY plane
    let mut area = 0.0f32;
    let n = points.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += points[i][0] * points[j][1];
        area -= points[j][0] * points[i][1];
    }
    Ok(area.abs() / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Serialize tests that touch the global MARKUP store to prevent data races.
    static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    /// Reset the global MARKUP store to a clean state.
    fn reset_markup() {
        let mut store = MARKUP.lock().unwrap();
        *store = MarkupStore::new();
    }

    #[test]
    fn test_create_annotation_set() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset_markup();

        let set_id = create_annotation_set("Test Set".to_string());
        assert!(!set_id.is_empty());

        // Verify the set exists in the listing
        let listing = list_annotation_sets();
        assert!(listing.contains("Test Set"));
        assert!(listing.contains(&set_id));
    }

    #[test]
    fn test_annotation_crud() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset_markup();

        let set_id = create_annotation_set("CRUD Test".to_string());
        set_active_annotation_set(set_id.clone()).unwrap();

        // Add a pin annotation
        add_pin_annotation(
            "pin1".to_string(),
            1.0, 2.0, 3.0,
            "Test Pin".to_string(),
            1.0, 0.0, 0.0, 1.0,
        )
        .unwrap();

        // Verify it exists
        let json = get_annotations_json();
        assert!(json.contains("pin1"));
        assert!(json.contains("Test Pin"));

        // Remove it
        remove_annotation("pin1".to_string()).unwrap();

        // Verify it is gone
        let json_after = get_annotations_json();
        assert!(!json_after.contains("pin1"));
    }

    #[test]
    fn test_json_serialization_roundtrip() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset_markup();

        let set_id = create_annotation_set("Roundtrip Set".to_string());
        set_active_annotation_set(set_id.clone()).unwrap();

        add_pin_annotation(
            "rt_pin".to_string(),
            5.0, 6.0, 7.0,
            "Roundtrip".to_string(),
            0.5, 0.5, 0.5, 1.0,
        )
        .unwrap();

        add_dimension_line(
            "rt_dim".to_string(),
            0.0, 0.0, 0.0,
            3.0, 4.0, 0.0,
            0.5,
        )
        .unwrap();

        // Export
        let exported = export_markup_json();
        assert!(exported.contains("Roundtrip Set"));
        assert!(exported.contains("rt_pin"));
        assert!(exported.contains("rt_dim"));

        // Import into a fresh store
        reset_markup();
        import_markup_json(exported.clone()).unwrap();

        // Verify the imported store matches
        let re_exported = export_markup_json();
        // Parse both and compare structurally
        let orig: serde_json::Value = serde_json::from_str(&exported).unwrap();
        let reimp: serde_json::Value = serde_json::from_str(&re_exported).unwrap();
        assert_eq!(orig, reimp);
    }

    #[test]
    fn test_dimension_text_computation() {
        // No global state needed -- purely functional
        // 3-4-5 triangle: distance should be 5.0
        let text = MarkupStore::compute_dimension_text([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]);
        assert_eq!(text, "5.00 m");

        // Pure Z distance
        let text2 = MarkupStore::compute_dimension_text([0.0, 0.0, 0.0], [0.0, 0.0, 10.0]);
        assert_eq!(text2, "10.00 m");

        // Same point
        let text3 = MarkupStore::compute_dimension_text([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]);
        assert_eq!(text3, "0.00 m");
    }

    #[test]
    fn test_area_computation() {
        // No global state needed -- purely functional
        // Unit square: (0,0), (1,0), (1,1), (0,1) -> area = 1.0
        let points_json = "[[0,0,0],[1,0,0],[1,1,0],[0,1,0]]".to_string();
        let area = compute_area_from_points(points_json).unwrap();
        assert!((area - 1.0).abs() < 1e-6);

        // Triangle: (0,0), (4,0), (0,3) -> area = 6.0
        let tri_json = "[[0,0,0],[4,0,0],[0,3,0]]".to_string();
        let tri_area = compute_area_from_points(tri_json).unwrap();
        assert!((tri_area - 6.0).abs() < 1e-6);

        // Too few points
        let bad_json = "[[0,0,0],[1,1,1]]".to_string();
        assert!(compute_area_from_points(bad_json).is_err());
    }
}
