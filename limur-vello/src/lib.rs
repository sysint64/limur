use cosmic_text::{Buffer, FontSystem};
use limur::{
    Border, BorderRadius, BorderSide, ClipShape, ColorRgb, ColorRgba, Gradient, Rect, ShaderParam,
    View,
    assets::Assets,
    profiler,
    render::{Fill, RenderCommand, RenderCompositionLayer, Renderer},
    text::{FontResources, TextsResources},
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{collections::HashMap, sync::Arc};
use vello::wgpu::util::DeviceExt;
use vello::{
    AaConfig, Glyph, RenderParams, RendererOptions, Scene,
    kurbo::{Affine, RoundedRect, RoundedRectRadii, Stroke},
    peniko::{
        self, Blob, Brush, Color, Fill as VelloFill, FontData, Gradient as VelloGradient, StyleRef,
    },
    util::RenderContext,
    wgpu,
};
use vello_svg::usvg;

/// Cache for FontData to avoid repeated allocations
struct FontCache {
    cache: HashMap<cosmic_text::fontdb::ID, FontData>,
}

impl FontCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    fn get_or_insert(
        &mut self,
        font_id: cosmic_text::fontdb::ID,
        font_system: &mut FontSystem,
    ) -> Option<&FontData> {
        if let std::collections::hash_map::Entry::Vacant(e) = self.cache.entry(font_id)
            && let Some(font) = font_system.get_font(font_id)
        {
            let font_data = FontData::new(Blob::new(Arc::new(font.data().to_vec())), 0);
            e.insert(font_data);
        }

        self.cache.get(&font_id)
    }

    fn _clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipeline for blitting one texture onto another (compositing layers)
struct CompositeBlitPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl CompositeBlitPipeline {
    fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composite_blit_shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
                struct VertexOutput {
                    @builtin(position) position: vec4<f32>,
                    @location(0) uv: vec2<f32>,
                };

                @vertex
                fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
                    var pos = array<vec2<f32>, 3>(
                        vec2<f32>(-1.0, -1.0),
                        vec2<f32>( 3.0, -1.0),
                        vec2<f32>(-1.0,  3.0),
                    );
                    var uv = array<vec2<f32>, 3>(
                        vec2<f32>(0.0, 1.0),
                        vec2<f32>(2.0, 1.0),
                        vec2<f32>(0.0, -1.0),
                    );
                    var out: VertexOutput;
                    out.position = vec4<f32>(pos[idx], 0.0, 1.0);
                    out.uv = uv[idx];
                    return out;
                }

                @group(0) @binding(0) var src_texture: texture_2d<f32>;
                @group(0) @binding(1) var src_sampler: sampler;

                @fragment
                fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                    return textureSample(src_texture, src_sampler, in.uv);
                }
                "#
                .into(),
            ),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("composite_blit_bind_group_layout"),
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
            label: Some("composite_blit_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("composite_blit_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
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
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("composite_blit_sampler"),
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

    fn blit(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        src_view: &wgpu::TextureView,
        dst_view: &wgpu::TextureView,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite_blit_bind_group"),
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
            label: Some("composite_blit_pass"),
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
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

// ── Backdrop blur pipeline ─────────────────────────────────────────────

/// Uniform data for the backdrop blur shader
#[repr(C)]
#[derive(Copy, Clone)]
struct BlurUniforms {
    /// Rect in pixels: x, y, width, height
    rect: [f32; 4],
    /// Texture dimensions: width, height
    tex_size: [f32; 2],
    /// Blur radius in pixels
    blur_radius: f32,
    _pad: f32,
    /// Tint color (RGBA, premultiplied)
    tint_color: [f32; 4],
}

impl BlurUniforms {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

struct BackdropBlurPipeline {
    h_pipeline: wgpu::RenderPipeline,
    v_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl BackdropBlurPipeline {
    fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backdrop_blur_shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
                struct Uniforms {
                    rect: vec4<f32>,      // x, y, width, height in pixels
                    tex_size: vec2<f32>,  // texture width, height
                    blur_radius: f32,
                    _pad: f32,
                    tint_color: vec4<f32>, // RGBA tint applied after blur
                };

                @group(0) @binding(0) var src_texture: texture_2d<f32>;
                @group(0) @binding(1) var src_sampler: sampler;
                @group(0) @binding(2) var<uniform> uniforms: Uniforms;

                struct VertexOutput {
                    @builtin(position) position: vec4<f32>,
                    @location(0) uv: vec2<f32>,
                };

                @vertex
                fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
                    var local_pos = array<vec2<f32>, 6>(
                        vec2<f32>(0.0, 0.0),
                        vec2<f32>(1.0, 0.0),
                        vec2<f32>(0.0, 1.0),
                        vec2<f32>(0.0, 1.0),
                        vec2<f32>(1.0, 0.0),
                        vec2<f32>(1.0, 1.0),
                    );

                    let lp = local_pos[idx];

                    let pixel_x = uniforms.rect.x + lp.x * uniforms.rect.z;
                    let pixel_y = uniforms.rect.y + lp.y * uniforms.rect.w;
                    let ndc_x = (pixel_x / uniforms.tex_size.x) * 2.0 - 1.0;
                    let ndc_y = 1.0 - (pixel_y / uniforms.tex_size.y) * 2.0;

                    let uv_x = pixel_x / uniforms.tex_size.x;
                    let uv_y = pixel_y / uniforms.tex_size.y;

                    var out: VertexOutput;
                    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
                    out.uv = vec2<f32>(uv_x, uv_y);
                    return out;
                }

                // Precomputed 1D Gaussian weight.
                // sigma chosen so that the kernel radius ≈ 2*sigma,
                // giving a smooth falloff that reaches ~0 at the edges.
                fn gaussian_weight(offset: f32, sigma: f32) -> f32 {
                    return exp(-(offset * offset) / (2.0 * sigma * sigma));
                }

                // Premultiplied alpha-over: src over dst
                fn blend_over(src: vec4<f32>, dst: vec4<f32>) -> vec4<f32> {
                    return src + dst * (1.0 - src.a);
                }

                @fragment
                fn fs_horizontal(in: VertexOutput) -> @location(0) vec4<f32> {
                    let radius = i32(uniforms.blur_radius);
                    if (radius <= 0) {
                        return textureSample(src_texture, src_sampler, in.uv);
                    }

                    let texel_x = 1.0 / uniforms.tex_size.x;
                    let sigma = f32(radius) * 0.5;

                    var color = vec4<f32>(0.0);
                    var total_weight = 0.0;
                    for (var dx = -radius; dx <= radius; dx = dx + 1) {
                        let w = gaussian_weight(f32(dx), sigma);
                        let sample_uv = vec2<f32>(
                            clamp(in.uv.x + f32(dx) * texel_x, 0.0, 1.0),
                            in.uv.y
                        );
                        color = color + textureSample(src_texture, src_sampler, sample_uv) * w;
                        total_weight = total_weight + w;
                    }

                    return color / total_weight;
                }

                @fragment
                fn fs_vertical(in: VertexOutput) -> @location(0) vec4<f32> {
                    let radius = i32(uniforms.blur_radius);
                    if (radius <= 0) {
                        let base = textureSample(src_texture, src_sampler, in.uv);
                        return blend_over(uniforms.tint_color, base);
                    }

                    let texel_y = 1.0 / uniforms.tex_size.y;
                    let sigma = f32(radius) * 0.5;

                    var color = vec4<f32>(0.0);
                    var total_weight = 0.0;
                    for (var dy = -radius; dy <= radius; dy = dy + 1) {
                        let w = gaussian_weight(f32(dy), sigma);
                        let sample_uv = vec2<f32>(
                            in.uv.x,
                            clamp(in.uv.y + f32(dy) * texel_y, 0.0, 1.0)
                        );
                        color = color + textureSample(src_texture, src_sampler, sample_uv) * w;
                        total_weight = total_weight + w;
                    }

                    let blurred = color / total_weight;

                    // Apply tint over the blurred backdrop (premultiplied alpha over)
                    return blend_over(uniforms.tint_color, blurred);
                }
                "#
                .into(),
            ),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("backdrop_blur_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("backdrop_blur_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let make_pipeline = |label: &str, entry_point: &str| -> wgpu::RenderPipeline {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry_point),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };

        let h_pipeline = make_pipeline("backdrop_blur_h_pipeline", "fs_horizontal");
        let v_pipeline = make_pipeline("backdrop_blur_v_pipeline", "fs_vertical");

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("backdrop_blur_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            h_pipeline,
            v_pipeline,
            bind_group_layout,
            sampler,
        }
    }

    /// Run a single blur pass (horizontal or vertical)
    fn draw_pass(
        &self,
        pipeline: &wgpu::RenderPipeline,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        src_view: &wgpu::TextureView,
        dst_view: &wgpu::TextureView,
        uniforms: &BlurUniforms,
    ) {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("blur_uniform_buffer"),
            contents: uniforms.as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("backdrop_blur_bind_group"),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("backdrop_blur_pass"),
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
            ..Default::default()
        });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..6, 0..1);
    }

    /// Two-pass separable Gaussian blur:
    ///   snapshot → horizontal blur → composite
    ///   composite → copy to snapshot → vertical blur → composite
    fn draw_blur(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        snapshot_view: &wgpu::TextureView,
        composite_view: &wgpu::TextureView,
        composite_tex: &wgpu::Texture,
        snapshot_tex: &wgpu::Texture,
        uniforms: &BlurUniforms,
        width: u32,
        height: u32,
    ) {
        // Pass 1: horizontal blur — read snapshot, write composite
        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("blur_h_encoder"),
            });
            self.draw_pass(
                &self.h_pipeline,
                device,
                &mut encoder,
                snapshot_view,
                composite_view,
                uniforms,
            );
            queue.submit([encoder.finish()]);
        }

        // Copy composite back to snapshot for the vertical pass
        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("blur_copy_encoder"),
            });
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: composite_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: snapshot_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit([encoder.finish()]);
        }

        // Pass 2: vertical blur — read snapshot (h-blurred), write composite
        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("blur_v_encoder"),
            });
            self.draw_pass(
                &self.v_pipeline,
                device,
                &mut encoder,
                snapshot_view,
                composite_view,
                uniforms,
            );
            queue.submit([encoder.finish()]);
        }
    }
}

