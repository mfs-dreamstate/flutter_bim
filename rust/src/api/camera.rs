use flutter_rust_bridge::frb;

use crate::bim::geometry::BoundingBox;
use super::state::with_state;

// ----------------------------------------------------------------
// Existing camera controls
// ----------------------------------------------------------------

/// Orbit the camera around the target
#[frb(sync)]
pub fn orbit_camera(delta_x: f32, delta_y: f32) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.orbit_camera(delta_x, delta_y);
        Ok(())
    })
}

/// Zoom the camera in/out
#[frb(sync)]
pub fn zoom_camera(delta: f32) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.zoom_camera(delta);
        Ok(())
    })
}

/// Fit camera to current model bounds (primary model)
#[frb(sync)]
pub fn fit_camera_to_model() -> Result<(), String> {
    with_state(|s| {
        let bounds = {
            let reg_model = s.registry.get_primary_model().ok_or("No model loaded")?;
            reg_model.bounds.ok_or("Model has no bounds")?
        };
        let r = s.renderer()?;
        r.fit_camera_to_bounds(bounds.min, bounds.max);
        Ok(())
    })
}

/// Fit camera to all visible models
#[frb(sync)]
pub fn fit_camera_to_all_models() -> Result<(), String> {
    with_state(|s| {
        if s.registry.is_empty() {
            return Err("No models loaded".to_string());
        }
        let bounds = s
            .registry
            .get_combined_bounds()
            .ok_or("No visible models with bounds")?;
        let r = s.renderer()?;
        r.fit_camera_to_bounds(bounds.min, bounds.max);
        Ok(())
    })
}

// ----------------------------------------------------------------
// Pan camera
// ----------------------------------------------------------------

/// Pan the camera (translate position and target together)
#[frb(sync)]
pub fn pan_camera(delta_x: f32, delta_y: f32) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.camera.pan(delta_x, delta_y);
        Ok(())
    })
}

// ----------------------------------------------------------------
// Get / Set camera state
// ----------------------------------------------------------------

/// Get the current camera position
#[frb(sync)]
pub fn get_camera_position() -> Result<(f32, f32, f32), String> {
    with_state(|s| {
        let r = s.renderer()?;
        let p = r.camera.position();
        Ok((p[0], p[1], p[2]))
    })
}

/// Get the current camera target
#[frb(sync)]
pub fn get_camera_target() -> Result<(f32, f32, f32), String> {
    with_state(|s| {
        let r = s.renderer()?;
        let t = r.camera.target();
        Ok((t[0], t[1], t[2]))
    })
}

/// Set the camera position
#[frb(sync)]
pub fn set_camera_position(x: f32, y: f32, z: f32) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.camera.set_position([x, y, z]);
        Ok(())
    })
}

/// Set the camera target
#[frb(sync)]
pub fn set_camera_target(x: f32, y: f32, z: f32) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.camera.set_target([x, y, z]);
        Ok(())
    })
}

// ----------------------------------------------------------------
// Orthographic projection
// ----------------------------------------------------------------

/// Enable or disable orthographic projection
#[frb(sync)]
pub fn set_orthographic(enabled: bool) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.camera.set_orthographic(enabled);
        Ok(())
    })
}

/// Check if orthographic projection is currently active
#[frb(sync)]
pub fn is_orthographic() -> bool {
    with_state(|s| {
        s.renderer
            .as_ref()
            .map_or(false, |r| r.camera.is_orthographic())
    })
}

// ----------------------------------------------------------------
// Turntable orbit mode
// ----------------------------------------------------------------

/// Enable or disable turntable orbit mode (constrained Y-up rotation)
#[frb(sync)]
pub fn set_turntable_mode(enabled: bool) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.camera.set_turntable_mode(enabled);
        Ok(())
    })
}

/// Check if turntable orbit mode is active
#[frb(sync)]
pub fn is_turntable_mode() -> bool {
    with_state(|s| {
        s.renderer
            .as_ref()
            .map_or(false, |r| r.camera.is_turntable_mode())
    })
}

// ----------------------------------------------------------------
// Walkthrough (first-person) mode
// ----------------------------------------------------------------

/// Enable or disable first-person walkthrough mode
#[frb(sync)]
pub fn set_walkthrough_mode(enabled: bool) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.camera.set_walkthrough_mode(enabled);
        Ok(())
    })
}

/// Move the camera in walkthrough mode (forward/right/up)
#[frb(sync)]
pub fn walk_camera(forward: f32, right: f32, up: f32) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.camera.walk_forward(forward);
        r.camera.walk_right(right);
        r.camera.walk_up(up);
        Ok(())
    })
}

/// Rotate the camera view direction in walkthrough mode (yaw/pitch)
#[frb(sync)]
pub fn look_camera(delta_x: f32, delta_y: f32) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.camera.look_around(delta_x, delta_y);
        Ok(())
    })
}

