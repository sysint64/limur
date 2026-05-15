use crate::{
    text::{TextAtlasBindGroup, TextResources},
    vector_resources::VectorResources,
};

pub struct VectorRenderer {
    render_pipeline: wgpu::RenderPipeline,
    pub(crate) bind_group: VectorBindGroup,
    pub(crate) text_atlas_bind_group: TextAtlasBindGroup,
}

struct VectorBindGroup {
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl VectorBindGroup {
    pub fn new(
        context: &sumi::GraphicsContext,
        resources: &VectorResources,
        globals_buffer: &wgpu::Buffer,
    ) -> Self {
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Vector Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
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

        let bind_group = Self::create_bind_group(context, &layout, resources, globals_buffer);

        Self { layout, bind_group }
    }

    fn create_bind_group(
        context: &sumi::GraphicsContext,
        layout: &wgpu::BindGroupLayout,
        resources: &VectorResources,
        globals_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Vector Bind group"),
                layout: layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(
                            resources.data.gpu_buffer().as_entire_buffer_binding(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(
                            resources
                                .gradient_stops
                                .gpu_buffer()
                                .as_entire_buffer_binding(),
                        ),
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

    fn rebuild(
        &mut self,
        context: &sumi::GraphicsContext,
        resources: &VectorResources,
        globals_buffer: &wgpu::Buffer,
    ) {
        self.bind_group =
            Self::create_bind_group(context, &self.layout, resources, globals_buffer);
    }
}

impl VectorRenderer {
    pub fn new(
        context: &sumi::GraphicsContext,
        resources: &VectorResources,
        text_resources: &TextResources,
        globals_buffer: &wgpu::Buffer,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Vector Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/vector.wgsl").into()),
            });

        let bind_group = VectorBindGroup::new(context, resources, globals_buffer);
        let text_atlas_bind_group = TextAtlasBindGroup::new(context, text_resources);

        let render_pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Vector Render Pipeline Layout"),
                    bind_group_layouts: &[
                        Some(&bind_group.layout),
                        Some(&text_atlas_bind_group.layout),
                    ],
                    immediate_size: 0,
                });

        let render_pipeline =
            context
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Vector Render Pipeline"),
                    layout: Some(&render_pipeline_layout),
                    cache: None,
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: target_format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    // Dual-source blending: src0 is premultiplied color,
                                    // src1 is per-channel coverage (uniform alpha for shapes,
                                    // RGB mask for subpixel glyphs).
                                    // out.rgb = src0.rgb + dst.rgb * (1 - src1.rgb)
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrc1,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent::OVER,
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: context.default_depth_stencil(),
                    multisample: context.default_multisample(),
                    multiview_mask: context.default_multiview_mask(),
                });

        Self {
            render_pipeline,
            bind_group,
            text_atlas_bind_group,
        }
    }

    pub fn rebuild(
        &mut self,
        context: &sumi::GraphicsContext,
        resources: &VectorResources,
        globals_buffer: &wgpu::Buffer,
    ) {
        self.bind_group.rebuild(context, resources, globals_buffer);
    }

    pub fn rebuild_text_atlas(
        &mut self,
        context: &sumi::GraphicsContext,
        resources: &TextResources,
    ) {
        self.text_atlas_bind_group.rebuild(context, resources);
    }

    #[inline]
    pub fn bind(&self, context: &sumi::GraphicsContext<'_>, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group.bind_group, &[]);
        render_pass.set_bind_group(1, &self.text_atlas_bind_group.bind_group, &[]);
    }
}
