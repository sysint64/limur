use clew as ui;
use clew::prelude::*;
use clew_desktop::{
    app::{Application, ApplicationDelegate},
    window::Window,
    window_manager::{WindowDescriptor, WindowManager},
};
use clew_vello::VelloRenderer;
use pollster::FutureExt;

struct TodoApplication;

impl ApplicationDelegate<()> for TodoApplication {
    fn on_start(
        &mut self,
        window_manager: &mut WindowManager<Self, ()>,
        shortcuts_registry: &mut ui::ShortcutsRegistry,
    ) where
        Self: std::marker::Sized,
    {
        clew_widgets::init_shortcuts(shortcuts_registry);

        window_manager.spawn_window(
            MainWindow,
            WindowDescriptor {
                title: "100k Non Virtualized Buttons".to_string(),
                name: Some("clew-example".to_string()),
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

impl Window<TodoApplication, ()> for MainWindow {
    fn build(&mut self, _: &mut TodoApplication, ctx: &mut ui::BuildContext) {
        let response = ui::scroll_area()
            .scroll_direction(ui::ScrollDirection::Both)
            .fill_max_size()
            .padding(ui::EdgeInsets::all(16.))
            .build(ctx, |ctx| {
                ui::vstack().build(ctx, |ctx| {
                    ui::gap().height(32.).build(ctx);

                    ui::text("100k Non Virtualized Buttons")
                        .font_size(24.)
                        .build(ctx);

                    ui::text("1000 Buttons per layer").build(ctx);

                    for li in 0..2 {
                        ui::hstack().fill_max_size().build(ctx, |ctx| {
                            for lj in 0..2 {
                                ui::layer()
                                    .margin(ui::EdgeInsets::all(4.))
                                    .padding(ui::EdgeInsets::all(8.))
                                    .background(
                                        ui::decoration()
                                            .color(ui::ColorRgba::from_hex(0x00000000))
                                            .border(ui::Border::all(ui::BorderSide::new(
                                                2.,
                                                ui::ColorRgba::from_hex(0xFF00FF00),
                                            )))
                                            .build(ctx),
                                    )
                                    .id(li * 2 + lj)
                                    .build(ctx, |ctx| layer_body(ctx, li * 2 + lj));
                            }
                        });
                    }
                });
            });

        ui::text(&format!("Cycle Time: {:?}", ctx.cycle_time()))
            .padding(ui::EdgeInsets::all(16.))
            .color(ui::ColorRgba::from_hex(0xFFFF0000))
            .background(
                ui::decoration()
                    .color(ui::ColorRgba::from_hex(0xFF000000).with_opacity(0.8))
                    .build(ctx),
            )
            .build(ctx);

        if response.overflow_x {
            ctx.provide(response.clone(), |ctx| {
                clew_widgets::horizontal_scroll_bar().build(ctx);
            });
        }

        if response.overflow_y {
            ctx.provide(response.clone(), |ctx| {
                clew_widgets::vertical_scroll_bar().build(ctx);
            });
        }
    }
}

fn layer_body(ctx: &mut ui::BuildContext, layer_id: u32) {
    // 1024 Buttons
    ui::vstack().build(ctx, |ctx| {
        for i in 0..32 {
            ui::hstack().build(ctx, |ctx| {
                for j in 0..32 {
                    let title = format!("Button {layer_id}: {i}_{j}");

                    if clew_widgets::button(&title).id(&title).build(ctx).clicked() {
                        // if ui::button_id("Button", (i, j)).show(ctx) {
                        println!("Button {layer_id}: {i}_{j} Clicked");
                    }
                }
            });
        }
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .init();

    log::info!("Starting app");
    Application::run_application(TodoApplication)?;

    Ok(())
}
