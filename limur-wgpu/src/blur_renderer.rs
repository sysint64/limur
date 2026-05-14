use std::mem;
use wgpu::util::DeviceExt;

/// Per-instance data matching blur.wgsl `InstanceInput` (locations 0–2).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlurInstance {
    rect:       [f32; 4], // x, y, w, h in pixels (UI-space, y-down)
    tex_params: [f32; 4], // [tex_width, tex_height, blur_radius, 0]
    tint:       [f32; 4], // premultiplied RGBA; applied in the V-pass
}

impl BlurInstance {
    pub fn new(
        rect: [f32; 4],
        tex_w: f32,
        tex_h: f32,
        blur_radius: f32,
        tint: [f32; 4],
    ) -> Self {
        Self {
            rect,
            tex_params: [tex_w, tex_h, blur_radius, 0.0],
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
            ],
        }
    }
}

/// Two-pass separable Gaussian blur.
///
/// `apply()` encodes two render passes into `encoder`:
///   H-pass: composite → ping   (horizontal blur; ping cleared first)
///   V-pass: ping → composite   (vertical blur + optional tint; loads existing composite)
///
/// Only the rect specified in `instance` is affected; pixels outside it are untouched.
pub struct BlurRenderer {
    h_pipeline:         wgpu::RenderPipeline,
    v_pipeline:         wgpu::RenderPipeline,
    bind_group_layout:  wgpu::BindGroupLayout,
    sampler:            wgpu::Sampler,
}

impl BlurRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("Blur Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/blur.wgsl").into()),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label:   Some("Blur Bind Group Layout"),
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
                label:               Some("Blur Pipeline Layout"),
                bind_group_layouts:  &[Some(&bind_group_layout)],
                immediate_size:      0,
            });

        let make_pipeline = |entry: &'static str| -> wgpu::RenderPipeline {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label:  Some("Blur Pipeline"),
                layout: Some(&pipeline_layout),
                cache:  None,
                vertex: wgpu::VertexState {
                    module:               &shader,
                    entry_point:          Some("vs_main"),
                    buffers:              &[BlurInstance::desc()],
                    compilation_options:  wgpu::PipelineCompilationOptions::default(),
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
                    topology:            wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format:  Some(wgpu::IndexFormat::Uint16),
                    cull_mode:           None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample:   wgpu::MultisampleState::default(),
                multiview_mask: None,
            })
        };

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label:            Some("Blur Sampler"),
            mag_filter:       wgpu::FilterMode::Linear,
            min_filter:       wgpu::FilterMode::Linear,
            address_mode_u:   wgpu::AddressMode::ClampToEdge,
            address_mode_v:   wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self {
            h_pipeline:        make_pipeline("fs_horizontal"),
            v_pipeline:        make_pipeline("fs_vertical"),
            bind_group_layout,
            sampler,
        }
    }

    /// Encode H-pass (composite → ping) and V-pass (ping → composite) into `encoder`.
    ///
    /// Pass `tint = [0,0,0,0]` to skip tinting in the V-pass (use the overlay pass instead).
    pub fn apply(
        &self,
        device:         &wgpu::Device,
        encoder:        &mut wgpu::CommandEncoder,
        composite_view: &wgpu::TextureView,
        ping_view:      &wgpu::TextureView,
        instance:       &BlurInstance,
    ) {
        // H-pass quad is expanded vertically by blur_radius on each side so that
        // the V-pass can sample ping without hitting transparent borders at the edges.
        let radius = instance.tex_params[2];
        let expanded = BlurInstance {
            rect: [
                instance.rect[0],
                instance.rect[1] - radius,
                instance.rect[2],
                instance.rect[3] + 2.0 * radius,
            ],
            tex_params: instance.tex_params,
            tint: instance.tint,
        };

        let h_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Blur H-Pass Instance Buffer"),
            contents: bytemuck::bytes_of(&expanded),
            usage:    wgpu::BufferUsages::VERTEX,
        });
        let v_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Blur V-Pass Instance Buffer"),
            contents: bytemuck::bytes_of(instance),
            usage:    wgpu::BufferUsages::VERTEX,
        });

        let make_bg = |src: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label:   Some("Blur Bind Group"),
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

        // H-pass: composite → ping (expanded quad so V-pass edges have valid data)
        {
            let bg   = make_bg(composite_view);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blur H-Pass"),
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

        // V-pass: ping → composite (original rect; ping has valid data in radius border)
        {
            let bg   = make_bg(ping_view);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blur V-Pass"),
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
