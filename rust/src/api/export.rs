use flutter_rust_bridge::frb;

use super::state::with_state;

/// Renderer statistics
#[derive(Debug, Clone)]
pub struct RenderStats {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub triangle_count: u32,
    pub vertex_count: u32,
    pub element_count: u32,
}

/// Save current frame as PNG to the given path
pub async fn export_screenshot(path: String) -> Result<(), String> {
    let (image_data, width, height) = with_state(|s| {
        let r = s.renderer()?;
        let data = r.render_frame()?;
        let scene = r
            .scene
            .as_ref()
            .ok_or_else(|| "Scene not initialized".to_string())?;
        Ok::<_, String>((data, scene.width, scene.height))
    })?;

    match image::save_buffer(
        &path,
        &image_data,
        width,
        height,
        image::ColorType::Rgba8,
    ) {
        Ok(_) => {
            tracing::info!("Screenshot saved to: {}", path);
            Ok(())
        }
        Err(e) => Err(format!("Failed to save screenshot: {}", e)),
    }
}

/// Get current frame as RGBA bytes
#[frb(sync)]
pub fn get_current_frame_rgba() -> Result<Vec<u8>, String> {
    with_state(|s| {
        let r = s.renderer()?;
        r.render_frame()
    })
}

/// Get renderer statistics
#[frb(sync)]
pub fn get_render_stats() -> Result<RenderStats, String> {
    with_state(|s| {
        let r = s.renderer.as_ref().ok_or("Renderer not initialized")?;
        let element_count: usize = s
            .registry
            .models()
            .values()
            .map(|reg| reg.model.element_count)
            .sum();

        Ok(RenderStats {
            fps: 60.0,
            frame_time_ms: 16.67,
            triangle_count: r.scene.as_ref().map(|s| s.num_indices / 3).unwrap_or(0),
            vertex_count: 0,
            element_count: element_count as u32,
        })
    })
}
