use std::mem;

use glam::{Mat4, Vec2, Vec4};
use limur::ColorStop;

use sumi::{
    BumpInstances, GraphicsContext, resources::instancing_geometry::InstancingGeometry,
    resources::vertex::TexturedVertex,
};

const GRADIENT_SAMPLES: usize = 8;

// ── Instance ID ───────────────────────────────────────────────────────────────

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct RectInstanceId {
    value: u32,
}

impl InstanceId for RectInstanceId {
    fn index(&self) -> usize {
        self.value as usize
    }
}

// ── Fill descriptor ───────────────────────────────────────────────────────────

pub enum RectFill<'a> {
    None,
    Solid(Vec4),
    Linear {
        start: (f32, f32),
        end: (f32, f32),
        stops: &'a [ColorStop],
    },
    Radial {
        center: (f32, f32),
        radius: f32,
        stops: &'a [ColorStop],
    },
}

// ── Instance data (336 bytes, 21 attributes at locations 5–25) ───────────────

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RectInstance {
    screen_origin: [f32; 2],         // loc  4,     8 bytes
    mvp_matrix: [[f32; 4]; 4],       // locs 5–8,   64 bytes
    fill_color: [f32; 4],            // loc  9,     16 bytes
    border_color_top: [f32; 4],      // loc 10,     16 bytes
    border_color_right: [f32; 4],    // loc 11,     16 bytes
    border_color_bottom: [f32; 4],   // loc 12,     16 bytes
    border_color_left: [f32; 4],     // loc 13,     16 bytes
    border_widths: [f32; 4],         // loc 14,     16 bytes  [top, right, bottom, left]
    border_radii: [f32; 4],          // loc 15,     16 bytes  [tl, tr, br, bl]
    size_and_grad: [f32; 4],         // loc 16,     16 bytes  [w, h, gradient_type, shape_type]
    gradient_p0: [f32; 4],           // loc 17,     16 bytes
    gradient_samples: [[f32; 4]; 8], // locs 18–25, 128 bytes
}

fn bake_gradient(stops: &[ColorStop]) -> [[f32; 4]; GRADIENT_SAMPLES] {
    let mut out = [[0.0f32; 4]; GRADIENT_SAMPLES];
    if stops.is_empty() {
        return out;
    }
    if stops.len() == 1 {
        let c = stops[0].color;
        return [[c.r, c.g, c.b, c.a]; GRADIENT_SAMPLES];
    }
    for i in 0..GRADIENT_SAMPLES {
        let t = i as f32 / (GRADIENT_SAMPLES - 1) as f32;
        // Find the two stops that bracket t.
        let mut lo = &stops[0];
        let mut hi = &stops[stops.len() - 1];
        for pair in stops.windows(2) {
            if t >= pair[0].offset && t <= pair[1].offset {
                lo = &pair[0];
                hi = &pair[1];
                break;
            }
        }
        let range = hi.offset - lo.offset;
        let f = if range < 1e-6 {
            0.0
        } else {
            ((t - lo.offset) / range).clamp(0.0, 1.0)
        };
        let a = lo.color;
        let b = hi.color;
        out[i] = [
            a.r + (b.r - a.r) * f,
            a.g + (b.g - a.g) * f,
            a.b + (b.b - a.b) * f,
            a.a + (b.a - a.a) * f,
        ];
    }
    out
}

