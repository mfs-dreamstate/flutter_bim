//! Scene Rendering
//!
//! Manages offscreen rendering and frame generation.

use super::{
    camera::{Camera, Frustum},
    pipeline::{
        RenderPipeline, RenderMode, MSAA_SAMPLE_COUNT, DEPTH_STENCIL_FORMAT,
        FxaaResources, FxaaUniformData, create_fxaa_pipeline,
        SectionFillUniform, SectionFillResources, create_section_fill_pipeline,
        ShadowUniform, ShadowResources, create_shadow_pipeline,
        SsaoResources, SsaoParams, SsaoCompositeParams,
        create_ssao_pipeline, generate_ssao_noise,
        EnvironmentUniform,
        ComputeCullResources, create_compute_cull_pipeline, VertexStreamConfig,
    },
    vertex::{InstanceData, Vertex, generate_unit_box},
};
use bytemuck;
use glam::{Mat4, Vec3};

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
    pub direction: [f32; 3],
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

/// Maximum number of simultaneous section planes
pub const MAX_SECTION_PLANES: usize = 6;

/// Single section plane data (GPU-compatible layout)
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SectionPlaneData {
    pub origin: [f32; 3],
    pub enabled: f32,
    pub normal: [f32; 3],
    pub _padding: f32,
}

impl SectionPlaneData {
    pub fn new() -> Self {
        Self {
            origin: [0.0, 0.0, 0.0],
            enabled: 0.0,
            normal: [0.0, 1.0, 0.0],
            _padding: 0.0,
        }
    }

    pub fn from_origin_normal(origin: [f32; 3], normal: [f32; 3]) -> Self {
        Self {
            origin,
            enabled: 1.0,
            normal,
            _padding: 0.0,
        }
    }
}

/// Uniform buffer for multiple section planes (up to 6)
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SectionPlanesUniform {
    pub planes: [SectionPlaneData; MAX_SECTION_PLANES],
    pub count: u32,
    pub _pad1: u32,
    pub _pad2: u32,
    pub _pad3: u32,
}

impl SectionPlanesUniform {
    pub fn new() -> Self {
        Self {
            planes: [SectionPlaneData::new(); MAX_SECTION_PLANES],
            count: 0,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        }
    }

    /// Set a single plane at index 0 (backward compatible with old API)
    pub fn set(&mut self, origin: [f32; 3], normal: [f32; 3]) {
        self.planes[0] = SectionPlaneData::from_origin_normal(origin, normal);
        if self.count == 0 {
            self.count = 1;
        }
    }

    /// Disable all planes
    pub fn disable(&mut self) {
        for plane in &mut self.planes {
            plane.enabled = 0.0;
        }
        self.count = 0;
    }

    /// Add a new section plane, returns the index or None if full
    pub fn add_plane(&mut self, origin: [f32; 3], normal: [f32; 3]) -> Option<usize> {
        let idx = self.count as usize;
        if idx >= MAX_SECTION_PLANES {
            return None;
        }
        self.planes[idx] = SectionPlaneData::from_origin_normal(origin, normal);
        self.count += 1;
        Some(idx)
    }

    /// Remove the section plane at the given index, shifting subsequent planes down
    pub fn remove_plane(&mut self, index: usize) -> bool {
        if index >= self.count as usize {
            return false;
        }
        // Shift planes down
        for i in index..(self.count as usize - 1) {
            self.planes[i] = self.planes[i + 1];
        }
        self.count -= 1;
        // Clear the now-unused slot
        self.planes[self.count as usize] = SectionPlaneData::new();
        true
    }

    /// Set all planes from a list of (origin, normal) pairs
    pub fn set_planes(&mut self, planes: &[([f32; 3], [f32; 3])]) {
        let count = planes.len().min(MAX_SECTION_PLANES);
        for i in 0..MAX_SECTION_PLANES {
            if i < count {
                self.planes[i] = SectionPlaneData::from_origin_normal(planes[i].0, planes[i].1);
            } else {
                self.planes[i] = SectionPlaneData::new();
            }
        }
        self.count = count as u32;
    }

    /// Get the current number of active planes
    pub fn plane_count(&self) -> usize {
        self.count as usize
    }
}

/// Uniform buffer for section box (6-plane clipping region)
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SectionBoxUniform {
    min: [f32; 3],
    pub enabled: f32, // 0.0 = disabled, 1.0 = enabled
    max: [f32; 3],
    _padding: f32,
}

impl SectionBoxUniform {
    pub fn new() -> Self {
        Self {
            min: [-1000.0, -1000.0, -1000.0],
            enabled: 0.0,
            max: [1000.0, 1000.0, 1000.0],
            _padding: 0.0,
        }
    }

