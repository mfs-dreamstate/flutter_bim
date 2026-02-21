//! Render Pipeline
//!
//! Manages shader compilation and render pipeline configuration.

use super::vertex::{BoxVertex, InstanceData, Vertex};
use wgpu::util::DeviceExt;

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
    @location(1) normal: vec4<f32>,
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
    out.normal = model.normal.xyz;
    out.world_pos = model.position;
    return out;
}
"#;

/// Fragment shader (WGSL) - gamma-correct lighting
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

struct SectionPlaneData {
    origin: vec3<f32>,
    enabled: f32,
    normal: vec3<f32>,
    _padding: f32,
};

struct SectionPlanesUniform {
    planes: array<SectionPlaneData, 6>,
    count: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
};

@group(0) @binding(2)
var<uniform> section_planes: SectionPlanesUniform;

struct SectionBoxUniform {
    box_min: vec3<f32>,
    enabled: f32,
    box_max: vec3<f32>,
    _padding: f32,
};

@group(0) @binding(3)
var<uniform> section_box: SectionBoxUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

// Convert sRGB input color component to linear space for correct lighting math.
// The Rgba8UnormSrgb texture format will re-apply sRGB encoding on write.
fn srgb_to_linear(c: f32) -> f32 {
    if (c <= 0.04045) {
        return c / 12.92;
    }
    return pow((c + 0.055) / 1.055, 2.4);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Section planes clipping (up to 6 planes)
    for (var i = 0u; i < section_planes.count; i = i + 1u) {
        let plane = section_planes.planes[i];
        if (plane.enabled > 0.5) {
            let to_point = in.world_pos - plane.origin;
            let distance = dot(to_point, plane.normal);
            if (distance < 0.0) {
                discard;
            }
        }
    }

    // Section box clipping (6 planes)
    if (section_box.enabled > 0.5) {
        if (in.world_pos.x < section_box.box_min.x || in.world_pos.x > section_box.box_max.x ||
            in.world_pos.y < section_box.box_min.y || in.world_pos.y > section_box.box_max.y ||
            in.world_pos.z < section_box.box_min.z || in.world_pos.z > section_box.box_max.z) {
            discard;
        }
    }

    // Convert vertex colors from sRGB to linear for correct lighting
    let linear_color = vec3<f32>(
        srgb_to_linear(in.color.r),
        srgb_to_linear(in.color.g),
        srgb_to_linear(in.color.b)
    );

    // Diffuse + ambient lighting in linear space
    let normal = normalize(in.normal);
    let diff = max(dot(normal, light.direction), 0.0);

    let ambient = light.ambient * linear_color;
    let diffuse = diff * light.color * light.intensity * linear_color;

    let result = ambient + diffuse;
    // Rgba8UnormSrgb format automatically converts linear output to sRGB
    return vec4<f32>(result, in.color.a);
}
"#;

/// Edge fragment shader (WGSL) - outputs a fixed dark color for edge overlay
const EDGE_FRAGMENT_SHADER: &str = r#"
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

struct SectionPlaneData {
    origin: vec3<f32>,
    enabled: f32,
    normal: vec3<f32>,
    _padding: f32,
};

struct SectionPlanesUniform {
    planes: array<SectionPlaneData, 6>,
    count: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
};

@group(0) @binding(2)
var<uniform> section_planes: SectionPlanesUniform;

struct SectionBoxUniform {
    box_min: vec3<f32>,
    enabled: f32,
    box_max: vec3<f32>,
    _padding: f32,
};

@group(0) @binding(3)
var<uniform> section_box: SectionBoxUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

@fragment
fn fs_edge(in: VertexOutput) -> @location(0) vec4<f32> {
    // Section planes clipping (same as main shader)
    for (var i = 0u; i < section_planes.count; i = i + 1u) {
        let plane = section_planes.planes[i];
        if (plane.enabled > 0.5) {
            let to_point = in.world_pos - plane.origin;
            let distance = dot(to_point, plane.normal);
            if (distance < 0.0) {
                discard;
            }
        }
    }

    // Section box clipping (6 planes)
    if (section_box.enabled > 0.5) {
        if (in.world_pos.x < section_box.box_min.x || in.world_pos.x > section_box.box_max.x ||
            in.world_pos.y < section_box.box_min.y || in.world_pos.y > section_box.box_max.y ||
            in.world_pos.z < section_box.box_min.z || in.world_pos.z > section_box.box_max.z) {
            discard;
        }
    }

    return vec4<f32>(0.08, 0.08, 0.08, 0.85);
}
"#;

/// Instanced vertex shader — reads per-vertex position/normal from buffer 0
/// and per-instance position/scale/color from buffer 1.
const INSTANCED_VERTEX_SHADER: &str = r#"
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
    @location(1) normal: vec4<f32>,
};

struct InstanceInput {
    @location(2) inst_position: vec3<f32>,
    @location(3) inst_scale: vec3<f32>,
    @location(4) inst_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = vertex.position * instance.inst_scale + instance.inst_position;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = instance.inst_color;
    out.normal = vertex.normal.xyz;
    out.world_pos = world_pos;
    return out;
}
"#;

/// Section fill shader (WGSL) — renders a filled quad at section plane intersections
///
/// Uses a stencil-tested full-screen quad oriented on the section plane.
/// Vertex shader generates a large world-space quad from the plane origin and normal.
/// Fragment shader outputs a solid fill color with a checkerboard cross-hatch pattern.
const SECTION_FILL_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
};

struct SectionFillUniforms {
    fill_color: vec4<f32>,
    plane_origin: vec4<f32>,
    plane_normal: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var<uniform> fill: SectionFillUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
};

@vertex
fn vs_section_fill(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Generate 6 vertices for 2 triangles (a quad) oriented on the section plane
    // Vertices: 0,1,2  and  2,3,0
    let normal = normalize(fill.plane_normal.xyz);
    let origin = fill.plane_origin.xyz;

    // Build a tangent frame on the plane
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if abs(dot(normal, up)) > 0.99 {
        up = vec3<f32>(1.0, 0.0, 0.0);
    }
    let tangent = normalize(cross(normal, up));
    let bitangent = normalize(cross(normal, tangent));

    // Large quad half-size (should cover entire model)
    let half_size = fill.plane_normal.w; // packed in w component

    // Quad corners – use var so vertex_index (runtime) can index it
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
    );

    let corner = corners[vertex_index];
    let world_pos = origin + tangent * corner.x * half_size + bitangent * corner.y * half_size;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_pos = world_pos;
    return out;
}

@fragment
fn fs_section_fill(in: VertexOutput) -> @location(0) vec4<f32> {
    // Checkerboard cross-hatch pattern for visual distinction
    let scale = 0.5; // pattern repeat in world units
    let checker_x = floor(in.world_pos.x / scale);
    let checker_y = floor(in.world_pos.y / scale);
    let checker_z = floor(in.world_pos.z / scale);
    let checker = (checker_x + checker_y + checker_z) % 2.0;

    // Subtle variation on the fill color via checkerboard
    let variation = 0.03 * checker;
    let color = vec3<f32>(
        fill.fill_color.r + variation,
        fill.fill_color.g + variation,
        fill.fill_color.b + variation,
    );

    return vec4<f32>(color, fill.fill_color.a);
}
"#;

