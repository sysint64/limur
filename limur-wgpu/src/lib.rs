mod backdrop_filter;
mod rect_renderer;

use backdrop_filter::{BackdropFilterRenderer, BlitRenderer, BlurInstance};
use glam::{Vec2, Vec4};
use limur::{
    Border, BorderRadius, BorderSide, ClipShape, ColorRgba, Gradient, PhysicalSize, Rect,
    ShaderParam, View,
    assets::Assets,
    profiler,
    render::{Fill, RenderCommand, RenderCompositionLayer, Renderer},
    text::{FontResources, TextsResources},
};
use rect_renderer::{RectFill, RectInstance, RectRenderer};
use std::sync::Arc;
use sumi::Instances;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

pub struct WgpuRenderer<'a> {
    gapi: sumi::Graphics,
    resources: Resources<'a>,
    renderers: Renderers,
    compositor: Option<Compositor>,
}

struct Resources<'a> {
    plane: sumi::PlaneResources,
    centered_plane: sumi::CenteredPlaneResources,
    text: sumi::TextsResources<'a>,
}

struct Renderers {
    colored_plane: sumi::ColoredPlaneRenderer,
    rect: RectRenderer,
    text: sumi::TextRenderer,
    layer_blit: BlitRenderer,
    surface_blit: BlitRenderer,
    backdrop_filter: BackdropFilterRenderer,
}

// ── Compositor textures ───────────────────────────────────────────────────────
// layer    — MSAA resolves here each segment (cleared to transparent per-segment)
// composite — accumulated frame; cleared once at frame start with fill_color
// ping      — scratch buffer for the H-blur pass

struct Compositor {
    width: u32,
    height: u32,
    layer_texture: wgpu::Texture,
    layer_view: wgpu::TextureView,
    composite_texture: wgpu::Texture,
    composite_view: wgpu::TextureView,
    ping_texture: wgpu::Texture,
    ping_view: wgpu::TextureView,
}

impl Compositor {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat, width: u32, height: u32) -> Self {
        let make_tex = |label: &str, usage: wgpu::TextureUsages| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };

        let rt_bind = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;

        let layer_tex = make_tex("Layer Texture", rt_bind);
        let layer_view = layer_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let composite_tex = make_tex("Composite Texture", rt_bind);
        let composite_view = composite_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let ping_tex = make_tex("Blur Ping Texture", rt_bind);
        let ping_view = ping_tex.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            width,
            height,
            layer_texture: layer_tex,
            layer_view,
            composite_texture: composite_tex,
            composite_view,
            ping_texture: ping_tex,
            ping_view,
        }
    }
}

impl<'a> WgpuRenderer<'a> {
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

        let mut context = gapi.graphics_context(4);

        let resources = Resources {
            plane: sumi::PlaneResources::new(&mut context),
            centered_plane: sumi::CenteredPlaneResources::new(&mut context),
            text: sumi::TextsResources::new(),
        };

        let fmt = context.surface_texture_format;

        // Premultiplied alpha-over blend for layer → composite.
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

        let renderers = Renderers {
            colored_plane: sumi::ColoredPlaneRenderer::new(
                &mut context,
                sumi::BumpInstances::new(128),
            ),
            rect: RectRenderer::new(&mut context, sumi::BumpInstances::new(128)),
            text: sumi::TextRenderer::new(&mut context),
            layer_blit: BlitRenderer::new(context.device, fmt, Some(alpha_over)),
            surface_blit: BlitRenderer::new(context.device, fmt, None),
            backdrop_filter: BackdropFilterRenderer::new(context.device, fmt),
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
            self.compositor = Some(Compositor::new(
                &self.gapi.device,
                self.gapi.surface_texture_format,
                width,
                height,
            ));
        }
    }
}

