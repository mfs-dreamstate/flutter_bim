//! Scene Rendering
//!
//! Manages offscreen rendering and frame generation.

use super::{camera::Camera, pipeline::{RenderPipeline, RenderMode, MSAA_SAMPLE_COUNT}, vertex::Vertex};
use bytemuck;
use glam::Mat4;

/// Uniform buffer for camera matrices
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    _padding: f32,
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 0.0],
            _padding: 0.0,
        }
    }

    pub fn update(&mut self, camera: &Camera) {
        self.view_proj = camera.view_projection_matrix().to_cols_array_2d();
        self.camera_pos = camera.position();
    }
}

/// Uniform buffer for lighting
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    direction: [f32; 3],
    _padding1: f32,
    color: [f32; 3],
    intensity: f32,
    ambient: [f32; 3],
    _padding2: f32,
}

impl LightUniform {
    pub fn new() -> Self {
        Self {
            // Light coming from upper-right-front
            direction: [0.5, 0.8, 0.3],
            _padding1: 0.0,
            // Warm white light
            color: [1.0, 0.98, 0.95],
            intensity: 1.0,
            // Soft ambient
            ambient: [0.15, 0.17, 0.2],
            _padding2: 0.0,
        }
    }

    pub fn set_direction(&mut self, x: f32, y: f32, z: f32) {
        // Normalize the direction
        let len = (x * x + y * y + z * z).sqrt();
        if len > 0.0001 {
            self.direction = [x / len, y / len, z / len];
        }
    }

    pub fn set_color(&mut self, r: f32, g: f32, b: f32) {
        self.color = [r, g, b];
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.max(0.0);
    }

    pub fn set_ambient(&mut self, r: f32, g: f32, b: f32) {
        self.ambient = [r, g, b];
    }
}

/// Uniform buffer for section plane
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SectionPlaneUniform {
    origin: [f32; 3],
    enabled: f32, // 0.0 = disabled, 1.0 = enabled
    normal: [f32; 3],
    _padding: f32,
}

impl SectionPlaneUniform {
    pub fn new() -> Self {
        Self {
            origin: [0.0, 0.0, 0.0],
            enabled: 0.0,
            normal: [0.0, 1.0, 0.0],
            _padding: 0.0,
        }
    }

    pub fn set(&mut self, origin: [f32; 3], normal: [f32; 3]) {
        self.origin = origin;
        self.normal = normal;
        self.enabled = 1.0;
    }

    pub fn disable(&mut self) {
        self.enabled = 0.0;
    }
}

/// Scene renderer for offscreen rendering
pub struct SceneRenderer {
    pub width: u32,
    pub height: u32,
    pub pipeline: Option<RenderPipeline>,
    pub camera_buffer: Option<wgpu::Buffer>,
    pub light_buffer: Option<wgpu::Buffer>,
    pub light_uniform: LightUniform,
    pub section_plane_buffer: Option<wgpu::Buffer>,
    pub section_plane_uniform: SectionPlaneUniform,
    pub bind_group: Option<wgpu::BindGroup>,
    pub msaa_texture: Option<wgpu::Texture>,    // MSAA render target
    pub color_texture: Option<wgpu::Texture>,   // Resolve target (for reading)
    pub depth_texture: Option<wgpu::Texture>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub num_indices: u32,
    pub render_mode: RenderMode,
    // Persistent read buffer to avoid allocation each frame
    pub read_buffer: Option<wgpu::Buffer>,
    pub padded_bytes_per_row: u32,
}