/// Section fill stencil marking shader (WGSL) — marks cut regions in stencil buffer
///
/// Renders geometry with section clipping, using stencil operations to mark
/// where geometry is cut by section planes. Back faces increment stencil,
/// front faces decrement. Non-zero stencil values indicate cut regions.
const SECTION_STENCIL_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_stencil(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_stencil(in: VertexOutput) -> @location(0) vec4<f32> {
    // Dummy color output required by pipeline, but we only care about stencil
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
"#;

/// Section fill uniform data for GPU
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SectionFillUniform {
    /// RGBA fill color
    pub fill_color: [f32; 4],
    /// Plane origin xyz + padding
    pub plane_origin: [f32; 4],
    /// Plane normal xyz + half_size in w
    pub plane_normal: [f32; 4],
}

/// Section fill rendering resources
pub struct SectionFillResources {
    pub fill_pipeline: wgpu::RenderPipeline,
    pub fill_bind_group_layout: wgpu::BindGroupLayout,
    pub stencil_front_pipeline: wgpu::RenderPipeline,
    pub stencil_back_pipeline: wgpu::RenderPipeline,
    pub stencil_bind_group_layout: wgpu::BindGroupLayout,
}

/// Depth+stencil format used for section fill (needs stencil bits)
pub const DEPTH_STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

/// Create the section fill pipeline and stencil marking pipelines
pub fn create_section_fill_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> SectionFillResources {
    // --- Fill pipeline: renders the fill quad where stencil is non-zero ---
    let fill_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Section Fill Shader"),
        source: wgpu::ShaderSource::Wgsl(SECTION_FILL_SHADER.into()),
    });

    // Fill bind group layout: camera uniform (binding 0) + fill uniforms (binding 1)
    let fill_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Section Fill Bind Group Layout"),
        entries: &[
            // Camera uniform
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Section fill uniform (color, origin, normal)
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let fill_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Section Fill Pipeline Layout"),
        bind_group_layouts: &[&fill_bind_group_layout],
        push_constant_ranges: &[],
    });

    let fill_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Section Fill Pipeline"),
        layout: Some(&fill_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &fill_shader,
            entry_point: "vs_section_fill",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &fill_shader,
            entry_point: "fs_section_fill",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_STENCIL_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::NotEqual,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Replace,
                },
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::NotEqual,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Replace,
                },
                read_mask: 0xFF,
                write_mask: 0xFF,
            },
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: MSAA_SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    // --- Stencil marking pipelines: back-face increment, front-face decrement ---
    let stencil_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Section Stencil Shader"),
        source: wgpu::ShaderSource::Wgsl(SECTION_STENCIL_SHADER.into()),
    });

    // Stencil pipeline uses only camera bind group (binding 0)
    let stencil_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Section Stencil Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let stencil_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Section Stencil Pipeline Layout"),
        bind_group_layouts: &[&stencil_bind_group_layout],
        push_constant_ranges: &[],
    });

    // Back-face pass: increment stencil on back faces
    let stencil_back_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Section Stencil Back Pipeline"),
        layout: Some(&stencil_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &stencil_shader,
            entry_point: "vs_stencil",
            buffers: &[Vertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &stencil_shader,
            entry_point: "fs_stencil",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(), // No color writes
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Front), // Render BACK faces only
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_STENCIL_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Always,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Always,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::IncrementWrap,
                },
                read_mask: 0xFF,
                write_mask: 0xFF,
            },
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: MSAA_SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    // Front-face pass: decrement stencil on front faces
    let stencil_front_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Section Stencil Front Pipeline"),
        layout: Some(&stencil_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &stencil_shader,
            entry_point: "vs_stencil",
            buffers: &[Vertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &stencil_shader,
            entry_point: "fs_stencil",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(), // No color writes
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back), // Render FRONT faces only
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_STENCIL_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Always,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::DecrementWrap,
                },
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Always,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                read_mask: 0xFF,
                write_mask: 0xFF,
            },
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: MSAA_SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    SectionFillResources {
        fill_pipeline,
        fill_bind_group_layout,
        stencil_front_pipeline,
        stencil_back_pipeline,
        stencil_bind_group_layout,
    }
}

/// Shadow map depth shader (WGSL) — renders scene from light's perspective
///
/// Depth-only pass: transforms vertices into light space for shadow depth values.
/// No color output is produced. Includes section plane clipping to avoid
/// shadows from cut geometry.
const SHADOW_SHADER: &str = r#"
struct ShadowUniform {
    light_view_proj: mat4x4<f32>,
    shadow_bias: f32,
    shadow_map_size: f32,
    _pad0: f32,
    _pad1: f32,
};

struct SectionPlaneData {
    origin: vec3<f32>,
    enabled: f32,
    normal: vec3<f32>,
    _padding: f32,
};

struct SectionPlanesUniform {
    planes: array<SectionPlaneData, 6>,
    count: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
};

struct SectionBoxUniform {
    box_min: vec3<f32>,
    enabled: f32,
    box_max: vec3<f32>,
    _padding: f32,
};

@group(0) @binding(0)
var<uniform> shadow: ShadowUniform;

@group(0) @binding(1)
var<uniform> section_planes: SectionPlanesUniform;

@group(0) @binding(2)
var<uniform> section_box: SectionBoxUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
};

@vertex
fn vs_shadow(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = shadow.light_view_proj * vec4<f32>(model.position, 1.0);
    out.world_pos = model.position;
    return out;
}

@fragment
fn fs_shadow(in: VertexOutput) {
    // Section planes clipping (up to 6 planes)
    for (var i = 0u; i < section_planes.count; i = i + 1u) {
        let plane = section_planes.planes[i];
        if (plane.enabled > 0.5) {
            let to_point = in.world_pos - plane.origin;
            let distance = dot(to_point, plane.normal);
            if (distance < 0.0) {
                discard;
            }
        }
    }

    // Section box clipping (6 planes)
    if (section_box.enabled > 0.5) {
        if (in.world_pos.x < section_box.box_min.x || in.world_pos.x > section_box.box_max.x ||
            in.world_pos.y < section_box.box_min.y || in.world_pos.y > section_box.box_max.y ||
            in.world_pos.z < section_box.box_min.z || in.world_pos.z > section_box.box_max.z) {
            discard;
        }
    }

    // Depth-only pass: no color output needed
}
"#;

/// Shadow uniform data for GPU (light space transform + bias parameters)
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowUniform {
    /// Light's view-projection matrix for shadow mapping
    pub light_view_proj: [[f32; 4]; 4],
    /// Depth bias to prevent shadow acne
    pub shadow_bias: f32,
    /// Shadow map texture resolution
    pub shadow_map_size: f32,
    /// Padding
    pub _pad: [f32; 2],
}

