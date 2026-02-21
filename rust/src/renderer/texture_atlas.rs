//! Texture atlas management for material rendering.
//!
//! Packs multiple small textures into a large atlas texture,
//! reducing draw calls and texture binding changes.
//!
//! Uses the shelf-next-fit algorithm: maintains horizontal "shelves" (rows)
//! of varying heights. Each new texture is placed on the first shelf that fits,
//! or a new shelf is created.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Rectangle in the atlas (pixel coordinates).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AtlasRect {
    /// X offset in pixels from the left edge of the atlas.
    pub x: u32,
    /// Y offset in pixels from the top edge of the atlas.
    pub y: u32,
    /// Width of the rectangle in pixels.
    pub width: u32,
    /// Height of the rectangle in pixels.
    pub height: u32,
}

/// UV coordinates for a sub-texture within the atlas.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AtlasUV {
    /// Minimum U coordinate (left edge, 0.0..1.0).
    pub u_min: f32,
    /// Minimum V coordinate (top edge, 0.0..1.0).
    pub v_min: f32,
    /// Maximum U coordinate (right edge, 0.0..1.0).
    pub u_max: f32,
    /// Maximum V coordinate (bottom edge, 0.0..1.0).
    pub v_max: f32,
}

/// A sub-texture entry in the atlas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasEntry {
    /// Unique identifier for this entry.
    pub id: String,
    /// Pixel-space rectangle in the atlas.
    pub rect: AtlasRect,
    /// Normalized UV coordinates for sampling.
    pub uv: AtlasUV,
}

/// Atlas packing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasConfig {
    /// Atlas texture width in pixels (default 2048).
    pub width: u32,
    /// Atlas texture height in pixels (default 2048).
    pub height: u32,
    /// Padding pixels between entries (default 2).
    pub padding: u32,
    /// Maximum size for a single entry in pixels (default 512).
    pub max_entry_size: u32,
    /// Whether to rotate entries for better packing (default false).
    pub allow_rotation: bool,
}

impl Default for AtlasConfig {
    fn default() -> Self {
        Self {
            width: 2048,
            height: 2048,
            padding: 2,
            max_entry_size: 512,
            allow_rotation: false,
        }
    }
}

/// A horizontal shelf (row) in the atlas.
pub struct Shelf {
    /// Y offset of this shelf in the atlas.
    pub y: u32,
    /// Height of this shelf (determined by the tallest entry placed on it).
    pub height: u32,
    /// Next available X position on this shelf.
    pub x_cursor: u32,
}

/// Shelf-based texture atlas packer.
///
/// Uses the shelf-next-fit algorithm: maintains horizontal "shelves" (rows)
/// of varying heights. Each new texture is placed on the first shelf that fits,
/// or a new shelf is created.
pub struct TextureAtlas {
    /// Packing configuration.
    pub config: AtlasConfig,
    /// All packed entries.
    pub entries: Vec<AtlasEntry>,
    /// Horizontal shelves used for packing.
    pub shelves: Vec<Shelf>,
    /// RGBA pixel data (width * height * 4 bytes).
    pub pixels: Vec<u8>,
    /// Fraction of atlas area currently used (0.0..1.0).
    pub utilization: f32,
}

impl TextureAtlas {
    /// Create an empty atlas with cleared (transparent black) pixels.
    pub fn new(config: AtlasConfig) -> Self {
        let pixel_count = (config.width * config.height * 4) as usize;
        Self {
            pixels: vec![0u8; pixel_count],
            entries: Vec::new(),
            shelves: Vec::new(),
            utilization: 0.0,
            config,
        }
    }

