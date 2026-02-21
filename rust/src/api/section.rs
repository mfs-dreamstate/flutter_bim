use flutter_rust_bridge::frb;

use super::state::with_state;
use std::sync::{LazyLock, Mutex};

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

/// Set the section plane (backward compatible - sets plane index 0)
#[frb(sync)]
pub fn set_section_plane(
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    normal_x: f32,
    normal_y: f32,
    normal_z: f32,
) -> Result<(), String> {
    let length = (normal_x * normal_x + normal_y * normal_y + normal_z * normal_z).sqrt();
    if length < 0.0001 {
        return Err("Normal vector cannot be zero".to_string());
    }

    let normalized_normal = [normal_x / length, normal_y / length, normal_z / length];

    with_state(|s| {
        s.section_plane = Some(SectionPlane {
            origin: [origin_x, origin_y, origin_z],
            normal: normalized_normal,
            enabled: true,
        });

        if let Some(r) = s.renderer.as_mut() {
            r.set_section_plane(Some(([origin_x, origin_y, origin_z], normalized_normal)))?;
        }

        Ok(())
    })
}

/// Enable or disable the section plane
#[frb(sync)]
pub fn set_section_plane_enabled(enabled: bool) -> Result<(), String> {
    with_state(|s| {
        if let Some(ref mut p) = s.section_plane {
            p.enabled = enabled;

            if let Some(r) = s.renderer.as_mut() {
                if enabled {
                    r.set_section_plane(Some((p.origin, p.normal)))?;
                } else {
                    r.set_section_plane(None)?;
                }
            }

            Ok(())
        } else {
            Err("No section plane defined".to_string())
        }
    })
}

/// Clear the section plane
#[frb(sync)]
pub fn clear_section_plane() -> Result<(), String> {
    with_state(|s| {
        s.section_plane = None;

        if let Some(r) = s.renderer.as_mut() {
            r.set_section_plane(None)?;
        }

        Ok(())
    })
}

/// Check if section plane is active
#[frb(sync)]
pub fn is_section_plane_active() -> bool {
    with_state(|s| {
        s.section_plane
            .as_ref()
            .map(|p| p.enabled)
            .unwrap_or(false)
    })
}