/// Check if walkthrough mode is active
#[frb(sync)]
pub fn is_walkthrough_mode() -> bool {
    with_state(|s| {
        s.renderer
            .as_ref()
            .map_or(false, |r| r.camera.is_walkthrough_mode())
    })
}

// ----------------------------------------------------------------
// Named viewpoints (save / restore)
// ----------------------------------------------------------------

/// Named camera viewpoint data for the Flutter bridge
#[derive(Debug, Clone)]
pub struct ViewpointData {
    pub name: String,
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub fov: f32,
    pub orthographic: bool,
}

/// Save the current camera state as a named viewpoint
#[frb(sync)]
pub fn save_viewpoint(name: String) -> Result<ViewpointData, String> {
    with_state(|s| {
        let r = s.renderer()?;
        let vp = r.camera.save_viewpoint(name);
        let data = ViewpointData {
            name: vp.name,
            position: vp.position.to_array(),
            target: vp.target.to_array(),
            fov: vp.fov,
            orthographic: vp.orthographic,
        };
        // Store in app state, replacing any existing viewpoint with same name
        s.viewpoints.retain(|v| v.name != data.name);
        s.viewpoints.push(data.clone());
        Ok(data)
    })
}

/// Restore a named viewpoint by name
#[frb(sync)]
pub fn restore_viewpoint(name: String) -> Result<(), String> {
    with_state(|s| {
        let vp_data = s
            .viewpoints
            .iter()
            .find(|v| v.name == name)
            .cloned()
            .ok_or_else(|| format!("Viewpoint '{}' not found", name))?;

        let camera_vp = crate::renderer::camera::CameraViewpoint {
            name: vp_data.name,
            position: glam::Vec3::from_array(vp_data.position),
            target: glam::Vec3::from_array(vp_data.target),
            up: glam::Vec3::Y,
            fov: vp_data.fov,
            orthographic: vp_data.orthographic,
        };

        let r = s.renderer()?;
        r.camera.restore_viewpoint(&camera_vp);
        Ok(())
    })
}

/// Get all saved viewpoints
#[frb(sync)]
pub fn get_viewpoints() -> Vec<ViewpointData> {
    with_state(|s| s.viewpoints.clone())
}

/// Delete a named viewpoint
#[frb(sync)]
pub fn delete_viewpoint(name: String) -> Result<(), String> {
    with_state(|s| {
        let before = s.viewpoints.len();
        s.viewpoints.retain(|v| v.name != name);
        if s.viewpoints.len() == before {
            Err(format!("Viewpoint '{}' not found", name))
        } else {
            Ok(())
        }
    })
}

// ----------------------------------------------------------------
// Smooth animated camera transitions
// ----------------------------------------------------------------

/// Start a smooth animated transition to a new camera position and target
#[frb(sync)]
pub fn animate_camera_to(
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    target_x: f32,
    target_y: f32,
    target_z: f32,
    duration: f32,
) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.camera.start_transition(
            glam::Vec3::new(pos_x, pos_y, pos_z),
            glam::Vec3::new(target_x, target_y, target_z),
            duration,
        );
        Ok(())
    })
}

/// Advance the camera animation by delta_time seconds.
/// Returns true if the animation is still active.
#[frb(sync)]
pub fn tick_camera_animation(delta_time: f32) -> bool {
    with_state(|s| {
        s.renderer
            .as_mut()
            .map_or(false, |r| r.camera.tick_animation(delta_time))
    })
}

/// Check if a camera animation is currently in progress
#[frb(sync)]
pub fn is_camera_animating() -> bool {
    with_state(|s| {
        s.renderer
            .as_ref()
            .map_or(false, |r| r.camera.is_animating())
    })
}

// ----------------------------------------------------------------
// Click-to-zoom: fit camera to elements
// ----------------------------------------------------------------

/// Expand bounds by a padding factor (10% on each axis).
fn pad_bounds(min: [f32; 3], max: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let padding = [
        (max[0] - min[0]) * 0.1,
        (max[1] - min[1]) * 0.1,
        (max[2] - min[2]) * 0.1,
    ];
    (
        [min[0] - padding[0], min[1] - padding[1], min[2] - padding[2]],
        [max[0] + padding[0], max[1] + padding[1], max[2] + padding[2]],
    )
}

/// Fit the camera to a single element's bounding box (with 10% padding).
/// Searches for the element across all visible models.
#[frb(sync)]
pub fn fit_camera_to_element(element_id: i32) -> Result<(), String> {
    with_state(|s| {
        let bounds = find_element_bounds(s, element_id)
            .ok_or_else(|| format!("Element {} not found", element_id))?;

        let (padded_min, padded_max) = pad_bounds(bounds.min, bounds.max);
        let r = s.renderer()?;
        r.fit_camera_to_bounds(padded_min, padded_max);
        Ok(())
    })
}