    /// Pack a sub-texture into the atlas using shelf-next-fit.
    ///
    /// Returns the UV coordinates for the packed texture on success, or an
    /// error string if the texture cannot fit.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for this entry.
    /// * `width` - Width of the sub-texture in pixels.
    /// * `height` - Height of the sub-texture in pixels.
    /// * `rgba_pixels` - RGBA pixel data (must be width * height * 4 bytes).
    pub fn pack(
        &mut self,
        id: &str,
        width: u32,
        height: u32,
        rgba_pixels: &[u8],
    ) -> Result<AtlasUV, String> {
        let expected_size = (width * height * 4) as usize;
        if rgba_pixels.len() != expected_size {
            return Err(format!(
                "Invalid pixel data size: expected {} bytes, got {}",
                expected_size,
                rgba_pixels.len()
            ));
        }

        if width > self.config.max_entry_size || height > self.config.max_entry_size {
            return Err(format!(
                "Entry {}x{} exceeds max_entry_size {}",
                width, height, self.config.max_entry_size
            ));
        }

        let padded_w = width + self.config.padding;
        let padded_h = height + self.config.padding;

        if padded_w > self.config.width || padded_h > self.config.height {
            return Err(format!(
                "Entry {}x{} (with padding) exceeds atlas size {}x{}",
                padded_w, padded_h, self.config.width, self.config.height
            ));
        }

        // Try to find a shelf that fits
        let mut placed = false;
        let mut place_x = 0u32;
        let mut place_y = 0u32;

        for shelf in &mut self.shelves {
            if shelf.height >= padded_h && shelf.x_cursor + padded_w <= self.config.width {
                place_x = shelf.x_cursor;
                place_y = shelf.y;
                shelf.x_cursor += padded_w;
                placed = true;
                break;
            }
        }

        // If no shelf fits, create a new one
        if !placed {
            let shelf_y = if self.shelves.is_empty() {
                0
            } else {
                let last = self.shelves.last().unwrap();
                last.y + last.height
            };

            if shelf_y + padded_h > self.config.height {
                return Err(format!(
                    "Atlas full: cannot fit {}x{} entry (no vertical space remaining)",
                    width, height
                ));
            }

            place_x = 0;
            place_y = shelf_y;

            self.shelves.push(Shelf {
                y: shelf_y,
                height: padded_h,
                x_cursor: padded_w,
            });
        }

        // Copy pixel data into the atlas
        self.blit_pixels(place_x, place_y, width, height, rgba_pixels);

        // Compute UV coordinates
        let uv = AtlasUV {
            u_min: place_x as f32 / self.config.width as f32,
            v_min: place_y as f32 / self.config.height as f32,
            u_max: (place_x + width) as f32 / self.config.width as f32,
            v_max: (place_y + height) as f32 / self.config.height as f32,
        };

        let rect = AtlasRect {
            x: place_x,
            y: place_y,
            width,
            height,
        };

        self.entries.push(AtlasEntry {
            id: id.to_string(),
            rect,
            uv,
        });

        self.recalculate_utilization();

        Ok(uv)
    }