    pub fn set(&mut self, min: [f32; 3], max: [f32; 3]) {
        self.min = min;
        self.max = max;
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
    pub section_plane_uniform: SectionPlanesUniform,
    pub section_box_buffer: Option<wgpu::Buffer>,
    pub section_box_uniform: SectionBoxUniform,
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
    // Edge rendering (wireframe overlay on top of solid geometry)
    pub edge_rendering_enabled: bool,
    // FXAA post-process anti-aliasing
    pub fxaa: Option<FxaaResources>,
    pub fxaa_texture: Option<wgpu::Texture>,
    pub fxaa_texture_view: Option<wgpu::TextureView>,
    pub fxaa_ssao_bind_group: Option<wgpu::BindGroup>,
    // Screen-space size threshold: skip elements smaller than this many pixels
    pub min_screen_pixels: f32,
    // Section fill (cap faces at section plane intersections)
    pub section_fill: Option<SectionFillResources>,
    pub section_fill_uniform_buffer: Option<wgpu::Buffer>,
    pub section_fill_bind_group: Option<wgpu::BindGroup>,
    pub section_fill_stencil_bind_group: Option<wgpu::BindGroup>,
    pub section_fill_enabled: bool,
    pub section_fill_color: [f32; 4],
    // Shadow mapping
    pub shadow: Option<ShadowResources>,
    pub shadow_bind_group: Option<wgpu::BindGroup>,
    pub shadow_enabled: bool,
    pub shadow_map_size: u32,
    // SSAO (Screen-Space Ambient Occlusion)
    pub ssao: Option<SsaoResources>,
    pub ssao_texture: Option<wgpu::Texture>,
    pub ssao_texture_view: Option<wgpu::TextureView>,
    pub ssao_blur_texture: Option<wgpu::Texture>,
    pub ssao_blur_texture_view: Option<wgpu::TextureView>,
    pub ssao_noise_texture: Option<wgpu::Texture>,
    pub ssao_noise_texture_view: Option<wgpu::TextureView>,
    pub ssao_bind_group: Option<wgpu::BindGroup>,
    pub ssao_blur_bind_group: Option<wgpu::BindGroup>,
    pub ssao_composite_bind_group: Option<wgpu::BindGroup>,
    pub ssao_composite_texture: Option<wgpu::Texture>,
    pub ssao_composite_texture_view: Option<wgpu::TextureView>,
    pub ssao_depth_texture: Option<wgpu::Texture>,
    pub ssao_depth_texture_view: Option<wgpu::TextureView>,
    pub ssao_enabled: bool,
    pub ssao_radius: f32,
    pub ssao_intensity: f32,
    pub ssao_noise_data: Option<Vec<u8>>,
    // Environment-based lighting (IBL)
    pub env_uniform: EnvironmentUniform,
    pub env_uniform_buffer: Option<wgpu::Buffer>,
    pub env_enabled: bool,
    // Compute shader frustum culling
    pub compute_cull_resources: Option<ComputeCullResources>,
    pub compute_cull_state: Option<ComputeCullState>,
    // FastNav: adaptive quality during camera interaction
    pub interaction_active: bool,
    pub fastnav_min_screen_pixels: f32, // higher threshold during interaction
    // Cached compute cull visibility from previous frame (one-frame latency)
    pub compute_cull_visibility: Vec<bool>,
    pub compute_cull_auto: bool, // auto-enable when element_count > threshold
    // Double-buffered readback
    pub read_buffer_b: Option<wgpu::Buffer>,
    pub pixel_buffer_b: Vec<u8>,
    pub use_buffer_a: bool, // toggle between A/B each frame
    pub has_previous_frame: bool, // false for the very first frame
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
            section_plane_uniform: SectionPlanesUniform::new(),
            section_box_buffer: None,
            section_box_uniform: SectionBoxUniform::new(),
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
            edge_rendering_enabled: false,
            fxaa: None,
            fxaa_texture: None,
            fxaa_texture_view: None,
            fxaa_ssao_bind_group: None,
            min_screen_pixels: 2.0,
            section_fill: None,
            section_fill_uniform_buffer: None,
            section_fill_bind_group: None,
            section_fill_stencil_bind_group: None,
            section_fill_enabled: true,
            section_fill_color: [0.85, 0.85, 0.85, 1.0],
            shadow: None,
            shadow_bind_group: None,
            shadow_enabled: false,
            shadow_map_size: 2048,
            // SSAO
            ssao: None,
            ssao_texture: None,
            ssao_texture_view: None,
            ssao_blur_texture: None,
            ssao_blur_texture_view: None,
            ssao_noise_texture: None,
            ssao_noise_texture_view: None,
            ssao_bind_group: None,
            ssao_blur_bind_group: None,
            ssao_composite_bind_group: None,
            ssao_composite_texture: None,
            ssao_composite_texture_view: None,
            ssao_depth_texture: None,
            ssao_depth_texture_view: None,
            ssao_enabled: false,
            ssao_radius: 0.5,
            ssao_intensity: 1.0,
            ssao_noise_data: None,
            // Environment
            env_uniform: EnvironmentUniform::new(),
            env_uniform_buffer: None,
            env_enabled: false,
            // Compute culling
            compute_cull_resources: None,
            compute_cull_state: None,
            // FastNav
            interaction_active: false,
            fastnav_min_screen_pixels: 8.0, // skip more small elements during interaction
            // Compute culling
            compute_cull_visibility: Vec::new(),
            compute_cull_auto: true, // auto-enable for large models
            // Double-buffered readback
            read_buffer_b: None,
            pixel_buffer_b: Vec::new(),
            use_buffer_a: true,
            has_previous_frame: false,
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

    /// Enable or disable edge rendering (wireframe overlay on solid geometry)
    pub fn set_edge_rendering(&mut self, enabled: bool) {
        self.edge_rendering_enabled = enabled;
    }

    /// Check if edge rendering is enabled
    pub fn is_edge_rendering_enabled(&self) -> bool {
        self.edge_rendering_enabled
    }

    /// Enable or disable FXAA post-process anti-aliasing
    pub fn set_fxaa_enabled(&mut self, enabled: bool) {
        if let Some(fxaa) = &mut self.fxaa {
            fxaa.enabled = enabled;
        }
    }

    /// Check if FXAA is enabled
    pub fn is_fxaa_enabled(&self) -> bool {
        self.fxaa.as_ref().map_or(false, |f| f.enabled)
    }

    /// Set the minimum screen-space size in pixels for an element to be drawn.
    /// Elements whose projected size is smaller than this threshold are skipped.
    pub fn set_min_screen_pixels(&mut self, pixels: f32) {
        self.min_screen_pixels = pixels.max(0.0);
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

        // Create section box uniform buffer
        let section_box_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Section Box Buffer"),
            contents: bytemuck::cast_slice(&[self.section_box_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group with camera, light, section plane, and section box
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
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: section_box_buffer.as_entire_binding(),
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING,
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
            format: DEPTH_STENCIL_FORMAT,
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
            label: Some("Persistent Read Buffer A"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let read_buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Persistent Read Buffer B"),
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

        // Create FXAA pipeline and output texture
        let surface_format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let mut fxaa = create_fxaa_pipeline(device, surface_format);

        // Create FXAA output texture (same format/size as color texture, used as FXAA output)
        let fxaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("FXAA Output Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let fxaa_texture_view = fxaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Update FXAA uniform buffer with actual texture size
        let fxaa_uniform_data = FxaaUniformData {
            tex_size: [self.width as f32, self.height as f32],
        };
        // Write initial uniform data (buffer was created with dummy values)
        // We re-create the buffer with the correct size so it's ready immediately
        fxaa.uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("FXAA Uniform Buffer"),
            contents: bytemuck::cast_slice(&[fxaa_uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create the FXAA bind group with the color (resolve) texture as input
        // The color_texture is the resolve target (sample_count=1) that FXAA will read from
        fxaa.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FXAA Bind Group"),
            layout: &fxaa.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&fxaa.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: fxaa.uniform_buffer.as_entire_binding(),
                },
            ],
        }));

        // Create section fill pipeline and resources
        let surface_fmt = wgpu::TextureFormat::Rgba8UnormSrgb;
        let section_fill = create_section_fill_pipeline(
            device,
            surface_fmt,
            &pipeline.camera_bind_group_layout,
        );

        // Section fill uniform buffer
        let section_fill_uniform = SectionFillUniform {
            fill_color: self.section_fill_color,
            plane_origin: [0.0, 0.0, 0.0, 0.0],
            plane_normal: [0.0, 1.0, 0.0, 500.0], // default: Y-up, 500 unit half-size
        };
        let section_fill_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Section Fill Uniform Buffer"),
            contents: bytemuck::cast_slice(&[section_fill_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Section fill bind group (camera + fill uniforms)
        let section_fill_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Section Fill Bind Group"),
            layout: &section_fill.fill_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: section_fill_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Stencil bind group (camera only)
        let section_fill_stencil_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Section Stencil Bind Group"),
            layout: &section_fill.stencil_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
            ],
        });

        // Create shadow pipeline and resources
        let shadow = create_shadow_pipeline(device, self.shadow_map_size);
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Bind Group"),
            layout: &shadow.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shadow.uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Create SSAO pipeline and resources
        let ssao_resources = create_ssao_pipeline(device, surface_format, self.width, self.height);

        // SSAO textures: R8Unorm for occlusion values
        let ssao_tex_desc = wgpu::TextureDescriptor {
            label: Some("SSAO Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let ssao_texture = device.create_texture(&ssao_tex_desc);
        let ssao_texture_view = ssao_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let ssao_blur_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO Blur Texture"),
            ..ssao_tex_desc
        });
        let ssao_blur_texture_view = ssao_blur_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // SSAO composite output texture (same as color texture format)
        let ssao_composite_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO Composite Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let ssao_composite_texture_view = ssao_composite_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Separate depth texture for SSAO (non-MSAA, filterable Depth32Float)
        let ssao_depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO Depth Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let ssao_depth_texture_view = ssao_depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 4x4 noise texture (random tangent-space rotation vectors)
        let noise_data = generate_ssao_noise();
        let ssao_noise_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO Noise Texture"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // Noise data will be uploaded lazily on first render_frame call (needs a queue).
        let ssao_noise_texture_view = ssao_noise_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // SSAO bind groups (must be created before moving resources into self)
        let ssao_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSAO Bind Group"),
            layout: &ssao_resources.ssao_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&ssao_depth_texture_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&ssao_resources.depth_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&ssao_noise_texture_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&ssao_resources.noise_sampler) },
                wgpu::BindGroupEntry { binding: 4, resource: ssao_resources.params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: ssao_resources.kernel_buffer.as_entire_binding() },
            ],
        });

        let ssao_blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSAO Blur Bind Group"),
            layout: &ssao_resources.blur_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&ssao_texture_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&ssao_resources.linear_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&ssao_depth_texture_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&ssao_resources.depth_sampler) },
                wgpu::BindGroupEntry { binding: 4, resource: ssao_resources.blur_params_buffer.as_entire_binding() },
            ],
        });

        let ssao_composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSAO Composite Bind Group"),
            layout: &ssao_resources.composite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&color_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&ssao_resources.linear_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&ssao_blur_texture_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&ssao_resources.linear_sampler) },
                wgpu::BindGroupEntry { binding: 4, resource: ssao_resources.composite_params_buffer.as_entire_binding() },
            ],
        });

        // Create a second FXAA bind group that reads from the SSAO composite texture
        // (used when SSAO is active and FXAA is enabled)
        let fxaa_ssao_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FXAA SSAO Bind Group"),
            layout: &fxaa.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&ssao_composite_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&fxaa.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: fxaa.uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Environment lighting uniform buffer
        let env_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Environment Uniform Buffer"),
            contents: bytemuck::cast_slice(&[self.env_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // --- Assign all resources to self (must happen after all bind groups are created) ---
        self.pipeline = Some(pipeline);
        self.camera_buffer = Some(camera_buffer);
        self.light_buffer = Some(light_buffer);
        self.section_plane_buffer = Some(section_plane_buffer);
        self.section_box_buffer = Some(section_box_buffer);
        self.bind_group = Some(bind_group);
        self.msaa_texture = msaa_texture;
        self.color_texture = Some(color_texture);
        self.depth_texture = Some(depth_texture);
        self.color_view = Some(color_view);
        self.depth_view = Some(depth_view);
        self.msaa_view = msaa_view;
        self.read_buffer = Some(read_buffer);
        self.read_buffer_b = Some(read_buffer_b);
        self.has_previous_frame = false;
        self.use_buffer_a = true;
        self.padded_bytes_per_row = padded_bytes_per_row;
        self.unit_box_vertex_buffer = Some(unit_box_vertex_buffer);
        self.unit_box_index_buffer = Some(unit_box_index_buffer);
        self.unit_box_index_count = unit_box_index_count;
        self.fxaa = Some(fxaa);
        self.fxaa_texture = Some(fxaa_texture);
        self.fxaa_texture_view = Some(fxaa_texture_view);
        self.fxaa_ssao_bind_group = Some(fxaa_ssao_bind_group);
        self.section_fill = Some(section_fill);
        self.section_fill_uniform_buffer = Some(section_fill_uniform_buffer);
        self.section_fill_bind_group = Some(section_fill_bind_group);
        self.section_fill_stencil_bind_group = Some(section_fill_stencil_bind_group);
        self.shadow = Some(shadow);
        self.shadow_bind_group = Some(shadow_bind_group);
        self.ssao = Some(ssao_resources);
        self.ssao_texture = Some(ssao_texture);
        self.ssao_texture_view = Some(ssao_texture_view);
        self.ssao_blur_texture = Some(ssao_blur_texture);
        self.ssao_blur_texture_view = Some(ssao_blur_texture_view);
        self.ssao_noise_texture = Some(ssao_noise_texture);
        self.ssao_noise_texture_view = Some(ssao_noise_texture_view);
        self.ssao_bind_group = Some(ssao_bind_group);
        self.ssao_blur_bind_group = Some(ssao_blur_bind_group);
        self.ssao_composite_bind_group = Some(ssao_composite_bind_group);
        self.ssao_composite_texture = Some(ssao_composite_texture);
        self.ssao_composite_texture_view = Some(ssao_composite_texture_view);
        self.ssao_depth_texture = Some(ssao_depth_texture);
        self.ssao_depth_texture_view = Some(ssao_depth_texture_view);
        self.ssao_noise_data = Some(noise_data);
        self.env_uniform_buffer = Some(env_uniform_buffer);

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

    /// Set section plane at index 0 (backward compatible, or None to disable all)
    pub fn set_section_plane(&mut self, plane: Option<([f32; 3], [f32; 3])>) {
        if let Some((origin, normal)) = plane {
            self.section_plane_uniform.set(origin, normal);
        } else {
            self.section_plane_uniform.disable();
        }
    }

    /// Set multiple section planes at once
    pub fn set_section_planes(&mut self, planes: &[([f32; 3], [f32; 3])]) {
        self.section_plane_uniform.set_planes(planes);
    }

    /// Add a section plane, returns the index or None if full
    pub fn add_section_plane(&mut self, origin: [f32; 3], normal: [f32; 3]) -> Option<usize> {
        self.section_plane_uniform.add_plane(origin, normal)
    }

    /// Remove section plane at the given index
    pub fn remove_section_plane(&mut self, index: usize) -> bool {
        self.section_plane_uniform.remove_plane(index)
    }

    /// Get current section plane count
    pub fn get_section_plane_count(&self) -> usize {
        self.section_plane_uniform.plane_count()
    }

    /// Clear all section planes
    pub fn clear_section_planes(&mut self) {
        self.section_plane_uniform.disable();
    }

    /// Update section plane uniform buffer with current settings
    pub fn update_section_plane(&self, queue: &wgpu::Queue) {
        if let Some(buffer) = &self.section_plane_buffer {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[self.section_plane_uniform]));
        }
    }

    /// Set section box (or None to disable)
    pub fn set_section_box(&mut self, box_bounds: Option<([f32; 3], [f32; 3])>) {
        if let Some((min, max)) = box_bounds {
            self.section_box_uniform.set(min, max);
        } else {
            self.section_box_uniform.disable();
        }
    }

    /// Update section box uniform buffer with current settings
    pub fn update_section_box(&self, queue: &wgpu::Queue) {
        if let Some(buffer) = &self.section_box_buffer {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[self.section_box_uniform]));
        }
    }

    // ----------------------------------------------------------------
    // Section fill (cap faces at section plane intersections)
    // ----------------------------------------------------------------

    /// Enable or disable section fill (cap faces at section plane intersections)
    pub fn set_section_fill_enabled(&mut self, enabled: bool) {
        self.section_fill_enabled = enabled;
    }

    /// Check if section fill is enabled
    pub fn is_section_fill_enabled(&self) -> bool {
        self.section_fill_enabled
    }

    /// Set the section fill color (RGBA, 0.0-1.0)
    pub fn set_section_fill_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.section_fill_color = [r, g, b, a];
    }

    /// Get the current section fill color
    pub fn get_section_fill_color(&self) -> [f32; 4] {
        self.section_fill_color
    }

    // ----------------------------------------------------------------
    // Shadow mapping
    // ----------------------------------------------------------------

    /// Enable or disable shadow mapping
    pub fn set_shadows_enabled(&mut self, enabled: bool) {
        self.shadow_enabled = enabled;
    }

    /// Check if shadow mapping is enabled
    pub fn is_shadows_enabled(&self) -> bool {
        self.shadow_enabled
    }

    /// Set the shadow map resolution (e.g. 1024, 2048, 4096)
    pub fn set_shadow_map_size(&mut self, size: u32) {
        self.shadow_map_size = size.max(256).min(8192);
    }

    // ----------------------------------------------------------------
    // SSAO (Screen-Space Ambient Occlusion)
    // ----------------------------------------------------------------

    /// Enable or disable Screen-Space Ambient Occlusion
    pub fn set_ssao_enabled(&mut self, enabled: bool) {
        self.ssao_enabled = enabled;
    }

    /// Check if SSAO is enabled
    pub fn is_ssao_enabled(&self) -> bool {
        self.ssao_enabled
    }

    /// Set the SSAO sample radius in view-space units
    pub fn set_ssao_radius(&mut self, radius: f32) {
        self.ssao_radius = radius.max(0.01).min(5.0);
    }

    /// Set the SSAO intensity (multiplier for occlusion darkness)
    pub fn set_ssao_intensity(&mut self, intensity: f32) {
        self.ssao_intensity = intensity.max(0.0).min(5.0);
    }

    // ----------------------------------------------------------------
    // Environment-Based Lighting (IBL)
    // ----------------------------------------------------------------

    /// Enable or disable environment-based ambient lighting
    pub fn set_environment_enabled(&mut self, enabled: bool) {
        self.env_enabled = enabled;
        self.env_uniform.enabled = if enabled { 1 } else { 0 };
    }

    /// Check if environment lighting is enabled
    pub fn is_environment_enabled(&self) -> bool {
        self.env_enabled
    }

    /// Set the hemisphere colors for environment lighting (RGB, 0.0-1.0)
    pub fn set_environment_colors(&mut self, sky: [f32; 3], ground: [f32; 3], horizon: [f32; 3]) {
        self.env_uniform.sky_color = [sky[0], sky[1], sky[2], 1.0];
        self.env_uniform.ground_color = [ground[0], ground[1], ground[2], 1.0];
        self.env_uniform.horizon_color = [horizon[0], horizon[1], horizon[2], 1.0];
    }

    /// Set the environment lighting intensity multiplier
    pub fn set_environment_intensity(&mut self, intensity: f32) {
        self.env_uniform.intensity = intensity.max(0.0);
    }

    /// Compute a light view-projection matrix for directional shadow mapping.
    /// Fits the shadow frustum to the given scene bounds.
    fn compute_light_view_proj(
        light_dir: [f32; 3],
        scene_min: [f32; 3],
        scene_max: [f32; 3],
    ) -> Mat4 {
        let light_dir = Vec3::new(light_dir[0], light_dir[1], light_dir[2]).normalize();
        let center = Vec3::new(
            (scene_min[0] + scene_max[0]) * 0.5,
            (scene_min[1] + scene_max[1]) * 0.5,
            (scene_min[2] + scene_max[2]) * 0.5,
        );
        let extent = Vec3::new(
            scene_max[0] - scene_min[0],
            scene_max[1] - scene_min[1],
            scene_max[2] - scene_min[2],
        );
        let radius = extent.length() * 0.5;

        // Light positioned far behind the scene along the light direction
        let light_pos = center - light_dir * radius * 2.0;

        // Choose a stable up vector that doesn't align with light direction
        let up = if light_dir.y.abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };

        let view = Mat4::look_at_rh(light_pos, center, up);
        let proj = Mat4::orthographic_rh(
            -radius, radius,
            -radius, radius,
            0.01, radius * 4.0,
        );

        proj * view
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
        // Upload SSAO noise texture if pending (one-time lazy upload)
        if let Some(noise_data) = self.ssao_noise_data.take() {
            if let Some(noise_tex) = &self.ssao_noise_texture {
                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: noise_tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &noise_data,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * 4), // 4 pixels * 4 bytes (RGBA8)
                        rows_per_image: Some(4),
                    },
                    wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
                );
            }
        }

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

        // GPU compute culling: read previous frame's results, then update frustum for next frame
        if let Some(state) = &self.compute_cull_state {
            if state.enabled && state.element_count > 0 {
                // Read previous frame's visibility results from staging buffer
                if let Some(staging) = &state.result_staging_buffer {
                    let slice = staging.slice(..);
                    slice.map_async(wgpu::MapMode::Read, |_| {});
                    device.poll(wgpu::Maintain::Wait);
                    let mapped = slice.get_mapped_range();
                    let data: &[u32] = bytemuck::cast_slice(&mapped);
                    self.compute_cull_visibility = data.iter().map(|&v| v != 0).collect();
                    drop(mapped);
                    staging.unmap();
                }
                // Update frustum planes for this frame's compute cull dispatch
                let planes = frustum.planes();
                self.update_frustum_planes(queue, &planes);
            }
        }

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
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
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
                    // Screen-space size threshold culling for non-instanced elements
                    // FastNav: use higher threshold during interaction to skip more small elements
                    let effective_min_pixels = if self.interaction_active {
                        self.fastnav_min_screen_pixels
                    } else {
                        self.min_screen_pixels
                    };
                    let cam_pos = camera.position();
                    let screen_height = self.height as f32;
                    let use_gpu_cull = self.use_compute_culling();
                    for (idx, range) in self.element_draw_ranges.iter().enumerate() {
                        // Use GPU compute culling results when available (previous frame),
                        // fall back to CPU frustum test otherwise
                        if use_gpu_cull {
                            if idx < self.compute_cull_visibility.len()
                                && !self.compute_cull_visibility[idx]
                            {
                                continue;
                            }
                        } else if !frustum.intersects_aabb(range.aabb_min, range.aabb_max) {
                            continue;
                        }
                        // Estimate screen-space size and skip tiny elements
                        if effective_min_pixels > 0.0 {
                            let cx = (range.aabb_min[0] + range.aabb_max[0]) * 0.5;
                            let cy = (range.aabb_min[1] + range.aabb_max[1]) * 0.5;
                            let cz = (range.aabb_min[2] + range.aabb_max[2]) * 0.5;
                            let dx = cx - cam_pos[0];
                            let dy = cy - cam_pos[1];
                            let dz = cz - cam_pos[2];
                            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                            if dist > 0.001 {
                                // World-space size: max extent of the AABB
                                let sx = range.aabb_max[0] - range.aabb_min[0];
                                let sy = range.aabb_max[1] - range.aabb_min[1];
                                let sz = range.aabb_max[2] - range.aabb_min[2];
                                let world_size = sx.max(sy).max(sz);
                                let screen_size = world_size / dist * screen_height;
                                if screen_size < effective_min_pixels {
                                    continue;
                                }
                            }
                        }
                        let start = range.index_start;
                        let end = start + range.index_count;
                        render_pass.draw_indexed(start..end, 0, 0..1);
                    }
                }
            }

            // Edge rendering pass (wireframe overlay on top of solid geometry).
            // Only in Shaded mode — skip for Wireframe (already wireframe) and XRay (looks wrong).
            // FastNav: skip edge rendering during camera interaction for 2-4x speedup
            if self.edge_rendering_enabled && self.render_mode == RenderMode::Shaded && !self.interaction_active {
                if visible_instance_count > 0 {
                    // Instanced edge pass
                    if let (Some(pipeline), Some(unit_vb), Some(unit_ib), Some(inst_buf), Some(bg)) = (
                        &self.pipeline,
                        &self.unit_box_vertex_buffer,
                        &self.unit_box_index_buffer,
                        &self.instance_buffer,
                        &self.bind_group,
                    ) {
                        if let Some(edge_pipe) = pipeline.get_instanced_edge_pipeline() {
                            render_pass.set_pipeline(edge_pipe);
                            render_pass.set_bind_group(0, bg, &[]);
                            render_pass.set_vertex_buffer(0, unit_vb.slice(..));
                            render_pass.set_vertex_buffer(1, inst_buf.slice(..));
                            render_pass.set_index_buffer(unit_ib.slice(..), wgpu::IndexFormat::Uint32);
                            render_pass.draw_indexed(0..self.unit_box_index_count, 0, 0..visible_instance_count);
                        }
                    }
                } else if let (Some(pipeline), Some(vb), Some(ib), Some(bg)) = (
                    &self.pipeline,
                    &self.vertex_buffer,
                    &self.index_buffer,
                    &self.bind_group,
                ) {
                    // Non-instanced edge pass
                    if let Some(edge_pipe) = pipeline.get_edge_pipeline() {
                        render_pass.set_pipeline(edge_pipe);
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
            }
        }

        // Section fill pass: draw solid caps at section plane intersections
        // Only for non-instanced geometry (has vertex/index buffers)
        let has_active_planes = self.section_plane_uniform.count > 0;
        if self.section_fill_enabled && has_active_planes {
            if let (
                Some(section_fill),
                Some(fill_uniform_buf),
                Some(fill_bg),
                Some(stencil_bg),
                Some(vb),
                Some(ib),
            ) = (
                &self.section_fill,
                &self.section_fill_uniform_buffer,
                &self.section_fill_bind_group,
                &self.section_fill_stencil_bind_group,
                &self.vertex_buffer,
                &self.index_buffer,
            ) {
                // Compute model bounds from element draw ranges (for quad half-size)
                let half_size = if !self.element_draw_ranges.is_empty() {
                    let mut scene_min = [f32::MAX; 3];
                    let mut scene_max = [f32::MIN; 3];
                    for range in &self.element_draw_ranges {
                        for i in 0..3 {
                            scene_min[i] = scene_min[i].min(range.aabb_min[i]);
                            scene_max[i] = scene_max[i].max(range.aabb_max[i]);
                        }
                    }
                    let extent = [
                        scene_max[0] - scene_min[0],
                        scene_max[1] - scene_min[1],
                        scene_max[2] - scene_min[2],
                    ];
                    let max_extent = extent[0].max(extent[1]).max(extent[2]);
                    max_extent * 2.0 // generous size to cover all geometry
                } else {
                    500.0 // default fallback
                };

                // Render section fill for each active plane
                for i in 0..self.section_plane_uniform.count as usize {
                    let plane = &self.section_plane_uniform.planes[i];
                    if plane.enabled < 0.5 {
                        continue;
                    }

                    // Update section fill uniform for this plane
                    let fill_uniform = SectionFillUniform {
                        fill_color: self.section_fill_color,
                        plane_origin: [plane.origin[0], plane.origin[1], plane.origin[2], 0.0],
                        plane_normal: [plane.normal[0], plane.normal[1], plane.normal[2], half_size],
                    };
                    queue.write_buffer(fill_uniform_buf, 0, bytemuck::cast_slice(&[fill_uniform]));

                    // Determine render target for section fill
                    let (render_view, resolve_target): (&wgpu::TextureView, Option<&wgpu::TextureView>) =
                        if self.msaa_view.is_some() {
                            (self.msaa_view.as_ref().unwrap(), Some(color_view))
                        } else {
                            (color_view, None)
                        };

                    // Pass 1: Stencil marking — render back faces (increment) then front faces (decrement)
                    {
                        let mut stencil_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Section Stencil Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: render_view,
                                resolve_target,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load, // Keep existing color
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                view: depth_view,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Load, // Keep existing depth
                                    store: wgpu::StoreOp::Store,
                                }),
                                stencil_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(0), // Clear stencil for this plane
                                    store: wgpu::StoreOp::Store,
                                }),
                            }),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                        // Back faces: increment stencil
                        stencil_pass.set_pipeline(&section_fill.stencil_back_pipeline);
                        stencil_pass.set_bind_group(0, stencil_bg, &[]);
                        stencil_pass.set_vertex_buffer(0, vb.slice(..));
                        stencil_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                        stencil_pass.draw_indexed(0..self.num_indices, 0, 0..1);

                        // Front faces: decrement stencil
                        stencil_pass.set_pipeline(&section_fill.stencil_front_pipeline);
                        stencil_pass.draw_indexed(0..self.num_indices, 0, 0..1);
                    }

                    // Pass 2: Fill quad — render only where stencil is non-zero
                    {
                        let mut fill_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Section Fill Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: render_view,
                                resolve_target,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                view: depth_view,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                }),
                                stencil_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Load, // Read stencil from marking pass
                                    store: wgpu::StoreOp::Store,
                                }),
                            }),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                        fill_pass.set_pipeline(&section_fill.fill_pipeline);
                        fill_pass.set_bind_group(0, fill_bg, &[]);
                        fill_pass.set_stencil_reference(0); // Compare against 0; draw where != 0
                        fill_pass.draw(0..6, 0..1); // 6 vertices for the quad (2 triangles)
                    }
                }
            }
        }

        // Shadow map pass: render scene from light perspective (if enabled)
        if self.shadow_enabled {
            if let (Some(shadow), Some(shadow_bg), Some(vb), Some(ib)) = (
                &self.shadow,
                &self.shadow_bind_group,
                &self.vertex_buffer,
                &self.index_buffer,
            ) {
                // Compute light view-projection from scene bounds and light direction
                let scene_min;
                let scene_max;
                if !self.element_draw_ranges.is_empty() {
                    let mut smin = [f32::MAX; 3];
                    let mut smax = [f32::MIN; 3];
                    for range in &self.element_draw_ranges {
                        for j in 0..3 {
                            smin[j] = smin[j].min(range.aabb_min[j]);
                            smax[j] = smax[j].max(range.aabb_max[j]);
                        }
                    }
                    scene_min = smin;
                    scene_max = smax;
                } else {
                    scene_min = [-10.0, -10.0, -10.0];
                    scene_max = [10.0, 10.0, 10.0];
                }

                let light_vp = Self::compute_light_view_proj(
                    self.light_uniform.direction,
                    scene_min,
                    scene_max,
                );
                let shadow_uniform = ShadowUniform {
                    light_view_proj: light_vp.to_cols_array_2d(),
                    shadow_bias: 0.005,
                    shadow_map_size: shadow.map_size as f32,
                    _pad: [0.0; 2],
                };
                queue.write_buffer(&shadow.uniform_buffer, 0, bytemuck::cast_slice(&[shadow_uniform]));

                // Shadow depth pass
                {
                    let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Shadow Map Pass"),
                        color_attachments: &[],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &shadow.depth_texture_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    shadow_pass.set_pipeline(&shadow.pipeline);
                    shadow_pass.set_bind_group(0, shadow_bg, &[]);
                    shadow_pass.set_vertex_buffer(0, vb.slice(..));
                    shadow_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    shadow_pass.draw_indexed(0..self.num_indices, 0, 0..1);
                }
            }
        }

        // Dispatch GPU compute culling for next frame (runs asynchronously on GPU)
        // Results will be read back at the start of the next render_frame call
        self.run_compute_cull(&mut encoder);

        // SSAO passes: generate occlusion, blur, and composite with color
        // This runs after all geometry passes (main, edges, section fill, shadow)
        // but before FXAA post-processing.
        // FastNav: skip SSAO during camera interaction for faster frames
        let ssao_active = self.ssao_enabled && self.ssao.is_some() && !self.interaction_active;
        if ssao_active {
            // Update SSAO params with current camera projection matrices
            if let Some(ssao_res) = &self.ssao {
                let proj = camera.projection_matrix();
                let inv_proj = proj.inverse();
                let ssao_params = SsaoParams {
                    projection: proj.to_cols_array_2d(),
                    inv_projection: inv_proj.to_cols_array_2d(),
                    kernel_size: 32,
                    radius: self.ssao_radius,
                    bias: 0.025,
                    intensity: self.ssao_intensity,
                    screen_width: self.width as f32,
                    screen_height: self.height as f32,
                    _pad: [0.0; 2],
                };
                queue.write_buffer(&ssao_res.params_buffer, 0, bytemuck::cast_slice(&[ssao_params]));

                // Update composite params with environment settings
                let composite_params = SsaoCompositeParams {
                    screen_width: self.width as f32,
                    screen_height: self.height as f32,
                    ssao_enabled: 1,
                    env_enabled: if self.env_enabled { 1 } else { 0 },
                    sky_color: self.env_uniform.sky_color,
                    ground_color: self.env_uniform.ground_color,
                    horizon_color: self.env_uniform.horizon_color,
                    env_intensity: self.env_uniform.intensity,
                    _pad: [0.0; 3],
                };
                queue.write_buffer(&ssao_res.composite_params_buffer, 0, bytemuck::cast_slice(&[composite_params]));
            }

            // SSAO depth pre-pass: render scene depth to non-MSAA depth texture
            // (SSAO needs a single-sample depth buffer for screen-space sampling)
            if let (Some(ssao_depth_view), Some(_pipeline_res), Some(_bg)) = (
                &self.ssao_depth_texture_view,
                &self.pipeline,
                &self.bind_group,
            ) {
                let mut depth_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("SSAO Depth Pre-pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: ssao_depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                // We need a depth-only pipeline. Reuse the shadow pipeline approach,
                // but with the camera view-proj. For simplicity, use the main pipeline
                // but with no color writes (the depth will still be written).
                // Actually, we can't easily do this without a separate depth-only pipeline
                // that uses our camera uniform. Let's use the existing shadow pipeline
                // but update the shadow uniform to use camera's view-proj.
                if let (Some(shadow_res), Some(shadow_bg)) = (&self.shadow, &self.shadow_bind_group) {
                    let cam_vp = camera.view_projection_matrix();
                    let depth_uniform = ShadowUniform {
                        light_view_proj: cam_vp.to_cols_array_2d(),
                        shadow_bias: 0.0,
                        shadow_map_size: self.width as f32,
                        _pad: [0.0; 2],
                    };
                    queue.write_buffer(&shadow_res.uniform_buffer, 0, bytemuck::cast_slice(&[depth_uniform]));

                    depth_pass.set_pipeline(&shadow_res.pipeline);
                    depth_pass.set_bind_group(0, shadow_bg, &[]);

                    if visible_instance_count > 0 {
                        if let (Some(unit_vb), Some(unit_ib)) = (
                            &self.unit_box_vertex_buffer,
                            &self.unit_box_index_buffer,
                        ) {
                            // For instanced rendering, we'd need an instanced shadow pipeline.
                            // For now, skip SSAO depth pass for instanced-only scenes.
                            // SSAO works best with non-instanced geometry anyway.
                            let _ = (unit_vb, unit_ib);
                        }
                    } else if let (Some(vb), Some(ib)) = (&self.vertex_buffer, &self.index_buffer) {
                        depth_pass.set_vertex_buffer(0, vb.slice(..));
                        depth_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                        depth_pass.draw_indexed(0..self.num_indices, 0, 0..1);
                    }
                }
            }

            // SSAO sampling pass: read depth + noise, output occlusion
            if let (Some(ssao_view), Some(ssao_bg)) = (
                &self.ssao_texture_view,
                &self.ssao_bind_group,
            ) {
                if let Some(ssao_res) = &self.ssao {
                    let mut ssao_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("SSAO Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: ssao_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    ssao_pass.set_pipeline(&ssao_res.ssao_pipeline);
                    ssao_pass.set_bind_group(0, ssao_bg, &[]);
                    ssao_pass.draw(0..3, 0..1);
                }
            }

            // SSAO blur pass: smooth the noisy occlusion result
            if let (Some(blur_view), Some(blur_bg)) = (
                &self.ssao_blur_texture_view,
                &self.ssao_blur_bind_group,
            ) {
                if let Some(ssao_res) = &self.ssao {
                    let mut blur_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("SSAO Blur Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: blur_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    blur_pass.set_pipeline(&ssao_res.blur_pipeline);
                    blur_pass.set_bind_group(0, blur_bg, &[]);
                    blur_pass.draw(0..3, 0..1);
                }
            }

            // SSAO composite pass: multiply color by occlusion factor
            if let (Some(composite_view), Some(composite_bg)) = (
                &self.ssao_composite_texture_view,
                &self.ssao_composite_bind_group,
            ) {
                if let Some(ssao_res) = &self.ssao {
                    let mut composite_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("SSAO Composite Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: composite_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    composite_pass.set_pipeline(&ssao_res.composite_pipeline);
                    composite_pass.set_bind_group(0, composite_bg, &[]);
                    composite_pass.draw(0..3, 0..1);
                }
            }
        }

        // Determine which texture to read back from (FXAA output or resolve/color)
        let fxaa_enabled = self.fxaa.as_ref().map_or(false, |f| f.enabled);

        // FXAA post-process pass
        // When SSAO is active, FXAA reads from the SSAO composite texture instead of color
        if fxaa_enabled {
            if let Some(fxaa) = &self.fxaa {
                // Select the appropriate bind group based on SSAO state
                let bind_group_ref = if ssao_active {
                    self.fxaa_ssao_bind_group.as_ref()
                } else {
                    fxaa.bind_group.as_ref()
                };
                if let (Some(fxaa_view), Some(bind_group)) = (&self.fxaa_texture_view, bind_group_ref) {
                    let mut fxaa_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("FXAA Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: fxaa_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    fxaa_pass.set_pipeline(&fxaa.pipeline);
                    fxaa_pass.set_bind_group(0, bind_group, &[]);
                    fxaa_pass.draw(0..3, 0..1); // Full-screen triangle
                }
            }
        }

        // Double-buffered readback: copy current frame to one buffer while
        // reading the previous frame from the other buffer.
        let padded_bytes_per_row = self.padded_bytes_per_row;
        let bytes_per_pixel = 4u32;
        let unpadded_row = (self.width * bytes_per_pixel) as usize;
        let expected_size = unpadded_row * self.height as usize;

        // Choose the source texture for readback:
        // Priority: FXAA output > SSAO composite output > color/resolve
        let readback_texture = if fxaa_enabled {
            self.fxaa_texture.as_ref().unwrap()
        } else if ssao_active {
            self.ssao_composite_texture.as_ref().unwrap_or(self.color_texture.as_ref().unwrap())
        } else {
            self.color_texture.as_ref().unwrap()
        };

        // Select which buffer to write to this frame (alternating A/B)
        let write_buffer = if self.use_buffer_a {
            self.read_buffer.as_ref().unwrap()
        } else {
            self.read_buffer_b.as_ref().unwrap()
        };

        // Copy current frame's texture to the write buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: readback_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: write_buffer,
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

        // Submit GPU work (render + copy)
        queue.submit(std::iter::once(encoder.finish()));

        if !self.has_previous_frame {
            // First frame: must wait synchronously since there's no previous frame to return
            let buffer_slice = write_buffer.slice(..);
            buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
            device.poll(wgpu::Maintain::Wait);

            let data = buffer_slice.get_mapped_range();
            self.pixel_buffer.resize(expected_size, 0);
            for y in 0..self.height as usize {
                let src_start = y * padded_bytes_per_row as usize;
                let dst_start = y * unpadded_row;
                self.pixel_buffer[dst_start..dst_start + unpadded_row]
                    .copy_from_slice(&data[src_start..src_start + unpadded_row]);
            }
            drop(data);
            write_buffer.unmap();

            self.has_previous_frame = true;
            self.use_buffer_a = !self.use_buffer_a;

            let result = std::mem::take(&mut self.pixel_buffer);
            self.pixel_buffer = Vec::with_capacity(expected_size);
            result
        } else {
            // Read from the OTHER buffer (previous frame's data) while GPU
            // processes the copy into the write buffer
            let read_buffer = if self.use_buffer_a {
                // We're writing to A, so read from B (previous frame)
                self.read_buffer_b.as_ref().unwrap()
            } else {
                // We're writing to B, so read from A (previous frame)
                self.read_buffer.as_ref().unwrap()
            };

            // Wait for GPU to finish (ensures the previous frame's copy is done)
            let buffer_slice = read_buffer.slice(..);
            buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
            device.poll(wgpu::Maintain::Wait);

            let data = buffer_slice.get_mapped_range();
            // Use the alternate pixel buffer for double-buffering
            let pixel_buf = if self.use_buffer_a {
                &mut self.pixel_buffer_b
            } else {
                &mut self.pixel_buffer
            };
            pixel_buf.resize(expected_size, 0);
            for y in 0..self.height as usize {
                let src_start = y * padded_bytes_per_row as usize;
                let dst_start = y * unpadded_row;
                pixel_buf[dst_start..dst_start + unpadded_row]
                    .copy_from_slice(&data[src_start..src_start + unpadded_row]);
            }
            drop(data);
            read_buffer.unmap();

            self.use_buffer_a = !self.use_buffer_a;

            // Return the previous frame's pixels
            let result = if !self.use_buffer_a {
                // We just flipped, so the data we read is in pixel_buffer_b
                let r = std::mem::take(&mut self.pixel_buffer_b);
                self.pixel_buffer_b = Vec::with_capacity(expected_size);
                r
            } else {
                let r = std::mem::take(&mut self.pixel_buffer);
                self.pixel_buffer = Vec::with_capacity(expected_size);
                r
            };
            result
        }
    }

    /// Set interaction active state for FastNav adaptive quality
    pub fn set_interaction_active(&mut self, active: bool) {
        self.interaction_active = active;
    }

    /// Check if compute culling is ready and should be used
    fn use_compute_culling(&self) -> bool {
        self.compute_cull_auto
            && self.compute_cull_state.as_ref().map_or(false, |s| s.enabled && s.element_count > 0)
            && !self.compute_cull_visibility.is_empty()
    }
}

// Need to add buffer init descriptor
use wgpu::util::DeviceExt;

// ====================================================================
// Compute Shader Frustum Culling State
// ====================================================================

/// GPU-side state for compute shader frustum culling.
///
/// Holds the frustum planes uniform buffer, AABB storage buffer,
/// visibility output buffer, and an optional staging buffer for
/// CPU readback of visibility results.
pub struct ComputeCullState {
    pub frustum_buffer: wgpu::Buffer,
    pub aabb_buffer: Option<wgpu::Buffer>,
    pub visibility_buffer: Option<wgpu::Buffer>,
    pub result_staging_buffer: Option<wgpu::Buffer>,
    pub bind_group: Option<wgpu::BindGroup>,
    pub element_count: u32,
    pub enabled: bool,
}

impl ComputeCullState {
    /// Create a new compute cull state with the given frustum uniform buffer.
    pub fn new(frustum_buffer: wgpu::Buffer) -> Self {
        Self {
            frustum_buffer,
            aabb_buffer: None,
            visibility_buffer: None,
            result_staging_buffer: None,
            bind_group: None,
            element_count: 0,
            enabled: true,
        }
    }
}

impl SceneRenderer {
    /// Initialize the compute culling pipeline and frustum uniform buffer.
    ///
    /// Must be called after `initialize()` so that a wgpu device is available.
    /// Stores the pipeline resources and creates the frustum planes uniform buffer.
    pub fn init_compute_culling(&mut self, device: &wgpu::Device) -> bool {
        if let Some(resources) = create_compute_cull_pipeline(device) {
            // Create the frustum planes uniform buffer (6 x vec4<f32> = 96 bytes)
            let frustum_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Compute Cull Frustum Buffer"),
                size: 6 * 4 * 4, // 6 planes * 4 floats * 4 bytes = 96 bytes
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            self.compute_cull_resources = Some(resources);
            self.compute_cull_state = Some(ComputeCullState::new(frustum_buffer));
            true
        } else {
            false
        }
    }

    /// Upload AABB data for compute culling.
    ///
    /// Each AABB is 6 floats: [min_x, min_y, min_z, max_x, max_y, max_z].
    /// The data is expanded to pairs of vec4<f32> on the GPU (with w=0 padding).
    pub fn upload_aabbs_for_compute(&mut self, device: &wgpu::Device, aabbs: &[[f32; 6]]) {
        let (Some(resources), Some(state)) = (&self.compute_cull_resources, &mut self.compute_cull_state) else {
            return;
        };

        let element_count = aabbs.len() as u32;
        state.element_count = element_count;

        if element_count == 0 {
            state.aabb_buffer = None;
            state.visibility_buffer = None;
            state.result_staging_buffer = None;
            state.bind_group = None;
            return;
        }

        // Pack AABBs as pairs of vec4<f32>: [min_x, min_y, min_z, 0.0, max_x, max_y, max_z, 0.0]
        let mut aabb_data: Vec<f32> = Vec::with_capacity(aabbs.len() * 8);
        for aabb in aabbs {
            aabb_data.extend_from_slice(&[aabb[0], aabb[1], aabb[2], 0.0]); // min + padding
            aabb_data.extend_from_slice(&[aabb[3], aabb[4], aabb[5], 0.0]); // max + padding
        }

        let aabb_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Compute Cull AABB Buffer"),
            contents: bytemuck::cast_slice(&aabb_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Visibility output buffer (one u32 per element)
        let visibility_size = (element_count as u64) * 4;
        let visibility_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Cull Visibility Buffer"),
            size: visibility_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Staging buffer for CPU readback
        let result_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Cull Staging Buffer"),
            size: visibility_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Cull Bind Group"),
            layout: &resources.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: state.frustum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: aabb_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: visibility_buffer.as_entire_binding(),
                },
            ],
        });

        state.aabb_buffer = Some(aabb_buffer);
        state.visibility_buffer = Some(visibility_buffer);
        state.result_staging_buffer = Some(result_staging_buffer);
        state.bind_group = Some(bind_group);
    }

    /// Dispatch the compute culling shader.
    ///
    /// Encodes a compute pass that tests all AABBs against the current frustum planes.
    /// The number of workgroups is ceil(element_count / 64).
    pub fn run_compute_cull(&self, encoder: &mut wgpu::CommandEncoder) {
        let (Some(resources), Some(state)) = (&self.compute_cull_resources, &self.compute_cull_state) else {
            return;
        };

        if !state.enabled || state.element_count == 0 {
            return;
        }

        let Some(bind_group) = &state.bind_group else {
            return;
        };

        let workgroup_count = (state.element_count + 63) / 64;

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Cull Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&resources.pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Copy visibility results to staging buffer for potential CPU readback
        if let (Some(vis_buf), Some(staging_buf)) = (&state.visibility_buffer, &state.result_staging_buffer) {
            let copy_size = (state.element_count as u64) * 4;
            encoder.copy_buffer_to_buffer(vis_buf, 0, staging_buf, 0, copy_size);
        }
    }

    /// Read back the visibility results from the staging buffer.
    ///
    /// Returns `None` if compute culling is not initialized or no results are available.
    /// NOTE: This requires the staging buffer to have been mapped (async operation).
    /// In practice, results may also be consumed directly on the GPU via indirect draw.
    pub fn read_visibility_results(&self) -> Option<Vec<bool>> {
        let state = self.compute_cull_state.as_ref()?;
        let staging = state.result_staging_buffer.as_ref()?;

        if state.element_count == 0 {
            return Some(Vec::new());
        }

        // Attempt to read from the mapped staging buffer
        let slice = staging.slice(..);
        // Note: In a real async workflow, map_async + poll would be called before this.
        // This synchronous attempt will only succeed if the buffer is already mapped.
        let mapped = slice.get_mapped_range();
        let data: &[u32] = bytemuck::cast_slice(&mapped);
        let results: Vec<bool> = data.iter().map(|&v| v != 0).collect();
        drop(mapped);
        staging.unmap();
        Some(results)
    }

    /// Enable or disable compute frustum culling.
    pub fn set_compute_culling_enabled(&mut self, enabled: bool) {
        if let Some(state) = &mut self.compute_cull_state {
            state.enabled = enabled;
        }
    }

    /// Update the frustum planes uniform buffer with new plane equations.
    ///
    /// Each plane is [a, b, c, d] where ax + by + cz + d = 0.
    pub fn update_frustum_planes(&self, queue: &wgpu::Queue, planes: &[[f32; 4]; 6]) {
        if let Some(state) = &self.compute_cull_state {
            let plane_data: Vec<f32> = planes.iter().flat_map(|p| p.iter().copied()).collect();
            queue.write_buffer(&state.frustum_buffer, 0, bytemuck::cast_slice(&plane_data));
        }
    }
}

