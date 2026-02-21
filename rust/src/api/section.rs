use flutter_rust_bridge::frb;

use super::state::with_state;

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

/// Set the section plane
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
