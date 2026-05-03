use std::sync::Arc;

use glam::{Vec2, Vec4};
use limur::{
    Border, BorderRadius, BorderSide, ClipShape, ColorRgb, ColorRgba, Gradient, PhysicalSize, Rect,
    ShaderParam, View,
    assets::Assets,
    profiler,
    render::{Fill, RenderCommand, RenderCompositionLayer, Renderer},
    text::{FontResources, TextsResources},
};
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
    text: sumi::TextsResources<'a>,
}

struct Renderers {
    colored_plane: sumi::ColoredPlaneRenderer,
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
            text: sumi::TextsResources::new(),
        };

        let mut fonts = sumi::FontResources::new();

        let mut renderers = Renderers {
            colored_plane: sumi::ColoredPlaneRenderer::new(
                &mut context,
                sumi::BumpInstances::new(128),
            ),
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
                            let transforms = sumi::Transforms2D {
                                position: Vec2::new(
                                    boundary.x,
                                    context.view.size_unscaled.y - boundary.y - boundary.height,
                                ),
                                scaling: Vec2::new(boundary.width, boundary.height),
                                rotation: 0.,
                            };

                            let model_matrix = sumi::transforms_create_2d_model_matrix(&transforms);
                            let mvp_matrix = context.view.screen_camera_matrix * model_matrix;

                            let instance_id = self.renderers.colored_plane.instances().insert(
                                sumi::ColoredPlaneInstance::new(
                                    &mvp_matrix,
                                    &Vec4::new(1.0, 1.0, 0.0, 0.8),
                                ),
                            );

                            self.renderers
                                .colored_plane
                                .instances()
                                .load_instance_to_gpu(
                                    &context,
                                    sumi::LoadToGPUSchedule::NextFrame,
                                    instance_id,
                                );

                            self.renderers.colored_plane.render_instance(
                                &mut context,
                                &self.resources.plane,
                                instance_id,
                            );
                        }
                        RenderCommand::Oval {
                            boundary,
                            fill,
                            border,
                            ..
                        } => {
                            //
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

                // self.process_compoisition_layer(
                //     view,
                //     fonts,
                //     text,
                //     assets,
                //     &context,
                //     &layer.commands,
                // );
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