// ====================================================================
// Vertex Buffer Streaming
// ====================================================================

/// A spatial chunk of vertex data for streaming.
///
/// Tracks a contiguous range of vertices, its bounding box, current
/// distance to camera, and whether it is currently resident on the GPU.
#[derive(Debug, Clone)]
pub struct VertexChunk {
    pub vertex_offset: usize,
    pub vertex_count: usize,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub distance_to_camera: f32,
    pub on_gpu: bool,
}

/// Manages streaming of vertex data to/from GPU memory in chunks.
///
/// Partitions a large vertex buffer into spatial chunks and loads/evicts
/// them based on camera proximity and a per-frame upload budget.
pub struct VertexStreamManager {
    pub chunks: Vec<VertexChunk>,
    pub loaded_chunks: Vec<usize>,
    pub config: VertexStreamConfig,
    pub total_vertices: usize,
    pub gpu_buffer: Option<wgpu::Buffer>,
}

impl VertexStreamManager {
    /// Create a new vertex stream manager with the given configuration.
    pub fn new(config: VertexStreamConfig) -> Self {
        Self {
            chunks: Vec::new(),
            loaded_chunks: Vec::new(),
            config,
            total_vertices: 0,
            gpu_buffer: None,
        }
    }

    /// Partition a vertex range into chunks based on spatial bounds.
    ///
    /// `bounds` provides one (min, max) bounding box per chunk-sized group
    /// of vertices. If bounds has fewer entries than chunks, remaining chunks
    /// get a default bounding box.
    pub fn partition_vertices(
        &mut self,
        total_vertices: usize,
        bounds: &[([f32; 3], [f32; 3])],
    ) {
        self.total_vertices = total_vertices;
        self.chunks.clear();
        self.loaded_chunks.clear();

        if total_vertices == 0 {
            return;
        }

        let chunk_size = self.config.chunk_size.max(1);
        let num_chunks = (total_vertices + chunk_size - 1) / chunk_size;

        for i in 0..num_chunks {
            let offset = i * chunk_size;
            let count = (total_vertices - offset).min(chunk_size);

            let (bounds_min, bounds_max) = if i < bounds.len() {
                (bounds[i].0, bounds[i].1)
            } else {
                ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0])
            };