// ── Main Renderer ──────────────────────────────────────────────────────

pub struct VelloRenderer {
    render_cx: RenderContext,
    surface: Option<vello::util::RenderSurface<'static>>,
    renderer: Option<vello::Renderer>,
    scene: Scene,
    font_cache: FontCache,

    /// Texture where each individual layer is rendered by Vello
    layer_texture: Option<wgpu::Texture>,
    layer_view: Option<wgpu::TextureView>,

    /// Accumulated composite: layers are blitted onto this one by one
    composite_texture: Option<wgpu::Texture>,
    composite_view: Option<wgpu::TextureView>,

    /// Snapshot of composite before current layer, used as blur source.
    /// We need this because we can't read and write the same texture simultaneously.
    composite_snapshot_texture: Option<wgpu::Texture>,
    composite_snapshot_view: Option<wgpu::TextureView>,

    /// Pipeline used to blit layer_texture onto composite_texture
    composite_blit: Option<CompositeBlitPipeline>,

    /// Pipeline for backdrop blur effect
    backdrop_blur: Option<BackdropBlurPipeline>,

    current_width: u32,
    current_height: u32,
}

impl VelloRenderer {
    pub async fn new<W>(window: Arc<W>, width: u32, height: u32) -> Self
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        let mut render_cx = RenderContext::new();