/// Fit the camera to the combined bounding box of multiple elements (with 10% padding).
#[frb(sync)]
pub fn fit_camera_to_elements(element_ids: Vec<i32>) -> Result<(), String> {
    with_state(|s| {
        if element_ids.is_empty() {
            return Err("No element IDs provided".to_string());
        }

        let id_set: std::collections::HashSet<i32> = element_ids.iter().copied().collect();
        let combined = compute_combined_bounds_for_ids(s, &id_set)
            .ok_or("None of the specified elements were found")?;

        let (padded_min, padded_max) = pad_bounds(combined.min, combined.max);
        let r = s.renderer()?;
        r.fit_camera_to_bounds(padded_min, padded_max);
        Ok(())
    })
}

/// Fit the camera to all elements in a given storey (with 10% padding).
#[frb(sync)]
pub fn fit_camera_to_storey(storey_name: String) -> Result<(), String> {
    with_state(|s| {
        let mut combined: Option<BoundingBox> = None;

        for (_model_id, reg_model) in s.registry.iter_visible() {
            let model = &reg_model.model;

            // Build storey name lookup
            let storey_name_map: std::collections::HashMap<i32, String> = model
                .storeys
                .iter()
                .map(|st| (st.id, st.name.clone()))
                .collect();

            // Find element IDs in the target storey
            let storey_elem_ids: std::collections::HashSet<i32> = model
                .element_to_storey
                .iter()
                .filter_map(|(elem_id, storey_id)| {
                    let sname = storey_name_map
                        .get(storey_id)
                        .cloned()
                        .unwrap_or_default();
                    if sname == storey_name { Some(*elem_id) } else { None }
                })
                .collect();

            for elem in reg_model.elements() {
                if storey_elem_ids.contains(&elem.id) {
                    combined = Some(match combined {
                        None => elem.bounds.clone(),
                        Some(existing) => existing.union(&elem.bounds),
                    });
                }
            }
        }

        let bounds = combined.ok_or_else(|| format!("No elements found in storey '{}'", storey_name))?;
        let (padded_min, padded_max) = pad_bounds(bounds.min, bounds.max);
        let r = s.renderer()?;
        r.fit_camera_to_bounds(padded_min, padded_max);
        Ok(())
    })
}

/// Fit the camera to the combined bounding box of all currently selected elements.
#[frb(sync)]
pub fn fit_camera_to_selected() -> Result<(), String> {
    with_state(|s| {
        if s.selected_elements.is_empty() {
            return Err("No elements selected".to_string());
        }

        let selected = s.selected_elements.clone();
        let combined = compute_combined_bounds_for_ids(s, &selected)
            .ok_or("Selected elements have no geometry")?;

        let (padded_min, padded_max) = pad_bounds(combined.min, combined.max);
        let r = s.renderer()?;
        r.fit_camera_to_bounds(padded_min, padded_max);
        Ok(())
    })
}

/// Fit the camera to all elements of a given type (with 10% padding).
#[frb(sync)]
pub fn fit_camera_to_element_type(element_type: String) -> Result<(), String> {
    with_state(|s| {
        let mut combined: Option<BoundingBox> = None;

        for (_model_id, reg_model) in s.registry.iter_visible() {
            for elem in reg_model.elements() {
                if elem.element_type.eq_ignore_ascii_case(&element_type) {
                    combined = Some(match combined {
                        None => elem.bounds.clone(),
                        Some(existing) => existing.union(&elem.bounds),
                    });
                }
            }
        }

        let bounds = combined.ok_or_else(|| format!("No elements of type '{}' found", element_type))?;
        let (padded_min, padded_max) = pad_bounds(bounds.min, bounds.max);
        let r = s.renderer()?;
        r.fit_camera_to_bounds(padded_min, padded_max);
        Ok(())
    })
}

// ----------------------------------------------------------------
// Internal helpers for camera fitting
// ----------------------------------------------------------------

/// Find a single element's bounding box across all visible models.
fn find_element_bounds(
    s: &super::state::AppState,
    element_id: i32,
) -> Option<BoundingBox> {
    for (_model_id, reg_model) in s.registry.iter_visible() {
        for elem in reg_model.elements() {
            if elem.id == element_id {
                return Some(elem.bounds.clone());
            }
        }
    }
    None
}

/// Compute the combined bounding box for a set of element IDs.
fn compute_combined_bounds_for_ids(
    s: &super::state::AppState,
    ids: &std::collections::HashSet<i32>,
) -> Option<BoundingBox> {
    let mut combined: Option<BoundingBox> = None;
    for (_model_id, reg_model) in s.registry.iter_visible() {
        for elem in reg_model.elements() {
            if ids.contains(&elem.id) {
                combined = Some(match combined {
                    None => elem.bounds.clone(),
                    Some(existing) => existing.union(&elem.bounds),
                });
            }
        }
    }
    combined
}