    /// Pack a solid color swatch into the atlas.
    ///
    /// Useful for materials that have a flat color without a texture map.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for this entry.
    /// * `color` - RGBA color bytes.
    /// * `size` - Width and height of the swatch in pixels.
    pub fn pack_color(
        &mut self,
        id: &str,
        color: [u8; 4],
        size: u32,
    ) -> Result<AtlasUV, String> {
        let pixel_count = (size * size) as usize;
        let mut rgba = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            rgba.extend_from_slice(&color);
        }
        self.pack(id, size, size, &rgba)
    }

    /// Look up UV coordinates by entry ID.
    pub fn get_uv(&self, id: &str) -> Option<AtlasUV> {
        self.entries.iter().find(|e| e.id == id).map(|e| e.uv)
    }

    /// Remove an entry by ID.
    ///
    /// Marks the space as available by clearing the pixels to transparent black,
    /// but does not defragment. Returns `true` if the entry was found and removed.
    pub fn remove(&mut self, id: &str) -> bool {
        let pos = self.entries.iter().position(|e| e.id == id);
        if let Some(idx) = pos {
            let entry = self.entries.remove(idx);
            // Clear the pixel region
            self.clear_rect(entry.rect.x, entry.rect.y, entry.rect.width, entry.rect.height);
            self.recalculate_utilization();
            true
        } else {
            false
        }
    }

    /// Repack all entries to reclaim fragmented space.
    ///
    /// Rebuilds the shelves from scratch by re-inserting all entries sorted by
    /// height (tallest first) for better packing. Returns the number of bytes
    /// reclaimed.
    pub fn defragment(&mut self) -> usize {
        if self.entries.is_empty() {
            return 0;
        }

        let old_used = self.used_area();

        // Collect all entry data (id, width, height, pixel data)
        let mut entry_data: Vec<(String, u32, u32, Vec<u8>)> = Vec::new();
        for entry in &self.entries {
            let r = entry.rect;
            let mut pixels = vec![0u8; (r.width * r.height * 4) as usize];
            for row in 0..r.height {
                let src_offset =
                    ((r.y + row) * self.config.width + r.x) as usize * 4;
                let dst_offset = (row * r.width) as usize * 4;
                let row_bytes = (r.width * 4) as usize;
                pixels[dst_offset..dst_offset + row_bytes]
                    .copy_from_slice(&self.pixels[src_offset..src_offset + row_bytes]);
            }
            entry_data.push((entry.id.clone(), r.width, r.height, pixels));
        }

        // Sort by height descending for better shelf packing
        entry_data.sort_by(|a, b| b.2.cmp(&a.2));

        // Reset atlas state
        self.entries.clear();
        self.shelves.clear();
        self.pixels.fill(0);

        // Re-pack all entries
        for (id, w, h, pixels) in &entry_data {
            // Ignore errors during defrag (entries that previously fit should still fit)
            let _ = self.pack(&id, *w, *h, pixels);
        }

        let new_used = self.used_area();
        if old_used > new_used {
            ((old_used - new_used) * 4) as usize
        } else {
            0
        }
    }

    /// Current space utilization as a fraction (0.0..1.0).
    pub fn utilization(&self) -> f32 {
        self.utilization
    }

    /// Approximate remaining pixels available in the atlas.
    ///
    /// This is an estimate based on shelf space; actual available area depends
    /// on entry sizes and fragmentation.
    pub fn remaining_area(&self) -> u32 {
        let total = self.config.width * self.config.height;
        let used = self.used_area();
        total.saturating_sub(used)
    }

    /// Number of entries currently packed in the atlas.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get raw RGBA pixel data for GPU upload.
    pub fn get_atlas_pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Resize the atlas to new dimensions and repack all entries.
    ///
    /// All existing entries are preserved if they fit in the new size.
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        // Collect all entry data
        let mut entry_data: Vec<(String, u32, u32, Vec<u8>)> = Vec::new();
        for entry in &self.entries {
            let r = entry.rect;
            let mut pixels = vec![0u8; (r.width * r.height * 4) as usize];
            for row in 0..r.height {
                let src_offset =
                    ((r.y + row) * self.config.width + r.x) as usize * 4;
                let dst_offset = (row * r.width) as usize * 4;
                let row_bytes = (r.width * 4) as usize;
                pixels[dst_offset..dst_offset + row_bytes]
                    .copy_from_slice(&self.pixels[src_offset..src_offset + row_bytes]);
            }
            entry_data.push((entry.id.clone(), r.width, r.height, pixels));
        }

        // Sort by height descending
        entry_data.sort_by(|a, b| b.2.cmp(&a.2));

        // Reset with new dimensions
        self.config.width = new_width;
        self.config.height = new_height;
        self.entries.clear();
        self.shelves.clear();
        self.pixels = vec![0u8; (new_width * new_height * 4) as usize];
        self.utilization = 0.0;

        // Re-pack
        for (id, w, h, pixels) in &entry_data {
            let _ = self.pack(&id, *w, *h, pixels);
        }
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Copy RGBA pixel data into the atlas at the given position.
    fn blit_pixels(&mut self, x: u32, y: u32, width: u32, height: u32, rgba: &[u8]) {
        let atlas_w = self.config.width;
        for row in 0..height {
            let src_offset = (row * width * 4) as usize;
            let dst_offset = ((y + row) * atlas_w + x) as usize * 4;
            let row_bytes = (width * 4) as usize;
            self.pixels[dst_offset..dst_offset + row_bytes]
                .copy_from_slice(&rgba[src_offset..src_offset + row_bytes]);
        }
    }

    /// Clear a rectangular region to transparent black.
    fn clear_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        let atlas_w = self.config.width;
        for row in 0..height {
            let offset = ((y + row) * atlas_w + x) as usize * 4;
            let row_bytes = (width * 4) as usize;
            for byte in &mut self.pixels[offset..offset + row_bytes] {
                *byte = 0;
            }
        }
    }

    /// Calculate the total used area from all entries (in pixels).
    fn used_area(&self) -> u32 {
        self.entries
            .iter()
            .map(|e| e.rect.width * e.rect.height)
            .sum()
    }

    /// Recalculate the utilization ratio.
    fn recalculate_utilization(&mut self) {
        let total = self.config.width * self.config.height;
        if total == 0 {
            self.utilization = 0.0;
        } else {
            self.utilization = self.used_area() as f32 / total as f32;
        }
    }
}