        let surface = render_cx
            .create_surface(window.clone(), width, height, wgpu::PresentMode::Fifo)
            .await
            .expect("Failed to create surface");

        #[cfg(target_os = "macos")]
        #[allow(invalid_reference_casting)]
        unsafe {
            if let Some(hal_surface) = surface.surface.as_hal::<wgpu::hal::api::Metal>() {
                let raw = (&*hal_surface) as *const wgpu::hal::metal::Surface
                    as *mut wgpu::hal::metal::Surface;
                (*raw).present_with_transaction = true;
            }
        }

        let device = &render_cx.devices[surface.dev_id].device;

        let renderer = vello::Renderer::new(device, RendererOptions::default())
            .expect("Failed to create Vello renderer");

        let mut config = surface.config.clone();
        config.desired_maximum_frame_latency = 3;
        surface.surface.configure(device, &config);

        let composite_blit = CompositeBlitPipeline::new(device, wgpu::TextureFormat::Rgba8Unorm);
        let backdrop_blur = BackdropBlurPipeline::new(device, wgpu::TextureFormat::Rgba8Unorm);

        Self {
            render_cx,
            surface: Some(surface),
            renderer: Some(renderer),
            scene: Scene::new(),
            font_cache: FontCache::new(),
            layer_texture: None,
            layer_view: None,
            composite_texture: None,
            composite_view: None,
            composite_snapshot_texture: None,
            composite_snapshot_view: None,
            composite_blit: Some(composite_blit),
            backdrop_blur: Some(backdrop_blur),

            current_width: width,
            current_height: height,
        }
    }

    fn ensure_textures(&mut self, width: u32, height: u32) {
        let Some(device_id) = self.surface.as_ref().map(|s| s.dev_id) else {
            return;
        };
        let device = &self.render_cx.devices[device_id].device;

        let needs_recreate = |tex: &Option<wgpu::Texture>| {
            tex.as_ref().map_or(true, |t| {
                t.size().width != width || t.size().height != height
            })
        };

        if needs_recreate(&self.layer_texture)
            || needs_recreate(&self.composite_texture)
            || needs_recreate(&self.composite_snapshot_texture)
        {
            let desc = wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            };

            let layer_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("layer_target"),
                ..desc
            });
            self.layer_view = Some(layer_tex.create_view(&Default::default()));
            self.layer_texture = Some(layer_tex);

            let composite_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("composite_target"),
                ..desc
            });
            self.composite_view = Some(composite_tex.create_view(&Default::default()));
            self.composite_texture = Some(composite_tex);

            let snapshot_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("composite_snapshot"),
                ..desc
            });
            self.composite_snapshot_view = Some(snapshot_tex.create_view(&Default::default()));
            self.composite_snapshot_texture = Some(snapshot_tex);
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        if self.current_width == width && self.current_height == height {
            return;
        }

        self.current_width = width;
        self.current_height = height;

        if let Some(surface) = &mut self.surface {
            self.render_cx.resize_surface(surface, width, height);
        }

        self.ensure_textures(width, height);
    }

    pub fn begin_frame(&mut self) {
        self.scene.reset();
    }

    /// Snapshot composite_texture → composite_snapshot_texture so we can
    /// read from snapshot while writing blurred output to composite.
    fn snapshot_composite(&mut self) {
        let Some(composite_tex) = &self.composite_texture else {
            return;
        };
        let Some(snapshot_tex) = &self.composite_snapshot_texture else {
            return;
        };
        let Some(surface) = &self.surface else { return };

        let device = &self.render_cx.devices[surface.dev_id].device;
        let queue = &self.render_cx.devices[surface.dev_id].queue;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("snapshot_composite"),
        });

        let size = wgpu::Extent3d {
            width: self.current_width,
            height: self.current_height,
            depth_or_array_layers: 1,
        };

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: composite_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: snapshot_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            size,
        );

        queue.submit([encoder.finish()]);
    }

    /// Execute backdrop blur: snapshot composite, then two-pass Gaussian blur
    fn execute_backdrop_blur(
        &mut self,
        boundary: Rect<f32>,
        blur_radius: f32,
        tint_color: ColorRgba,
    ) {
        self.snapshot_composite();

        let Some(snapshot_view) = &self.composite_snapshot_view else {
            return;
        };
        let Some(composite_view) = &self.composite_view else {
            return;
        };
        let Some(composite_tex) = &self.composite_texture else {
            return;
        };
        let Some(snapshot_tex) = &self.composite_snapshot_texture else {
            return;
        };
        let Some(surface) = &self.surface else { return };
        let Some(backdrop_blur) = &self.backdrop_blur else {
            return;
        };

        let device = &self.render_cx.devices[surface.dev_id].device;
        let queue = &self.render_cx.devices[surface.dev_id].queue;

        // Premultiply the tint color for correct blending in the shader
        let tint_premul = [
            tint_color.r * tint_color.a,
            tint_color.g * tint_color.a,
            tint_color.b * tint_color.a,
            tint_color.a,
        ];

        let uniforms = BlurUniforms {
            rect: [boundary.x, boundary.y, boundary.width, boundary.height],
            tex_size: [self.current_width as f32, self.current_height as f32],
            blur_radius,
            _pad: 0.0,
            tint_color: tint_premul,
        };

        backdrop_blur.draw_blur(
            device,
            queue,
            snapshot_view,
            composite_view,
            composite_tex,
            snapshot_tex,
            &uniforms,
            self.current_width,
            self.current_height,
        );
    }

    pub fn process_commands(
        &mut self,
        view: &View,
        fonts: &mut FontResources,
        text: &mut TextsResources,
        assets: &Assets,
        commands: &[RenderCommand],
    ) {
        for command in commands {
            match command {
                RenderCommand::Shape {
                    boundary,
                    fill,
                    border_radius,
                    border,
                    shape,
                } => match shape {
                    limur::BoxShape::Rect => self.draw_rect(
                        *boundary,
                        fill.as_ref(),
                        border_radius.as_ref(),
                        border.as_ref(),
                    ),
                    limur::BoxShape::Oval => self.draw_oval(
                        *boundary,
                        fill.as_ref(),
                        border
                            .map(|it| {
                                it.top
                                    .or(it.bottom)
                                    .or(it.left)
                                    .or(it.right)
                                    .unwrap_or(BorderSide::default())
                            })
                            .as_ref(),
                    ),
                },
                RenderCommand::Text {
                    x,
                    y,
                    text_id,
                    tint_color,
                    ..
                } => {
                    let color = tint_color
                        .map(|c| convert_rgba_color(&c))
                        .unwrap_or_else(|| Color::from_rgba8(0, 0, 0, 255));

                    text.get_mut(*text_id).with_buffer_mut(|buffer| {
                        let brush = Brush::Solid(color);

                        for run in buffer.layout_runs() {
                            let line_y = y + run.line_y.round();

                            let mut font_glyphs: HashMap<
                                cosmic_text::fontdb::ID,
                                Vec<(Glyph, f32)>,
                            > = HashMap::new();

                            for glyph in run.glyphs.iter() {
                                let physical = glyph.physical((*x, line_y), 1.0);
                                let font_size = f32::from_bits(physical.cache_key.font_size_bits);

                                let vello_glyph = Glyph {
                                    id: physical.cache_key.glyph_id as u32,
                                    x: x + glyph.x + glyph.x_offset,
                                    y: glyph.y - glyph.y_offset + line_y,
                                };

                                font_glyphs
                                    .entry(glyph.font_id)
                                    .or_default()
                                    .push((vello_glyph, font_size));
                            }

                            for (font_id, glyphs) in font_glyphs {
                                if let Some(vello_font) = self
                                    .font_cache
                                    .get_or_insert(font_id, &mut fonts.font_system)
                                {
                                    let font_size = glyphs
                                        .first()
                                        .map(|(_, s)| *s)
                                        .unwrap_or((12.0 * view.scale_factor) as f32);
                                    let glyph_iter = glyphs.into_iter().map(|(g, _)| g);

                                    self.scene
                                        .draw_glyphs(vello_font)
                                        .font_size(font_size)
                                        .brush(&brush)
                                        .draw(StyleRef::Fill(peniko::Fill::NonZero), glyph_iter);
                                }
                            }
                        }
                    });
                }
                RenderCommand::PushClip { rect, shape, .. } => match shape {
                    ClipShape::Rect => {
                        self.scene.push_clip_layer(
                            Affine::IDENTITY,
                            &vello::kurbo::Rect::new(
                                rect.x as f64,
                                rect.y as f64,
                                (rect.x + rect.width) as f64,
                                (rect.y + rect.height) as f64,
                            ),
                        );
                    }
                    ClipShape::RoundedRect { border_radius } => self.scene.push_clip_layer(
                        Affine::IDENTITY,
                        &vello::kurbo::RoundedRect::new(
                            rect.x as f64,
                            rect.y as f64,
                            (rect.x + rect.width) as f64,
                            (rect.y + rect.height) as f64,
                            RoundedRectRadii {
                                top_left: border_radius.top_left as f64,
                                top_right: border_radius.top_right as f64,
                                bottom_right: border_radius.bottom_right as f64,
                                bottom_left: border_radius.bottom_left as f64,
                            },
                        ),
                    ),
                    ClipShape::Oval => {
                        let center = vello::kurbo::Point::new(
                            (rect.x + rect.width / 2.0) as f64,
                            (rect.y + rect.height / 2.0) as f64,
                        );
                        let radii = vello::kurbo::Vec2::new(
                            (rect.width / 2.0) as f64,
                            (rect.height / 2.0) as f64,
                        );

                        self.scene.push_clip_layer(
                            Affine::IDENTITY,
                            &vello::kurbo::Ellipse::new(center, radii, 0.0),
                        );
                    }
                },
                RenderCommand::PopClip => {
                    self.scene.pop_layer();
                }
                RenderCommand::Svg {
                    boundary,
                    asset_id,
                    tint_color,
                    ..
                } => {
                    if let Some(tree) = assets.get_svg_tree(asset_id) {
                        self.draw_svg(tree, *boundary, *tint_color);
                    } else {
                        log::warn!("SVG with ID = {} not found", asset_id);
                    }
                }
                RenderCommand::BackdropFilter { boundary, shader } => {
                    self.flush_scene_to_composite();

                    let mut blur_radius = 10.0f32;
                    let mut tint_color = ColorRgba::TRANSPARENT;

                    for (id, param) in &shader.params {
                        match id {
                            0 => {
                                if let ShaderParam::Float(r) = param {
                                    blur_radius = *r;
                                }
                            }
                            1 => {
                                if let ShaderParam::Color(c) = param {
                                    tint_color = *c;
                                }
                            }
                            _ => {}
                        }
                    }

                    self.execute_backdrop_blur(*boundary, blur_radius, tint_color);
                }
                RenderCommand::OuterBoxShadow { .. } => {
                    // TODO
                }
                RenderCommand::InnerBoxShadow { .. } => {
                    // TODO
                }
            }
        }
    }

    /// Render whatever is currently in self.scene into composite_texture,
    /// then reset the scene so we can keep adding commands after.
    fn flush_scene_to_composite(&mut self) {
        if self.scene.encoding().is_empty() {
            return;
        }

        let Some(layer_view) = &self.layer_view else {
            return;
        };
        let Some(composite_view) = &self.composite_view else {
            return;
        };
        let Some(surface) = &self.surface else { return };
        let Some(renderer) = &mut self.renderer else {
            return;
        };

        let device = &self.render_cx.devices[surface.dev_id].device;
        let queue = &self.render_cx.devices[surface.dev_id].queue;

        let render_params = RenderParams {
            base_color: Color::TRANSPARENT,
            width: self.current_width,
            height: self.current_height,
            antialiasing_method: AaConfig::Msaa16,
        };

        renderer
            .render_to_texture(device, queue, &self.scene, layer_view, &render_params)
            .expect("Failed to render scene to layer texture");

        // Blit layer onto composite (premultiplied alpha over)
        if let Some(blit) = &self.composite_blit {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("flush_blit_encoder"),
            });
            blit.blit(device, &mut encoder, layer_view, composite_view);
            queue.submit([encoder.finish()]);
        }

        self.scene.reset();
    }

    fn clear_composite(&mut self, clear_color: Color) {
        let Some(composite_view) = &self.composite_view else {
            return;
        };
        let Some(surface) = &self.surface else { return };

        let device = &self.render_cx.devices[surface.dev_id].device;
        let queue = &self.render_cx.devices[surface.dev_id].queue;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("clear_composite"),
        });

        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear_composite_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: composite_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: clear_color.components[0] as f64,
                        g: clear_color.components[1] as f64,
                        b: clear_color.components[2] as f64,
                        a: clear_color.components[3] as f64,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        drop(_pass);

        queue.submit([encoder.finish()]);
    }

    fn present_composite(&mut self) {
        let Some(composite_view) = &self.composite_view else {
            return;
        };
        let Some(surface) = &self.surface else { return };

        let device = &self.render_cx.devices[surface.dev_id].device;
        let queue = &self.render_cx.devices[surface.dev_id].queue;

        let surface_texture = surface
            .surface
            .get_current_texture()
            .expect("Failed to get surface texture");

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("final_blit"),
        });

        surface.blitter.copy(
            device,
            &mut encoder,
            composite_view,
            &surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );

        queue.submit([encoder.finish()]);
        surface_texture.present();

        device.poll(wgpu::PollType::Poll).unwrap();
    }

    // ── Shape drawing methods ──────────────────────────────────────────

    pub fn draw_rect(
        &mut self,
        boundary: Rect<f32>,
        fill: Option<&Fill>,
        border_radius: Option<&BorderRadius>,
        border: Option<&Border>,
    ) {
        let rect = vello::kurbo::Rect::new(
            boundary.x as f64,
            boundary.y as f64,
            (boundary.x + boundary.width) as f64,
            (boundary.y + boundary.height) as f64,
        );

        let shape = if let Some(br) = border_radius {
            RoundedRect::from_rect(
                rect,
                RoundedRectRadii::new(
                    br.top_left as f64,
                    br.top_right as f64,
                    br.bottom_right as f64,
                    br.bottom_left as f64,
                ),
            )
        } else {
            RoundedRect::from_rect(rect, 0.0)
        };

        if let Some(fill) = fill
            && let Some(brush) = create_brush_from_fill(fill, boundary)
        {
            self.scene
                .fill(VelloFill::NonZero, Affine::IDENTITY, &brush, None, &shape);
        }

        if let Some(border) = border {
            self.draw_border(&shape, border);
        }
    }

    fn draw_border(&mut self, shape: &RoundedRect, border: &Border) {
        let (max_width, color) = get_border_params(border);

        if max_width > 0.0 {
            let stroke = Stroke::new(max_width as f64);
            let brush = Brush::Solid(convert_rgba_color(&color));

            self.scene
                .stroke(&stroke, Affine::IDENTITY, &brush, None, shape);
        }
    }

    pub fn draw_oval(
        &mut self,
        boundary: Rect<f32>,
        fill: Option<&Fill>,
        border: Option<&BorderSide>,
    ) {
        let ellipse = vello::kurbo::Ellipse::new(
            (
                (boundary.x + boundary.width / 2.0) as f64,
                (boundary.y + boundary.height / 2.0) as f64,
            ),
            (
                (boundary.width / 2.0) as f64,
                (boundary.height / 2.0) as f64,
            ),
            0.0,
        );

        if let Some(fill) = fill
            && let Some(brush) = create_brush_from_fill(fill, boundary)
        {
            self.scene
                .fill(VelloFill::NonZero, Affine::IDENTITY, &brush, None, &ellipse);
        }

        if let Some(border_side) = border
            && border_side.width > 0.0
        {
            let stroke = Stroke::new(border_side.width as f64);
            let brush = Brush::Solid(convert_rgba_color(&border_side.color));
            self.scene
                .stroke(&stroke, Affine::IDENTITY, &brush, None, &ellipse);
        }
    }

    pub fn draw_text(
        &mut self,
        font_system: &mut FontSystem,
        buffer: &Buffer,
        x: f32,
        y: f32,
        color: Color,
    ) {
        let brush = Brush::Solid(color);

        let mut font_glyphs: HashMap<cosmic_text::fontdb::ID, Vec<(Glyph, f32)>> = HashMap::new();

        for run in buffer.layout_runs() {
            let line_y = y + run.line_y;

            for glyph in run.glyphs.iter() {
                let physical = glyph.physical((x, line_y), 1.0);
                let font_size = f32::from_bits(physical.cache_key.font_size_bits);

                let vello_glyph = Glyph {
                    id: physical.cache_key.glyph_id as u32,
                    x: physical.x as f32,
                    y: physical.y as f32,
                };

                font_glyphs
                    .entry(glyph.font_id)
                    .or_default()
                    .push((vello_glyph, font_size));
            }
        }

        for (font_id, glyphs) in font_glyphs {
            if let Some(vello_font) = self.font_cache.get_or_insert(font_id, font_system) {
                let font_size = glyphs.first().map(|(_, s)| *s).unwrap_or(16.0);
                let glyph_iter = glyphs.into_iter().map(|(g, _)| g);

                self.scene
                    .draw_glyphs(vello_font)
                    .font_size(font_size)
                    .brush(&brush)
                    .draw(StyleRef::Fill(peniko::Fill::NonZero), glyph_iter);
            }
        }
    }

    pub fn draw_svg(
        &mut self,
        tree: &usvg::Tree,
        boundary: Rect<f32>,
        tint_color: Option<ColorRgba>,
    ) {
        let sx = boundary.width / tree.size().width();
        let sy = boundary.height / tree.size().height();

        let transform = Affine::scale_non_uniform(sx as f64, sy as f64)
            .then_translate((boundary.x as f64, boundary.y as f64).into());

        let svg_scene = vello_svg::render_tree(tree);

        if let Some(tint) = tint_color {
            let clip_rect = vello::kurbo::Rect::new(
                boundary.x as f64,
                boundary.y as f64,
                (boundary.x + boundary.width) as f64,
                (boundary.y + boundary.height) as f64,
            );

            self.scene.push_layer(
                peniko::BlendMode::default(),
                1.0,
                Affine::IDENTITY,
                &clip_rect,
            );

            self.scene.append(&svg_scene, Some(transform));

            let tint_brush = Brush::Solid(convert_rgba_color(&tint));
            self.scene.push_layer(
                peniko::BlendMode::new(peniko::Mix::Normal, peniko::Compose::SrcIn),
                1.0,
                Affine::IDENTITY,
                &clip_rect,
            );
            self.scene.fill(
                VelloFill::NonZero,
                Affine::IDENTITY,
                &tint_brush,
                None,
                &clip_rect,
            );
            self.scene.pop_layer();

            self.scene.pop_layer();
        } else {
            self.scene.append(&svg_scene, Some(transform));
        }
    }
}

