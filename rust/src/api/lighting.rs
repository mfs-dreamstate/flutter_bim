use flutter_rust_bridge::frb;

use super::state::with_state;

/// Set the directional light direction (will be normalized)
#[frb(sync)]
pub fn set_light_direction(x: f32, y: f32, z: f32) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.set_light_direction(x, y, z)
    })
}

/// Set the directional light color (RGB, 0.0-1.0)
#[frb(sync)]
pub fn set_light_color(r: f32, g: f32, b: f32) -> Result<(), String> {
    with_state(|s| {
        let renderer = s.renderer()?;
        renderer.set_light_color(r, g, b)
    })
}

/// Set the directional light intensity (0.0+)
#[frb(sync)]
pub fn set_light_intensity(intensity: f32) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.set_light_intensity(intensity)
    })
}

/// Set the ambient light color (RGB, 0.0-1.0)
#[frb(sync)]
pub fn set_ambient_color(r: f32, g: f32, b: f32) -> Result<(), String> {
    with_state(|s| {
        let renderer = s.renderer()?;
        renderer.set_ambient_color(r, g, b)
    })
}

/// Set the render mode
/// 0 = Shaded (default), 1 = Wireframe, 2 = X-Ray
#[frb(sync)]
pub fn set_render_mode(mode: i32) -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        let render_mode = match mode {
            0 => crate::renderer::RenderMode::Shaded,
            1 => crate::renderer::RenderMode::Wireframe,
            2 => crate::renderer::RenderMode::XRay,
            _ => return Err(format!("Invalid render mode: {}", mode)),
        };
        r.set_render_mode(render_mode)
    })
}

/// Get the current render mode
/// Returns: 0 = Shaded, 1 = Wireframe, 2 = X-Ray
#[frb(sync)]
pub fn get_render_mode() -> Result<i32, String> {
    with_state(|s| {
        let r = s.renderer.as_ref().ok_or("Renderer not initialized")?;
        Ok(match r.get_render_mode()? {
            crate::renderer::RenderMode::Shaded => 0,
            crate::renderer::RenderMode::Wireframe => 1,
            crate::renderer::RenderMode::XRay => 2,
        })
    })
}

/// Check if wireframe rendering is supported on this device
#[frb(sync)]
pub fn is_wireframe_supported() -> bool {
    with_state(|s| {
        s.renderer
            .as_ref()
            .map(|r| r.gpu.wireframe_supported())
            .unwrap_or(false)
    })
}

/// Set element color by ID
#[frb(sync)]
pub fn set_element_color(element_id: i32, r: u8, g: u8, b: u8) -> Result<(), String> {
    with_state(|s| {
        let renderer = s.renderer()?;
        renderer.set_element_color(
            element_id as usize,
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
        )
    })
}

/// Reset all element colors to defaults
#[frb(sync)]
pub fn reset_element_colors() -> Result<(), String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.reset_element_colors()
    })
}

/// Color elements by type
#[frb(sync)]
pub fn color_by_type() -> Result<(), String> {
    with_state(|s| {
        if s.registry.is_empty() {
            return Err("No model loaded".to_string());
        }

        let _type_colors: std::collections::HashMap<&str, [u8; 3]> = [
            ("IfcWall", [200, 200, 200]),
            ("IfcSlab", [150, 150, 150]),
            ("IfcBeam", [139, 69, 19]),
            ("IfcColumn", [160, 82, 45]),
            ("IfcDoor", [210, 180, 140]),
            ("IfcWindow", [173, 216, 230]),
            ("IfcStair", [188, 143, 143]),
            ("IfcRoof", [178, 34, 34]),
            ("IfcSpace", [240, 255, 240]),
            ("IfcBuildingElementProxy", [192, 192, 192]),
        ]
        .iter()
        .cloned()
        .collect();

        let _r = s.renderer()?;

        let total_elements: usize = s
            .registry
            .models()
            .values()
            .map(|reg| reg.model.element_count)
            .sum();

        tracing::info!(
            "Color-by-type requested for {} total elements (stub implementation)",
            total_elements
        );
        Ok(())
    })
}
