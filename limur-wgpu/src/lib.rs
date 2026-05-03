mod rect_renderer;

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

        let mut resources = Resources {
            plane: sumi::PlaneResources::new(&mut context),
            centered_plane: sumi::CenteredPlaneResources::new(&mut context),
            text: sumi::TextsResources::new(),
        };

        let mut fonts = sumi::FontResources::new();

        let mut renderers = Renderers {
            colored_plane: sumi::ColoredPlaneRenderer::new(
                &mut context,
                sumi::BumpInstances::new(128),
            ),
            rect: RectRenderer::new(&mut context, sumi::BumpInstances::new(128)),
            text: sumi::TextRenderer::new(&mut context),
        };

        Self {
            gapi,
            resources,
            renderers,
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
        // Clear per-frame instance data.
        self.renderers.rect.instances().clear();

        let mut encoder =
            self.gapi
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        let (optimal, output) = match self.gapi.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => (true, surface_texture),
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                log::warn!("Get current surface texture: suboptimal");

                (false, surface_texture)
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                panic!("Get current surface texture: timout");
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                panic!("Get current surface texture: occluded");
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                panic!("Get current surface texture: outdate");
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                panic!("Get current surface texture: lost");
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                panic!("Get current surface texture: validation error")
            }
        };

        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let fill_color = fill_color.unwrap_or(ColorRgba::from_hex(0xFF000000));
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.gapi.msaa_texture_view,
                    resolve_target: Some(&surface_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(to_wgpu_color(
                            self.gapi.surface_texture_format,
                            fill_color,
                        )),
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

            for layer in composition_layers {
                for command in &layer.commands {
                    match command {
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
                                * sumi::transforms_create_2d_model_matrix(&sumi::Transforms2D {
                                    position: pos,
                                    scaling: size,
                                    rotation: 0.0,
                                });

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

                            let id =
                                self.renderers.rect.instances().insert(RectInstance::new(
                                    &mvp,
                                    size,
                                    to_rect_fill(fill),
                                    color_t, width_t,
                                    color_r, width_r,
                                    color_b, width_b,
                                    color_l, width_l,
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
                        RenderCommand::Oval { boundary, fill, border } => {
                            let boundary = snap_rect(*boundary);

                            let pos = Vec2::new(
                                boundary.x + boundary.width * 0.5,
                                context.view.size_unscaled.y
                                    - boundary.y
                                    - boundary.height * 0.5,
                            );
                            let size = Vec2::new(boundary.width, boundary.height);
                            let mvp = context.view.screen_camera_matrix
                                * sumi::transforms_create_2d_model_matrix(&sumi::Transforms2D {
                                    position: pos,
                                    scaling: size,
                                    rotation: 0.0,
                                });

                            let (border_color, border_width) = extract_side(*border);
                            let border_width = snap_width(border_width);

                            let id = self.renderers.rect.instances().insert(
                                RectInstance::new_oval(
                                    &mvp,
                                    size,
                                    to_rect_fill(fill),
                                    border_color,
                                    border_width,
                                ),
                            );

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
                        RenderCommand::Text {
                            x,
                            y,
                            text_id,
                            tint_color,
                            ..
                        } => {
                            //
                        }
                        RenderCommand::PushClip { rect, shape, .. } => {
                            //
                        }
                        RenderCommand::PopClip => {
                            //
                        }
                        RenderCommand::Svg {
                            boundary,
                            asset_id,
                            tint_color,
                            ..
                        } => {
                            //
                        }
                        RenderCommand::BackdropFilter { boundary, shader } => {
                            //
                        }
                    }
                }
            }
        }

        self.gapi.queue.submit(std::iter::once(encoder.finish()));
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
    }
}

// Boundaries and widths arriving from limur are already in physical pixels
// (limur applies scale_factor via .px() before emitting RenderCommands).
// So snapping just means rounding to the nearest whole physical pixel.

fn snap_rect(rect: Rect<f32>) -> Rect<f32> {
    let x      = rect.x.round();
    let y      = rect.y.round();
    let right  = (rect.x + rect.width).round();
    let bottom = (rect.y + rect.height).round();
    Rect { x, y, width: right - x, height: bottom - y }
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
