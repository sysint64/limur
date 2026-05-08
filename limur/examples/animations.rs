use std::time::Duration;

use limur as ui;
use limur::prelude::*;
use limur_desktop::{
    app::{Application, ApplicationDelegate},
    window::Window,
    window_manager::{WindowDescriptor, WindowManager},
};
use limur_vello::VelloRenderer;
use limur_wgpu::WgpuRenderer;
use pollster::FutureExt;

struct AnimationsApplication;

impl ApplicationDelegate<()> for AnimationsApplication {
    fn on_start(
        &mut self,
        window_manager: &mut WindowManager<Self, ()>,
        _: &mut ui::ShortcutsRegistry,
    ) where
        Self: std::marker::Sized,
    {
        window_manager.spawn_window(
            MainWindow::new(),
            WindowDescriptor {
                title: "Animations".to_string(),
                name: Some("limur-example".to_string()),
                width: 800,
                height: 600,
                resizable: true,
                fill_color: Some(ui::ColorRgba::from_hex(0xFF121212)),
            },
        );
    }

    fn create_renderer(window: std::sync::Arc<winit::window::Window>) -> Box<dyn ui::Renderer> {
        Box::new(WgpuRenderer::new(window.clone()).block_on())
        // Box::new(
        //     VelloRenderer::new(
        //         window.clone(),
        //         window.inner_size().width,
        //         window.inner_size().height,
        //     )
        //     .block_on(),
        // )
    }
}

struct MainWindow {
    offset_y: ui::Tween<f64>,
    mx: ui::Damp<f64>,
    my: ui::Damp<f64>,
    keyframes: ui::Keyframes<f64>,
    color1: ui::Keyframes<ui::ColorRgba>,
    color2: ui::Keyframes<ui::ColorRgba>,
    gradient_angle: ui::Tween<f32>,
    circle_opacity: ui::Damp<f32>,
}

impl MainWindow {
    fn new() -> Self {
        let mut gradient_angle = ui::Tween::new(0.0)
            .duration(Duration::from_secs(10))
            .curve(ui::curves::linear)
            .repeat(ui::Repeat::Loop);

        gradient_angle.tween_to(std::f32::consts::TAU);

        // First color in gradient - starts at red, offset by 0
        let mut color1 = ui::Keyframes::new(ui::ColorRgba::from_hex(0xFFEF4444))
            .default_curve(ui::curves::ease_in_out_sine)
            .repeat(ui::Repeat::Loop)
            .tween(
                Duration::from_millis(500),
                ui::ColorRgba::from_hex(0xFFF97316),
            )
            .tween(
                Duration::from_millis(500),
                ui::ColorRgba::from_hex(0xFFEAB308),
            )
            .tween(
                Duration::from_millis(500),
                ui::ColorRgba::from_hex(0xFF22C55E),
            )
            .tween(
                Duration::from_millis(500),
                ui::ColorRgba::from_hex(0xFF06B6D4),
            )
            .tween(
                Duration::from_millis(500),
                ui::ColorRgba::from_hex(0xFF3B82F6),
            )
            .tween(
                Duration::from_millis(500),
                ui::ColorRgba::from_hex(0xFFA855F7),
            )
            .tween(
                Duration::from_millis(500),
                ui::ColorRgba::from_hex(0xFFEC4899),
            )
            .tween(
                Duration::from_millis(500),
                ui::ColorRgba::from_hex(0xFFEF4444),
            );

        // Second color - starts at cyan (offset in the rainbow)
        let mut color2 = ui::Keyframes::new(ui::ColorRgba::from_hex(0xFF06B6D4))
            .default_curve(ui::curves::ease_in_out_sine)
            .repeat(ui::Repeat::Loop)
            .tween(
                Duration::from_millis(300),
                ui::ColorRgba::from_hex(0xFF3B82F6),
            )
            .tween(
                Duration::from_millis(300),
                ui::ColorRgba::from_hex(0xFFA855F7),
            )
            .tween(
                Duration::from_millis(300),
                ui::ColorRgba::from_hex(0xFFEC4899),
            )
            .tween(
                Duration::from_millis(300),
                ui::ColorRgba::from_hex(0xFFEF4444),
            )
            .tween(
                Duration::from_millis(300),
                ui::ColorRgba::from_hex(0xFFF97316),
            )
            .tween(
                Duration::from_millis(300),
                ui::ColorRgba::from_hex(0xFFEAB308),
            )
            .tween(
                Duration::from_millis(300),
                ui::ColorRgba::from_hex(0xFF22C55E),
            )
            .tween(
                Duration::from_millis(300),
                ui::ColorRgba::from_hex(0xFF06B6D4),
            );

        color1.play();
        color2.play();

        Self {
            offset_y: ui::Tween::new(0.)
                .duration(Duration::from_secs(1))
                .curve(ui::curves::ease_out_elastic),
            mx: ui::Damp::new(0.).speed(10.),
            my: ui::Damp::new(0.).speed(10.),
            keyframes: ui::Keyframes::new(0.0)
                .default_curve(ui::curves::ease_in_out_quad)
                .repeat(ui::Repeat::Once),
            color1,
            color2,
            gradient_angle,
            circle_opacity: ui::Damp::new(1.).speed(8.),
        }
    }

