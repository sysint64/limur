use std::mem;
use wgpu::util::DeviceExt;

// ── Blit renderer ─────────────────────────────────────────────────────────────
// Simple full-screen triangle blit: src_texture → render target.
// Blend mode is set at construction time — create two instances for:
//   * layer → composite  (premultiplied alpha-over)
//   * composite → surface (replace / no blend)

pub struct BlitRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl BlitRenderer {
    pub fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        blend: Option<wgpu::BlendState>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/blit.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Blit Bind Group Layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blit Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: Some(&pipeline_layout),
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
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Blit Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }

    pub fn blit(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        src_view: &wgpu::TextureView,
        dst_view: &wgpu::TextureView,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Blit Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: dst_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

// ── Blur instance (48 bytes, 3 attributes at locations 0–2) ──────────────────

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlurInstance {
    rect: [f32; 4],       // loc 0: x, y, w, h in pixels
    tex_params: [f32; 4], // loc 1: [tex_w, tex_h, blur_radius, 0]
    tint: [f32; 4],       // loc 2: RGBA premultiplied
}

impl BlurInstance {
    pub fn new(rect: [f32; 4], tex_w: f32, tex_h: f32, blur_radius: f32, tint: [f32; 4]) -> Self {
        Self {
            rect,
            tex_params: [tex_w, tex_h, blur_radius, 0.0],
            tint,
        }
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
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
            ],
        }
    }
}

// ── Backdrop filter renderer ──────────────────────────────────────────────────

pub struct BackdropFilterRenderer {
    h_pipeline: wgpu::RenderPipeline,
    v_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl BackdropFilterRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blur Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/blur.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Blur Bind Group Layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blur Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let make_pipeline = |label, entry_point| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                cache: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[BlurInstance::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry_point),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None, // overwrite destination directly
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
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
            })
        };

        let h_pipeline = make_pipeline("Blur H Pipeline", "fs_horizontal");
        let v_pipeline = make_pipeline("Blur V Pipeline", "fs_vertical");

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Blur Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self {
            h_pipeline,
            v_pipeline,
            bind_group_layout,
            sampler,
        }
    }

    /// Two-pass separable Gaussian blur over the rect region.
    ///
    /// - H-pass: `composite` → `ping`   (reads full composite, writes rect region)
    /// - V-pass: `ping` → `composite`   (reads ping rect region, writes composite)
    ///
    /// `ping` is cleared to transparent before the H-pass so the V-pass can safely
    /// clamp to [0,1] without sampling garbage outside the rect region.
    pub fn apply(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        composite_view: &wgpu::TextureView,
        ping_view: &wgpu::TextureView,
        instance: &BlurInstance,
    ) {
        let instance_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Blur Instance Buffer"),
            contents: bytemuck::bytes_of(instance),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // ── H-pass: composite → ping ─────────────────────────────────────────
        {
            let bind_group = self.make_bind_group(device, composite_view);
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Blur H Encoder"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Blur H Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: ping_view,
                        resolve_target: None,
                        // Clear ping so outside-rect pixels are transparent,
                        // making V-pass sampling safe everywhere.
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.h_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(0, instance_buf.slice(..));
                pass.draw(0..4, 0..1);
            }
            queue.submit([encoder.finish()]);
        }

        // ── V-pass: ping → composite ─────────────────────────────────────────
        {
            let bind_group = self.make_bind_group(device, ping_view);
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Blur V Encoder"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Blur V Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: composite_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.v_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(0, instance_buf.slice(..));
                pass.draw(0..4, 0..1);
            }
            queue.submit([encoder.finish()]);
        }
    }

    fn make_bind_group(
        &self,
        device: &wgpu::Device,
        src_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blur Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }
}
