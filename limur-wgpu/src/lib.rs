mod blit_renderer;
mod vector_renderer;
mod vector_resources;

use blit_renderer::BlitRenderer;
use glam::{Vec2, Vec4};
use limur::{
    BorderRadius, BorderSide, ClipShape, ColorRgba, Gradient, PhysicalSize, Rect, ShaderParam,
    View,
    assets::Assets,
    render::{RenderCommand, RenderCompositionLayer, Renderer},
    text::{FontResources, TextsResources},
};
use std::sync::Arc;
use sumi::{InstancingGeometry, PlaneResources, RenderInstances};
use vector_renderer::{VectorInstance, VectorInstanceId, VectorRenderer};
use vector_resources::{VectorData, VectorResources};
use winit::window::Window;

pub struct WgpuRenderer {
    gapi: sumi::Graphics,
    resources: Resources,
    renderers: Renderers,
    compositor: Option<Compositor>,
}

struct Resources {
    plane: sumi::PlaneResources,
    vector_instances: sumi::BumpInstances<VectorInstanceId, VectorInstance>,
    vector: VectorResources,
}

struct Renderers {
    layer_blit: BlitRenderer,
    surface_blit: BlitRenderer,
    vector: VectorRenderer,
}

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

        let mut context = gapi.graphics_context(4);

        let mut resources = Resources {
            plane: sumi::PlaneResources::new(&mut context),
            vector: VectorResources::new(),
            vector_instances: sumi::BumpInstances::new(128),
        };

        let fmt = context.surface_texture_format;

        // Premultiplied alpha-over blend for layer -> composite.
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
            layer_blit: BlitRenderer::new(context.device, fmt, Some(alpha_over)),
            surface_blit: BlitRenderer::new(context.device, fmt, None),
            vector: VectorRenderer::new(&context, &mut resources.vector),
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

        let width = self.gapi.config.width;
        let height = self.gapi.config.height;

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

        let mut clip_stack: Vec<(Rect<f32>, ClipShape)> = Vec::new();
        let mut clip_depth: u32 = 0;

        for layer in composition_layers {
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

                self.resources.vector.data.clear();
                self.resources.vector.gradient_stops.clear();
                self.resources.vector_instances.clear();

                self.renderers.vector.bind(&context);

                let instances = &mut self.resources.vector_instances;

                for command in &layer.commands {
                    match command {
                        RenderCommand::Shape {
                            boundary,
                            fill,
                            border_radius,
                            border,
                            shape,
                        } => {
                            let boundary = snap_rect(*boundary);

                            let pos = Vec2::new(
                                boundary.x,
                                context.view.size_unscaled.y - boundary.y - boundary.height,
                            );

                            let mvp = context.view.screen_camera_matrix
                                * sumi::transforms_create_2d_model_matrix(&sumi::Transforms2D {
                                    position: pos,
                                    scaling: Vec2::new(boundary.width, boundary.height),
                                    rotation: 0.0,
                                });

                            instances.insert(VectorInstance::new(mvp));

                            let gradient_params = self
                                .resources
                                .vector
                                .maybe_add_gradient(&context, fill.as_ref());

                            self.resources.vector.data.push(VectorData::shape(
                                &context,
                                boundary,
                                fill.as_ref(),
                                *border_radius,
                                *border,
                                *shape,
                                gradient_params,
                            ));

                            // let id = self.renderers.colored_plane.instances().insert(
                            //     ColoredPlaneInstance::new(&mvp, &Vec4::new(1., 0., 0., 1.)),
                            // );

                            // self.renderers
                            //     .colored_plane
                            //     .instances()
                            //     .load_instance_to_gpu(
                            //         &context,
                            //         sumi::LoadToGPUSchedule::NextFrame,
                            //         id,
                            //     );

                            // self.renderers.colored_plane.render_instance(
                            //     &mut context,
                            //     &self.resources.plane,
                            //     id,
                            // );

                            // let boundary = snap_rect(*boundary);

                            // let pos = Vec2::new(
                            //     boundary.x,
                            //     context.view.size_unscaled.y - boundary.y - boundary.height,
                            // );
                            // let size = Vec2::new(boundary.width, boundary.height);
                            // let blur = 20.;
                            // let expand = 3.0 * blur;

                            // let mvp = context.view.screen_camera_matrix
                            //     * sumi::transforms_create_2d_model_matrix(&sumi::Transforms2D {
                            //         position: pos - Vec2::new(expand, expand),
                            //         scaling: size + Vec2::new(expand * 2.0, expand * 2.0),
                            //         rotation: 0.0,
                            //     });

                            // let (color_t, width_t) =
                            //     extract_side(border.as_ref().and_then(|b| b.top));
                            // let (color_r, width_r) =
                            //     extract_side(border.as_ref().and_then(|b| b.right));
                            // let (color_b, width_b) =
                            //     extract_side(border.as_ref().and_then(|b| b.bottom));
                            // let (color_l, width_l) =
                            //     extract_side(border.as_ref().and_then(|b| b.left));
                            // let (width_t, width_r, width_b, width_l) = (
                            //     snap_width(width_t),
                            //     snap_width(width_r),
                            //     snap_width(width_b),
                            //     snap_width(width_l),
                            // );

                            // let radii = border_radius.unwrap_or(BorderRadius::ZERO);
                            // let snap_r = |r: f32| r.round();

                            // let id = self.renderers.rect.instances().insert(RectInstance::new(
                            //     &mvp,
                            //     Vec2::new(boundary.x, boundary.y),
                            //     size,
                            //     to_rect_fill(fill),
                            //     color_t,
                            //     width_t,
                            //     color_r,
                            //     width_r,
                            //     color_b,
                            //     width_b,
                            //     color_l,
                            //     width_l,
                            //     [
                            //         snap_r(radii.top_left),
                            //         snap_r(radii.top_right),
                            //         snap_r(radii.bottom_right),
                            //         snap_r(radii.bottom_left),
                            //     ],
                            // ));

                            // self.renderers.rect.instances().load_instance_to_gpu(
                            //     &context,
                            //     sumi::LoadToGPUSchedule::NextFrame,
                            //     id,
                            // );

                            // self.renderers.rect.render_instance(
                            //     &mut context,
                            //     &self.resources.plane,
                            //     id,
                            // );
                        }
                        RenderCommand::OuterBoxShadow {
                            boundary,
                            box_shadow,
                            border_radius,
                            shape,
                        } => {
                            // let boundary = snap_rect(*boundary);

                            // let pos = Vec2::new(
                            //     boundary.x,
                            //     context.view.size_unscaled.y - boundary.y - boundary.height,
                            // );
                            // let size = Vec2::new(boundary.width, boundary.height);
                            // let blur = 20.;
                            // let expand = 3.0 * blur;

                            // let mvp = context.view.screen_camera_matrix
                            //     * sumi::transforms_create_2d_model_matrix(&sumi::Transforms2D {
                            //         position: pos - Vec2::new(expand, expand),
                            //         scaling: size + Vec2::new(expand * 2.0, expand * 2.0),
                            //         rotation: 0.0,
                            //     });

                            // instances.insert(VectorInstance::new(mvp));
                        }
                        RenderCommand::InnerBoxShadow {
                            boundary,
                            box_shadow,
                            border_radius,
                            shape,
                        } => {
                            // TODO
                        }
                        RenderCommand::Text { .. } => {}
                        RenderCommand::PushClip { rect, shape, .. } => {}
                        RenderCommand::PopClip => {}
                        RenderCommand::Svg { .. } => {}
                        RenderCommand::BackdropFilter { boundary, shader } => {}
                    }
                }

                self.renderers.vector.bind(&context);
                instances.bind(1, &context);

                for range in instances.drain(&context) {
                    self.resources.plane.render_instances(&mut context, range);
                }

                self.resources.vector.flush(&context);
                self.resources.vector_instances.upload_all(&context);
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

// fn to_rect_fill(fill: &Option<Fill>) -> RectFill<'_> {
//     match fill {
//         Some(Fill::Color(c)) => RectFill::Solid(to_vec4(*c)),
//         None | Some(Fill::None) => RectFill::None,
//         Some(Fill::Gradient(Gradient::Linear(g))) => RectFill::Linear {
//             start: g.start,
//             end: g.end,
//             stops: &g.stops,
//         },
//         Some(Fill::Gradient(Gradient::Radial(g))) => RectFill::Radial {
//             center: g.center,
//             radius: g.radius,
//             stops: &g.stops,
//         },
//         Some(Fill::Gradient(Gradient::Sweep(_))) => RectFill::None,
//     }
// }

pub(crate) fn to_wgpu_color(format: wgpu::TextureFormat, color: ColorRgba) -> wgpu::Color {
    if format.is_srgb() {
        wgpu::Color {
            r: srgb_to_linear(color.r as f64),
            g: srgb_to_linear(color.g as f64),
            b: srgb_to_linear(color.b as f64),
            a: color.a as f64,
        }
    } else {
        wgpu::Color {
            r: color.r as f64,
            g: color.g as f64,
            b: color.b as f64,
            a: color.a as f64,
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

// fn make_clip_instance(
//     rect: Rect<f32>,
//     shape: &ClipShape,
//     view: &sumi::GraphicsView,
// ) -> RectInstance {
//     let boundary = snap_rect(rect);
//     let pos = Vec2::new(
//         boundary.x,
//         view.size_unscaled.y - boundary.y - boundary.height,
//     );
//     let size = Vec2::new(boundary.width, boundary.height);
//     let mvp_matrix = view.screen_camera_matrix
//         * sumi::transforms_create_2d_model_matrix(&sumi::Transforms2D {
//             position: pos,
//             scaling: size,
//             rotation: 0.0,
//         });

//     match shape {
//         ClipShape::Rect => RectInstance::new(
//             &mvp_matrix,
//             pos,
//             size,
//             RectFill::None,
//             Vec4::ZERO,
//             0.0,
//             Vec4::ZERO,
//             0.0,
//             Vec4::ZERO,
//             0.0,
//             Vec4::ZERO,
//             0.0,
//             [0.0, 0.0, 0.0, 0.0],
//         ),
//         ClipShape::RoundedRect { border_radius } => RectInstance::new(
//             &mvp_matrix,
//             pos,
//             size,
//             RectFill::None,
//             Vec4::ZERO,
//             0.0,
//             Vec4::ZERO,
//             0.0,
//             Vec4::ZERO,
//             0.0,
//             Vec4::ZERO,
//             0.0,
//             [
//                 border_radius.top_left,
//                 border_radius.top_right,
//                 border_radius.bottom_right,
//                 border_radius.bottom_left,
//             ],
//         ),
//         ClipShape::Oval => {
//             RectInstance::new_oval(&mvp_matrix, pos, size, RectFill::None, Vec4::ZERO, 0.0)
//         }
//     }
// }
