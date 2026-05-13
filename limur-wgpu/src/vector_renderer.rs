use glam::Mat4;
use sumi::prelude::*;

use crate::{
    text::{TextAtlasBindGroup, TextResources},
    vector_resources::VectorResources,
};

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct VectorInstanceId {
    value: u32,
}

impl sumi::SlotId for VectorInstanceId {
    fn from_index(index: usize) -> Self {
        Self {
            value: index as u32,
        }
    }

    fn index(&self) -> usize {
        self.value as usize
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VectorInstance {
    mvp_matrix: [[f32; 4]; 4],
}

impl VectorInstance {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // mvp_matrix col 0
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // mvp_matrix col 1
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // mvp_matrix col 2
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // mvp_matrix col 3
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }

    pub(crate) fn new(mvp: Mat4) -> Self {
        Self {
            mvp_matrix: mvp.to_cols_array_2d(),
        }
    }
}

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
    pub fn new(context: &sumi::GraphicsContext, resources: &VectorResources) -> Self {
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Vector Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
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
                ],
            });

        let bind_group = Self::create_bind_group(context, &layout, resources);

        Self { layout, bind_group }
    }

    fn create_bind_group(
        context: &sumi::GraphicsContext,
        layout: &wgpu::BindGroupLayout,
        resources: &VectorResources,
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
                ],
            })
    }

    fn rebuild(&mut self, context: &sumi::GraphicsContext, resources: &VectorResources) {
        self.bind_group = Self::create_bind_group(context, &self.layout, resources);
    }
}

impl VectorRenderer {
    pub fn new(
        context: &sumi::GraphicsContext,
        resources: &VectorResources,
        text_resources: &TextResources,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Vector Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/vector.wgsl").into()),
            });

        let bind_group = VectorBindGroup::new(context, resources);
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
                        buffers: &[sumi::TexturedVertex::desc(), VectorInstance::desc()],
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
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent::OVER,
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: sumi::PlaneResources::primitive(),
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

    pub fn rebuild(&mut self, context: &sumi::GraphicsContext, resources: &VectorResources) {
        self.bind_group.rebuild(context, resources);
    }

    pub fn rebuild_text_atlas(
        &mut self,
        context: &sumi::GraphicsContext,
        resources: &TextResources,
    ) {
        self.text_atlas_bind_group.rebuild(context, resources);
    }

    #[inline]
    pub fn bind(&self, context: &sumi::GraphicsContext<'_, '_>) {
        context.render_pass().set_pipeline(&self.render_pipeline);
        context
            .render_pass()
            .set_bind_group(0, &self.bind_group.bind_group, &[]);
        context
            .render_pass()
            .set_bind_group(1, &self.text_atlas_bind_group.bind_group, &[]);
    }
}