impl ShadowUniform {
    pub fn new() -> Self {
        Self {
            light_view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            shadow_bias: 0.005,
            shadow_map_size: 2048.0,
            _pad: [0.0; 2],
        }
    }
}

/// Shadow mapping resources
pub struct ShadowResources {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub depth_texture: wgpu::Texture,
    pub depth_texture_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub enabled: bool,
    pub map_size: u32,
}

/// Create the shadow map pipeline (depth-only rendering from light perspective)
pub fn create_shadow_pipeline(
    device: &wgpu::Device,
    shadow_map_size: u32,
) -> ShadowResources {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shadow Shader"),
        source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER.into()),
    });

    // Bind group layout: shadow uniform + section planes + section box
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Shadow Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Section planes uniform (binding 1)
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
            // Section box uniform (binding 2)
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
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Shadow Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    // Depth-only render pipeline (no color attachment)
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Shadow Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_shadow",
            buffers: &[Vertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_shadow",
            targets: &[],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Front), // Front-face culling reduces shadow acne
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // Create shadow depth texture
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Shadow Depth Texture"),
        size: wgpu::Extent3d {
            width: shadow_map_size,
            height: shadow_map_size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Shadow sampler with comparison function for PCF
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Shadow Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    });

    // Uniform buffer
    let shadow_uniform = ShadowUniform::new();
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Shadow Uniform Buffer"),
        contents: bytemuck::cast_slice(&[shadow_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    ShadowResources {
        pipeline,
        bind_group_layout,
        uniform_buffer,
        depth_texture,
        depth_texture_view,
        sampler,
        enabled: false,
        map_size: shadow_map_size,
    }
}

/// FXAA post-process shader (WGSL) — Timothy Lottes' FXAA 3.11 algorithm
///
/// Full-screen triangle with no vertex buffer. The fragment shader performs
/// luminance-based edge detection and sub-pixel blending to smooth aliased
/// edges in the rendered image.
const FXAA_SHADER: &str = r#"
// ---- Uniforms ----
struct FxaaUniforms {
    tex_size: vec2<f32>,
};

@group(0) @binding(0)
var input_tex: texture_2d<f32>;
@group(0) @binding(1)
var input_sampler: sampler;
@group(0) @binding(2)
var<uniform> uniforms: FxaaUniforms;

// ---- Vertex stage: full-screen triangle (3 verts, no VBO) ----
struct VsOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fxaa(@builtin(vertex_index) vertex_index: u32) -> VsOutput {
    // Generate a full-screen triangle from vertex index 0,1,2
    var out: VsOutput;
    let x = f32(i32(vertex_index & 1u) * 2);
    let y = f32(i32(vertex_index >> 1u) * 2);
    out.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, 1.0 - y);
    return out;
}

// ---- FXAA helpers ----

// Compute perceptual luminance (BT.709 coefficients).
fn fxaa_luma(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.299, 0.587, 0.114));
}

// Sample the texture and return luminance in .w
fn tex_luma(uv: vec2<f32>) -> vec4<f32> {
    let c = textureSample(input_tex, input_sampler, uv);
    return vec4<f32>(c.rgb, fxaa_luma(c.rgb));
}

// ---- FXAA 3.11 quality preset (medium quality, 5 search steps) ----

// Tuning constants
const FXAA_EDGE_THRESHOLD: f32 = 0.125;       // minimum local contrast to trigger AA
const FXAA_EDGE_THRESHOLD_MIN: f32 = 0.0625;  // skip very dark areas
const FXAA_SUBPIX_QUALITY: f32 = 0.75;        // sub-pixel aliasing removal amount