    fn configure_keyframes_once(&mut self) {
        self.keyframes = ui::Keyframes::new(0.)
            .default_curve(ui::curves::ease_in_out_quad)
            .repeat(ui::Repeat::Once)
            .tween(Duration::from_millis(220), -120.0)
            .hold(Duration::from_millis(120), -120.0)
            .tween_with_curve(
                Duration::from_millis(520),
                80.0,
                ui::curves::ease_out_bounce,
            )
            .tween(Duration::from_millis(260), 0.0);

        self.keyframes.play();
    }

    fn configure_keyframes_loop(&mut self) {
        self.keyframes = ui::Keyframes::new(0.)
            .default_curve(ui::curves::smooth_step)
            .repeat(ui::Repeat::Loop)
            .tween(Duration::from_millis(350), -60.0)
            .tween(Duration::from_millis(350), 60.0)
            .tween(Duration::from_millis(350), 0.0);

        self.keyframes.play();
    }

    fn configure_keyframes_pingpong_6(&mut self) {
        self.keyframes = ui::Keyframes::new(0.)
            .default_curve(ui::curves::ease_in_out_sine)
            .repeat(ui::Repeat::PingPongNCycles(6))
            .tween(Duration::from_millis(300), -90.0)
            .tween(Duration::from_millis(300), 90.0)
            .tween(Duration::from_millis(300), 0.0);

        self.keyframes.play();
    }
}

impl Window<AnimationsApplication, ()> for MainWindow {
    fn build(&mut self, _: &mut AnimationsApplication, ctx: &mut ui::BuildContext) {
        let mouse_pos = ui::Vec2::new(
            ctx.input().mouse_x / ctx.view().scale_factor,
            ctx.input().mouse_y / ctx.view().scale_factor,
        );

        self.mx.approach(mouse_pos.x);
        self.my.approach(mouse_pos.y);

        let rect = ui::Rect::from_pos_size(
            ui::Vec2::new(self.mx.value() - 24., self.my.value() - 24.),
            ui::Vec2::new(48., 48.),
        );

        if ui::point_with_rect_hit_test(mouse_pos, rect) {
            self.circle_opacity.approach(0.5);
        } else {
            self.circle_opacity.approach(1.);
        }

        ui::zstack().fill_max_size().build(ctx, |ctx| {
            ui::zstack()
                .fill_max_size()
                .align_x(ui::AlignX::Center)
                .align_y(ui::AlignY::Center)
                .offset_y(self.offset_y.resolve(ctx))
                .build(ctx, |ctx| {
                    ui::vstack()
                        .spacing(12.)
                        .cross_axis_alignment(ui::CrossAxisAlignment::Center)
                        .build(ctx, |ctx| {
                            ui::text(&format!("Tween Offset: {}", self.offset_y.value()))
                                .build(ctx);
                            ui::text(&format!("Tween Status: {:?}", self.offset_y.status()))
                                .build(ctx);

                            ui::text(&format!("Keyframes Offset: {}", self.keyframes.value()))
                                .build(ctx);
                            ui::text(&format!("Keyframes Status: {:?}", self.keyframes.status()))
                                .build(ctx);

                            ui::text(&format!("MX Status: {:?}", self.mx.status())).build(ctx);
                            ui::text(&format!("MY Status: {:?}", self.my.status())).build(ctx);

                            if limur_widgets::button("Move Up (Tween)")
                                .build(ctx)
                                .clicked()
                            {
                                self.offset_y.tween_to(-100.);
                            }

                            limur_widgets::button("Move Down (Tween)");

                            if limur_widgets::button("Move Down (Tween)")
                                .build(ctx)
                                .clicked()
                            {
                                self.offset_y.tween_to(100.);
                            }

                            if limur_widgets::button("Go Home (Tween)")
                                .build(ctx)
                                .clicked()
                            {
                                self.offset_y.tween_to(0.);
                            }

                            if limur_widgets::button("Play Keyframes (Once)")
                                .build(ctx)
                                .clicked()
                            {
                                self.configure_keyframes_once();
                            }

                            if limur_widgets::button("Loop Keyframes").build(ctx).clicked() {
                                self.configure_keyframes_loop();
                            }

                            if limur_widgets::button("PingPong Keyframes x6")
                                .build(ctx)
                                .clicked()
                            {
                                self.configure_keyframes_pingpong_6();
                            }

                            if limur_widgets::button("Stop Keyframes (Set 0)")
                                .build(ctx)
                                .clicked()
                            {
                                self.keyframes.set(0.0);
                            }
                        });
                });

            // Keyframes position box
            ui::decorated_box()
                .shape(ui::BoxShape::Rect)
                .border_radius(ui::BorderRadius::all(24.))
                .add_linear_gradient(ui::LinearGradient::angled(
                    self.gradient_angle.resolve(ctx),
                    (self.color1.resolve(ctx), self.color2.resolve(ctx)),
                ))
                .width(200.)
                .height(200.)
                .offset(40., 200. + self.keyframes.resolve(ctx))
                .build(ctx);

            // Mouse follower
            ui::decorated_box()
                .shape(ui::BoxShape::Oval)
                .color(
                    ui::ColorRgba::from_hex(0xFFFF0000)
                        .with_opacity(self.circle_opacity.resolve(ctx)),
                )
                .width(48.)
                .height(48.)
                .offset(self.mx.resolve(ctx) - 24., self.my.resolve(ctx) - 24.)
                .ignore_pointer(true)
                .build(ctx);
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .init();

    log::info!("Starting app");
    Application::run_application(AnimationsApplication)?;

    Ok(())
}