impl RectInstance {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mvp: &Mat4,
        position: Vec2,
        size: Vec2,
        fill: RectFill<'_>,
        border_color_top: Vec4,
        border_width_top: f32,
        border_color_right: Vec4,
        border_width_right: f32,
        border_color_bottom: Vec4,
        border_width_bottom: f32,
        border_color_left: Vec4,
        border_width_left: f32,
        // [top_left, top_right, bottom_right, bottom_left]
        border_radii: [f32; 4],
    ) -> Self {
        let (fill_color, gradient_type, gradient_p0, gradient_samples) = match fill {
            RectFill::None => ([0.0f32; 4], 0.0f32, [0.0f32; 4], [[0.0f32; 4]; 8]),
            RectFill::Solid(c) => (c.to_array(), 0.0, [0.0f32; 4], [[0.0f32; 4]; 8]),
            RectFill::Linear { start, end, stops } => (
                [0.0f32; 4],
                1.0,
                [start.0, start.1, end.0, end.1],
                bake_gradient(stops),
            ),
            RectFill::Radial {
                center,
                radius,
                stops,
            } => (
                [0.0f32; 4],
                2.0,
                [center.0, center.1, radius, 0.0],
                bake_gradient(stops),
            ),
        };

        Self {
            mvp_matrix: mvp.to_cols_array_2d(),
            fill_color,
            screen_origin: position.to_array(),
            border_color_top: border_color_top.to_array(),
            border_color_right: border_color_right.to_array(),
            border_color_bottom: border_color_bottom.to_array(),
            border_color_left: border_color_left.to_array(),
            border_widths: [
                border_width_top,
                border_width_right,
                border_width_bottom,
                border_width_left,
            ],
            border_radii,
            size_and_grad: [size.x, size.y, gradient_type, 0.0],
            gradient_p0,
            gradient_samples,
        }
    }

    /// Convenience constructor for an oval (ellipse) with a uniform border.
    pub fn new_oval(
        mvp: &Mat4,
        position: Vec2,
        size: Vec2,
        fill: RectFill<'_>,
        border_color: Vec4,
        border_width: f32,
    ) -> Self {
        let mut inst = Self::new(
            mvp,
            position,
            size,
            fill,
            border_color,
            border_width,
            border_color,
            border_width,
            border_color,
            border_width,
            border_color,
            border_width,
            [0.0; 4],
        );
        inst.size_and_grad[3] = 1.0; // shape_type = 1 (ellipse)
        inst
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // screen_origin: [f32; 2] — 8 bytes
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // mvp_matrix col 0
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // mvp_matrix col 1
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // mvp_matrix col 2
                wgpu::VertexAttribute {
                    offset: 40,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // mvp_matrix col 3
                wgpu::VertexAttribute {
                    offset: 56,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // fill_color
                wgpu::VertexAttribute {
                    offset: 72,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // border_color_top
                wgpu::VertexAttribute {
                    offset: 88,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // border_color_right
                wgpu::VertexAttribute {
                    offset: 104,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // border_color_bottom
                wgpu::VertexAttribute {
                    offset: 120,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // border_color_left
                wgpu::VertexAttribute {
                    offset: 136,
                    shader_location: 13,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // border_widths
                wgpu::VertexAttribute {
                    offset: 152,
                    shader_location: 14,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // border_radii
                wgpu::VertexAttribute {
                    offset: 168,
                    shader_location: 15,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // size_and_grad
                wgpu::VertexAttribute {
                    offset: 184,
                    shader_location: 16,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // gradient_p0
                wgpu::VertexAttribute {
                    offset: 200,
                    shader_location: 17,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // gradient_samples[0]
                wgpu::VertexAttribute {
                    offset: 216,
                    shader_location: 18,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // gradient_samples[1]
                wgpu::VertexAttribute {
                    offset: 232,
                    shader_location: 19,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // gradient_samples[2]
                wgpu::VertexAttribute {
                    offset: 248,
                    shader_location: 20,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // gradient_samples[3]
                wgpu::VertexAttribute {
                    offset: 264,
                    shader_location: 21,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // gradient_samples[4]
                wgpu::VertexAttribute {
                    offset: 280,
                    shader_location: 22,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // gradient_samples[5]
                wgpu::VertexAttribute {
                    offset: 296,
                    shader_location: 23,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // gradient_samples[6]
                wgpu::VertexAttribute {
                    offset: 312,
                    shader_location: 24,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // gradient_samples[7]
                wgpu::VertexAttribute {
                    offset: 328,
                    shader_location: 25,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

// ── Renderer ──────────────────────────────────────────────────────────────────

/// Depth/stencil format used by the layer render pass and all rect pipelines.
pub const STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

pub struct RectRenderer<
    I: Instances<RectInstanceId, RectInstance> = BumpInstances<RectInstanceId, RectInstance>,
> {
    render_pipeline: wgpu::RenderPipeline,
    /// Draws clip shape to stencil with IncrementClamp; no color write.
    clip_incr_pipeline: wgpu::RenderPipeline,
    /// Draws clip shape to stencil with DecrementClamp; no color write.
    clip_decr_pipeline: wgpu::RenderPipeline,
    /// Non-MSAA pass: multiplies layer_view alpha by the clip SDF outer_alpha,
    /// smoothing the clip boundary. blend = (Zero, SrcAlpha).
    clip_aa_pipeline: wgpu::RenderPipeline,
    instances: I,
}

impl<I: Instances<RectInstanceId, RectInstance>> RectRenderer<I> {
    pub fn new(context: &GraphicsContext<'_, '_>, mut instances: I) -> Self {
        instances.create_buffer(context, |index, _| RectInstanceId {
            value: index as u32,
        });

        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Rect Shader"),
                // source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rect.wgsl").into()),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
            });

        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Rect Pipeline Layout"),
                    bind_group_layouts: &[],
                    immediate_size: 0,
                });

        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: Some(wgpu::IndexFormat::Uint16),
            front_face: wgpu::FrontFace::Cw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        };
        let multisample = wgpu::MultisampleState {
            count: context.sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        // ── Main render pipeline ──────────────────────────────────────────────
        // Tests stencil == reference (set per-frame to current clip depth).
        // Never writes to the stencil.
        let render_pipeline =
            context
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Rect Render Pipeline"),
                    layout: Some(&pipeline_layout),
                    cache: None,
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[TexturedVertex::desc(), RectInstance::desc()],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: context.surface_texture_format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent::OVER,
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive,
                    // depth_stencil: Some(wgpu::DepthStencilState {
                    //     format: STENCIL_FORMAT,
                    //     depth_write_enabled: Some(false),
                    //     depth_compare: Some(wgpu::CompareFunction::Always),
                    //     stencil: wgpu::StencilState {
                    //         front: wgpu::StencilFaceState {
                    //             compare: wgpu::CompareFunction::Equal,
                    //             fail_op: wgpu::StencilOperation::Keep,
                    //             depth_fail_op: wgpu::StencilOperation::Keep,
                    //             pass_op: wgpu::StencilOperation::Keep,
                    //         },
                    //         back: wgpu::StencilFaceState::default(),
                    //         read_mask: 0xFF,
                    //         write_mask: 0x00,
                    //     },
                    //     bias: wgpu::DepthBiasState::default(),
                    // }),
                    depth_stencil: None,
                    multisample,
                    multiview_mask: None,
                });

        // ── Clip stencil pipelines ────────────────────────────────────────────
        // Shared setup: no color write; stencil test always passes; the rect
        // shader's discard (outer SDF) ensures only in-shape pixels write.
        let make_clip_pipeline = |label, pass_op| {
            context
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(&pipeline_layout),
                    cache: None,
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[TexturedVertex::desc(), RectInstance::desc()],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: context.surface_texture_format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::empty(),
                        })],
                    }),
                    primitive,
                    depth_stencil: None,
                    // depth_stencil: Some(wgpu::DepthStencilState {
                    //     format: STENCIL_FORMAT,
                    //     depth_write_enabled: Some(false),
                    //     depth_compare: Some(wgpu::CompareFunction::Always),
                    //     stencil: wgpu::StencilState {
                    //         front: wgpu::StencilFaceState {
                    //             compare: wgpu::CompareFunction::Always,
                    //             fail_op: wgpu::StencilOperation::Keep,
                    //             depth_fail_op: wgpu::StencilOperation::Keep,
                    //             pass_op,
                    //         },
                    //         back: wgpu::StencilFaceState::default(),
                    //         read_mask: 0xFF,
                    //         write_mask: 0xFF,
                    //     },
                    //     bias: wgpu::DepthBiasState::default(),
                    // }),
                    multisample,
                    multiview_mask: None,
                })
        };

        let clip_incr_pipeline = make_clip_pipeline(
            "Rect Clip Incr Pipeline",
            wgpu::StencilOperation::IncrementClamp,
        );
        let clip_decr_pipeline = make_clip_pipeline(
            "Rect Clip Decr Pipeline",
            wgpu::StencilOperation::DecrementClamp,
        );

        // ── Clip AA pipeline ──────────────────────────────────────────────────
        // Non-MSAA pass on the resolved layer_view; blend=(Zero, SrcAlpha) so
        // the destination is multiplied by outer_alpha, smoothing clip edges.
        let multiply_alpha = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::SrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::SrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let clip_aa_pipeline =
            context
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Rect Clip AA Pipeline"),
                    layout: Some(&pipeline_layout),
                    cache: None,
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[TexturedVertex::desc(), RectInstance::desc()],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_clip_aa"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: context.surface_texture_format,
                            blend: Some(multiply_alpha),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive,
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview_mask: None,
                });

        Self {
            render_pipeline,
            clip_incr_pipeline,
            clip_decr_pipeline,
            clip_aa_pipeline,
            instances,
        }
    }

    pub fn instances(&mut self) -> &mut I {
        &mut self.instances
    }

    pub fn render_instance<T>(
        &mut self,
        context: &mut GraphicsContext<'_, '_>,
        geometry: &T,
        id: RectInstanceId,
    ) where
        T: InstancingGeometry,
    {
        debug_assert!(self.instances.contains(id), "Invalid RectInstanceId");

        context.render_pass().set_pipeline(&self.render_pipeline);
        context
            .render_pass()
            .set_vertex_buffer(1, self.instances.gpu_buffer().slice(..));
        geometry.render_instances(context, id.value..id.value + 1);
    }

    /// Draw the clip shape to the stencil buffer with IncrementClamp.
    /// Call `render_pass.set_stencil_reference(new_depth)` after this.
    pub fn render_clip_push<T>(
        &mut self,
        context: &mut GraphicsContext<'_, '_>,
        geometry: &T,
        id: RectInstanceId,
    ) where
        T: InstancingGeometry,
    {
        debug_assert!(self.instances.contains(id), "Invalid RectInstanceId");
        context.render_pass().set_pipeline(&self.clip_incr_pipeline);
        context
            .render_pass()
            .set_vertex_buffer(1, self.instances.gpu_buffer().slice(..));
        geometry.render_instances(context, id.value..id.value + 1);
    }

    /// Draw the clip shape to the stencil buffer with DecrementClamp.
    /// Call `render_pass.set_stencil_reference(new_depth)` after this.
    pub fn render_clip_pop<T>(
        &mut self,
        context: &mut GraphicsContext<'_, '_>,
        geometry: &T,
        id: RectInstanceId,
    ) where
        T: InstancingGeometry,
    {
        debug_assert!(self.instances.contains(id), "Invalid RectInstanceId");
        context.render_pass().set_pipeline(&self.clip_decr_pipeline);
        context
            .render_pass()
            .set_vertex_buffer(1, self.instances.gpu_buffer().slice(..));
        geometry.render_instances(context, id.value..id.value + 1);
    }

    /// Non-MSAA pass: multiply layer_view pixels by the clip SDF outer_alpha.
    /// Must be called on a render pass targeting the resolved (non-MSAA) layer_view
    /// with blend=(Zero, SrcAlpha). Smooths clip boundary anti-aliasing.
    pub fn render_clip_aa<T>(
        &mut self,
        context: &mut GraphicsContext<'_, '_>,
        geometry: &T,
        id: RectInstanceId,
    ) where
        T: InstancingGeometry,
    {
        debug_assert!(self.instances.contains(id), "Invalid RectInstanceId");
        context.render_pass().set_pipeline(&self.clip_aa_pipeline);
        context
            .render_pass()
            .set_vertex_buffer(1, self.instances.gpu_buffer().slice(..));
        geometry.render_instances(context, id.value..id.value + 1);
    }
}
