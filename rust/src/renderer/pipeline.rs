//! Render Pipeline
//!
//! Manages shader compilation and render pipeline configuration.

use super::vertex::Vertex;

/// Vertex shader (WGSL)
const VERTEX_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
};

struct LightUniform {
    direction: vec3<f32>,
    _padding1: f32,
    color: vec3<f32>,
    intensity: f32,
    ambient: vec3<f32>,
    _padding2: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var<uniform> light: LightUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.color = model.color;
    out.normal = model.normal;
    out.world_pos = model.position;
    return out;
}
"#;

/// Fragment shader (WGSL) - optimized for mobile
const FRAGMENT_SHADER: &str = r#"
struct LightUniform {
    direction: vec3<f32>,
    _padding1: f32,
    color: vec3<f32>,
    intensity: f32,
    ambient: vec3<f32>,
    _padding2: f32,
};

@group(0) @binding(1)
var<uniform> light: LightUniform;

struct SectionPlaneUniform {
    origin: vec3<f32>,
    enabled: f32,
    normal: vec3<f32>,
    _padding: f32,
};

@group(0) @binding(2)
var<uniform> section_plane: SectionPlaneUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Section plane clipping
    if (section_plane.enabled > 0.5) {
        let to_point = in.world_pos - section_plane.origin;
        let distance = dot(to_point, section_plane.normal);
        if (distance < 0.0) {
            discard;
        }
    }

    // Simple diffuse + ambient lighting (fast)
    let normal = normalize(in.normal);
    let diff = max(dot(normal, light.direction), 0.0);

    let ambient = light.ambient * in.color.rgb;
    let diffuse = diff * light.color * light.intensity * in.color.rgb;

    let result = ambient + diffuse;
    return vec4<f32>(result, in.color.a);
}
"#;

/// Render mode for the scene
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    #[default]
    Shaded,
    Wireframe,
}

/// MSAA sample count (1 = disabled, 4 = 4x MSAA)
/// Using 1 for mobile performance - can increase on desktop
pub const MSAA_SAMPLE_COUNT: u32 = 1;

/// Render pipeline wrapper
pub struct RenderPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub wireframe_pipeline: Option<wgpu::RenderPipeline>,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
}

impl RenderPipeline {
    /// Create a new render pipeline
    /// If wireframe_supported is true, creates a wireframe pipeline as well
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        Self::new_with_features(device, surface_format, false)
    }

    /// Create a new render pipeline with optional wireframe support
    pub fn new_with_features(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        wireframe_supported: bool,
    ) -> Self {
        // Create shader modules
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(VERTEX_SHADER.into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(FRAGMENT_SHADER.into()),
        });

        // Create bind group layout for camera, light, and section plane uniforms
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // Camera uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Light uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Section plane uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("Camera Bind Group Layout"),
            });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Create wireframe pipeline only if the feature is supported
        let wireframe_pipeline = if wireframe_supported {
            Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Wireframe Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader,
                    entry_point: "vs_main",
                    buffers: &[Vertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, // No culling for wireframe
                    polygon_mode: wgpu::PolygonMode::Line,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: MSAA_SAMPLE_COUNT,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            }))
        } else {
            None
        };

        Self {
            pipeline,
            wireframe_pipeline,
            camera_bind_group_layout,
        }
    }

    /// Get the appropriate pipeline for the render mode
    pub fn get_pipeline(&self, mode: RenderMode) -> &wgpu::RenderPipeline {
        match mode {
            RenderMode::Shaded => &self.pipeline,
            RenderMode::Wireframe => self.wireframe_pipeline.as_ref().unwrap_or(&self.pipeline),
        }
    }
}