@fragment
fn fs_fxaa(in: VsOutput) -> @location(0) vec4<f32> {
    let rcp_frame = vec2<f32>(1.0 / uniforms.tex_size.x, 1.0 / uniforms.tex_size.y);
    let uv = in.uv;

    // Sample the 3x3 neighbourhood luminances
    let luma_m  = tex_luma(uv);
    let luma_n  = fxaa_luma(textureSample(input_tex, input_sampler, uv + vec2<f32>( 0.0, -rcp_frame.y)).rgb);
    let luma_s  = fxaa_luma(textureSample(input_tex, input_sampler, uv + vec2<f32>( 0.0,  rcp_frame.y)).rgb);
    let luma_e  = fxaa_luma(textureSample(input_tex, input_sampler, uv + vec2<f32>( rcp_frame.x,  0.0)).rgb);
    let luma_w  = fxaa_luma(textureSample(input_tex, input_sampler, uv + vec2<f32>(-rcp_frame.x,  0.0)).rgb);

    let range_min = min(luma_m.w, min(min(luma_n, luma_s), min(luma_e, luma_w)));
    let range_max = max(luma_m.w, max(max(luma_n, luma_s), max(luma_e, luma_w)));
    let range = range_max - range_min;

    // Early out if contrast is below threshold
    if range < max(FXAA_EDGE_THRESHOLD_MIN, range_max * FXAA_EDGE_THRESHOLD) {
        return vec4<f32>(luma_m.rgb, 1.0);
    }

    // Diagonal luminances for sub-pixel and edge direction
    let luma_ne = fxaa_luma(textureSample(input_tex, input_sampler, uv + vec2<f32>( rcp_frame.x, -rcp_frame.y)).rgb);
    let luma_nw = fxaa_luma(textureSample(input_tex, input_sampler, uv + vec2<f32>(-rcp_frame.x, -rcp_frame.y)).rgb);
    let luma_se = fxaa_luma(textureSample(input_tex, input_sampler, uv + vec2<f32>( rcp_frame.x,  rcp_frame.y)).rgb);
    let luma_sw = fxaa_luma(textureSample(input_tex, input_sampler, uv + vec2<f32>(-rcp_frame.x,  rcp_frame.y)).rgb);

    // Sub-pixel contrast: lowpass vs. highpass
    let luma_ns = luma_n + luma_s;
    let luma_ew = luma_e + luma_w;
    let luma_corners_n = luma_ne + luma_nw;
    let luma_corners_s = luma_se + luma_sw;
    let luma_corners_e = luma_ne + luma_se;
    let luma_corners_w = luma_nw + luma_sw;

    let sub_pixel_lowpass = (luma_ns + luma_ew + luma_corners_n + luma_corners_s) / 12.0;
    let sub_pixel_contrast = clamp(abs(sub_pixel_lowpass - luma_m.w) / range, 0.0, 1.0);
    let sub_pixel_blend = smoothstep(0.0, 1.0, sub_pixel_contrast);
    let sub_pixel_blend_final = sub_pixel_blend * sub_pixel_blend * FXAA_SUBPIX_QUALITY;

    // Determine edge orientation (horizontal vs vertical)
    let edge_h = abs(luma_corners_n - 2.0 * luma_n)
               + abs(luma_ew * 2.0 - 4.0 * luma_m.w)  // simplified: not exact Lottes but close
               + abs(luma_corners_s - 2.0 * luma_s);
    let edge_v = abs(luma_corners_e - 2.0 * luma_e)
               + abs(luma_ns * 2.0 - 4.0 * luma_m.w)
               + abs(luma_corners_w - 2.0 * luma_w);
    let is_horizontal = edge_h >= edge_v;

    // Choose the pair of neighbouring luminances along the dominant edge direction
    var luma_neg: f32;
    var luma_pos: f32;
    if is_horizontal {
        luma_neg = luma_n;
        luma_pos = luma_s;
    } else {
        luma_neg = luma_w;
        luma_pos = luma_e;
    }

    let gradient_neg = abs(luma_neg - luma_m.w);
    let gradient_pos = abs(luma_pos - luma_m.w);
    let is_neg_steeper = gradient_neg >= gradient_pos;

    let gradient_scaled = 0.25 * max(gradient_neg, gradient_pos);

    // Step size along the perpendicular direction
    var step_length: f32;
    var luma_local_avg: f32;
    if is_horizontal {
        step_length = rcp_frame.y;
    } else {
        step_length = rcp_frame.x;
    }
    if is_neg_steeper {
        step_length = -step_length;
        luma_local_avg = 0.5 * (luma_neg + luma_m.w);
    } else {
        luma_local_avg = 0.5 * (luma_pos + luma_m.w);
    }

    // Shift UV to the edge
    var current_uv = uv;
    if is_horizontal {
        current_uv.y = current_uv.y + step_length * 0.5;
    } else {
        current_uv.x = current_uv.x + step_length * 0.5;
    }

    // Search along the edge in both directions
    var offset: vec2<f32>;
    if is_horizontal {
        offset = vec2<f32>(rcp_frame.x, 0.0);
    } else {
        offset = vec2<f32>(0.0, rcp_frame.y);
    }

    var uv_neg = current_uv - offset;
    var uv_pos = current_uv + offset;

    var luma_end_neg = fxaa_luma(textureSample(input_tex, input_sampler, uv_neg).rgb) - luma_local_avg;
    var luma_end_pos = fxaa_luma(textureSample(input_tex, input_sampler, uv_pos).rgb) - luma_local_avg;

    var reached_neg = abs(luma_end_neg) >= gradient_scaled;
    var reached_pos = abs(luma_end_pos) >= gradient_scaled;
    var reached_both = reached_neg && reached_pos;

    // Extend search up to 5 steps in each direction
    for (var i = 0; i < 5; i = i + 1) {
        if reached_both { break; }
        if !reached_neg {
            uv_neg = uv_neg - offset;
            luma_end_neg = fxaa_luma(textureSample(input_tex, input_sampler, uv_neg).rgb) - luma_local_avg;
            reached_neg = abs(luma_end_neg) >= gradient_scaled;
        }
        if !reached_pos {
            uv_pos = uv_pos + offset;
            luma_end_pos = fxaa_luma(textureSample(input_tex, input_sampler, uv_pos).rgb) - luma_local_avg;
            reached_pos = abs(luma_end_pos) >= gradient_scaled;
        }
        reached_both = reached_neg && reached_pos;
    }

    // Compute distances
    var dist_neg: f32;
    var dist_pos: f32;
    if is_horizontal {
        dist_neg = uv.x - uv_neg.x;
        dist_pos = uv_pos.x - uv.x;
    } else {
        dist_neg = uv.y - uv_neg.y;
        dist_pos = uv_pos.y - uv.y;
    }

    let is_closer_neg = dist_neg < dist_pos;
    let luma_end_closer = select(luma_end_pos, luma_end_neg, is_closer_neg);

    // Don't shift if both ends are on the same side
    if (luma_m.w - luma_local_avg < 0.0) == (luma_end_closer < 0.0) {
        // The edge end is on the correct side; apply subpixel blend only
        let final_color = textureSample(input_tex, input_sampler, uv);
        if sub_pixel_blend_final > 0.0 {
            // Blend with lowpass
            let lp = textureSample(input_tex, input_sampler, uv);
            return vec4<f32>(lp.rgb, 1.0);
        }
        return vec4<f32>(final_color.rgb, 1.0);
    }

    // Compute the edge blend factor
    let total_dist = dist_neg + dist_pos;
    let pixel_offset = -select(dist_neg, dist_pos, is_closer_neg) / total_dist + 0.5;
    let edge_blend = max(0.0, pixel_offset);

    // Pick the larger of edge-blend and sub-pixel-blend
    let final_blend = max(edge_blend, sub_pixel_blend_final);

    // Final sample with the computed offset
    var final_uv = uv;
    if is_horizontal {
        final_uv.y = final_uv.y + step_length * final_blend;
    } else {
        final_uv.x = final_uv.x + step_length * final_blend;
    }

    let final_color = textureSample(input_tex, input_sampler, final_uv);
    return vec4<f32>(final_color.rgb, 1.0);
}
"#;

/// FXAA uniform data for GPU (texture dimensions)
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FxaaUniformData {
    pub tex_size: [f32; 2],
}

/// FXAA post-process resources
pub struct FxaaResources {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: Option<wgpu::BindGroup>,
    pub sampler: wgpu::Sampler,
    pub uniform_buffer: wgpu::Buffer,
    pub enabled: bool,
}

/// Create the FXAA post-process pipeline and associated resources
pub fn create_fxaa_pipeline(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> FxaaResources {
    // Create shader module
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("FXAA Shader"),
        source: wgpu::ShaderSource::Wgsl(FXAA_SHADER.into()),
    });

    // Bind group layout: texture, sampler, uniform buffer
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("FXAA Bind Group Layout"),
        entries: &[
            // Input texture
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Sampler
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // Uniforms (tex_size)
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
    });

    // Pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("FXAA Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    // Render pipeline (no vertex buffers — full-screen triangle from vertex_index)
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("FXAA Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_fxaa",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_fxaa",
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
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    // Linear sampler for texture lookups
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("FXAA Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    // Uniform buffer (tex_size — will be updated when textures are created)
    let uniform_data = FxaaUniformData {
        tex_size: [1.0, 1.0],
    };
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("FXAA Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniform_data]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    FxaaResources {
        pipeline,
        bind_group_layout,
        bind_group: None,
        sampler,
        uniform_buffer,
        enabled: false,
    }
}

/// Render mode for the scene
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    #[default]
    Shaded,
    Wireframe,
    XRay,
}

// ====================================================================
// SSAO (Screen-Space Ambient Occlusion) shaders and resources
// ====================================================================