impl Renderer for VelloRenderer {
    fn process_commands(
        &mut self,
        view: &View,
        composition_layers: &[RenderCompositionLayer],
        fill_color: Option<ColorRgba>,
        fonts: &mut FontResources,
        text: &mut TextsResources,
        assets: &Assets,
    ) {
        let _g = profiler::scope_named("vello::render");

        let width = view.physical_size.width;
        let height = view.physical_size.height;

        self.resize(width, height);
        self.ensure_textures(width, height);

        let base_color = fill_color
            .map(|c| convert_rgba_color(&c))
            .unwrap_or(Color::TRANSPARENT);

        self.clear_composite(base_color);

        for layer in composition_layers {
            self.scene.reset();
            self.process_commands(view, fonts, text, assets, &layer.commands);

            // Flush any remaining scene content after all commands
            // (BackdropFilter may have already flushed mid-layer)
            self.flush_scene_to_composite();
        }

        self.present_composite();
    }
}

// ── Helper functions ───────────────────────────────────────────────────

fn convert_rgba_color(color: &ColorRgba) -> Color {
    Color::from_rgba8(
        (color.r * 255.) as u8,
        (color.g * 255.) as u8,
        (color.b * 255.) as u8,
        (color.a * 255.) as u8,
    )
}

fn convert_rgb_color(color: &ColorRgb) -> Color {
    Color::from_rgb8(
        (color.r * 255.) as u8,
        (color.g * 255.) as u8,
        (color.b * 255.) as u8,
    )
}

