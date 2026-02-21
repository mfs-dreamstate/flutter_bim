//! 2D Drawing Overlay System
//!
//! Allows overlaying 2D floor plans, drawings, or images on top of the 3D model
//! for comparison and verification workflows.

use super::vertex::Vertex;

/// Drawing overlay representation
pub struct DrawingOverlay {
    pub id: String,
    pub texture: Option<wgpu::Texture>,
    pub texture_view: Option<wgpu::TextureView>,
    pub sampler: Option<wgpu::Sampler>,
    pub bind_group: Option<wgpu::BindGroup>,
    pub width: u32,
    pub height: u32,
    pub position: [f32; 3],  // World position
    pub scale: [f32; 2],     // Width and height in world units
    pub rotation: f32,       // Rotation around Z axis (radians)
    pub opacity: f32,        // 0.0 to 1.0
    pub visible: bool,
}

impl DrawingOverlay {
    pub fn new(id: String) -> Self {
        Self {
            id,
            texture: None,
            texture_view: None,
            sampler: None,
            bind_group: None,
            width: 0,
            height: 0,
            position: [0.0, 0.0, 0.0],
            scale: [10.0, 10.0], // Default 10m x 10m
            rotation: 0.0,
            opacity: 0.7,
            visible: true,
        }
    }

    /// Upload texture data to GPU
    pub fn upload_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba_data: &[u8],
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<(), String> {
        if rgba_data.len() != (width * height * 4) as usize {
            return Err(format!(
                "Invalid data size: expected {} bytes, got {}",
                width * height * 4,
                rgba_data.len()
            ));
        }

        // Create texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Overlay Texture: {}", self.id)),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload data
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        // Create texture view
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("Overlay Sampler: {}", self.id)),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some(&format!("Overlay Bind Group: {}", self.id)),
        });

        self.texture = Some(texture);
        self.texture_view = Some(texture_view);
        self.sampler = Some(sampler);
        self.bind_group = Some(bind_group);
        self.width = width;
        self.height = height;

        Ok(())
    }

    /// Generate quad mesh for this overlay in world space
    pub fn generate_quad_mesh(&self) -> (Vec<Vertex>, Vec<u32>) {
        let half_w = self.scale[0] / 2.0;
        let half_h = self.scale[1] / 2.0;

        // Apply rotation
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        let transform_point = |x: f32, y: f32| -> [f32; 3] {
            let rotated_x = x * cos_r - y * sin_r;
            let rotated_y = x * sin_r + y * cos_r;
            [
                self.position[0] + rotated_x,
                self.position[1] + rotated_y,
                self.position[2],
            ]
        };

        let color = [1.0, 1.0, 1.0, self.opacity];
        let normal = [0.0, 0.0, 1.0];
        let vertices = vec![
            Vertex::new(transform_point(-half_w, -half_h), normal, color),
            Vertex::new(transform_point(half_w, -half_h), normal, color),
            Vertex::new(transform_point(half_w, half_h), normal, color),
            Vertex::new(transform_point(-half_w, half_h), normal, color),
        ];

        let indices = vec![0, 1, 2, 0, 2, 3];

        (vertices, indices)
    }
}

// ---------------------------------------------------------------------------
// Annotation & Markup types
// ---------------------------------------------------------------------------

/// Types of annotations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AnnotationType {
    Pin { label: String, color: [f32; 4] },
    Callout { text: String, background_color: [f32; 4], border_color: [f32; 4] },
    Redline { points: Vec<[f32; 3]>, color: [f32; 4], width: f32 },
    DimensionLine { start: [f32; 3], end: [f32; 3], offset: f32, text_override: Option<String> },
    LeaderLine { anchor: [f32; 3], label_position: [f32; 3], text: String },
    CloudRegion { points: Vec<[f32; 3]>, color: [f32; 4] },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Annotation {
    pub id: String,
    pub position: [f32; 3],
    pub annotation_type: AnnotationType,
    pub visible: bool,
    pub created_at: u64, // unix timestamp millis
}

impl Annotation {
    pub fn new(id: String, position: [f32; 3], annotation_type: AnnotationType) -> Self {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            id,
            position,
            annotation_type,
            visible: true,
            created_at,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnnotationSet {
    pub id: String,
    pub name: String,
    pub annotations: Vec<Annotation>,
    pub visible: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarkupStore {
    pub sets: Vec<AnnotationSet>,
    pub active_set_id: Option<String>,
}

impl MarkupStore {
    pub fn new() -> Self {
        Self {
            sets: Vec::new(),
            active_set_id: None,
        }
    }

    /// Create a new annotation set and return its ID.
    pub fn create_set(&mut self, name: String) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.sets.push(AnnotationSet {
            id: id.clone(),
            name,
            annotations: Vec::new(),
            visible: true,
        });
        id
    }

    /// Delete an annotation set by ID. Returns true if found and removed.
    pub fn delete_set(&mut self, set_id: &str) -> bool {
        let len_before = self.sets.len();
        self.sets.retain(|s| s.id != set_id);
        if self.active_set_id.as_deref() == Some(set_id) {
            self.active_set_id = None;
        }
        self.sets.len() < len_before
    }

    /// Add an annotation to the specified set.
    pub fn add_annotation(&mut self, set_id: &str, annotation: Annotation) -> Result<(), String> {
        let set = self
            .sets
            .iter_mut()
            .find(|s| s.id == set_id)
            .ok_or_else(|| format!("Annotation set not found: {}", set_id))?;
        set.annotations.push(annotation);
        Ok(())
    }

    /// Remove an annotation by ID from the specified set. Returns true if found.
    pub fn remove_annotation(&mut self, set_id: &str, annotation_id: &str) -> bool {
        if let Some(set) = self.sets.iter_mut().find(|s| s.id == set_id) {
            let len_before = set.annotations.len();
            set.annotations.retain(|a| a.id != annotation_id);
            set.annotations.len() < len_before
        } else {
            false
        }
    }

    pub fn get_set(&self, set_id: &str) -> Option<&AnnotationSet> {
        self.sets.iter().find(|s| s.id == set_id)
    }

    pub fn get_set_mut(&mut self, set_id: &str) -> Option<&mut AnnotationSet> {
        self.sets.iter_mut().find(|s| s.id == set_id)
    }

    /// Serialize the entire markup store to pretty JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    /// Deserialize a markup store from JSON.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Failed to parse markup JSON: {}", e))
    }

    /// Compute the 3D Euclidean distance between two points and format as "X.XX m".
    pub fn compute_dimension_text(start: [f32; 3], end: [f32; 3]) -> String {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        format!("{:.2} m", distance)
    }
}
