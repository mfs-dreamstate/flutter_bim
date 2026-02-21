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
