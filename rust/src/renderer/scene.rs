//! Scene Rendering
//!
//! Manages offscreen rendering and frame generation.

use super::{camera::{Camera, Frustum}, pipeline::{RenderPipeline, RenderMode, MSAA_SAMPLE_COUNT}, vertex::{InstanceData, Vertex, generate_unit_box}};
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

/// Per-element draw range for frustum culling
#[derive(Debug, Clone, Copy)]
pub struct ElementDrawRange {
    pub index_start: u32,
    pub index_count: u32,
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
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
    // Cached texture views (immutable after init, avoids re-creation each frame)
    pub color_view: Option<wgpu::TextureView>,
    pub depth_view: Option<wgpu::TextureView>,
    pub msaa_view: Option<wgpu::TextureView>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub num_indices: u32,
    pub render_mode: RenderMode,
    // Persistent read buffer to avoid allocation each frame
    pub read_buffer: Option<wgpu::Buffer>,
    pub padded_bytes_per_row: u32,
    // Persistent pixel buffer to avoid allocation each frame
    pub pixel_buffer: Vec<u8>,
    // Per-element draw ranges for frustum culling (non-instanced fallback)
    pub element_draw_ranges: Vec<ElementDrawRange>,
    // Instanced rendering
    pub unit_box_vertex_buffer: Option<wgpu::Buffer>,
    pub unit_box_index_buffer: Option<wgpu::Buffer>,
    pub unit_box_index_count: u32,
    pub instances: Vec<InstanceData>,
    pub visible_instances: Vec<InstanceData>,
    pub instance_buffer: Option<wgpu::Buffer>,
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
            color_view: None,
            depth_view: None,
            msaa_view: None,
            vertex_buffer: None,
            index_buffer: None,
            num_indices: 0,
            render_mode: RenderMode::default(),
            read_buffer: None,
            padded_bytes_per_row: 0,
            pixel_buffer: Vec::new(),
            element_draw_ranges: Vec::new(),
            unit_box_vertex_buffer: None,
            unit_box_index_buffer: None,
            unit_box_index_count: 0,
            instances: Vec::new(),
            visible_instances: Vec::new(),
            instance_buffer: None,
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

        // Upload shared unit box for instanced rendering (24 verts, 36 indices)
        let (box_verts, box_indices) = generate_unit_box();
        let unit_box_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Unit Box Vertex Buffer"),
            contents: bytemuck::cast_slice(&box_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let unit_box_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Unit Box Index Buffer"),
            contents: bytemuck::cast_slice(&box_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let unit_box_index_count = box_indices.len() as u32;

        // Cache texture views (immutable after creation)
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let msaa_view = msaa_texture.as_ref().map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()));

