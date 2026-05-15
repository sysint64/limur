mod blit_renderer;
mod blur_renderer;
mod liquid_glass_renderer;
mod text;
mod vector_renderer;
mod vector_resources;

use blit_renderer::BlitRenderer;
use blur_renderer::{BlurInstance, BlurRenderer};
use glam::Vec2;
use limur::{
    ColorRgba, PhysicalSize, Rect, ShaderId, ShaderParam, View,
    assets::Assets,
    render::{RenderCommand, RenderCompositionLayer, Renderer},
    text::{FontResources, TextsResources},
};
use liquid_glass_renderer::{LiquidGlassInstance, LiquidGlassRenderer};
use std::sync::Arc;
use text::TextResources;
use vector_renderer::VectorRenderer;
use vector_resources::{VectorData, VectorResources};
use winit::window::Window;

/// All intermediate compositor textures use this format so that blending always
/// happens in linear light space, regardless of the final surface format.
/// Rgba16Float is universally supported on desktop, iOS, and Android.
const COMPOSITOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    screen_size: [f32; 2],
}

pub struct WgpuRenderer {
    gapi: sumi::Graphics,
    resources: Resources,
    renderers: Renderers,
    compositor: Option<Compositor>,
}

struct Resources {
    vector: VectorResources,
    text: TextResources,
    globals_buffer: wgpu::Buffer,
    /// Index into shape_data of the first shape in the current vector batch.
    vector_batch_start: u32,
}

struct Renderers {
    layer_blit: BlitRenderer,
    surface_blit: BlitRenderer,
    vector: VectorRenderer,
    blur: BlurRenderer,
    liquid_glass: LiquidGlassRenderer,
}

#[derive(Copy, Clone, PartialEq)]
enum Pipeline {
    Vector,
}

struct Compositor {
    width: u32,
    height: u32,
    msaa_texture: wgpu::Texture,
    msaa_view: wgpu::TextureView,
    layer_texture: wgpu::Texture,
    layer_view: wgpu::TextureView,
    composite_texture: wgpu::Texture,
    composite_view: wgpu::TextureView,
    ping_texture: wgpu::Texture,
    ping_view: wgpu::TextureView,
}

impl Compositor {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let make_tex = |label: &str, sample_count: u32, usage: wgpu::TextureUsages| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: COMPOSITOR_FORMAT,
                usage,
                view_formats: &[],
            })
        };

        let rt_only = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let rt_bind = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;

        let msaa_tex = make_tex("Compositor MSAA", 4, rt_only);
        let msaa_view = msaa_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let layer_tex = make_tex("Layer Texture", 1, rt_bind);
        let layer_view = layer_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let composite_tex = make_tex("Composite Texture", 1, rt_bind);
        let composite_view = composite_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let ping_tex = make_tex("Blur Ping Texture", 1, rt_bind);
        let ping_view = ping_tex.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            width,
            height,
            msaa_texture: msaa_tex,
            msaa_view,
            layer_texture: layer_tex,
            layer_view,
            composite_texture: composite_tex,
            composite_view,
            ping_texture: ping_tex,
            ping_view,
        }
    }
}

impl WgpuRenderer {
    pub async fn new(window: Arc<Window>) -> Self {
        unsafe {
            sumi::Graphics::init_memory();
        };

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone()).unwrap();

        #[cfg(target_os = "macos")]
        #[allow(invalid_reference_casting)]
        unsafe {
            if let Some(hal_surface) = surface.as_hal::<wgpu::hal::api::Metal>() {
                let raw = (&*hal_surface) as *const wgpu::hal::metal::Surface
                    as *mut wgpu::hal::metal::Surface;
                (*raw).present_with_transaction = true;
            }
        }

        let (adapter, device, queue) =
            sumi::Graphics::wgpu_request_device(&instance, &surface).await;

        let size = window.inner_size();
        let view_size = Vec2::new(size.width as f32, size.height as f32);

        let mut gapi = sumi::Graphics::new(sumi::GraphicsCreateParams {
            id: 0,
            view_size,
            device,
            queue,
            adapter,
            surface,
        });

        gapi.set_scale_factor(window.scale_factor());
        gapi.set_view_size(view_size);

        let context = gapi.graphics_context(4);

        let globals_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screen Size Uniform"),
            size: 8, // vec2<f32>
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut resources = Resources {
            vector: VectorResources::new(),
            text: TextResources::new(&context, COMPOSITOR_FORMAT),
            globals_buffer,
            vector_batch_start: 0,
        };

        let surface_format = context.surface_texture_format;

        // Premultiplied alpha-over blend for layer -> composite (both in linear Rgba16Float).
        let alpha_over = wgpu::BlendState {
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
        };