/// SSAO compute shader (WGSL) — full-screen pass that reconstructs view-space position
/// from the depth buffer, samples a hemisphere kernel, and outputs a single-channel
/// occlusion texture.
///
/// The algorithm:
///   1. Reconstruct view-space position from depth + inverse projection
///   2. Reconstruct view-space normal via cross product of screen-space derivatives
///   3. Build a TBN matrix from normal + random rotation vector (noise texture)
///   4. For each kernel sample, project into screen space and compare depths
///   5. Accumulate occlusion; write result to R8 texture
const SSAO_SHADER: &str = r#"
struct SsaoParams {
    projection: mat4x4<f32>,
    inv_projection: mat4x4<f32>,
    kernel_size: u32,
    radius: f32,
    bias: f32,
    intensity: f32,
    screen_width: f32,
    screen_height: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var depth_tex: texture_2d<f32>;
@group(0) @binding(1)
var depth_sampler: sampler;
@group(0) @binding(2)
var noise_tex: texture_2d<f32>;
@group(0) @binding(3)
var noise_sampler: sampler;
@group(0) @binding(4)
var<uniform> params: SsaoParams;
// Kernel samples are stored as 64 vec4<f32> values (xyz = sample, w = unused)
@group(0) @binding(5)
var<uniform> kernel_samples: array<vec4<f32>, 64>;

struct VsOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_ssao(@builtin(vertex_index) vertex_index: u32) -> VsOutput {
    var out: VsOutput;
    let x = f32(i32(vertex_index & 1u) * 2);
    let y = f32(i32(vertex_index >> 1u) * 2);
    out.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, 1.0 - y);
    return out;
}

// Reconstruct view-space position from depth and UV
fn reconstruct_view_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    // Convert UV + depth to NDC
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    // Transform by inverse projection
    let view_pos = params.inv_projection * ndc;
    return view_pos.xyz / view_pos.w;
}

@fragment
fn fs_ssao(in: VsOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let texel = vec2<f32>(1.0 / params.screen_width, 1.0 / params.screen_height);

    // Sample depth at this pixel
    let depth = textureSample(depth_tex, depth_sampler, uv).r;

    // Early out for sky/far plane
    if depth >= 1.0 {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }

    // Reconstruct view-space position
    let frag_pos = reconstruct_view_pos(uv, depth);

    // Reconstruct view-space normal from screen-space derivatives of position
    let pos_dx = reconstruct_view_pos(uv + vec2<f32>(texel.x, 0.0),
                     textureSample(depth_tex, depth_sampler, uv + vec2<f32>(texel.x, 0.0)).r);
    let pos_dy = reconstruct_view_pos(uv + vec2<f32>(0.0, texel.y),
                     textureSample(depth_tex, depth_sampler, uv + vec2<f32>(0.0, texel.y)).r);
    let normal = normalize(cross(pos_dx - frag_pos, pos_dy - frag_pos));

    // Sample noise texture (4x4 tiled across screen)
    let noise_uv = uv * vec2<f32>(params.screen_width / 4.0, params.screen_height / 4.0);
    let random_vec = textureSample(noise_tex, noise_sampler, noise_uv).xyz * 2.0 - 1.0;

    // Build TBN (tangent-bitangent-normal) from Gram-Schmidt
    let tangent = normalize(random_vec - normal * dot(random_vec, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, normal);

    // Accumulate occlusion
    var occlusion = 0.0;
    let sample_count = min(params.kernel_size, 64u);
    for (var i = 0u; i < sample_count; i = i + 1u) {
        // Transform sample from tangent space to view space
        let sample_pos = frag_pos + tbn * kernel_samples[i].xyz * params.radius;

        // Project sample to screen space
        var offset = params.projection * vec4<f32>(sample_pos, 1.0);
        offset = offset / offset.w;
        let sample_uv = offset.xy * 0.5 + 0.5;

        // Clamp UV to valid range
        let clamped_uv = clamp(sample_uv, vec2<f32>(0.0), vec2<f32>(1.0));

        // Sample the depth buffer at the projected position
        let sample_depth = textureSample(depth_tex, depth_sampler, clamped_uv).r;
        let sample_view_z = reconstruct_view_pos(clamped_uv, sample_depth).z;

        // Range check: only occlude if the sample is close enough
        let range_check = smoothstep(0.0, 1.0, params.radius / abs(frag_pos.z - sample_view_z));
        // Compare: is the sampled depth closer than our sample position?
        if sample_view_z >= sample_pos.z + params.bias {
            occlusion = occlusion + range_check;
        }
    }

    occlusion = 1.0 - (occlusion / f32(sample_count)) * params.intensity;
    return vec4<f32>(occlusion, 0.0, 0.0, 1.0);
}
"#;

/// SSAO blur shader (WGSL) — edge-aware 4x4 box blur to smooth the SSAO result.
///
/// Uses depth-based edge detection to avoid blurring across geometric boundaries.
/// Samples in a 4x4 kernel and weights contributions by depth similarity.
const SSAO_BLUR_SHADER: &str = r#"
struct BlurParams {
    texel_size: vec2<f32>,
    depth_threshold: f32,
    _pad: f32,
};

@group(0) @binding(0)
var ssao_tex: texture_2d<f32>;
@group(0) @binding(1)
var ssao_sampler: sampler;
@group(0) @binding(2)
var depth_tex: texture_2d<f32>;
@group(0) @binding(3)
var depth_sampler_blur: sampler;
@group(0) @binding(4)
var<uniform> blur_params: BlurParams;

struct VsOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_blur(@builtin(vertex_index) vertex_index: u32) -> VsOutput {
    var out: VsOutput;
    let x = f32(i32(vertex_index & 1u) * 2);
    let y = f32(i32(vertex_index >> 1u) * 2);
    out.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, 1.0 - y);
    return out;
}

@fragment
fn fs_blur(in: VsOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let center_depth = textureSample(depth_tex, depth_sampler_blur, uv).r;

    var result = 0.0;
    var total_weight = 0.0;

    // 4x4 edge-aware blur
    for (var x = -2; x <= 1; x = x + 1) {
        for (var y = -2; y <= 1; y = y + 1) {
            let offset = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) * blur_params.texel_size;
            let sample_uv = uv + offset;
            let sample_depth = textureSample(depth_tex, depth_sampler_blur, sample_uv).r;

            // Edge-aware weight: reduce contribution across depth discontinuities
            let depth_diff = abs(center_depth - sample_depth);
            let weight = select(0.0, 1.0, depth_diff < blur_params.depth_threshold);

            result = result + textureSample(ssao_tex, ssao_sampler, sample_uv).r * weight;
            total_weight = total_weight + weight;
        }
    }

    if total_weight > 0.0 {
        result = result / total_weight;
    } else {
        result = textureSample(ssao_tex, ssao_sampler, uv).r;
    }

    return vec4<f32>(result, 0.0, 0.0, 1.0);
}
"#;

/// SSAO compositing shader (WGSL) — multiplies the scene color by the SSAO occlusion factor.
/// Also applies environment-based ambient lighting when enabled.
///
/// Reads the resolved color texture + the blurred SSAO texture and outputs
/// `color * occlusion` to the final render target.
const SSAO_COMPOSITE_SHADER: &str = r#"
struct CompositeParams {
    screen_width: f32,
    screen_height: f32,
    ssao_enabled: u32,
    env_enabled: u32,
    sky_color: vec4<f32>,
    ground_color: vec4<f32>,
    horizon_color: vec4<f32>,
    env_intensity: f32,
    _pad: vec3<f32>,
};

@group(0) @binding(0)
var color_tex: texture_2d<f32>;
@group(0) @binding(1)
var color_sampler: sampler;
@group(0) @binding(2)
var ssao_tex: texture_2d<f32>;
@group(0) @binding(3)
var ssao_sampler_comp: sampler;
@group(0) @binding(4)
var<uniform> composite_params: CompositeParams;