impl<'a> Renderer for WgpuRenderer<'a> {
    fn process_commands(
        &mut self,
        view: &View,
        composition_layers: &[RenderCompositionLayer],
        fill_color: Option<ColorRgba>,
        fonts: &mut FontResources,
        text: &mut TextsResources,
        assets: &Assets,
    ) {
        self.renderers.rect.instances().clear();
        self.ensure_compositor();

        let width = self.gapi.config.width;
        let height = self.gapi.config.height;

        // ── Acquire surface texture early (needed at the end for present) ────
        let (optimal, output) = match self.gapi.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => (true, t),
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                log::warn!("Get current surface texture: suboptimal");
                (false, t)
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

        // ── Clear composite with fill_color ──────────────────────────────────
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
                        load: wgpu::LoadOp::Clear(to_wgpu_color(
                            self.gapi.surface_texture_format,
                            fill,
                        )),
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

        // ── Flatten commands, split at BackdropFilter boundaries ─────────────
        // Each entry in `segments` is a slice of non-BackdropFilter commands.
        // `blurs` holds the BackdropFilter commands that follow each segment.
        let all_cmds: Vec<&RenderCommand> = composition_layers
            .iter()
            .flat_map(|l| l.commands.iter())
            .collect();

        let mut segments: Vec<Vec<&RenderCommand>> = vec![vec![]];
        let mut blurs: Vec<&RenderCommand> = vec![];

        for cmd in &all_cmds {
            if matches!(cmd, RenderCommand::BackdropFilter { .. }) {
                segments.push(vec![]);
                blurs.push(cmd);
            } else {
                segments.last_mut().unwrap().push(cmd);
            }
        }

        // ── Render each segment then apply its following blur (if any) ───────
        for (i, segment) in segments.iter().enumerate() {
            if !segment.is_empty() {
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
                            view: &self.gapi.msaa_texture_view,
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

                    let mut context = self.gapi.render_pass(&mut render_pass, 4);

                    for cmd in segment {
                        match cmd {
                            RenderCommand::Rect {
                                boundary,
                                fill,
                                border_radius,
                                border,
                            } => {
                                let boundary = snap_rect(*boundary);

                                let pos = Vec2::new(
                                    boundary.x + boundary.width * 0.5,
                                    context.view.size_unscaled.y
                                        - boundary.y
                                        - boundary.height * 0.5,
                                );
                                let size = Vec2::new(boundary.width, boundary.height);
                                let mvp = context.view.screen_camera_matrix
                                    * sumi::transforms_create_2d_model_matrix(
                                        &sumi::Transforms2D {
                                            position: pos,
                                            scaling: size,
                                            rotation: 0.0,
                                        },
                                    );

                                let (color_t, width_t) =
                                    extract_side(border.as_ref().and_then(|b| b.top));
                                let (color_r, width_r) =
                                    extract_side(border.as_ref().and_then(|b| b.right));
                                let (color_b, width_b) =
                                    extract_side(border.as_ref().and_then(|b| b.bottom));
                                let (color_l, width_l) =
                                    extract_side(border.as_ref().and_then(|b| b.left));
                                let (width_t, width_r, width_b, width_l) = (
                                    snap_width(width_t),
                                    snap_width(width_r),
                                    snap_width(width_b),
                                    snap_width(width_l),
                                );

                                let radii = border_radius.unwrap_or(BorderRadius::ZERO);
                                let snap_r = |r: f32| r.round();

                                let id = self.renderers.rect.instances().insert(RectInstance::new(
                                    &mvp,
                                    size,
                                    to_rect_fill(fill),
                                    color_t,
                                    width_t,
                                    color_r,
                                    width_r,
                                    color_b,
                                    width_b,
                                    color_l,
                                    width_l,
                                    [
                                        snap_r(radii.top_left),
                                        snap_r(radii.top_right),
                                        snap_r(radii.bottom_right),
                                        snap_r(radii.bottom_left),
                                    ],
                                ));

                                self.renderers.rect.instances().load_instance_to_gpu(
                                    &context,
                                    sumi::LoadToGPUSchedule::NextFrame,
                                    id,
                                );

                                self.renderers.rect.render_instance(
                                    &mut context,
                                    &self.resources.centered_plane,
                                    id,
                                );
                            }
                            RenderCommand::Oval {
                                boundary,
                                fill,
                                border,
                            } => {
                                let boundary = snap_rect(*boundary);

                                let pos = Vec2::new(
                                    boundary.x + boundary.width * 0.5,
                                    context.view.size_unscaled.y
                                        - boundary.y
                                        - boundary.height * 0.5,
                                );
                                let size = Vec2::new(boundary.width, boundary.height);
                                let mvp = context.view.screen_camera_matrix
                                    * sumi::transforms_create_2d_model_matrix(
                                        &sumi::Transforms2D {
                                            position: pos,
                                            scaling: size,
                                            rotation: 0.0,
                                        },
                                    );

                                let (border_color, border_width) = extract_side(*border);
                                let border_width = snap_width(border_width);

                                let id =
                                    self.renderers
                                        .rect
                                        .instances()
                                        .insert(RectInstance::new_oval(
                                            &mvp,
                                            size,
                                            to_rect_fill(fill),
                                            border_color,
                                            border_width,
                                        ));

                                self.renderers.rect.instances().load_instance_to_gpu(
                                    &context,
                                    sumi::LoadToGPUSchedule::NextFrame,
                                    id,
                                );

                                self.renderers.rect.render_instance(
                                    &mut context,
                                    &self.resources.centered_plane,
                                    id,
                                );
                            }
                            RenderCommand::Text { .. } => {}
                            RenderCommand::PushClip { .. } => {}
                            RenderCommand::PopClip => {}
                            RenderCommand::Svg { .. } => {}
                            RenderCommand::BackdropFilter { .. } => unreachable!(),
                        }
                    }
                }

                // Blit layer → composite (alpha-over blend).
                let compositor = self.compositor.as_ref().unwrap();
                self.renderers.layer_blit.blit(
                    &self.gapi.device,
                    &mut encoder,
                    &compositor.layer_view,
                    &compositor.composite_view,
                );

                self.gapi.queue.submit([encoder.finish()]);
            }

            // Apply the backdrop filter that follows this segment (if any).
            if i < blurs.len() {
                if let RenderCommand::BackdropFilter { boundary, shader } = blurs[i] {
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

                    let tint_premul = [
                        tint_color.r * tint_color.a,
                        tint_color.g * tint_color.a,
                        tint_color.b * tint_color.a,
                        tint_color.a,
                    ];

                    let compositor = self.compositor.as_ref().unwrap();
                    let instance = BlurInstance::new(
                        [boundary.x, boundary.y, boundary.width, boundary.height],
                        width as f32,
                        height as f32,
                        blur_radius,
                        tint_premul,
                    );

                    self.renderers.backdrop_filter.apply(
                        &self.gapi.device,
                        &self.gapi.queue,
                        &compositor.composite_view,
                        &compositor.ping_view,
                        &instance,
                    );
                }
            }
        }

