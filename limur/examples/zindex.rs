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
                // Nested clips example (z: 20)
                ui::zstack()
                    .width(100.)
                    .height(100.)
                    .offset(60., 80.)
                    .zindex(20)
                    .clip(ui::Clip::Rect)
                    .build(ctx, |ctx| {
                        // Nested oval clip inside rect clip
                        ui::zstack()
                            .width(80.)
                            .height(80.)
                            .offset(30., 30.)
                            .zindex(1)
                            .clip(ui::Clip::Oval)
                            .build(ctx, |ctx| {
                                ui::decorated_box()
                                    .color(ui::ColorRgba::from_hex(0xFF66DDDD))
                                    .fill_max_size()
                                    .build(ctx);

                                ui::decorated_box()
                                    .color(ui::ColorRgba::from_hex(0xFFFFFFFF))
                                    .width(20.)
                                    .height(20.)
                                    .build(ctx);
                            });

                        ui::decorated_box()
                            .color(ui::ColorRgba::from_hex(0xFFDD66DD))
                            .fill_max_size()
                            .build(ctx);
                    });

                // A clipped group that acts as a "window" (z: 10)
                // The clip contains multiple items that should stay together
                ui::zstack()
                    .width(150.)
                    .height(150.)
                    .offset(-80., -80.)
                    .zindex(10)
                    .align_x(ui::AlignX::Center)
                    .align_y(ui::AlignY::Center)
                    .clip(ui::Clip::Oval)
                    .build(ctx, |ctx| {
                        // These three boxes inside the clip should maintain
                        // their relative order (stable sort) since they share z: 0
                        ui::decorated_box()
                            .color(ui::ColorRgba::from_hex(0xFFFF0000))
                            .width(100.)
                            .height(100.)
                            .offset(-25., -25.)
                            .build(ctx);

                        ui::decorated_box()
                            .color(ui::ColorRgba::from_hex(0xFF00FF00))
                            .width(100.)
                            .height(100.)
                            .offset(0., 0.)
                            .build(ctx);

                        ui::decorated_box()
                            .color(ui::ColorRgba::from_hex(0xFF0000FF))
                            .width(100.)
                            .height(100.)
                            .offset(25., 25.)
                            .build(ctx);
                    });

                // A card group without clipping (z: 5)
                // Content can overflow but moves as a unit
                ui::zstack()
                    .width(120.)
                    .height(80.)
                    .offset(80., 0.)
                    .zindex(5)
                    .build(ctx, |ctx| {
                        // Card background
                        ui::decorated_box()
                            .color(ui::ColorRgba::from_hex(0xFF444488))
                            .fill_max_size()
                            .build(ctx);

                        // Overflowing "badge" - stays with the card group
                        ui::decorated_box()
                            .color(ui::ColorRgba::from_hex(0xFFFF8800))
                            .width(30.)
                            .height(35.)
                            .offset(55., -15.) // Overflows top-right
                            .zindex(1)
                            .build(ctx);

                        // Card content - stable sort keeps these in order
                        ui::decorated_box()
                            .color(ui::ColorRgba::from_hex(0xFF6666AA))
                            .width(100.)
                            .height(35.)
                            .offset(0., -20.)
                            .build(ctx);

                        ui::decorated_box()
                            .color(ui::ColorRgba::from_hex(0xFF8888CC))
                            .width(100.)
                            .height(35.)
                            .offset(0., 5.)
                            .build(ctx);

                        ui::decorated_box()
                            .color(ui::ColorRgba::from_hex(0xFFAAAAEE))
                            .width(100.)
                            .height(35.)
                            .offset(0., 30.)
                            .build(ctx);
                    });

                // This single box (z: 7) should appear between the two groups
                ui::decorated_box()
                    .color(ui::ColorRgba::from_hex(0xFFFFFF00))
                    .width(60.)
                    .height(200.)
                    .offset(0., 0.)
                    .zindex(7)
                    .build(ctx);

                // Background layer (z: -100)
                ui::decorated_box()
                    .color(ui::ColorRgba::from_hex(0xFF222222))
                    .width(400.)
                    .height(400.)
                    .zindex(-100)
                    .build(ctx);
            });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracy_client::Client::start();

    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .init();

    log::info!("Starting app");
    Application::run_application(DemoApplication)?;

    Ok(())
}
