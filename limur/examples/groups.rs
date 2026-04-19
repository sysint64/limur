use limur as ui;
use limur::prelude::*;
use limur_desktop::{
    app::{Application, ApplicationDelegate},
    window::Window,
    window_manager::{WindowDescriptor, WindowManager},
};
use limur_vello::VelloRenderer;
use pollster::FutureExt;

struct DemoApplication;

impl ApplicationDelegate<()> for DemoApplication {
    fn on_start(
        &mut self,
        window_manager: &mut WindowManager<Self, ()>,
        _: &mut ui::ShortcutsRegistry,
    ) where
        Self: std::marker::Sized,
    {
        window_manager.spawn_window(
            MainWindow,
            WindowDescriptor {
                title: "Demo".to_string(),
                name: Some("limur-example".to_string()),
                width: 800,
                height: 600,
                resizable: true,
                fill_color: ui::ColorRgb::from_hex(0x121212),
            },
        );
    }

    fn create_renderer(window: std::sync::Arc<winit::window::Window>) -> Box<dyn ui::Renderer> {
        Box::new(
            VelloRenderer::new(
                window.clone(),
                window.inner_size().width,
                window.inner_size().height,
            )
            .block_on(),
        )
    }
}

pub struct MainWindow;

impl Window<DemoApplication, ()> for MainWindow {
    fn build(&mut self, _: &mut DemoApplication, ctx: &mut ui::BuildContext) {
        ui::zstack()
            .fill_max_size()
            .align_x(ui::AlignX::Center)
            .align_y(ui::AlignY::Center)
            .build(ctx, |ctx| {
                ui::vstack()
                    .cross_axis_alignment(ui::CrossAxisAlignment::Stretch)
                    .build(ctx, |ctx| {
                        group2(ctx, &["Frist", "Second", "Third", "Last"]);
                        group2(ctx, &["Frist", "Second", "Last"]);
                        group2(ctx, &["Frist", "Last"]);
                    });
            });
    }
}

fn group2(ctx: &mut ui::BuildContext, texts: &[&str]) {
    ui::hstack()
        .padding(ui::EdgeInsets::all(16.))
        .background(
            ui::decoration()
                .color(ui::ColorRgba::from_hex(0xFF880088))
                .border_radius(if ctx.position().is_first() {
                    ui::BorderRadius::top(8.)
                } else if ctx.position().is_last() {
                    ui::BorderRadius::bottom(8.)
                } else {
                    ui::BorderRadius::ZERO
                })
                .build(ctx),
        )
        .build(ctx, |ctx| {
            for text in texts {
                grouped(ctx, text);
            }
        });
}

fn grouped(ctx: &mut ui::BuildContext, text: &str) {
    ui::gesture_detector().clickable(true).build(ctx, |ctx| {
        let response = ctx.of::<ui::GestureDetectorResponse>().unwrap().clone();

        ui::text(text)
            .background(
                ui::decoration()
                    .border_radius(if ctx.position().is_first() {
                        ui::BorderRadius::left(8.)
                    } else if ctx.position().is_last() {
                        ui::BorderRadius::right(8.)
                    } else {
                        ui::BorderRadius::ZERO
                    })
                    .color(if response.is_hot() {
                        ui::ColorRgba::from_hex(0xFFAAAA00)
                    } else {
                        ui::ColorRgba::from_hex(0xFF888800)
                    })
                    .build(ctx),
            )
            .text_align(ui::TextAlign::Center)
            .text_vertical_align(ui::AlignY::Center)
            .padding(ui::EdgeInsets::symmetric(12., 8.))
            .build(ctx);
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .init();

    log::info!("Starting app");
    Application::run_application(DemoApplication)?;

    Ok(())
}