/// Set section plane from axis (X=0, Y=1, Z=2) and position
#[frb(sync)]
pub fn set_section_plane_from_axis(axis: i32, position: f32) -> Result<(), String> {
    let (normal_x, normal_y, normal_z) = match axis {
        0 => (1.0, 0.0, 0.0),
        1 => (0.0, 1.0, 0.0),
        2 => (0.0, 0.0, 1.0),
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

// =============================================================================
// Multiple Section Planes API
// =============================================================================

/// Add a new section plane, returns its index (0-5)
#[frb(sync)]
pub fn add_section_plane(
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    normal_x: f32,
    normal_y: f32,
    normal_z: f32,
) -> Result<i32, String> {
    let length = (normal_x * normal_x + normal_y * normal_y + normal_z * normal_z).sqrt();
    if length < 0.0001 {
        return Err("Normal vector cannot be zero".to_string());
    }
    let normalized = [normal_x / length, normal_y / length, normal_z / length];
    let origin = [origin_x, origin_y, origin_z];

    with_state(|s| {
        if let Some(r) = s.renderer.as_mut() {
            let idx = r.add_section_plane(origin, normalized)?;
            Ok(idx as i32)
        } else {
            Err("Renderer not initialized".to_string())
        }
    })
}

/// Remove the section plane at the given index
#[frb(sync)]
pub fn remove_section_plane_at(index: i32) -> Result<(), String> {
    if index < 0 {
        return Err(format!("Invalid index: {}", index));
    }
    with_state(|s| {
        if let Some(r) = s.renderer.as_mut() {
            r.remove_section_plane(index as usize)
        } else {
            Err("Renderer not initialized".to_string())
        }
    })
}

/// Get the number of active section planes
#[frb(sync)]
pub fn get_section_plane_count() -> i32 {
    with_state(|s| {
        if let Some(r) = s.renderer.as_ref() {
            r.get_section_plane_count().unwrap_or(0) as i32
        } else {
            0
        }
    })
}

/// Clear all section planes
#[frb(sync)]
pub fn clear_all_section_planes() -> Result<(), String> {
    with_state(|s| {
        s.section_plane = None;
        if let Some(r) = s.renderer.as_mut() {
            r.clear_section_planes()
        } else {
            Ok(())
        }
    })
}

/// Set multiple section planes from a JSON array of {origin: [x,y,z], normal: [x,y,z]}
#[frb(sync)]
pub fn set_multiple_section_planes(planes_json: String) -> Result<(), String> {
    #[derive(serde::Deserialize)]
    struct PlaneJson {
        origin: [f32; 3],
        normal: [f32; 3],
    }

    let parsed: Vec<PlaneJson> = serde_json::from_str(&planes_json)
        .map_err(|e| format!("Failed to parse planes JSON: {}", e))?;

    if parsed.len() > 6 {
        return Err("Maximum of 6 section planes supported".to_string());
    }

    let planes: Vec<([f32; 3], [f32; 3])> = parsed
        .into_iter()
        .map(|p| {
            let len = (p.normal[0] * p.normal[0]
                + p.normal[1] * p.normal[1]
                + p.normal[2] * p.normal[2])
                .sqrt();
            let normal = if len > 0.0001 {
                [p.normal[0] / len, p.normal[1] / len, p.normal[2] / len]
            } else {
                [0.0, 1.0, 0.0]
            };
            (p.origin, normal)
        })
        .collect();

    with_state(|s| {
        if let Some(r) = s.renderer.as_mut() {
            r.set_section_planes(planes)
        } else {
            Err("Renderer not initialized".to_string())
        }
    })
}

// =============================================================================
// Section Animation
// =============================================================================

/// State for a running section plane animation
#[derive(Debug, Clone)]
struct SectionAnimation {
    axis: i32,
    start_pos: f32,
    end_pos: f32,
    duration: f32,
    elapsed: f32,
}

static SECTION_ANIMATION: LazyLock<Mutex<Option<SectionAnimation>>> =
    LazyLock::new(|| Mutex::new(None));

/// Start a section plane animation along an axis
#[frb(sync)]
pub fn animate_section_plane(
    axis: i32,
    start_pos: f32,
    end_pos: f32,
    duration_secs: f32,
) -> Result<(), String> {
    if !(0..=2).contains(&axis) {
        return Err(format!("Invalid axis: {} (expected 0, 1, or 2)", axis));
    }
    if duration_secs <= 0.0 {
        return Err("Duration must be positive".to_string());
    }

    // Set the initial section plane position
    set_section_plane_from_axis(axis, start_pos)?;

    let mut anim = SECTION_ANIMATION.lock().unwrap();
    *anim = Some(SectionAnimation {
        axis,
        start_pos,
        end_pos,
        duration: duration_secs,
        elapsed: 0.0,
    });

    Ok(())
}

/// Advance the section animation by delta_time seconds.
/// Returns true if the animation is still running, false if finished.
#[frb(sync)]
pub fn tick_section_animation(delta_time: f32) -> bool {
    let mut anim_guard = SECTION_ANIMATION.lock().unwrap();
    let Some(ref mut anim) = *anim_guard else {
        return false;
    };

    anim.elapsed += delta_time;
    let t = (anim.elapsed / anim.duration).min(1.0);
    let pos = anim.start_pos + (anim.end_pos - anim.start_pos) * t;

    // Update the section plane position (ignore errors here, best-effort)
    let axis = anim.axis;
    let _ = set_section_plane_from_axis(axis, pos);

    if t >= 1.0 {
        *anim_guard = None;
        return false;
    }

    true
}

/// Check if a section animation is currently running
#[frb(sync)]
pub fn is_section_animating() -> bool {
    SECTION_ANIMATION
        .lock()
        .unwrap()
        .is_some()
}

// =============================================================================
// Section Box (6-plane clipping region)
// =============================================================================

/// Set section box (min/max corners for 6-plane clipping)
#[frb(sync)]
pub fn set_section_box(
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
) -> Result<(), String> {
    with_state(|s| {
        if let Some(r) = s.renderer.as_mut() {
            r.set_section_box(Some(([min_x, min_y, min_z], [max_x, max_y, max_z])))?;
        }
        Ok(())
    })
}

/// Clear the section box
#[frb(sync)]
pub fn clear_section_box() -> Result<(), String> {
    with_state(|s| {
        if let Some(r) = s.renderer.as_mut() {
            r.set_section_box(None)?;
        }
        Ok(())
    })
}

/// Check if section box is active
#[frb(sync)]
pub fn is_section_box_active() -> bool {
    with_state(|s| {
        s.renderer
            .as_ref()
            .and_then(|r| r.scene.as_ref())
            .map_or(false, |scene| scene.section_box_uniform.enabled > 0.5)
    })
}

/// Set section box from model bounds with padding
#[frb(sync)]
pub fn set_section_box_from_model(padding: f32) -> Result<(), String> {
    with_state(|s| {
        let bounds = s.registry.get_combined_bounds().ok_or("No model bounds")?;
        let min = [
            bounds.min[0] - padding,
            bounds.min[1] - padding,
            bounds.min[2] - padding,
        ];
        let max = [
            bounds.max[0] + padding,
            bounds.max[1] + padding,
            bounds.max[2] + padding,
        ];
        if let Some(r) = s.renderer.as_mut() {
            r.set_section_box(Some((min, max)))?;
        }
        Ok(())
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::renderer::scene::{SectionPlanesUniform, SectionPlaneData, MAX_SECTION_PLANES};

    #[test]
    fn test_section_planes_uniform_default() {
        let u = SectionPlanesUniform::new();
        assert_eq!(u.count, 0);
        for i in 0..MAX_SECTION_PLANES {
            assert_eq!(u.planes[i].enabled, 0.0);
        }
    }

    #[test]
    fn test_section_planes_uniform_set_single() {
        let mut u = SectionPlanesUniform::new();
        u.set([1.0, 2.0, 3.0], [0.0, 1.0, 0.0]);
        assert_eq!(u.count, 1);
        assert_eq!(u.planes[0].origin, [1.0, 2.0, 3.0]);
        assert_eq!(u.planes[0].normal, [0.0, 1.0, 0.0]);
        assert_eq!(u.planes[0].enabled, 1.0);
    }

    #[test]
    fn test_section_planes_uniform_add_remove() {
        let mut u = SectionPlanesUniform::new();

        // Add 3 planes
        let idx0 = u.add_plane([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        let idx1 = u.add_plane([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let idx2 = u.add_plane([2.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        assert_eq!(idx0, Some(0));
        assert_eq!(idx1, Some(1));
        assert_eq!(idx2, Some(2));
        assert_eq!(u.count, 3);

        // Remove middle plane
        assert!(u.remove_plane(1));
        assert_eq!(u.count, 2);
        // Plane 2 should have shifted down to index 1
        assert_eq!(u.planes[1].origin, [2.0, 0.0, 0.0]);
        assert_eq!(u.planes[1].normal, [0.0, 0.0, 1.0]);

        // Invalid removal
        assert!(!u.remove_plane(5));
    }

    #[test]
    fn test_section_planes_uniform_max_capacity() {
        let mut u = SectionPlanesUniform::new();
        for i in 0..MAX_SECTION_PLANES {
            let idx = u.add_plane([i as f32, 0.0, 0.0], [0.0, 1.0, 0.0]);
            assert_eq!(idx, Some(i));
        }
        assert_eq!(u.count, MAX_SECTION_PLANES as u32);
        // Adding a 7th should fail
        assert_eq!(u.add_plane([99.0, 0.0, 0.0], [0.0, 1.0, 0.0]), None);
    }

    #[test]
    fn test_section_planes_uniform_set_planes() {
        let mut u = SectionPlanesUniform::new();
        let planes = vec![
            ([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
            ([1.0, 1.0, 1.0], [0.0, 1.0, 0.0]),
        ];
        u.set_planes(&planes);
        assert_eq!(u.count, 2);
        assert_eq!(u.planes[0].origin, [0.0, 0.0, 0.0]);
        assert_eq!(u.planes[1].origin, [1.0, 1.0, 1.0]);
        // Unused planes should be disabled
        assert_eq!(u.planes[2].enabled, 0.0);
    }

    #[test]
    fn test_section_planes_uniform_disable() {
        let mut u = SectionPlanesUniform::new();
        u.add_plane([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        u.add_plane([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert_eq!(u.count, 2);

        u.disable();
        assert_eq!(u.count, 0);
        assert_eq!(u.planes[0].enabled, 0.0);
        assert_eq!(u.planes[1].enabled, 0.0);
    }

    #[test]
    fn test_section_plane_data_layout() {
        // Ensure GPU-compatible sizing: SectionPlaneData = 32 bytes
        assert_eq!(std::mem::size_of::<SectionPlaneData>(), 32);
        // SectionPlanesUniform = 6 * 32 + 16 = 208 bytes
        assert_eq!(std::mem::size_of::<SectionPlanesUniform>(), 208);
    }

    #[test]
    fn test_section_animation_state() {
        use super::*;

        // Initially not animating
        assert!(!is_section_animating());

        // We can't test animate_section_plane fully without a renderer,
        // but we can test the animation state directly
        {
            let mut anim = SECTION_ANIMATION.lock().unwrap();
            *anim = Some(SectionAnimation {
                axis: 1,
                start_pos: 0.0,
                end_pos: 10.0,
                duration: 1.0,
                elapsed: 0.0,
            });
        }
        assert!(is_section_animating());

        // Clear it
        {
            let mut anim = SECTION_ANIMATION.lock().unwrap();
            *anim = None;
        }
        assert!(!is_section_animating());
    }
}