        // ── Blit composite → surface ──────────────────────────────────────────
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
        // Compositor textures are lazily recreated in ensure_compositor().
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

// Boundaries and widths arriving from limur are already in physical pixels
// (limur applies scale_factor via .px() before emitting RenderCommands).
// Snapping just means rounding to the nearest whole physical pixel.

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

fn snap_width(w: f32) -> f32 {
    if w == 0.0 { 0.0 } else { w.round().max(1.0) }
}

fn to_vec4(c: ColorRgba) -> Vec4 {
    Vec4::new(c.r, c.g, c.b, c.a)
}

fn extract_side(side: Option<BorderSide>) -> (Vec4, f32) {
    match side {
        Some(s) => (to_vec4(s.color), s.width),
        None => (Vec4::ZERO, 0.0),
    }
}

fn to_rect_fill(fill: &Option<Fill>) -> RectFill<'_> {
    match fill {
        Some(Fill::Color(c)) => RectFill::Solid(to_vec4(*c)),
        None | Some(Fill::None) => RectFill::None,
        Some(Fill::Gradient(Gradient::Linear(g))) => RectFill::Linear {
            start: g.start,
            end: g.end,
            stops: &g.stops,
        },
        Some(Fill::Gradient(Gradient::Radial(g))) => RectFill::Radial {
            center: g.center,
            radius: g.radius,
            stops: &g.stops,
        },
        Some(Fill::Gradient(Gradient::Sweep(_))) => RectFill::None,
    }
}

fn to_wgpu_color(format: wgpu::TextureFormat, c: ColorRgba) -> wgpu::Color {
    if format.is_srgb() {
        wgpu::Color {
            r: srgb_to_linear(c.r as f64),
            g: srgb_to_linear(c.g as f64),
            b: srgb_to_linear(c.b as f64),
            a: c.a as f64,
        }
    } else {
        wgpu::Color {
            r: c.r as f64,
            g: c.g as f64,
            b: c.b as f64,
            a: c.a as f64,
        }
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
