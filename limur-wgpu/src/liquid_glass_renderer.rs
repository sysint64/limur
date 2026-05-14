use std::mem;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LiquidGlassInstance {
    pub boundary:   [f32; 4],  // x, y, w, h pixels
    pub params:     [f32; 4],  // power_factor, f_power, noise, glow_weight
    pub refraction: [f32; 4],  // a, b, c, d
    pub tint:       [f32; 4],  // premultiplied RGBA
}

impl LiquidGlassInstance {
    pub fn new(
        boundary:     [f32; 4],
        power_factor: f32,
        f_power:      f32,
        noise:        f32,
        glow_weight:  f32,
        a: f32, b: f32, c: f32, d: f32,
        tint: [f32; 4],
    ) -> Self {
        Self {
            boundary,
            params:     [power_factor, f_power, noise, glow_weight],
            refraction: [a, b, c, d],
            tint,
        }
    }

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct LiquidGlassRenderer {
    pipeline:          wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler:           wgpu::Sampler,
}

impl LiquidGlassRenderer {
    pub fn new(context: &sumi::GraphicsContext, format: wgpu::TextureFormat) -> Self {
        let device = context.device;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("Liquid Glass Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/liquid_glass.wgsl").into()),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label:   Some("Liquid Glass Bind Group Layout"),
                entries: &[
                    // composite texture
                    wgpu::BindGroupLayoutEntry {
                        binding:    0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type:    wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled:   false,
                        },
                        count: None,
                    },
                    // sampler
                    wgpu::BindGroupLayoutEntry {
                        binding:    1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // globals uniform (screen_size)
                    wgpu::BindGroupLayoutEntry {
                        binding:    2,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty:                 wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size:   None,
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label:              Some("Liquid Glass Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size:     0,
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:  Some("Liquid Glass Pipeline"),
            layout: Some(&pipeline_layout),
            cache:  None,
            vertex: wgpu::VertexState {
                module:              &shader,
                entry_point:         Some("vs_main"),
                buffers:             &[LiquidGlassInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module:              &shader,
                entry_point:         Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation:  wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation:  wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology:           wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                cull_mode:          None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample:   context.default_multisample(),
            multiview_mask: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label:          Some("Liquid Glass Sampler"),
            mag_filter:     wgpu::FilterMode::Linear,
            min_filter:     wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self { pipeline, bind_group_layout, sampler }
    }

    pub fn create_bind_group(
        &self,
        device:          &wgpu::Device,
        composite_view:  &wgpu::TextureView,
        globals_buffer:  &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Liquid Glass Bind Group"),
            layout:  &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding:  0,
                    resource: wgpu::BindingResource::TextureView(composite_view),
                },
                wgpu::BindGroupEntry {
                    binding:  1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding:  2,
                    resource: wgpu::BindingResource::Buffer(
                        globals_buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
        })
    }

    pub fn draw(
        &self,
        device:      &wgpu::Device,
        render_pass: &mut wgpu::RenderPass,
        bind_group:  &wgpu::BindGroup,
        instance:    &LiquidGlassInstance,
    ) {
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Liquid Glass Instance Buffer"),
            contents: bytemuck::bytes_of(instance),
            usage:    wgpu::BufferUsages::VERTEX,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.set_vertex_buffer(0, buf.slice(..));
        render_pass.draw(0..4, 0..1);
    }
}
