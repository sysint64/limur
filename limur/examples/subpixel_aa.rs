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

struct ExampleApplication;

impl ApplicationDelegate<()> for ExampleApplication {
    fn on_start(
        &mut self,
        window_manager: &mut WindowManager<Self, ()>,
        _: &mut ui::ShortcutsRegistry,
    ) where
        Self: std::marker::Sized,
    {
        window_manager.spawn_window(
            MainWindow { counter: 0 },
            WindowDescriptor {
                title: "Sub-pixel font rendering".to_string(),
                name: Some("limur-example".to_string()),
                width: 1024,
                height: 1024,
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

pub struct MainWindow {
    counter: i32,
}

impl Window<ExampleApplication, ()> for MainWindow {
    fn build(&mut self, _: &mut ExampleApplication, ctx: &mut ui::BuildContext) {
        ui::vstack()
            // .fill_max_size()
            .spacing(24.)
            .padding(ui::EdgeInsets::all(32.))
            .build(ctx, |ctx| {
                ui::gap().height(128.).build(ctx);

                section_label(ctx, "Test 1: Text on solid backgrounds");

                ui::hstack().spacing(16.).build(ctx, |ctx| {
                    text_on_solid(ctx, "Light on dark", 0x1a1a2e, 0xCDD6F4);
                    text_on_solid(ctx, "Dark on light", 0xEFF1F5, 0x1e1e2e);
                    text_on_solid(ctx, "White on blue", 0x1e66f5, 0xFFFFFF);
                });

                section_label(ctx, "Test 2: Text on gradient background");

                ui::text("This text spans across a gradient — watch for color fringing")
                    .font_size(18.)
                    .color(ui::ColorRgb::from_hex(0xFFFFFF))
                    .padding(ui::EdgeInsets::symmetric(24., 16.))
                    .background(
                        ui::decoration()
                            .add_linear_gradient(ui::LinearGradient::new(
                                (0.0, 0.5),
                                (1.0, 0.5),
                                vec![
                                    ui::ColorStop::new(
                                        0.0,
                                        ui::ColorRgb::from_hex(0x1e1e2e).with_alpha(1.0),
                                    ),
                                    ui::ColorStop::new(
                                        0.5,
                                        ui::ColorRgb::from_hex(0x45475a).with_alpha(1.0),
                                    ),
                                    ui::ColorStop::new(
                                        1.0,
                                        ui::ColorRgb::from_hex(0xCDD6F4).with_alpha(1.0),
                                    ),
                                ],
                            ))
                            .border_radius(ui::BorderRadius::all(8.))
                            .build(ctx),
                    )
                    .build(ctx);

                section_label(ctx, "Test 3: Overlapping text (zstack layering)");

                ui::zstack()
                    .padding(ui::EdgeInsets::symmetric(64., 32.))
                    .background(
                        ui::decoration()
                            .color(ui::ColorRgb::from_hex(0x313244).with_alpha(1.0))
                            .border_radius(ui::BorderRadius::all(8.))
                            .build(ctx),
                    )
                    .padding(ui::EdgeInsets::all(16.))
                    .build(ctx, |ctx| {
                        ui::text("BACKGROUND TEXT BACKGROUND TEXT BACKGROUND")
                            .font_size(24.)
                            .color(ui::ColorRgb::from_hex(0x585b70))
                            .build(ctx);

                        ui::text("Foreground overlapping text")
                            .font_size(18.)
                            .color(ui::ColorRgb::from_hex(0xF38BA8))
                            .build(ctx);
                    });

                section_label(ctx, "Test 4: Semi-transparent overlay on text");

                ui::zstack()
                    .padding(ui::EdgeInsets::symmetric(64., 32.))
                    .background(
                        ui::decoration()
                            .color(ui::ColorRgb::from_hex(0x181825).with_alpha(1.0))
                            .border_radius(ui::BorderRadius::all(8.))
                            .build(ctx),
                    )
                    .align_x(ui::AlignX::Center)
                    .align_y(ui::AlignY::Center)
                    .build(ctx, |ctx| {
                        ui::text("This text should be visible through the overlay above")
                            .font_size(16.)
                            .color(ui::ColorRgb::from_hex(0xCDD6F4))
                            .build(ctx);

                        ui::text("Overlay text")
                            .font_size(14.)
                            .color(ui::ColorRgb::from_hex(0xFFFFFF))
                            .text_align(ui::TextAlign::Center)
                            .text_vertical_align(ui::AlignY::Center)
                            .padding(ui::EdgeInsets::symmetric(24., 8.))
                            .background(
                                ui::decoration()
                                    .color(ui::ColorRgb::from_hex(0x1e66f5).with_alpha(0.5))
                                    .border_radius(ui::BorderRadius::all(4.))
                                    .build(ctx),
                            )
                            .build(ctx);
                    });

                section_label(
                    ctx,
                    "Test 5: Size comparison (subpixel AA matters most at small sizes)",
                );

                ui::vstack()
                    .spacing(4.)
                    .padding(ui::EdgeInsets::all(16.))
                    .background(
                        ui::decoration()
                            .color(ui::ColorRgb::from_hex(0x313244).with_alpha(1.0))
                            .border_radius(ui::BorderRadius::all(8.))
                            .build(ctx),
                    )
                    .build(ctx, |ctx| {
                        sized_text(
                            ctx,
                            10.,
                            "10px: The quick brown fox jumps (subpixel helps most here)",
                        );
                        sized_text(
                            ctx,
                            12.,
                            "12px: The quick brown fox jumps over the lazy dog",
                        );
                        sized_text(ctx, 14., "14px: The quick brown fox jumps over");
                        sized_text(ctx, 18., "18px: The quick brown fox");
                        sized_text(ctx, 24., "24px: Quick brown fox");
                    });

                section_label(
                    ctx,
                    "Test 6: Interactive — hover to change bg under subpixel text",
                );

                ui::hstack().spacing(8.).build(ctx, |ctx| {
                    for label in &["Hover me", "And me", "Me too", "Also me"] {
                        hover_button(ctx, label);
                    }
                });
            });

        ui::backdrop_filter(ui::ShaderId::FrostedGlass)
            .param(0, ui::ShaderParam::Float(10.))
            .param(
                1,
                ui::ShaderParam::Color(ui::ColorRgba::from_hex(0xFF0000FF).with_opacity(0.2)),
            )
            .offset(256., 256.)
            .width(300.)
            .height(200.)
            .build(ctx);

        ui::backdrop_filter(ui::ShaderId::FrostedGlass)
            .param(0, ui::ShaderParam::Float(10.))
            .param(
                1,
                ui::ShaderParam::Color(ui::ColorRgba::from_hex(0xFF00FF00).with_opacity(0.5)),
            )
            .offset(400., 300.)
            .width(300.)
            .height(200.)
            .clip(ui::Clip::Oval)
            .build(ctx);

        ui::profiler_overlay(ctx);
    }
}

fn section_label(ctx: &mut ui::BuildContext, label: &str) {
    ui::text(label)
        .font_size(14.)
        .color(ui::ColorRgba::from_hex(0x888888FF))
        .build(ctx);
}

fn text_on_solid(ctx: &mut ui::BuildContext, label: &str, bg: u32, fg: u32) {
    ui::text(label)
        .font_size(16.)
        .color(ui::ColorRgb::from_hex(fg))
        .text_align(ui::TextAlign::Center)
        .text_vertical_align(ui::AlignY::Center)
        .padding(ui::EdgeInsets::symmetric(32., 16.))
        .background(
            ui::decoration()
                .color(ui::ColorRgb::from_hex(bg))
                .border_radius(ui::BorderRadius::all(8.))
                .build(ctx),
        )
        .build(ctx);
}

fn sized_text(ctx: &mut ui::BuildContext, size: f32, label: &str) {
    ui::text(label)
        .font_size(size)
        .color(ui::ColorRgba::from_hex(0xCDD6F4FF))
        .build(ctx);
}

fn hover_button(ctx: &mut ui::BuildContext, label: &str) {
    ui::gesture_detector().clickable(true).build(ctx, |ctx| {
        let response = ctx.of::<ui::GestureDetectorResponse>().unwrap().clone();

        let bg_color = if response.is_hot() {
            0xFF475aFF
        } else {
            0xFF3244FF
        };

        ui::text(label)
            .font_size(16.)
            .color(ui::ColorRgba::from_hex(0xCDD6F4FF))
            .text_align(ui::TextAlign::Center)
            .text_vertical_align(ui::AlignY::Center)
            .padding(ui::EdgeInsets::symmetric(24., 12.))
            .background(
                ui::decoration()
                    .color(ui::ColorRgba::from_hex(bg_color))
                    .border_radius(ui::BorderRadius::vertical(8., 0.))
                    .add_shadow(ui::BoxShadow {
                        color: ui::ColorRgba::from_hex(0x1400FF00),
                        offset: ui::Vec2::new(0., 5.),
                        blur_radius: 0.,
                        spread_radius: 5.,
                        blur_style: ui::BoxShadowBlurStyle::Outer,
                    })
                    .build(ctx),
            )
            .build(ctx);
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .init();

    log::info!("Starting Subpixel AA Layer Test");
    Application::run_application(ExampleApplication)?;

    Ok(())
}