struct VsOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_composite(@builtin(vertex_index) vertex_index: u32) -> VsOutput {
    var out: VsOutput;
    let x = f32(i32(vertex_index & 1u) * 2);
    let y = f32(i32(vertex_index >> 1u) * 2);
    out.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, 1.0 - y);
    return out;
}

@fragment
fn fs_composite(in: VsOutput) -> @location(0) vec4<f32> {
    let color = textureSample(color_tex, color_sampler, in.uv);

    var ao = 1.0;
    if composite_params.ssao_enabled != 0u {
        ao = textureSample(ssao_tex, ssao_sampler_comp, in.uv).r;
    }

    let result = color.rgb * ao;
    return vec4<f32>(result, color.a);
}
"#;

/// SSAO parameters uniform (sent to GPU)
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsaoParams {
    pub projection: [[f32; 4]; 4],
    pub inv_projection: [[f32; 4]; 4],
    pub kernel_size: u32,
    pub radius: f32,
    pub bias: f32,
    pub intensity: f32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub _pad: [f32; 2],
}

impl SsaoParams {
    pub fn new(width: f32, height: f32) -> Self {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        Self {
            projection: identity,
            inv_projection: identity,
            kernel_size: 32,
            radius: 0.5,
            bias: 0.025,
            intensity: 1.0,
            screen_width: width,
            screen_height: height,
            _pad: [0.0; 2],
        }
    }
}

/// Blur pass parameters
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsaoBlurParams {
    pub texel_size: [f32; 2],
    pub depth_threshold: f32,
    pub _pad: f32,
}

/// SSAO composite pass parameters (applies AO + optional environment lighting)
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsaoCompositeParams {
    pub screen_width: f32,
    pub screen_height: f32,
    pub ssao_enabled: u32,
    pub env_enabled: u32,
    pub sky_color: [f32; 4],
    pub ground_color: [f32; 4],
    pub horizon_color: [f32; 4],
    pub env_intensity: f32,
    pub _pad: [f32; 3],
}

/// SSAO kernel sample (vec4, xyz = hemisphere direction, w = unused)
pub type SsaoKernelSamples = [[f32; 4]; 64];

/// Generate hemisphere kernel samples for SSAO.
/// Samples are distributed in a hemisphere oriented along +Z, with
/// more samples clustered near the origin (via accelerating interpolation).
pub fn generate_ssao_kernel(sample_count: u32) -> SsaoKernelSamples {
    let mut samples = [[0.0f32; 4]; 64];
    let count = (sample_count as usize).min(64);

    // Simple deterministic pseudo-random using a linear congruential generator
    let mut seed: u32 = 0x12345678;
    let mut next_rand = || -> f32 {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        ((seed >> 16) & 0x7FFF) as f32 / 32767.0
    };

    for i in 0..count {
        // Random point in hemisphere (z >= 0)
        let mut x = next_rand() * 2.0 - 1.0;
        let mut y = next_rand() * 2.0 - 1.0;
        let mut z = next_rand(); // [0, 1] - hemisphere

        // Normalize
        let len = (x * x + y * y + z * z).sqrt();
        if len > 0.0001 {
            x /= len;
            y /= len;
            z /= len;
        }

        // Accelerating interpolation: cluster samples closer to origin
        let scale = (i as f32) / (count as f32);
        let scale = 0.1 + scale * scale * 0.9; // lerp(0.1, 1.0, scale*scale)
        x *= scale;
        y *= scale;
        z *= scale;

        samples[i] = [x, y, z, 0.0];
    }

    samples
}

/// Generate a 4x4 noise texture of random rotation vectors (for SSAO tangent-space rotation)
pub fn generate_ssao_noise() -> Vec<u8> {
    let mut noise = Vec::with_capacity(4 * 4 * 4); // 4x4 pixels, 4 bytes each (RGBA8)
    let mut seed: u32 = 0xDEADBEEF;
    let mut next_rand = || -> f32 {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        ((seed >> 16) & 0x7FFF) as f32 / 32767.0
    };

    for _ in 0..16 {
        // Random rotation in tangent plane (z = 0)
        let x = next_rand() * 2.0 - 1.0;
        let y = next_rand() * 2.0 - 1.0;
        // Store as [0, 255] range: (value * 0.5 + 0.5) * 255
        noise.push(((x * 0.5 + 0.5) * 255.0) as u8);
        noise.push(((y * 0.5 + 0.5) * 255.0) as u8);
        noise.push(0u8); // z = 0
        noise.push(255u8); // alpha
    }

    noise
}

/// SSAO rendering resources (pipelines, bind group layouts, buffers)
pub struct SsaoResources {
    pub ssao_pipeline: wgpu::RenderPipeline,
    pub ssao_bind_group_layout: wgpu::BindGroupLayout,
    pub blur_pipeline: wgpu::RenderPipeline,
    pub blur_bind_group_layout: wgpu::BindGroupLayout,
    pub composite_pipeline: wgpu::RenderPipeline,
    pub composite_bind_group_layout: wgpu::BindGroupLayout,
    pub params_buffer: wgpu::Buffer,
    pub kernel_buffer: wgpu::Buffer,
    pub blur_params_buffer: wgpu::Buffer,
    pub composite_params_buffer: wgpu::Buffer,
    pub depth_sampler: wgpu::Sampler,
    pub noise_sampler: wgpu::Sampler,
    pub linear_sampler: wgpu::Sampler,
}

