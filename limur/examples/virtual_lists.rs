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
                title: "Virtual Lists".to_string(),
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
        ui::vstack()
            .fill_max_size()
            .padding(ui::EdgeInsets::symmetric(0., 8.))
            .build(ctx, |ctx| {
                ui::zstack()
                    .fill_max_size()
                    .margin(ui::EdgeInsets::symmetric(16., 8.))
                    .build(ctx, |ctx| {
                        let response = ui::virtual_list()
                            .fill_max_size()
                            .background(
                                ui::decoration()
                                    .color(ui::ColorRgba::from_hex(0xFFFF0000).with_opacity(0.2))
                                    .border_radius(ui::BorderRadius::all(16.))
                                    .build(ctx),
                            )
                            .items_count(10_000_000_000)
                            .item_size(32.)
                            .build(ctx, |ctx, index| {
                                ui::text(&format!("Item {index}"))
                                    .padding(ui::EdgeInsets::symmetric(16., 0.))
                                    .height(32.)
                                    .fill_max_width()
                                    .build(ctx);
                            });

                        if response.overflow_y {
                            ctx.provide(response.clone(), |ctx| {
                                limur_widgets::vertical_scroll_bar().build(ctx);
                            });
                        }
                    });

                ui::zstack()
                    .fill_max_size()
                    .margin(ui::EdgeInsets::symmetric(16., 8.))
                    .build(ctx, |ctx| {
                        let response = ui::virtual_list()
                            .fill_max_size()
                            .scroll_direction(ui::Axis::Horizontal)
                            .background(
                                ui::decoration()
                                    .color(ui::ColorRgba::from_hex(0xFFFF0000).with_opacity(0.2))
                                    .border_radius(ui::BorderRadius::all(16.))
                                    .build(ctx),
                            )
                            .items_count(10_000_000_000)
                            .item_size(150.)
                            .build(ctx, |ctx, index| {
                                ui::text(&format!("Item {index}"))
                                    .text_vertical_align(ui::AlignY::Center)
                                    .padding(ui::EdgeInsets::symmetric(16., 0.))
                                    .width(150.)
                                    .fill_max_height()
                                    .build(ctx);
                            });

                        if response.overflow_x {
                            ctx.provide(response.clone(), |ctx| {
                                limur_widgets::horizontal_scroll_bar().build(ctx);
                            });
                        }
                    });
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