impl SceneRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pipeline: None,
            camera_buffer: None,
            light_buffer: None,
            light_uniform: LightUniform::new(),
            section_plane_buffer: None,
            section_plane_uniform: SectionPlaneUniform::new(),
            bind_group: None,
            msaa_texture: None,
            color_texture: None,
            depth_texture: None,
            vertex_buffer: None,
            index_buffer: None,
            num_indices: 0,
            render_mode: RenderMode::default(),
            read_buffer: None,
            padded_bytes_per_row: 0,
        }
    }

    /// Set the render mode (shaded or wireframe)
    pub fn set_render_mode(&mut self, mode: RenderMode) {
        self.render_mode = mode;
    }

    /// Get the current render mode
    pub fn get_render_mode(&self) -> RenderMode {
        self.render_mode
    }

    /// Initialize rendering resources
    pub fn initialize(&mut self, device: &wgpu::Device) {
        self.initialize_with_features(device, false);
    }

    /// Initialize rendering resources with optional wireframe support
    pub fn initialize_with_features(&mut self, device: &wgpu::Device, wireframe_supported: bool) {
        // Create render pipeline
        let pipeline = RenderPipeline::new_with_features(
            device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wireframe_supported,
        );

        // Create camera uniform buffer
        let camera_uniform = CameraUniform::new();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create light uniform buffer (using stored light_uniform)
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[self.light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create section plane uniform buffer
        let section_plane_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Section Plane Buffer"),
            contents: bytemuck::cast_slice(&[self.section_plane_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group with camera, light, and section plane
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &pipeline.camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: section_plane_buffer.as_entire_binding(),
                },
            ],
            label: Some("Scene Bind Group"),
        });

        // Create MSAA render target texture (only if MSAA enabled)
        let msaa_texture = if MSAA_SAMPLE_COUNT > 1 {
            Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA Texture"),
                size: wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: MSAA_SAMPLE_COUNT,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }))
        } else {
            None
        };

        // Create color/output texture (sample_count = 1, for reading back)
        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Color Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Create depth texture (must match render target sample count)
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: if MSAA_SAMPLE_COUNT > 1 { MSAA_SAMPLE_COUNT } else { 1 },
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        // Create persistent read buffer for pixel readback
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = self.width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;
        let buffer_size = (padded_bytes_per_row * self.height) as u64;

        let read_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Persistent Read Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        self.pipeline = Some(pipeline);
        self.camera_buffer = Some(camera_buffer);
        self.light_buffer = Some(light_buffer);
        self.section_plane_buffer = Some(section_plane_buffer);
        self.bind_group = Some(bind_group);
        self.msaa_texture = msaa_texture;
        self.color_texture = Some(color_texture);
        self.depth_texture = Some(depth_texture);
        self.read_buffer = Some(read_buffer);
        self.padded_bytes_per_row = padded_bytes_per_row;
    }

    /// Upload mesh data to GPU from flat arrays (from ModelMesh)
    pub fn upload_mesh_from_arrays(
        &mut self,
        device: &wgpu::Device,
        vertices: &[f32],    // x,y,z triplets
        normals: &[f32],     // x,y,z triplets
        colors: &[f32],      // r,g,b,a quads
        indices: &[u32],
    ) {
        let vertex_count = vertices.len() / 3;
        let mut vertex_data = Vec::with_capacity(vertex_count);

        for i in 0..vertex_count {
            let pos_idx = i * 3;
            let col_idx = i * 4;

            vertex_data.push(Vertex::new(
                [vertices[pos_idx], vertices[pos_idx + 1], vertices[pos_idx + 2]],
                [normals[pos_idx], normals[pos_idx + 1], normals[pos_idx + 2]],
                [colors[col_idx], colors[col_idx + 1], colors[col_idx + 2], colors[col_idx + 3]],
            ));
        }

        self.upload_mesh(device, &vertex_data, indices);
    }

    /// Update light uniform buffer with current settings
    pub fn update_light(&self, queue: &wgpu::Queue) {
        if let Some(buffer) = &self.light_buffer {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[self.light_uniform]));
        }
    }

    /// Set light direction (normalized automatically)
    pub fn set_light_direction(&mut self, x: f32, y: f32, z: f32) {
        self.light_uniform.set_direction(x, y, z);
    }

    /// Set light color (RGB, 0.0-1.0)
    pub fn set_light_color(&mut self, r: f32, g: f32, b: f32) {
        self.light_uniform.set_color(r, g, b);
    }

    /// Set light intensity (0.0+)
    pub fn set_light_intensity(&mut self, intensity: f32) {
        self.light_uniform.set_intensity(intensity);
    }

    /// Set ambient light color (RGB, 0.0-1.0)
    pub fn set_ambient_color(&mut self, r: f32, g: f32, b: f32) {
        self.light_uniform.set_ambient(r, g, b);
    }

    /// Set section plane (or None to disable)
    pub fn set_section_plane(&mut self, plane: Option<([f32; 3], [f32; 3])>) {
        if let Some((origin, normal)) = plane {
            self.section_plane_uniform.set(origin, normal);
        } else {
            self.section_plane_uniform.disable();
        }
    }

    /// Update section plane uniform buffer with current settings
    pub fn update_section_plane(&self, queue: &wgpu::Queue) {
        if let Some(buffer) = &self.section_plane_buffer {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[self.section_plane_uniform]));
        }
    }

    /// Upload mesh data to GPU
    pub fn upload_mesh(&mut self, device: &wgpu::Device, vertices: &[Vertex], indices: &[u32]) {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.num_indices = indices.len() as u32;
    }

    /// Render a frame and return pixel data
    pub fn render_frame(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera: &Camera,
    ) -> Vec<u8> {
        // Update camera uniform
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update(camera);
        queue.write_buffer(
            self.camera_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        // Create texture views
        let color_view = self
            .color_texture
            .as_ref()
            .unwrap()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self
            .depth_texture
            .as_ref()
            .unwrap()
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Render pass (with or without MSAA)
        {
            // Determine render target and resolve target based on MSAA
            let (render_view, resolve_target) = if let Some(msaa_tex) = &self.msaa_texture {
                let msaa_view = msaa_tex.create_view(&wgpu::TextureViewDescriptor::default());
                (msaa_view, Some(color_view))
            } else {
                (color_view, None)
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_view,
                    resolve_target: resolve_target.as_ref(),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            // Nice soft blue-gray background
                            r: 0.18,
                            g: 0.22,
                            b: 0.28,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let (Some(pipeline), Some(vb), Some(ib), Some(bg)) = (
                &self.pipeline,
                &self.vertex_buffer,
                &self.index_buffer,
                &self.bind_group,
            ) {
                // Use the appropriate pipeline based on render mode
                render_pass.set_pipeline(pipeline.get_pipeline(self.render_mode));
                render_pass.set_bind_group(0, bg, &[]);
                render_pass.set_vertex_buffer(0, vb.slice(..));
                render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            }
        }

        // Use persistent read buffer
        let read_buffer = self.read_buffer.as_ref().unwrap();
        let padded_bytes_per_row = self.padded_bytes_per_row;
        let bytes_per_pixel = 4u32;

        // Copy texture to buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: self.color_texture.as_ref().unwrap(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: read_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        // Submit and wait
        queue.submit(std::iter::once(encoder.finish()));

        // Read pixels from persistent buffer
        let buffer_slice = read_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        receiver.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();

        // Remove padding and return pixel data
        let mut pixels = Vec::with_capacity((self.width * self.height * 4) as usize);
        for y in 0..self.height {
            let start = (y * padded_bytes_per_row) as usize;
            let end = start + (self.width * bytes_per_pixel) as usize;
            pixels.extend_from_slice(&data[start..end]);
        }

        // Must drop the mapped range before unmapping
        drop(data);
        read_buffer.unmap();

        pixels
    }
}

// Need to add buffer init descriptor
use wgpu::util::DeviceExt;