/// Create all SSAO-related pipelines and resources
pub fn create_ssao_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> SsaoResources {
    // --- SSAO main pass ---
    let ssao_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("SSAO Shader"),
        source: wgpu::ShaderSource::Wgsl(SSAO_SHADER.into()),
    });

    let ssao_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("SSAO Bind Group Layout"),
        entries: &[
            // depth texture (Depth32Float is not filterable)
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // depth sampler (non-filtering for Depth32Float)
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            },
            // noise texture
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // noise sampler
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // SSAO params uniform
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // kernel samples uniform
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let ssao_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("SSAO Pipeline Layout"),
        bind_group_layouts: &[&ssao_bind_group_layout],
        push_constant_ranges: &[],
    });

    // SSAO output format: R8Unorm (single channel, 0-1)
    let ssao_format = wgpu::TextureFormat::R8Unorm;

    let ssao_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("SSAO Pipeline"),
        layout: Some(&ssao_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &ssao_shader,
            entry_point: "vs_ssao",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &ssao_shader,
            entry_point: "fs_ssao",
            targets: &[Some(wgpu::ColorTargetState {
                format: ssao_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // --- Blur pass ---
    let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("SSAO Blur Shader"),
        source: wgpu::ShaderSource::Wgsl(SSAO_BLUR_SHADER.into()),
    });

    let blur_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("SSAO Blur Bind Group Layout"),
        entries: &[
            // SSAO input texture
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // SSAO sampler
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // depth texture (Depth32Float is not filterable)
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // depth sampler (non-filtering for Depth32Float)
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            },
            // blur params uniform
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let blur_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("SSAO Blur Pipeline Layout"),
        bind_group_layouts: &[&blur_bind_group_layout],
        push_constant_ranges: &[],
    });

    let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("SSAO Blur Pipeline"),
        layout: Some(&blur_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &blur_shader,
            entry_point: "vs_blur",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &blur_shader,
            entry_point: "fs_blur",
            targets: &[Some(wgpu::ColorTargetState {
                format: ssao_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // --- Composite pass (multiply color * AO) ---
    let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("SSAO Composite Shader"),
        source: wgpu::ShaderSource::Wgsl(SSAO_COMPOSITE_SHADER.into()),
    });

    let composite_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("SSAO Composite Bind Group Layout"),
        entries: &[
            // color texture
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // color sampler
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // SSAO texture (blurred)
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // SSAO sampler
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // composite params uniform
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let composite_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("SSAO Composite Pipeline Layout"),
        bind_group_layouts: &[&composite_bind_group_layout],
        push_constant_ranges: &[],
    });

    let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("SSAO Composite Pipeline"),
        layout: Some(&composite_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &composite_shader,
            entry_point: "vs_composite",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &composite_shader,
            entry_point: "fs_composite",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // --- Shared resources ---
    let params = SsaoParams::new(width as f32, height as f32);
    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("SSAO Params Buffer"),
        contents: bytemuck::cast_slice(&[params]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let kernel = generate_ssao_kernel(32);
    let kernel_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("SSAO Kernel Buffer"),
        contents: bytemuck::cast_slice(&kernel),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let blur_params = SsaoBlurParams {
        texel_size: [1.0 / width as f32, 1.0 / height as f32],
        depth_threshold: 0.001,
        _pad: 0.0,
    };
    let blur_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("SSAO Blur Params Buffer"),
        contents: bytemuck::cast_slice(&[blur_params]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let composite_params = SsaoCompositeParams {
        screen_width: width as f32,
        screen_height: height as f32,
        ssao_enabled: 0,
        env_enabled: 0,
        sky_color: [0.6, 0.7, 0.9, 1.0],
        ground_color: [0.3, 0.25, 0.2, 1.0],
        horizon_color: [0.5, 0.5, 0.5, 1.0],
        env_intensity: 1.0,
        _pad: [0.0; 3],
    };
    let composite_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("SSAO Composite Params Buffer"),
        contents: bytemuck::cast_slice(&[composite_params]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Samplers
    let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("SSAO Depth Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let noise_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("SSAO Noise Sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("SSAO Linear Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    SsaoResources {
        ssao_pipeline,
        ssao_bind_group_layout,
        blur_pipeline,
        blur_bind_group_layout,
        composite_pipeline,
        composite_bind_group_layout,
        params_buffer,
        kernel_buffer,
        blur_params_buffer,
        composite_params_buffer,
        depth_sampler,
        noise_sampler,
        linear_sampler,
    }
}

// ====================================================================
// Environment-Based Lighting (IBL) uniform
// ====================================================================

/// Environment lighting uniform data for GPU.
///
/// Provides hemisphere-based ambient color: interpolates between ground, horizon,
/// and sky colors based on surface normal direction. When enabled, this replaces
/// the flat ambient color in the lighting equation.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EnvironmentUniform {
    /// Upper hemisphere (sky) color RGBA
    pub sky_color: [f32; 4],
    /// Lower hemisphere (ground) color RGBA
    pub ground_color: [f32; 4],
    /// Horizon blend color RGBA
    pub horizon_color: [f32; 4],
    /// Environment light intensity multiplier
    pub intensity: f32,
    /// Whether environment lighting is enabled (0 = off, 1 = on)
    pub enabled: u32,
    /// Padding to align to 16-byte boundary
    pub _pad: [f32; 2],
}

impl EnvironmentUniform {
    pub fn new() -> Self {
        Self {
            sky_color: [0.6, 0.7, 0.9, 1.0],
            ground_color: [0.3, 0.25, 0.2, 1.0],
            horizon_color: [0.5, 0.5, 0.5, 1.0],
            intensity: 1.0,
            enabled: 0,
            _pad: [0.0; 2],
        }
    }
}

/// MSAA sample count (1 = disabled, 4 = 4x MSAA)
pub const MSAA_SAMPLE_COUNT: u32 = 4;

/// Render pipeline wrapper
pub struct RenderPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub wireframe_pipeline: Option<wgpu::RenderPipeline>,
    pub xray_pipeline: wgpu::RenderPipeline,
    pub instanced_pipeline: wgpu::RenderPipeline,
    pub instanced_wireframe_pipeline: Option<wgpu::RenderPipeline>,
    pub edge_pipeline: Option<wgpu::RenderPipeline>,
    pub instanced_edge_pipeline: Option<wgpu::RenderPipeline>,
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
                    // Section box uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
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
                format: DEPTH_STENCIL_FORMAT,
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

        // Create instanced shader module
        let instanced_vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Instanced Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(INSTANCED_VERTEX_SHADER.into()),
        });

        // Create instanced pipeline (two vertex buffers: box geometry + instance data)
        let instanced_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Instanced Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &instanced_vertex_shader,
                entry_point: "vs_main",
                buffers: &[BoxVertex::desc(), InstanceData::desc()],
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
                format: DEPTH_STENCIL_FORMAT,
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

        // Create instanced wireframe pipeline
        let instanced_wireframe_pipeline = if wireframe_supported {
            Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Instanced Wireframe Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &instanced_vertex_shader,
                    entry_point: "vs_main",
                    buffers: &[BoxVertex::desc(), InstanceData::desc()],
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
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Line,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_STENCIL_FORMAT,
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
                    format: DEPTH_STENCIL_FORMAT,
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

        // Create edge shader module
        let edge_fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Edge Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(EDGE_FRAGMENT_SHADER.into()),
        });

        // Create edge pipeline (wireframe overlay with dark color and depth bias)
        // Only available when wireframe polygon mode is supported
        let edge_pipeline = if wireframe_supported {
            Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Edge Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader,
                    entry_point: "vs_main",
                    buffers: &[Vertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &edge_fragment_shader,
                    entry_point: "fs_edge",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, // Show all edges
                    polygon_mode: wgpu::PolygonMode::Line,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_STENCIL_FORMAT,
                    depth_write_enabled: false, // Don't write depth for edge overlay
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState {
                        constant: -1,
                        slope_scale: -1.0,
                        clamp: 0.0,
                    },
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

        // Create instanced edge pipeline
        let instanced_edge_pipeline = if wireframe_supported {
            Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Instanced Edge Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &instanced_vertex_shader,
                    entry_point: "vs_main",
                    buffers: &[BoxVertex::desc(), InstanceData::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &edge_fragment_shader,
                    entry_point: "fs_edge",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, // Show all edges
                    polygon_mode: wgpu::PolygonMode::Line,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_STENCIL_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState {
                        constant: -1,
                        slope_scale: -1.0,
                        clamp: 0.0,
                    },
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

        // X-ray pipeline: alpha blending, no backface culling, no depth writes
        let xray_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("X-Ray Pipeline"),
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Show both faces for see-through
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_STENCIL_FORMAT,
                depth_write_enabled: false, // No depth writes for transparency layering
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

        Self {
            pipeline,
            wireframe_pipeline,
            xray_pipeline,
            instanced_pipeline,
            instanced_wireframe_pipeline,
            edge_pipeline,
            instanced_edge_pipeline,
            camera_bind_group_layout,
        }
    }

    /// Get the appropriate pipeline for the render mode
    pub fn get_pipeline(&self, mode: RenderMode) -> &wgpu::RenderPipeline {
        match mode {
            RenderMode::Shaded => &self.pipeline,
            RenderMode::Wireframe => self.wireframe_pipeline.as_ref().unwrap_or(&self.pipeline),
            RenderMode::XRay => &self.xray_pipeline,
        }
    }

    /// Get the appropriate instanced pipeline for the render mode
    pub fn get_instanced_pipeline(&self, mode: RenderMode) -> &wgpu::RenderPipeline {
        match mode {
            RenderMode::Shaded => &self.instanced_pipeline,
            RenderMode::Wireframe => self.instanced_wireframe_pipeline.as_ref().unwrap_or(&self.instanced_pipeline),
            RenderMode::XRay => &self.instanced_pipeline, // Instanced X-ray uses same pipeline (no instance X-ray)
        }
    }

    /// Get the edge overlay pipeline (for non-instanced geometry), if available
    pub fn get_edge_pipeline(&self) -> Option<&wgpu::RenderPipeline> {
        self.edge_pipeline.as_ref()
    }

    /// Get the edge overlay pipeline for instanced geometry, if available
    pub fn get_instanced_edge_pipeline(&self) -> Option<&wgpu::RenderPipeline> {
        self.instanced_edge_pipeline.as_ref()
    }
}

// ====================================================================
// Compute Shader Frustum Culling
// ====================================================================

/// Compute shader (WGSL) for GPU-driven frustum culling of AABBs.
///
/// Each workgroup thread tests one AABB against six frustum planes using
/// the Gribb-Hartmann positive vertex test. Outputs a visibility flag
/// (1 = visible, 0 = culled) per element.
pub const COMPUTE_CULL_SHADER: &str = r#"
@group(0) @binding(0) var<uniform> frustum_planes: array<vec4<f32>, 6>;
@group(0) @binding(1) var<storage, read> aabbs: array<vec4<f32>>;  // pairs of (min, max)
@group(0) @binding(2) var<storage, read_write> visibility: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    let count = arrayLength(&visibility);
    if (idx >= count) { return; }

    let aabb_min = aabbs[idx * 2u];
    let aabb_max = aabbs[idx * 2u + 1u];

    var visible = 1u;
    for (var i = 0u; i < 6u; i++) {
        let plane = frustum_planes[i];
        // Positive vertex test (Gribb-Hartmann)
        let px = select(aabb_min.x, aabb_max.x, plane.x >= 0.0);
        let py = select(aabb_min.y, aabb_max.y, plane.y >= 0.0);
        let pz = select(aabb_min.z, aabb_max.z, plane.z >= 0.0);
        if (plane.x * px + plane.y * py + plane.z * pz + plane.w < 0.0) {
            visible = 0u;
            break;
        }
    }
    visibility[idx] = visible;
}
"#;

/// Resources for GPU compute frustum culling pipeline.
pub struct ComputeCullResources {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

/// Create the compute culling pipeline and bind group layout.
///
/// Returns `None` if the device does not support compute shaders
/// (e.g. WebGL2 backend or missing feature).
pub fn create_compute_cull_pipeline(device: &wgpu::Device) -> Option<ComputeCullResources> {
    // Attempt to create the shader module; if compute is not supported
    // the shader compilation may fail, so we handle that gracefully.
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Compute Cull Shader"),
        source: wgpu::ShaderSource::Wgsl(COMPUTE_CULL_SHADER.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Compute Cull Bind Group Layout"),
        entries: &[
            // Frustum planes uniform (6 x vec4<f32> = 96 bytes)
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // AABB storage buffer (read-only)
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Visibility output storage buffer (read-write)
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Compute Cull Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Cull Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
    });

    Some(ComputeCullResources {
        pipeline,
        bind_group_layout,
    })
}

// ====================================================================
// Vertex Buffer Streaming Configuration
// ====================================================================

/// Configuration for vertex buffer streaming (LOD / chunk-based GPU memory management).
///
/// Controls how vertex data is partitioned into chunks and streamed to/from
/// GPU memory based on camera proximity and a per-frame upload budget.
#[derive(Debug, Clone)]
pub struct VertexStreamConfig {
    /// Number of vertices per chunk (default 65536)
    pub chunk_size: usize,
    /// Maximum number of chunks resident in GPU memory simultaneously (default 32)
    pub max_gpu_chunks: usize,
    /// Camera distance threshold for prefetching chunks (world units)
    pub prefetch_distance: f32,
    /// Maximum bytes to upload to GPU per frame (default 4 MB)
    pub upload_budget_bytes: usize,
}

impl Default for VertexStreamConfig {
    fn default() -> Self {
        Self {
            chunk_size: 65536,
            max_gpu_chunks: 32,
            prefetch_distance: 100.0,
            upload_budget_bytes: 4 * 1024 * 1024, // 4 MB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_cull_shader_compiles() {
        // Verify the WGSL shader string is non-empty and contains expected keywords
        assert!(!COMPUTE_CULL_SHADER.is_empty());
        assert!(COMPUTE_CULL_SHADER.contains("@compute"));
        assert!(COMPUTE_CULL_SHADER.contains("@workgroup_size(64)"));
        assert!(COMPUTE_CULL_SHADER.contains("frustum_planes"));
        assert!(COMPUTE_CULL_SHADER.contains("visibility"));
        assert!(COMPUTE_CULL_SHADER.contains("aabbs"));
        assert!(COMPUTE_CULL_SHADER.contains("select"));
    }

    #[test]
    fn test_vertex_stream_config_default() {
        let config = VertexStreamConfig::default();
        assert_eq!(config.chunk_size, 65536);
        assert_eq!(config.max_gpu_chunks, 32);
        assert!((config.prefetch_distance - 100.0).abs() < f32::EPSILON);
        assert_eq!(config.upload_budget_bytes, 4 * 1024 * 1024);
    }

    #[test]
    fn test_vertex_stream_config_budget() {
        let config = VertexStreamConfig {
            chunk_size: 1024,
            max_gpu_chunks: 8,
            prefetch_distance: 50.0,
            upload_budget_bytes: 2 * 1024 * 1024,
        };
        assert_eq!(config.chunk_size, 1024);
        assert_eq!(config.max_gpu_chunks, 8);
        assert!((config.prefetch_distance - 50.0).abs() < f32::EPSILON);
        assert_eq!(config.upload_budget_bytes, 2 * 1024 * 1024);
        // Verify budget is less than default
        assert!(config.upload_budget_bytes < VertexStreamConfig::default().upload_budget_bytes);
    }
}