fn create_brush_from_fill(fill: &Fill, rect: Rect<f32>) -> Option<Brush> {
    match fill {
        Fill::None => None,
        Fill::Color(color) => Some(Brush::Solid(convert_rgba_color(color))),
        Fill::Gradient(gradient) => create_gradient_brush(gradient, rect),
    }
}

fn create_gradient_brush(gradient: &Gradient, rect: Rect<f32>) -> Option<Brush> {
    match gradient {
        Gradient::Linear(linear) => {
            let start_x = rect.x + linear.start.0 * rect.width;
            let start_y = rect.y + linear.start.1 * rect.height;
            let end_x = rect.x + linear.end.0 * rect.width;
            let end_y = rect.y + linear.end.1 * rect.height;

            let stops: Vec<peniko::ColorStop> = linear
                .stops
                .iter()
                .map(|stop| peniko::ColorStop {
                    offset: stop.offset,
                    color: convert_rgba_color(&stop.color).into(),
                })
                .collect();

            let grad = VelloGradient::new_linear(
                (start_x as f64, start_y as f64),
                (end_x as f64, end_y as f64),
            )
            .with_stops(stops.as_slice());

            Some(Brush::Gradient(grad))
        }
        Gradient::Radial(radial) => {
            let center_x = rect.x + radial.center.0 * rect.width;
            let center_y = rect.y + radial.center.1 * rect.height;
            let radius = radial.radius * rect.width.max(rect.height);

            let stops: Vec<peniko::ColorStop> = radial
                .stops
                .iter()
                .map(|stop| peniko::ColorStop {
                    offset: stop.offset,
                    color: convert_rgba_color(&stop.color).into(),
                })
                .collect();

            let grad = VelloGradient::new_radial((center_x, center_y), radius)
                .with_stops(stops.as_slice());

            Some(Brush::Gradient(grad))
        }
        Gradient::Sweep(sweep) => {
            let center_x = rect.x + sweep.center.0 * rect.width;
            let center_y = rect.y + sweep.center.1 * rect.height;

            let stops: Vec<peniko::ColorStop> = sweep
                .stops
                .iter()
                .map(|stop| peniko::ColorStop {
                    offset: stop.offset,
                    color: convert_rgba_color(&stop.color).into(),
                })
                .collect();

            let grad =
                VelloGradient::new_sweep((center_x, center_y), sweep.start_angle, sweep.end_angle)
                    .with_stops(stops.as_slice());

            Some(Brush::Gradient(grad))
        }
    }
}

fn get_border_params(border: &Border) -> (f32, ColorRgba) {
    let max_width = [
        border.top.as_ref().map(|s| s.width).unwrap_or(0.0),
        border.right.as_ref().map(|s| s.width).unwrap_or(0.0),
        border.bottom.as_ref().map(|s| s.width).unwrap_or(0.0),
        border.left.as_ref().map(|s| s.width).unwrap_or(0.0),
    ]
    .into_iter()
    .fold(0.0f32, f32::max);

    let color = border
        .top
        .as_ref()
        .or(border.right.as_ref())
        .or(border.bottom.as_ref())
        .or(border.left.as_ref())
        .map(|s| s.color)
        .unwrap_or(ColorRgba::TRANSPARENT);

    (max_width, color)
}