        resources.vector.flush(&context);

        let renderers = Renderers {
            // layer -> composite: both Rgba16Float, no sRGB encoding needed.
            layer_blit: BlitRenderer::new(
                context.device,
                COMPOSITOR_FORMAT,
                Some(alpha_over),
                false,
            ),
            // composite -> surface: encode sRGB if the surface is not an sRGB format.
            surface_blit: BlitRenderer::new(
                context.device,
                surface_format,
                None,
                !surface_format.is_srgb(),
            ),
            vector: VectorRenderer::new(
                &context,
                &resources.vector,
                &resources.text,
                &resources.globals_buffer,
                COMPOSITOR_FORMAT,
            ),
            blur: BlurRenderer::new(context.device, COMPOSITOR_FORMAT),
            liquid_glass: LiquidGlassRenderer::new(context.device, COMPOSITOR_FORMAT),
        };

        Self {
            gapi,
            resources,
            renderers,
            compositor: None,
        }
    }

    fn ensure_compositor(&mut self) {
        let width = self.gapi.config.width;
        let height = self.gapi.config.height;

        let needs_rebuild = self
            .compositor
            .as_ref()
            .map_or(true, |c| c.width != width || c.height != height);

        if needs_rebuild {
            self.compositor = Some(Compositor::new(&self.gapi.device, width, height));
        }
    }
}

fn bind_pipeline(
    renderers: &Renderers,
    context: &sumi::GraphicsContext,
    render_pass: &mut wgpu::RenderPass,
    pipeline: Pipeline,
) {
    match pipeline {
        Pipeline::Vector => {
            renderers.vector.bind(&context, render_pass);
        }
    }
}

fn close_pipeline(
    resources: &mut Resources,
    renderers: &mut Renderers,
    context: &sumi::GraphicsContext,
    render_pass: &mut wgpu::RenderPass,
    current_pipeline: Pipeline,
) {
    match current_pipeline {
        Pipeline::Vector => {
            if resources.vector.take_buffer_resized() {
                renderers
                    .vector
                    .rebuild(&context, &resources.vector, &resources.globals_buffer);
            }

            resources.vector.flush(&context);
            renderers.vector.bind(&context, render_pass);

            let end = resources.vector.data.len() as u32;
            let range = resources.vector_batch_start..end;
            resources.vector_batch_start = end;
            render_pass.draw(0..4, range);
        }
    }
}

fn maybe_switch_pipeline(
    resources: &mut Resources,
    renderers: &mut Renderers,
    context: &sumi::GraphicsContext,
    render_pass: &mut wgpu::RenderPass,
    current_pipeline: Pipeline,
    new_pipeline: Pipeline,
) -> Pipeline {
    if current_pipeline != new_pipeline {
        close_pipeline(resources, renderers, context, render_pass, current_pipeline);
        bind_pipeline(renderers, context, render_pass, new_pipeline);
    }

    new_pipeline
}