        self.pipeline = Some(pipeline);
        self.camera_buffer = Some(camera_buffer);
        self.light_buffer = Some(light_buffer);
        self.section_plane_buffer = Some(section_plane_buffer);
        self.bind_group = Some(bind_group);
        self.msaa_texture = msaa_texture;
        self.color_texture = Some(color_texture);
        self.depth_texture = Some(depth_texture);
        self.color_view = Some(color_view);
        self.depth_view = Some(depth_view);
        self.msaa_view = msaa_view;
        self.read_buffer = Some(read_buffer);
        self.padded_bytes_per_row = padded_bytes_per_row;
        self.unit_box_vertex_buffer = Some(unit_box_vertex_buffer);
        self.unit_box_index_buffer = Some(unit_box_index_buffer);
        self.unit_box_index_count = unit_box_index_count;
        // Pre-allocate pixel buffer for frame readback (reused every frame)
        self.pixel_buffer = vec![0u8; (self.width * self.height * 4) as usize];
    }

    /// Upload mesh data to GPU from flat arrays (from ModelMesh)
    ///
    /// Writes packed 20-byte vertices directly into a mapped GPU buffer:
    ///   - position: 3×f32 (12 bytes)
    ///   - normal: Snorm8x4 (4 bytes, packed from f32)
    ///   - color: Unorm8x4 (4 bytes, packed from f32)
    /// No intermediate Vec<Vertex> allocation.
    pub fn upload_mesh_from_arrays(
        &mut self,
        device: &wgpu::Device,
        vertices: &[f32],    // x,y,z triplets
        normals: &[f32],     // x,y,z triplets
        colors: &[f32],      // r,g,b,a quads
        indices: &[u32],
    ) {
        let vertex_count = vertices.len() / 3;
        if vertex_count == 0 {
            self.num_indices = 0;
            return;
        }

        let vertex_size = std::mem::size_of::<Vertex>() as u64; // 20 bytes
        let buffer_size = vertex_count as u64 * vertex_size;

        // Align to COPY_BUFFER_ALIGNMENT (required for mapped_at_creation)
        let align = wgpu::COPY_BUFFER_ALIGNMENT;
        let aligned_size = ((buffer_size + align - 1) / align * align).max(align);

        // Create vertex buffer mapped for direct writing
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: aligned_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        // Write packed vertex data directly into GPU buffer memory
        {
            let mut mapped = vertex_buffer.slice(..).get_mapped_range_mut();

            for i in 0..vertex_count {
                let p = i * 3;  // position/normal source index
                let c = i * 4;  // color source index
                let d = i * 20; // destination byte offset (20 bytes per vertex)

                // Position: 3×f32 = 12 bytes
                let pos_bytes: &[u8] = bytemuck::cast_slice(&vertices[p..p + 3]);
                mapped[d..d + 12].copy_from_slice(pos_bytes);

                // Normal: pack f32 → Snorm8 (i8), 4 bytes
                mapped[d + 12] = (normals[p]     * 127.0) as i8 as u8;
                mapped[d + 13] = (normals[p + 1] * 127.0) as i8 as u8;
                mapped[d + 14] = (normals[p + 2] * 127.0) as i8 as u8;
                mapped[d + 15] = 0; // padding

                // Color: pack f32 → Unorm8 (u8), 4 bytes
                mapped[d + 16] = (colors[c]     * 255.0) as u8;
                mapped[d + 17] = (colors[c + 1] * 255.0) as u8;
                mapped[d + 18] = (colors[c + 2] * 255.0) as u8;
                mapped[d + 19] = (colors[c + 3] * 255.0) as u8;
            }
        }
        vertex_buffer.unmap();

        // Index buffer — create_buffer_init is already efficient (mapped_at_creation internally)
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.num_indices = indices.len() as u32;
    }

    /// Set per-element draw ranges for frustum culling.
    ///
    /// `triangle_starts` and `triangle_counts` come from ElementInfo.
    /// Each element's triangles are contiguous in the index buffer.
    pub fn set_element_draw_ranges(&mut self, ranges: Vec<ElementDrawRange>) {
        self.element_draw_ranges = ranges;
    }

    /// Set per-instance data for GPU instancing.
    /// Replaces the non-instanced mesh path for BIM models.
    pub fn set_instances(&mut self, device: &wgpu::Device, instances: Vec<InstanceData>) {
        if instances.is_empty() {
            self.instances = Vec::new();
            self.visible_instances = Vec::new();
            self.instance_buffer = None;
            return;
        }

        let buffer_size = (instances.len() * std::mem::size_of::<InstanceData>()) as u64;
        let align = wgpu::COPY_BUFFER_ALIGNMENT;
        let aligned_size = ((buffer_size + align - 1) / align * align).max(align);

        self.instance_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: aligned_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.visible_instances = Vec::with_capacity(instances.len());
        self.instances = instances;
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
    ///
    /// Reuses `self.pixel_buffer` to avoid allocating a new Vec<u8> each frame.
    pub fn render_frame(
        &mut self,
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

        // Compute frustum once for the frame (reused for both instance culling and draw ranges)
        let frustum = Frustum::from_view_projection(&camera.view_projection_matrix());

        // Prepare instanced rendering: frustum cull and write visible instances
        let visible_instance_count = if !self.instances.is_empty() {
            self.visible_instances.clear();
            for inst in &self.instances {
                let min = [
                    inst.position[0] - inst.scale[0],
                    inst.position[1] - inst.scale[1],
                    inst.position[2] - inst.scale[2],
                ];
                let max = [
                    inst.position[0] + inst.scale[0],
                    inst.position[1] + inst.scale[1],
                    inst.position[2] + inst.scale[2],
                ];
                if frustum.intersects_aabb(min, max) {
                    self.visible_instances.push(*inst);
                }
            }
            if let Some(buf) = &self.instance_buffer {
                if !self.visible_instances.is_empty() {
                    queue.write_buffer(buf, 0, bytemuck::cast_slice(&self.visible_instances));
                }
            }
            self.visible_instances.len() as u32
        } else {
            0
        };

        // Use cached texture views (created once during init)
        let color_view = self.color_view.as_ref().unwrap();
        let depth_view = self.depth_view.as_ref().unwrap();

        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Render pass (with or without MSAA)
        {
            // Determine render target and resolve target based on MSAA
            let (render_view, resolve_target): (&wgpu::TextureView, Option<&wgpu::TextureView>) =
                if self.msaa_view.is_some() {
                    (self.msaa_view.as_ref().unwrap(), Some(color_view))
                } else {
                    (color_view, None)
                };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_view,
                    resolve_target,
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
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if visible_instance_count > 0 {
                // INSTANCED PATH — single draw call for all visible BIM elements
                if let (Some(pipeline), Some(unit_vb), Some(unit_ib), Some(inst_buf), Some(bg)) = (
                    &self.pipeline,
                    &self.unit_box_vertex_buffer,
                    &self.unit_box_index_buffer,
                    &self.instance_buffer,
                    &self.bind_group,
                ) {
                    render_pass.set_pipeline(pipeline.get_instanced_pipeline(self.render_mode));
                    render_pass.set_bind_group(0, bg, &[]);
                    render_pass.set_vertex_buffer(0, unit_vb.slice(..));
                    render_pass.set_vertex_buffer(1, inst_buf.slice(..));
                    render_pass.set_index_buffer(unit_ib.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..self.unit_box_index_count, 0, 0..visible_instance_count);
                }
            } else if let (Some(pipeline), Some(vb), Some(ib), Some(bg)) = (
                &self.pipeline,
                &self.vertex_buffer,
                &self.index_buffer,
                &self.bind_group,
            ) {
                // NON-INSTANCED FALLBACK (test cube or legacy path)
                render_pass.set_pipeline(pipeline.get_pipeline(self.render_mode));
                render_pass.set_bind_group(0, bg, &[]);
                render_pass.set_vertex_buffer(0, vb.slice(..));
                render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);

                if self.element_draw_ranges.is_empty() {
                    render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
                } else {
                    for range in &self.element_draw_ranges {
                        if frustum.intersects_aabb(range.aabb_min, range.aabb_max) {
                            let start = range.index_start;
                            let end = start + range.index_count;
                            render_pass.draw_indexed(start..end, 0, 0..1);
                        }
                    }
                }
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

        // Map read buffer — no channel needed, poll(Wait) guarantees completion
        let buffer_slice = read_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);

        let data = buffer_slice.get_mapped_range();

        // Reuse persistent pixel buffer — avoids allocation each frame
        let unpadded_row = (self.width * bytes_per_pixel) as usize;
        let expected_size = unpadded_row * self.height as usize;
        self.pixel_buffer.resize(expected_size, 0);

        for y in 0..self.height as usize {
            let src_start = y * padded_bytes_per_row as usize;
            let dst_start = y * unpadded_row;
            self.pixel_buffer[dst_start..dst_start + unpadded_row]
                .copy_from_slice(&data[src_start..src_start + unpadded_row]);
        }

        // Must drop the mapped range before unmapping
        drop(data);
        read_buffer.unmap();

        // Zero-copy swap: take the filled buffer and pre-allocate for next frame
        let result = std::mem::take(&mut self.pixel_buffer);
        self.pixel_buffer = Vec::with_capacity(expected_size);
        result
    }
}

// Need to add buffer init descriptor
use wgpu::util::DeviceExt;
