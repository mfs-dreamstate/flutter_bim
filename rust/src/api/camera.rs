use flutter_rust_bridge::frb;

use super::state::with_state;

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