// ---------------------------------------------------------------------------
// Helper texture generators
// ---------------------------------------------------------------------------

/// Generate a checkerboard pattern texture (RGBA).
///
/// Useful as a placeholder for missing textures. The pattern alternates
/// between `color_a` and `color_b` in a grid of 8x8 pixel cells.
///
/// # Arguments
/// * `size` - Width and height of the texture in pixels.
/// * `color_a` - First checkerboard color (RGBA).
/// * `color_b` - Second checkerboard color (RGBA).
pub fn generate_checkerboard(size: u32, color_a: [u8; 4], color_b: [u8; 4]) -> Vec<u8> {
    let cell_size = 8u32;
    let pixel_count = (size * size) as usize;
    let mut pixels = Vec::with_capacity(pixel_count * 4);

    for y in 0..size {
        for x in 0..size {
            let checker = ((x / cell_size) + (y / cell_size)) % 2 == 0;
            let color = if checker { color_a } else { color_b };
            pixels.extend_from_slice(&color);
        }
    }

    pixels
}

/// Generate a gradient texture (RGBA).
///
/// Creates a linear gradient from `color_start` to `color_end` along the
/// specified axis.
///
/// # Arguments
/// * `width` - Texture width in pixels.
/// * `height` - Texture height in pixels.
/// * `color_start` - Gradient start color (RGBA).
/// * `color_end` - Gradient end color (RGBA).
/// * `horizontal` - If `true`, gradient runs left-to-right; otherwise top-to-bottom.
pub fn generate_gradient(
    width: u32,
    height: u32,
    color_start: [u8; 4],
    color_end: [u8; 4],
    horizontal: bool,
) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut pixels = Vec::with_capacity(pixel_count * 4);

    for y in 0..height {
        for x in 0..width {
            let t = if horizontal {
                if width <= 1 {
                    0.0
                } else {
                    x as f32 / (width - 1) as f32
                }
            } else {
                if height <= 1 {
                    0.0
                } else {
                    y as f32 / (height - 1) as f32
                }
            };

            let r = lerp_u8(color_start[0], color_end[0], t);
            let g = lerp_u8(color_start[1], color_end[1], t);
            let b = lerp_u8(color_start[2], color_end[2], t);
            let a = lerp_u8(color_start[3], color_end[3], t);
            pixels.extend_from_slice(&[r, g, b, a]);
        }
    }

    pixels
}

