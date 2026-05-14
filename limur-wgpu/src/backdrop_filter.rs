use glam::Mat4;
use std::mem;

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BackdropFilterInstance {
    mvp_matrix: [[f32; 4]; 4], // locations 2-5, same layout as VectorInstance
    tint: [f32; 4],            // location 6, premultiplied RGBA
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
    pub fn new(mvp: Mat4, tint: [f32; 4]) -> Self {
        Self {
            mvp_matrix: mvp.to_cols_array_2d(),
            tint,
        }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // mvp_matrix col 0
                wgpu::VertexAttribute { offset: 0,  shader_location: 2, format: wgpu::VertexFormat::Float32x4 },
                // mvp_matrix col 1
                wgpu::VertexAttribute { offset: 16, shader_location: 3, format: wgpu::VertexFormat::Float32x4 },
                // mvp_matrix col 2
                wgpu::VertexAttribute { offset: 32, shader_location: 4, format: wgpu::VertexFormat::Float32x4 },
                // mvp_matrix col 3
                wgpu::VertexAttribute { offset: 48, shader_location: 5, format: wgpu::VertexFormat::Float32x4 },
                // tint
                wgpu::VertexAttribute { offset: 64, shader_location: 6, format: wgpu::VertexFormat::Float32x4 },
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
                ],
            });

        let bind_group = Self::create_bind_group(context, &layout, resources, composite_view);

        Self { layout, bind_group }
    }

    fn create_bind_group(
        context: &sumi::GraphicsContext,
        layout: &wgpu::BindGroupLayout,
        resources: &BackdropFilterResources,
        composite_view: &wgpu::TextureView,
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
                ],
            })
    }

    fn rebuild(
        &mut self,
        context: &sumi::GraphicsContext,
        resources: &BackdropFilterResources,
        composite_view: &wgpu::TextureView,
    ) {
        self.bind_group = Self::create_bind_group(context, &self.layout, resources, composite_view);
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
                    buffers: &[sumi::TexturedVertex::desc(), BackdropFilterInstance::desc()],
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

    // /// Creates a bind group that samples `composite_view` as the backdrop.
    // /// Call this before starting the render pass so the texture view lifetime is satisfied.
    // pub fn make_bind_group(
    //     &self,
    //     device: &wgpu::Device,
    //     composite_view: &wgpu::TextureView,
    // ) -> wgpu::BindGroup {
    //     device.create_bind_group(&wgpu::BindGroupDescriptor {
    //         label: Some("Backdrop Filter Bind Group"),
    //         layout: &self.bind_group_layout,
    //         entries: &[
    //             wgpu::BindGroupEntry {
    //                 binding: 0,
    //                 resource: wgpu::BindingResource::TextureView(composite_view),
    //             },
    //             wgpu::BindGroupEntry {
    //                 binding: 1,
    //                 resource: wgpu::BindingResource::Sampler(&self.sampler),
    //             },
    //         ],
    //     })
    // }

    // /// Draws one backdrop filter rect within an already-open render pass.
    // /// The pipeline is switched here; the caller should rebind the previous pipeline afterwards.
    // pub fn apply_in_pass<'a>(
    //     &'a self,
    //     pass: &mut wgpu::RenderPass<'a>,
    //     bind_group: &'a wgpu::BindGroup,
    //     instance_buf: &'a wgpu::Buffer,
    // ) {
    //     pass.set_pipeline(&self.pipeline);
    //     pass.set_bind_group(0, bind_group, &[]);
    //     pass.set_vertex_buffer(0, instance_buf.slice(..));
    //     pass.draw(0..4, 0..1);
    // }

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
