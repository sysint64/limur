use std::mem;

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BackdropFilterInstance {
    boundary: [f32; 4], // [left, top, width, height] in top-left pixel space
    tint: [f32; 4],     // premultiplied RGBA
}

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct BackdropFilterInstanceId {
    value: u32,
}

impl sumi::SlotId for BackdropFilterInstanceId {
    fn from_index(index: usize) -> Self {
        Self {
            value: index as u32,
        }
    }

    fn index(&self) -> usize {
        self.value as usize
    }
}

impl BackdropFilterInstance {
    pub fn new(boundary: [f32; 4], tint: [f32; 4]) -> Self {
        Self { boundary, tint }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // boundary: [left, top, width, height]
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // tint
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct BackdropFilterRenderer {
    pipeline: wgpu::RenderPipeline,
}

pub struct BackdropFilterGroup {
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

pub struct BackdropFilterResources {
    pub(crate) sampler: wgpu::Sampler,
}

impl BackdropFilterResources {
    pub fn new(context: &sumi::GraphicsContext) -> Self {
        let sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Backdrop Filter Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self { sampler }
    }
}

impl BackdropFilterGroup {
    pub fn new(
        context: &sumi::GraphicsContext,
        resources: &BackdropFilterResources,
        composite_view: &wgpu::TextureView,
        globals_buffer: &wgpu::Buffer,
    ) -> Self {
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Backdrop Filter Bind Group Layout"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
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

        let bind_group =
            Self::create_bind_group(context, &layout, resources, composite_view, globals_buffer);

        Self { layout, bind_group }
    }

    fn create_bind_group(
        context: &sumi::GraphicsContext,
        layout: &wgpu::BindGroupLayout,
        resources: &BackdropFilterResources,
        composite_view: &wgpu::TextureView,
        globals_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Backdrop Filter Bind Group"),
                layout: layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(composite_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&resources.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(
                            globals_buffer.as_entire_buffer_binding(),
                        ),
                    },
                ],
            })
    }
}

impl BackdropFilterRenderer {
    pub fn new(context: &sumi::GraphicsContext, format: wgpu::TextureFormat) -> Self {
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Backdrop Filter Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("shaders/backdrop_filter.wgsl").into(),
                ),
            });

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Backdrop Filter Bind Group Layout"),
                    entries: &[
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
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

        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Backdrop Filter Pipeline Layout"),
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 0,
                });

        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Backdrop Filter Pipeline"),
                layout: Some(&pipeline_layout),
                cache: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[BackdropFilterInstance::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        // Premultiplied alpha-over into the layer texture.
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: Some(wgpu::IndexFormat::Uint16),
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: context.default_multisample(),
                multiview_mask: None,
            });

        Self { pipeline }
    }

    pub fn bind(
        &self,
        context: &sumi::GraphicsContext<'_>,
        render_pass: &mut wgpu::RenderPass,
        bind_group: &BackdropFilterGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group.bind_group, &[]);
    }
}