            self.chunks.push(VertexChunk {
                vertex_offset: offset,
                vertex_count: count,
                bounds_min,
                bounds_max,
                distance_to_camera: f32::MAX,
                on_gpu: false,
            });
        }
    }

    /// Recalculate distance from each chunk's center to the camera position,
    /// then sort chunks by distance (closest first).
    pub fn update_priorities(&mut self, camera_pos: [f32; 3]) {
        for chunk in &mut self.chunks {
            let cx = (chunk.bounds_min[0] + chunk.bounds_max[0]) * 0.5;
            let cy = (chunk.bounds_min[1] + chunk.bounds_max[1]) * 0.5;
            let cz = (chunk.bounds_min[2] + chunk.bounds_max[2]) * 0.5;

            let dx = cx - camera_pos[0];
            let dy = cy - camera_pos[1];
            let dz = cz - camera_pos[2];

            chunk.distance_to_camera = (dx * dx + dy * dy + dz * dz).sqrt();
        }

        // Sort chunk indices by distance (closest first)
        // We don't reorder chunks in the Vec (that would invalidate indices),
        // but update_priorities just updates the distances.
    }

    /// Return indices of chunks that should be uploaded to GPU.
    ///
    /// Selects the closest chunks that are not yet on GPU, within the
    /// per-frame upload budget and under the maximum GPU chunk limit.
    pub fn get_chunks_to_upload(&self) -> Vec<usize> {
        let mut candidates: Vec<(usize, f32)> = self.chunks.iter().enumerate()
            .filter(|(_, c)| !c.on_gpu && c.distance_to_camera <= self.config.prefetch_distance)
            .map(|(i, c)| (i, c.distance_to_camera))
            .collect();

        // Sort by distance (closest first)
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let current_gpu_count = self.loaded_chunks.len();
        let available_slots = self.config.max_gpu_chunks.saturating_sub(current_gpu_count);

        // Estimate bytes per vertex (20 bytes for packed Vertex format)
        let bytes_per_vertex = 20usize;
        let mut budget_remaining = self.config.upload_budget_bytes;
        let mut result = Vec::new();

        for (idx, _dist) in candidates {
            if result.len() >= available_slots {
                break;
            }
            let chunk_bytes = self.chunks[idx].vertex_count * bytes_per_vertex;
            if chunk_bytes > budget_remaining {
                break;
            }
            budget_remaining -= chunk_bytes;
            result.push(idx);
        }

        result
    }

    /// Return indices of chunks to evict from GPU memory.
    ///
    /// Evicts the farthest loaded chunks when the number of loaded chunks
    /// exceeds `max_gpu_chunks`.
    pub fn get_chunks_to_evict(&self) -> Vec<usize> {
        if self.loaded_chunks.len() <= self.config.max_gpu_chunks {
            return Vec::new();
        }

        let excess = self.loaded_chunks.len() - self.config.max_gpu_chunks;

        // Sort loaded chunks by distance (farthest first)
        let mut loaded_with_dist: Vec<(usize, f32)> = self.loaded_chunks.iter()
            .map(|&idx| (idx, self.chunks[idx].distance_to_camera))
            .collect();
        loaded_with_dist.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        loaded_with_dist.iter().take(excess).map(|&(idx, _)| idx).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_renderer_default_fxaa_disabled() {
        let scene = SceneRenderer::new(800, 600);
        // FXAA should be None before initialization
        assert!(scene.fxaa.is_none());
        assert!(!scene.is_fxaa_enabled());
    }

    #[test]
    fn test_min_screen_pixels_default_and_setter() {
        let mut scene = SceneRenderer::new(800, 600);
        // Default should be 2.0
        assert!((scene.min_screen_pixels - 2.0).abs() < f32::EPSILON);

        // Setting a positive value
        scene.set_min_screen_pixels(5.0);
        assert!((scene.min_screen_pixels - 5.0).abs() < f32::EPSILON);

        // Setting zero
        scene.set_min_screen_pixels(0.0);
        assert!((scene.min_screen_pixels - 0.0).abs() < f32::EPSILON);

        // Negative should be clamped to 0.0
        scene.set_min_screen_pixels(-3.0);
        assert!((scene.min_screen_pixels - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fxaa_enabled_without_initialization() {
        let mut scene = SceneRenderer::new(800, 600);
        // Should be safe to call even when fxaa is None
        scene.set_fxaa_enabled(true);
        assert!(!scene.is_fxaa_enabled()); // Still false because fxaa is None
    }

    #[test]
    fn test_screen_space_size_calculation() {
        // Test the screen-space size formula used in render_frame:
        // screen_size = world_size / distance * screen_height
        let world_size: f32 = 1.0; // 1 meter
        let distance: f32 = 100.0; // 100 meters away
        let screen_height: f32 = 1080.0;
        let screen_size = world_size / distance * screen_height;
        // 1/100 * 1080 = 10.8 pixels — should be drawn with default threshold of 2.0
        assert!(screen_size > 2.0);

        // Very far away: 1m at 10000m = 0.108 pixels — should be culled
        let far_distance: f32 = 10000.0;
        let far_screen_size = world_size / far_distance * screen_height;
        assert!(far_screen_size < 2.0);
    }

    #[test]
    fn test_element_draw_range_construction() {
        let range = ElementDrawRange {
            index_start: 0,
            index_count: 36,
            aabb_min: [-1.0, -1.0, -1.0],
            aabb_max: [1.0, 1.0, 1.0],
        };
        assert_eq!(range.index_start, 0);
        assert_eq!(range.index_count, 36);
        // AABB center should be at origin
        let cx = (range.aabb_min[0] + range.aabb_max[0]) * 0.5;
        let cy = (range.aabb_min[1] + range.aabb_max[1]) * 0.5;
        let cz = (range.aabb_min[2] + range.aabb_max[2]) * 0.5;
        assert!((cx).abs() < f32::EPSILON);
        assert!((cy).abs() < f32::EPSILON);
        assert!((cz).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fxaa_uniform_data_pod() {
        use super::super::pipeline::FxaaUniformData;
        // Ensure FxaaUniformData can be cast via bytemuck
        let data = FxaaUniformData {
            tex_size: [1920.0, 1080.0],
        };
        let bytes: &[u8] = bytemuck::bytes_of(&data);
        assert_eq!(bytes.len(), std::mem::size_of::<FxaaUniformData>());
        assert_eq!(bytes.len(), 8); // 2 * f32 = 8 bytes
    }

    // ---- Section fill tests ----

    #[test]
    fn test_section_fill_color_default_and_setter() {
        let mut scene = SceneRenderer::new(800, 600);
        // Default fill color should be light gray
        let default_color = scene.get_section_fill_color();
        assert!((default_color[0] - 0.85).abs() < f32::EPSILON);
        assert!((default_color[1] - 0.85).abs() < f32::EPSILON);
        assert!((default_color[2] - 0.85).abs() < f32::EPSILON);
        assert!((default_color[3] - 1.0).abs() < f32::EPSILON);

        // Set custom color
        scene.set_section_fill_color(1.0, 0.0, 0.0, 0.5);
        let color = scene.get_section_fill_color();
        assert!((color[0] - 1.0).abs() < f32::EPSILON);
        assert!((color[1] - 0.0).abs() < f32::EPSILON);
        assert!((color[2] - 0.0).abs() < f32::EPSILON);
        assert!((color[3] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_section_fill_enabled_default_and_toggle() {
        let mut scene = SceneRenderer::new(800, 600);
        // Default should be enabled
        assert!(scene.is_section_fill_enabled());

        // Disable
        scene.set_section_fill_enabled(false);
        assert!(!scene.is_section_fill_enabled());

        // Re-enable
        scene.set_section_fill_enabled(true);
        assert!(scene.is_section_fill_enabled());
    }

    #[test]
    fn test_section_fill_uniform_pod_and_size() {
        // SectionFillUniform must be Pod/Zeroable and have expected size
        let uniform = SectionFillUniform {
            fill_color: [0.85, 0.85, 0.85, 1.0],
            plane_origin: [0.0, 5.0, 0.0, 0.0],
            plane_normal: [0.0, 1.0, 0.0, 500.0],
        };
        let bytes: &[u8] = bytemuck::bytes_of(&uniform);
        // 3 * vec4<f32> = 3 * 16 = 48 bytes
        assert_eq!(bytes.len(), 48);
        assert_eq!(std::mem::size_of::<SectionFillUniform>(), 48);
    }

    // ---- Shadow mapping tests ----

    #[test]
    fn test_shadow_uniform_pod_and_size() {
        let uniform = ShadowUniform::new();
        let bytes: &[u8] = bytemuck::bytes_of(&uniform);
        // mat4x4 (64) + f32 (4) + f32 (4) + [f32;2] padding (8) = 80 bytes
        assert_eq!(bytes.len(), 80);
        assert_eq!(std::mem::size_of::<ShadowUniform>(), 80);
        // Default bias should be 0.005
        assert!((uniform.shadow_bias - 0.005).abs() < f32::EPSILON);
        // Default map size should be 2048
        assert!((uniform.shadow_map_size - 2048.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_shadow_enabled_default_and_toggle() {
        let mut scene = SceneRenderer::new(800, 600);
        // Default should be disabled
        assert!(!scene.is_shadows_enabled());

        // Enable
        scene.set_shadows_enabled(true);
        assert!(scene.is_shadows_enabled());

        // Disable
        scene.set_shadows_enabled(false);
        assert!(!scene.is_shadows_enabled());
    }

    #[test]
    fn test_shadow_map_size_clamping() {
        let mut scene = SceneRenderer::new(800, 600);
        // Default
        assert_eq!(scene.shadow_map_size, 2048);

        // Set valid size
        scene.set_shadow_map_size(4096);
        assert_eq!(scene.shadow_map_size, 4096);

        // Too small — clamped to 256
        scene.set_shadow_map_size(64);
        assert_eq!(scene.shadow_map_size, 256);

        // Too large — clamped to 8192
        scene.set_shadow_map_size(16384);
        assert_eq!(scene.shadow_map_size, 8192);
    }

    #[test]
    fn test_light_view_proj_computation() {
        // Verify the light view-projection matrix is computable and non-zero
        let light_dir = [0.5, 0.8, 0.3];
        let scene_min = [-10.0, -5.0, -10.0];
        let scene_max = [10.0, 15.0, 10.0];
        let vp = SceneRenderer::compute_light_view_proj(light_dir, scene_min, scene_max);
        let cols = vp.to_cols_array_2d();
        // Matrix should not be identity or zero
        let mut has_nonzero = false;
        for col in &cols {
            for &v in col {
                if v.abs() > 0.001 {
                    has_nonzero = true;
                }
            }
        }
        assert!(has_nonzero, "Light VP matrix should be non-zero");
    }

    // ---- SSAO tests ----

    #[test]
    fn test_ssao_params_pod_and_size() {
        use super::super::pipeline::SsaoParams;
        let params = SsaoParams::new(1920.0, 1080.0);
        let bytes: &[u8] = bytemuck::bytes_of(&params);
        // 2 * mat4x4 (128) + u32(4) + 3*f32(12) + 2*f32(8) + [f32;2] pad(8) = 160 bytes
        assert_eq!(std::mem::size_of::<SsaoParams>(), 160);
        assert_eq!(bytes.len(), 160);
        assert_eq!(params.kernel_size, 32);
        assert!((params.radius - 0.5).abs() < f32::EPSILON);
        assert!((params.bias - 0.025).abs() < f32::EPSILON);
        assert!((params.intensity - 1.0).abs() < f32::EPSILON);
        assert!((params.screen_width - 1920.0).abs() < f32::EPSILON);
        assert!((params.screen_height - 1080.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_ssao_enabled_default_and_toggle() {
        let mut scene = SceneRenderer::new(800, 600);
        // Default should be disabled
        assert!(!scene.is_ssao_enabled());
        assert!((scene.ssao_radius - 0.5).abs() < f32::EPSILON);
        assert!((scene.ssao_intensity - 1.0).abs() < f32::EPSILON);

        // Enable
        scene.set_ssao_enabled(true);
        assert!(scene.is_ssao_enabled());

        // Disable
        scene.set_ssao_enabled(false);
        assert!(!scene.is_ssao_enabled());
    }

    #[test]
    fn test_ssao_radius_and_intensity_setters() {
        let mut scene = SceneRenderer::new(800, 600);

        // Set valid radius
        scene.set_ssao_radius(1.5);
        assert!((scene.ssao_radius - 1.5).abs() < f32::EPSILON);

        // Clamp radius: too small
        scene.set_ssao_radius(0.0);
        assert!((scene.ssao_radius - 0.01).abs() < f32::EPSILON);

        // Clamp radius: too large
        scene.set_ssao_radius(10.0);
        assert!((scene.ssao_radius - 5.0).abs() < f32::EPSILON);

        // Set valid intensity
        scene.set_ssao_intensity(2.0);
        assert!((scene.ssao_intensity - 2.0).abs() < f32::EPSILON);

        // Clamp intensity: negative
        scene.set_ssao_intensity(-1.0);
        assert!((scene.ssao_intensity - 0.0).abs() < f32::EPSILON);

        // Clamp intensity: too large
        scene.set_ssao_intensity(10.0);
        assert!((scene.ssao_intensity - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_ssao_blur_params_pod_and_size() {
        use super::super::pipeline::SsaoBlurParams;
        let params = SsaoBlurParams {
            texel_size: [1.0 / 1920.0, 1.0 / 1080.0],
            depth_threshold: 0.001,
            _pad: 0.0,
        };
        let bytes: &[u8] = bytemuck::bytes_of(&params);
        // 2*f32 + f32 + f32 = 16 bytes
        assert_eq!(std::mem::size_of::<SsaoBlurParams>(), 16);
        assert_eq!(bytes.len(), 16);
        assert!((params.depth_threshold - 0.001).abs() < f32::EPSILON);
    }

    #[test]
    fn test_ssao_kernel_generation() {
        use super::super::pipeline::generate_ssao_kernel;
        let kernel = generate_ssao_kernel(32);
        // Should have 64 entries (array size), first 32 should be non-zero
        assert_eq!(kernel.len(), 64);

        // Check that the first 32 samples have non-zero values
        for i in 0..32 {
            let s = kernel[i];
            let len_sq = s[0] * s[0] + s[1] * s[1] + s[2] * s[2];
            assert!(len_sq > 0.0, "Kernel sample {} should be non-zero", i);
            // All samples should be within unit sphere (scaled by at most 1.0)
            assert!(len_sq <= 1.01, "Kernel sample {} length^2 = {} exceeds 1", i, len_sq);
        }

        // Last 32 should be zero (not generated)
        for i in 32..64 {
            assert_eq!(kernel[i], [0.0, 0.0, 0.0, 0.0]);
        }
    }

    #[test]
    fn test_ssao_noise_generation() {
        use super::super::pipeline::generate_ssao_noise;
        let noise = generate_ssao_noise();
        // 4x4 pixels * 4 bytes (RGBA) = 64 bytes
        assert_eq!(noise.len(), 64);
        // Alpha channel should be 255 for all pixels
        for i in 0..16 {
            assert_eq!(noise[i * 4 + 3], 255, "Alpha should be 255 at pixel {}", i);
        }
    }

    // ---- Environment lighting tests ----

    #[test]
    fn test_environment_uniform_pod_and_size() {
        use super::super::pipeline::EnvironmentUniform;
        let uniform = EnvironmentUniform::new();
        let bytes: &[u8] = bytemuck::bytes_of(&uniform);
        // 3 * vec4 (48) + f32 (4) + u32 (4) + [f32;2] pad (8) = 64 bytes
        assert_eq!(std::mem::size_of::<EnvironmentUniform>(), 64);
        assert_eq!(bytes.len(), 64);
        // Default values
        assert!((uniform.intensity - 1.0).abs() < f32::EPSILON);
        assert_eq!(uniform.enabled, 0);
    }

    #[test]
    fn test_environment_enabled_default_and_toggle() {
        let mut scene = SceneRenderer::new(800, 600);
        // Default should be disabled
        assert!(!scene.is_environment_enabled());

        // Enable
        scene.set_environment_enabled(true);
        assert!(scene.is_environment_enabled());
        assert_eq!(scene.env_uniform.enabled, 1);

        // Disable
        scene.set_environment_enabled(false);
        assert!(!scene.is_environment_enabled());
        assert_eq!(scene.env_uniform.enabled, 0);
    }

    #[test]
    fn test_environment_colors_and_intensity() {
        let mut scene = SceneRenderer::new(800, 600);

        // Set custom colors
        scene.set_environment_colors(
            [0.5, 0.6, 0.9],   // sky
            [0.2, 0.15, 0.1],  // ground
            [0.4, 0.4, 0.4],   // horizon
        );
        assert!((scene.env_uniform.sky_color[0] - 0.5).abs() < f32::EPSILON);
        assert!((scene.env_uniform.sky_color[1] - 0.6).abs() < f32::EPSILON);
        assert!((scene.env_uniform.sky_color[2] - 0.9).abs() < f32::EPSILON);
        assert!((scene.env_uniform.ground_color[0] - 0.2).abs() < f32::EPSILON);
        assert!((scene.env_uniform.ground_color[1] - 0.15).abs() < f32::EPSILON);
        assert!((scene.env_uniform.ground_color[2] - 0.1).abs() < f32::EPSILON);
        assert!((scene.env_uniform.horizon_color[0] - 0.4).abs() < f32::EPSILON);

        // Set intensity
        scene.set_environment_intensity(0.8);
        assert!((scene.env_uniform.intensity - 0.8).abs() < f32::EPSILON);

        // Negative intensity should be clamped to 0
        scene.set_environment_intensity(-1.0);
        assert!((scene.env_uniform.intensity - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_ssao_composite_params_pod_and_size() {
        use super::super::pipeline::SsaoCompositeParams;
        let params = SsaoCompositeParams {
            screen_width: 1920.0,
            screen_height: 1080.0,
            ssao_enabled: 1,
            env_enabled: 1,
            sky_color: [0.6, 0.7, 0.9, 1.0],
            ground_color: [0.3, 0.25, 0.2, 1.0],
            horizon_color: [0.5, 0.5, 0.5, 1.0],
            env_intensity: 1.0,
            _pad: [0.0; 3],
        };
        let bytes: &[u8] = bytemuck::bytes_of(&params);
        // 2*f32(8) + 2*u32(8) + 3*vec4(48) + f32(4) + [f32;3] pad(12) = 80 bytes
        assert_eq!(std::mem::size_of::<SsaoCompositeParams>(), 80);
        assert_eq!(bytes.len(), 80);
    }

    // ---- Compute culling tests ----

    #[test]
    fn test_compute_cull_state_default() {
        let scene = SceneRenderer::new(800, 600);
        // Compute culling should be None before init
        assert!(scene.compute_cull_resources.is_none());
        assert!(scene.compute_cull_state.is_none());
    }

    // ---- Vertex stream manager tests ----

    #[test]
    fn test_vertex_stream_manager_new() {
        let config = VertexStreamConfig::default();
        let manager = VertexStreamManager::new(config);
        assert!(manager.chunks.is_empty());
        assert!(manager.loaded_chunks.is_empty());
        assert_eq!(manager.total_vertices, 0);
        assert!(manager.gpu_buffer.is_none());
        assert_eq!(manager.config.chunk_size, 65536);
        assert_eq!(manager.config.max_gpu_chunks, 32);
    }

    #[test]
    fn test_vertex_stream_partition() {
        let config = VertexStreamConfig {
            chunk_size: 100,
            max_gpu_chunks: 4,
            prefetch_distance: 50.0,
            upload_budget_bytes: 1024 * 1024,
        };
        let mut manager = VertexStreamManager::new(config);

        // Partition 350 vertices into chunks of 100
        let bounds = vec![
            ([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]),
            ([10.0, 0.0, 0.0], [20.0, 10.0, 10.0]),
            ([20.0, 0.0, 0.0], [30.0, 10.0, 10.0]),
            ([30.0, 0.0, 0.0], [40.0, 10.0, 10.0]),
        ];
        manager.partition_vertices(350, &bounds);

        assert_eq!(manager.total_vertices, 350);
        assert_eq!(manager.chunks.len(), 4); // ceil(350/100) = 4
        assert_eq!(manager.chunks[0].vertex_offset, 0);
        assert_eq!(manager.chunks[0].vertex_count, 100);
        assert_eq!(manager.chunks[1].vertex_offset, 100);
        assert_eq!(manager.chunks[1].vertex_count, 100);
        assert_eq!(manager.chunks[2].vertex_offset, 200);
        assert_eq!(manager.chunks[2].vertex_count, 100);
        assert_eq!(manager.chunks[3].vertex_offset, 300);
        assert_eq!(manager.chunks[3].vertex_count, 50); // remaining
    }

    #[test]
    fn test_vertex_stream_update_priorities() {
        let config = VertexStreamConfig {
            chunk_size: 100,
            max_gpu_chunks: 4,
            prefetch_distance: 1000.0,
            upload_budget_bytes: 1024 * 1024,
        };
        let mut manager = VertexStreamManager::new(config);

        let bounds = vec![
            ([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]),    // center at (5,5,5)
            ([90.0, 0.0, 0.0], [100.0, 10.0, 10.0]),   // center at (95,5,5)
        ];
        manager.partition_vertices(200, &bounds);

        // Camera at origin
        manager.update_priorities([0.0, 0.0, 0.0]);

        // First chunk (center 5,5,5) should be closer than second (center 95,5,5)
        assert!(manager.chunks[0].distance_to_camera < manager.chunks[1].distance_to_camera);

        // Verify approximate distances
        let d0 = (5.0f32 * 5.0 + 5.0 * 5.0 + 5.0 * 5.0).sqrt();
        assert!((manager.chunks[0].distance_to_camera - d0).abs() < 0.01);
    }

    #[test]
    fn test_vertex_stream_chunks_to_upload() {
        let config = VertexStreamConfig {
            chunk_size: 100,
            max_gpu_chunks: 2,
            prefetch_distance: 200.0,
            upload_budget_bytes: 100 * 20 * 3, // budget for 3 chunks of 100 verts at 20 bytes each
        };
        let mut manager = VertexStreamManager::new(config);

        let bounds = vec![
            ([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]),    // close
            ([50.0, 0.0, 0.0], [60.0, 10.0, 10.0]),    // medium
            ([100.0, 0.0, 0.0], [110.0, 10.0, 10.0]),  // far
        ];
        manager.partition_vertices(300, &bounds);
        manager.update_priorities([0.0, 0.0, 0.0]);

        // No chunks on GPU yet, max_gpu_chunks = 2
        let to_upload = manager.get_chunks_to_upload();
        // Should select at most 2 chunks (limited by max_gpu_chunks - loaded_chunks)
        assert!(to_upload.len() <= 2);
        // First should be the closest chunk
        assert_eq!(to_upload[0], 0);
        if to_upload.len() > 1 {
            assert_eq!(to_upload[1], 1);
        }
    }

    #[test]
    fn test_vertex_stream_chunks_to_evict() {
        let config = VertexStreamConfig {
            chunk_size: 100,
            max_gpu_chunks: 2,
            prefetch_distance: 200.0,
            upload_budget_bytes: 1024 * 1024,
        };
        let mut manager = VertexStreamManager::new(config);

        let bounds = vec![
            ([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]),
            ([50.0, 0.0, 0.0], [60.0, 10.0, 10.0]),
            ([100.0, 0.0, 0.0], [110.0, 10.0, 10.0]),
        ];
        manager.partition_vertices(300, &bounds);
        manager.update_priorities([0.0, 0.0, 0.0]);

        // Simulate 3 chunks loaded (over the max of 2)
        manager.loaded_chunks = vec![0, 1, 2];
        manager.chunks[0].on_gpu = true;
        manager.chunks[1].on_gpu = true;
        manager.chunks[2].on_gpu = true;

        let to_evict = manager.get_chunks_to_evict();
        // Should evict 1 chunk (3 loaded - 2 max = 1 excess)
        assert_eq!(to_evict.len(), 1);
        // Should evict the farthest chunk (index 2)
        assert_eq!(to_evict[0], 2);
    }
}
