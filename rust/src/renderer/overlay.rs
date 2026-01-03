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

        let vertices = vec![
            Vertex {
                position: transform_point(-half_w, -half_h),
                color: [1.0, 1.0, 1.0, self.opacity],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: transform_point(half_w, -half_h),
                color: [1.0, 1.0, 1.0, self.opacity],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: transform_point(half_w, half_h),
                color: [1.0, 1.0, 1.0, self.opacity],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: transform_point(-half_w, half_h),
                color: [1.0, 1.0, 1.0, self.opacity],
                normal: [0.0, 0.0, 1.0],
            },
        ];

        let indices = vec![0, 1, 2, 0, 2, 3];

        (vertices, indices)
    }
}