impl Renderer for WgpuRenderer {
    fn process_commands(
        &mut self,
        view: &View,
        composition_layers: &[RenderCompositionLayer],
        fill_color: Option<ColorRgba>,
        fonts: &mut FontResources,
        text: &mut TextsResources,
        assets: &Assets,
    ) {
        self.ensure_compositor();

        let width = view.physical_size.width;
        let height = view.physical_size.height;

        let (optimal, output) = match self.gapi.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => (true, surface_texture),
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                log::warn!("Get current surface texture: suboptimal");
                (false, surface_texture)
            }
            wgpu::CurrentSurfaceTexture::Timeout => panic!("surface texture: timeout"),
            wgpu::CurrentSurfaceTexture::Occluded => panic!("surface texture: occluded"),
            wgpu::CurrentSurfaceTexture::Outdated => panic!("surface texture: outdated"),
            wgpu::CurrentSurfaceTexture::Lost => panic!("surface texture: lost"),
            wgpu::CurrentSurfaceTexture::Validation => panic!("surface texture: validation error"),
        };
        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let compositor = self.compositor.as_ref().unwrap();

        // Clear composite with fill_color
        {
            let fill = fill_color.unwrap_or(ColorRgba::from_hex(0xFF000000));
            let mut encoder =
                self.gapi
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Clear Composite Encoder"),
                    });
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Composite Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &compositor.composite_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Compositor is Rgba16Float (linear), so always linearize the clear color.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: srgb_to_linear(fill.r as f64),
                            g: srgb_to_linear(fill.g as f64),
                            b: srgb_to_linear(fill.b as f64),
                            a: fill.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            drop(_pass);
            self.gapi.queue.submit([encoder.finish()]);
        }

        // Write current screen size into the shared uniform before rendering.
        self.gapi.queue.write_buffer(
            &self.resources.globals_buffer,
            0,
            bytemuck::bytes_of(&Globals {
                screen_size: [width as f32, height as f32],
            }),
        );

        for layer in composition_layers {
            // Pre-pass: run blur on the composite texture before the layer render pass.
            for cmd in &layer.commands {
                if let RenderCommand::BackdropFilter { boundary, shader } = cmd {
                    match shader.id {
                        ShaderId::FrostedGlass => {
                            let mut blur_radius = 0.0f32;
                            let mut tint = [0.0f32; 4];

                            for (id, param) in &shader.params {
                                match id {
                                    0 => {
                                        if let ShaderParam::Float(r) = param {
                                            blur_radius = *r;
                                        }
                                    }
                                    1 => {
                                        if let ShaderParam::Color(c) = param {
                                            tint = [c.r * c.a, c.g * c.a, c.b * c.a, c.a];
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            let compositor = self.compositor.as_ref().unwrap();
                            let instance = BlurInstance::new(
                                [boundary.x, boundary.y, boundary.width, boundary.height],
                                width as f32,
                                height as f32,
                                blur_radius,
                                tint,
                            );
                            let mut blur_encoder = self.gapi.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("Blur Pre-Pass Encoder"),
                                },
                            );

                            self.renderers.blur.apply(
                                &self.gapi.device,
                                &mut blur_encoder,
                                &compositor.composite_view,
                                &compositor.ping_view,
                                &instance,
                            );
                            self.gapi.queue.submit([blur_encoder.finish()]);
                        }
                        ShaderId::LiquidGlass => {
                            let mut blur_radius = 0.0f32;
                            let mut tint = [0.0f32; 4];
                            let mut power_factor = 3.0f32;
                            let mut f_power = 1.0f32;
                            let mut noise = 0.06f32;
                            let mut glow_weight = 0.25f32;
                            let mut a = 0.7f32;
                            let mut b = 2.3f32;
                            let mut c = 5.2f32;
                            let mut d = 6.9f32;

                            for (id, param) in &shader.params {
                                match id {
                                    0 => {
                                        if let ShaderParam::Float(v) = param {
                                            blur_radius = *v;
                                        }
                                    }
                                    1 => {
                                        if let ShaderParam::Color(col) = param {
                                            tint = [
                                                col.r * col.a,
                                                col.g * col.a,
                                                col.b * col.a,
                                                col.a,
                                            ];
                                        }
                                    }
                                    2 => {
                                        if let ShaderParam::Float(v) = param {
                                            power_factor = *v;
                                        }
                                    }
                                    3 => {
                                        if let ShaderParam::Float(v) = param {
                                            f_power = *v;
                                        }
                                    }
                                    4 => {
                                        if let ShaderParam::Float(v) = param {
                                            noise = *v;
                                        }
                                    }
                                    5 => {
                                        if let ShaderParam::Float(v) = param {
                                            glow_weight = *v;
                                        }
                                    }
                                    6 => {
                                        if let ShaderParam::Float(v) = param {
                                            a = *v;
                                        }
                                    }
                                    7 => {
                                        if let ShaderParam::Float(v) = param {
                                            b = *v;
                                        }
                                    }
                                    8 => {
                                        if let ShaderParam::Float(v) = param {
                                            c = *v;
                                        }
                                    }
                                    9 => {
                                        if let ShaderParam::Float(v) = param {
                                            d = *v;
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            let compositor = self.compositor.as_ref().unwrap();
                            let instance = LiquidGlassInstance::new(
                                [boundary.x, boundary.y, boundary.width, boundary.height],
                                width as f32,
                                height as f32,
                                blur_radius,
                                tint,
                                power_factor,
                                f_power,
                                noise,
                                glow_weight,
                                a,
                                b,
                                c,
                                d,
                            );
                            let mut lg_encoder = self.gapi.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("Liquid Glass Pre-Pass Encoder"),
                                },
                            );

                            self.renderers.liquid_glass.apply(
                                &self.gapi.device,
                                &mut lg_encoder,
                                &compositor.composite_view,
                                &compositor.ping_view,
                                &instance,
                            );
                            self.gapi.queue.submit([lg_encoder.finish()]);
                        }
                    }
                }
            }

            let mut encoder =
                self.gapi
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Layer Encoder"),
                    });

            {
                let compositor = self.compositor.as_ref().unwrap();
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Layer Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &compositor.msaa_view,
                        resolve_target: Some(&compositor.layer_view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Discard,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                let context = self.gapi.graphics_context(4);

                self.resources.vector.data.clear();
                self.resources.vector.gradient_stops.clear();
                self.resources.vector_batch_start = 0;

                let mut current_pipeline = Pipeline::Vector;

                bind_pipeline(
                    &self.renderers,
                    &context,
                    &mut render_pass,
                    current_pipeline,
                );

                for command in &layer.commands {
                    match command {
                        RenderCommand::BackdropFilter { .. } => {}
                        _ => {
                            current_pipeline = maybe_switch_pipeline(
                                &mut self.resources,
                                &mut self.renderers,
                                &context,
                                &mut render_pass,
                                current_pipeline,
                                Pipeline::Vector,
                            );
                        }
                    }

                    match command {
                        RenderCommand::Shape {
                            boundary,
                            fill,
                            border_radius,
                            border,
                            shape,
                        } => {
                            let boundary = snap_rect(*boundary);
                            let gradient_params =
                                self.resources.vector.maybe_add_gradient(fill.as_ref());
                            self.resources.vector.data.push(VectorData::shape(
                                boundary,
                                fill.as_ref(),
                                *border_radius,
                                *border,
                                *shape,
                                gradient_params,
                            ));
                        }
                        RenderCommand::OuterBoxShadow {
                            boundary,
                            box_shadow,
                            border_radius,
                            shape,
                        } => {
                            let boundary = snap_rect(*boundary);
                            self.resources.vector.data.push(VectorData::shadow(
                                boundary,
                                *box_shadow,
                                *border_radius,
                                *shape,
                            ));
                        }
                        RenderCommand::InnerBoxShadow {
                            boundary,
                            box_shadow,
                            border_radius,
                            shape,
                        } => {
                            let boundary = snap_rect(*boundary);
                            self.resources.vector.data.push(VectorData::inner_shadow(
                                boundary,
                                *box_shadow,
                                *border_radius,
                                *shape,
                            ));
                        }
                        RenderCommand::Text {
                            boundary,
                            x,
                            y,
                            text_id,
                            tint_color,
                        } => {
                            let boundary = snap_rect(*boundary);
                            VectorData::text(
                                &context,
                                fonts,
                                text,
                                &mut self.resources.text,
                                &mut self.resources.vector,
                                view,
                                *text_id,
                                boundary,
                                *x,
                                *y,
                                *tint_color,
                            );
                        }
                        RenderCommand::PushClip { .. } => {}
                        RenderCommand::PopClip => {}
                        RenderCommand::Svg { .. } => {}
                        RenderCommand::BackdropFilter { .. } => {
                            // Handled in pre-pass above; no-op here.
                        }
                    }
                }

                close_pipeline(
                    &mut self.resources,
                    &mut self.renderers,
                    &context,
                    &mut render_pass,
                    current_pipeline,
                );
            }

            let compositor = self.compositor.as_ref().unwrap();
            self.renderers.layer_blit.blit(
                &self.gapi.device,
                &mut encoder,
                &compositor.layer_view,
                &compositor.composite_view,
            );

            self.gapi.queue.submit([encoder.finish()]);
        }

        {
            let compositor = self.compositor.as_ref().unwrap();
            let mut encoder =
                self.gapi
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Final Blit Encoder"),
                    });
            self.renderers.surface_blit.blit(
                &self.gapi.device,
                &mut encoder,
                &compositor.composite_view,
                &surface_view,
            );
            self.gapi.queue.submit([encoder.finish()]);
        }

        output.present();

        if !optimal {
            self.gapi.surface_configure();
        }
    }

    fn on_scale_factor_update(&mut self, scale_factor: f64) {
        self.gapi.set_scale_factor(scale_factor);
    }

    fn on_resized(&mut self, size: PhysicalSize) {
        self.gapi
            .set_view_size(Vec2::new(size.width as f32, size.height as f32));

        // NOTE(sysint64): Compositor textures are lazily recreated in ensure_compositor().
    }
}

fn snap_rect(rect: Rect<f32>) -> Rect<f32> {
    let x = rect.x.round();
    let y = rect.y.round();
    let right = (rect.x + rect.width).round();
    let bottom = (rect.y + rect.height).round();

    Rect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    }
}

pub fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
fn to_bottom_left_coordinates(view: &sumi::GraphicsView, coord: Vec2, size: Vec2) -> Vec2 {
    Vec2::new(coord.x, view.size.y - coord.y - size.y)
}

#[inline]
fn rect_to_bottom_left_coordinates(view: &sumi::GraphicsView, rect: Rect<f32>) -> Rect<f32> {
    let position = to_bottom_left_coordinates(
        view,
        Vec2::new(rect.x, rect.y),
        Vec2::new(rect.width, rect.height),
    );

    Rect::from_pos_size(limur::Vec2::new(position.x, position.y), rect.size())
}
