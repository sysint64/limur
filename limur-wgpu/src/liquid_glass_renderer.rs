use std::mem;
use wgpu::util::DeviceExt;

/// Per-instance data for both H-pass and V-refract pass.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LiquidGlassInstance {
    pub rect:       [f32; 4],  // x, y, w, h pixels
    pub tex_params: [f32; 4],  // tex_w, tex_h, blur_radius, 0
    pub tint:       [f32; 4],  // premultiplied RGBA
    pub params:     [f32; 4],  // power_factor, f_power, noise, glow_weight
    pub refraction: [f32; 4],  // a, b, c, d
}

impl LiquidGlassInstance {
    pub fn new(
        rect:         [f32; 4],
        tex_w:        f32,
        tex_h:        f32,
        blur_radius:  f32,
        tint:         [f32; 4],
        power_factor: f32,
        f_power:      f32,
        noise:        f32,
        glow_weight:  f32,
        a: f32, b: f32, c: f32, d: f32,
    ) -> Self {
        Self {
            rect,
            tex_params: [tex_w, tex_h, blur_radius, 0.0],
            tint,
            params:     [power_factor, f_power, noise, glow_weight],
            refraction: [a, b, c, d],
        }
    }

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x4 },
                wgpu::VertexAttribute { offset: 16, shader_location: 1, format: wgpu::VertexFormat::Float32x4 },
                wgpu::VertexAttribute { offset: 32, shader_location: 2, format: wgpu::VertexFormat::Float32x4 },
                wgpu::VertexAttribute { offset: 48, shader_location: 3, format: wgpu::VertexFormat::Float32x4 },
                wgpu::VertexAttribute { offset: 64, shader_location: 4, format: wgpu::VertexFormat::Float32x4 },
            ],
        }
    }
}

/// Two-pass liquid glass renderer (pre-pass into composite, like BlurRenderer).
///
/// `apply()` encodes two render passes:
///   H-pass:       composite → ping  (horizontal Gaussian, expanded Y quad)
///   V-refract:    ping → composite  (vertical blur + refraction + superellipse clip)
pub struct LiquidGlassRenderer {
    h_pipeline:        wgpu::RenderPipeline,
    v_pipeline:        wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler:           wgpu::Sampler,
}

impl LiquidGlassRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("Liquid Glass Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/liquid_glass.wgsl").into()),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label:   Some("Liquid Glass Bind Group Layout"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding:    1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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

        let make_pipeline = |entry: &'static str| -> wgpu::RenderPipeline {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    entry_point:         Some(entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::REPLACE,
                            alpha: wgpu::BlendComponent::REPLACE,
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
                multisample:   wgpu::MultisampleState::default(),
                multiview_mask: None,
            })
        };

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label:          Some("Liquid Glass Sampler"),
            mag_filter:     wgpu::FilterMode::Linear,
            min_filter:     wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self {
            h_pipeline:        make_pipeline("fs_horizontal"),
            v_pipeline:        make_pipeline("fs_vertical_refract"),
            bind_group_layout,
            sampler,
        }
    }

    /// Encode H-pass (composite → ping) and V-refract pass (ping → composite).
    pub fn apply(
        &self,
        device:         &wgpu::Device,
        encoder:        &mut wgpu::CommandEncoder,
        composite_view: &wgpu::TextureView,
        ping_view:      &wgpu::TextureView,
        instance:       &LiquidGlassInstance,
    ) {
        // H-pass quad expanded vertically by blur_radius (same trick as BlurRenderer).
        let radius = instance.tex_params[2];
        let expanded = LiquidGlassInstance {
            rect: [
                instance.rect[0],
                instance.rect[1] - radius,
                instance.rect[2],
                instance.rect[3] + 2.0 * radius,
            ],
            ..*instance
        };

        let h_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Liquid Glass H-Pass Buffer"),
            contents: bytemuck::bytes_of(&expanded),
            usage:    wgpu::BufferUsages::VERTEX,
        });
        let v_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Liquid Glass V-Pass Buffer"),
            contents: bytemuck::bytes_of(instance),
            usage:    wgpu::BufferUsages::VERTEX,
        });

        let make_bg = |src: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label:   Some("Liquid Glass Bind Group"),
                layout:  &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding:  0,
                        resource: wgpu::BindingResource::TextureView(src),
                    },
                    wgpu::BindGroupEntry {
                        binding:  1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            })
        };

        // H-pass: composite → ping (clear ping first)
        {
            let bg = make_bg(composite_view);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Liquid Glass H-Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view:           ping_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load:  wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes:         None,
                occlusion_query_set:      None,
                multiview_mask:           None,
            });
            pass.set_pipeline(&self.h_pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.set_vertex_buffer(0, h_buf.slice(..));
            pass.draw(0..4, 0..1);
        }

        // V-refract pass: ping → composite (LoadOp::Load so discard preserves original)
        {
            let bg = make_bg(ping_view);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Liquid Glass V-Refract Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view:           composite_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load:  wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes:         None,
                occlusion_query_set:      None,
                multiview_mask:           None,
            });
            pass.set_pipeline(&self.v_pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.set_vertex_buffer(0, v_buf.slice(..));
            pass.draw(0..4, 0..1);
        }
    }
}