/// Linear interpolation between two u8 values.
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let result = a as f32 * (1.0 - t) + b as f32 * t;
    result.round().clamp(0.0, 255.0) as u8
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_config_default() {
        let config = AtlasConfig::default();
        assert_eq!(config.width, 2048);
        assert_eq!(config.height, 2048);
        assert_eq!(config.padding, 2);
        assert_eq!(config.max_entry_size, 512);
        assert!(!config.allow_rotation);
    }

    #[test]
    fn test_atlas_new() {
        let config = AtlasConfig {
            width: 256,
            height: 256,
            ..Default::default()
        };
        let atlas = TextureAtlas::new(config);
        assert_eq!(atlas.entries.len(), 0);
        assert_eq!(atlas.shelves.len(), 0);
        assert_eq!(atlas.pixels.len(), 256 * 256 * 4);
        assert!((atlas.utilization - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_atlas_pack_single() {
        let config = AtlasConfig {
            width: 256,
            height: 256,
            padding: 0,
            max_entry_size: 256,
            allow_rotation: false,
        };
        let mut atlas = TextureAtlas::new(config);

        // Create a 16x16 red texture
        let red_pixels: Vec<u8> = (0..16 * 16)
            .flat_map(|_| [255u8, 0, 0, 255])
            .collect();

        let uv = atlas.pack("red_tile", 16, 16, &red_pixels).unwrap();
        assert!((uv.u_min - 0.0).abs() < 0.001);
        assert!((uv.v_min - 0.0).abs() < 0.001);
        assert!((uv.u_max - 16.0 / 256.0).abs() < 0.001);
        assert!((uv.v_max - 16.0 / 256.0).abs() < 0.001);

        assert_eq!(atlas.entry_count(), 1);

        // Verify pixel data was copied
        assert_eq!(atlas.pixels[0], 255); // R
        assert_eq!(atlas.pixels[1], 0);   // G
        assert_eq!(atlas.pixels[2], 0);   // B
        assert_eq!(atlas.pixels[3], 255); // A
    }

    #[test]
    fn test_atlas_pack_multiple() {
        let config = AtlasConfig {
            width: 256,
            height: 256,
            padding: 2,
            max_entry_size: 256,
            allow_rotation: false,
        };
        let mut atlas = TextureAtlas::new(config);

        let pixels_a: Vec<u8> = vec![255; 32 * 32 * 4];
        let pixels_b: Vec<u8> = vec![128; 32 * 32 * 4];
        let pixels_c: Vec<u8> = vec![64; 16 * 16 * 4];

        let uv_a = atlas.pack("a", 32, 32, &pixels_a).unwrap();
        let uv_b = atlas.pack("b", 32, 32, &pixels_b).unwrap();
        let uv_c = atlas.pack("c", 16, 16, &pixels_c).unwrap();

        assert_eq!(atlas.entry_count(), 3);

        // First entry at origin
        assert!((uv_a.u_min - 0.0).abs() < 0.001);
        assert!((uv_a.v_min - 0.0).abs() < 0.001);

        // Second entry should be to the right of the first (on the same shelf)
        assert!(uv_b.u_min > uv_a.u_max - 0.001);

        // Third entry could be on the same shelf or a new one
        assert!(atlas.entry_count() == 3);
        // All UVs should be valid (within 0..1)
        for uv in [uv_a, uv_b, uv_c] {
            assert!(uv.u_min >= 0.0 && uv.u_min <= 1.0);
            assert!(uv.v_min >= 0.0 && uv.v_min <= 1.0);
            assert!(uv.u_max >= 0.0 && uv.u_max <= 1.0);
            assert!(uv.v_max >= 0.0 && uv.v_max <= 1.0);
        }
    }

    #[test]
    fn test_atlas_pack_color_swatch() {
        let config = AtlasConfig {
            width: 256,
            height: 256,
            padding: 0,
            max_entry_size: 256,
            allow_rotation: false,
        };
        let mut atlas = TextureAtlas::new(config);

        let uv = atlas
            .pack_color("blue_material", [0, 0, 255, 255], 8)
            .unwrap();

        assert_eq!(atlas.entry_count(), 1);
        assert!(uv.u_max > uv.u_min);
        assert!(uv.v_max > uv.v_min);

        // Verify pixel data: first pixel should be blue
        assert_eq!(atlas.pixels[0], 0);   // R
        assert_eq!(atlas.pixels[1], 0);   // G
        assert_eq!(atlas.pixels[2], 255); // B
        assert_eq!(atlas.pixels[3], 255); // A
    }

    #[test]
    fn test_atlas_get_uv() {
        let config = AtlasConfig {
            width: 256,
            height: 256,
            padding: 0,
            max_entry_size: 256,
            allow_rotation: false,
        };
        let mut atlas = TextureAtlas::new(config);

        let pixels: Vec<u8> = vec![255; 16 * 16 * 4];
        let expected_uv = atlas.pack("my_texture", 16, 16, &pixels).unwrap();

        let found_uv = atlas.get_uv("my_texture").unwrap();
        assert!((found_uv.u_min - expected_uv.u_min).abs() < f32::EPSILON);
        assert!((found_uv.v_min - expected_uv.v_min).abs() < f32::EPSILON);
        assert!((found_uv.u_max - expected_uv.u_max).abs() < f32::EPSILON);
        assert!((found_uv.v_max - expected_uv.v_max).abs() < f32::EPSILON);

        // Non-existent entry
        assert!(atlas.get_uv("missing").is_none());
    }

    #[test]
    fn test_atlas_utilization() {
        let config = AtlasConfig {
            width: 100,
            height: 100,
            padding: 0,
            max_entry_size: 100,
            allow_rotation: false,
        };
        let mut atlas = TextureAtlas::new(config);

        // Pack a 50x50 texture into a 100x100 atlas => 25% utilization
        let pixels: Vec<u8> = vec![255; 50 * 50 * 4];
        atlas.pack("half", 50, 50, &pixels).unwrap();

        let util = atlas.utilization();
        assert!((util - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_atlas_remaining_area() {
        let config = AtlasConfig {
            width: 100,
            height: 100,
            padding: 0,
            max_entry_size: 100,
            allow_rotation: false,
        };
        let mut atlas = TextureAtlas::new(config);

        assert_eq!(atlas.remaining_area(), 10000);

        let pixels: Vec<u8> = vec![255; 50 * 50 * 4];
        atlas.pack("quarter", 50, 50, &pixels).unwrap();

        assert_eq!(atlas.remaining_area(), 7500);
    }

    #[test]
    fn test_atlas_remove() {
        let config = AtlasConfig {
            width: 256,
            height: 256,
            padding: 0,
            max_entry_size: 256,
            allow_rotation: false,
        };
        let mut atlas = TextureAtlas::new(config);

        let pixels: Vec<u8> = vec![255; 16 * 16 * 4];
        atlas.pack("to_remove", 16, 16, &pixels).unwrap();
        assert_eq!(atlas.entry_count(), 1);

        let removed = atlas.remove("to_remove");
        assert!(removed);
        assert_eq!(atlas.entry_count(), 0);
        assert!(atlas.get_uv("to_remove").is_none());

        // Verify pixels were cleared
        assert_eq!(atlas.pixels[0], 0);

        // Removing non-existent entry
        let removed_again = atlas.remove("to_remove");
        assert!(!removed_again);
    }

    #[test]
    fn test_atlas_pack_overflow() {
        let config = AtlasConfig {
            width: 32,
            height: 32,
            padding: 0,
            max_entry_size: 32,
            allow_rotation: false,
        };
        let mut atlas = TextureAtlas::new(config);

        // Fill the atlas completely
        let pixels: Vec<u8> = vec![255; 32 * 32 * 4];
        atlas.pack("full", 32, 32, &pixels).unwrap();

        // Try to pack another entry — should fail
        let small_pixels: Vec<u8> = vec![128; 4 * 4 * 4];
        let result = atlas.pack("overflow", 4, 4, &small_pixels);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_checkerboard() {
        let size = 16;
        let white = [255, 255, 255, 255];
        let black = [0, 0, 0, 255];
        let pixels = generate_checkerboard(size, white, black);

        assert_eq!(pixels.len(), (size * size * 4) as usize);

        // Top-left pixel (cell 0,0) should be color_a (white)
        assert_eq!(&pixels[0..4], &white);

        // Pixel at (8, 0) should be color_b (black) — next cell column
        let offset = (8 * 4) as usize;
        assert_eq!(&pixels[offset..offset + 4], &black);

        // Pixel at (8, 8) should be color_a again (both coordinates in odd cells)
        let offset2 = ((8 * size + 8) * 4) as usize;
        assert_eq!(&pixels[offset2..offset2 + 4], &white);
    }

    #[test]
    fn test_generate_gradient() {
        let width = 256;
        let height = 1;
        let start = [0, 0, 0, 255];
        let end = [255, 255, 255, 255];

        // Horizontal gradient
        let pixels = generate_gradient(width, height, start, end, true);
        assert_eq!(pixels.len(), (width * height * 4) as usize);

        // First pixel should be black
        assert_eq!(pixels[0], 0);
        assert_eq!(pixels[1], 0);
        assert_eq!(pixels[2], 0);

        // Last pixel should be white
        let last = ((width - 1) * 4) as usize;
        assert_eq!(pixels[last], 255);
        assert_eq!(pixels[last + 1], 255);
        assert_eq!(pixels[last + 2], 255);

        // Middle pixel should be approximately 128
        let mid = ((width / 2) * 4) as usize;
        assert!((pixels[mid] as i32 - 128).abs() <= 1);

        // Vertical gradient
        let v_pixels = generate_gradient(1, 256, start, end, false);
        assert_eq!(v_pixels.len(), 256 * 4);
        assert_eq!(v_pixels[0], 0); // top = black
        let v_last = (255 * 4) as usize;
        assert_eq!(v_pixels[v_last], 255); // bottom = white
    }
}
