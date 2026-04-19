use limur::prelude::*;
use limur::{self as ui, scroll_area};
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
                title: "Custom Scrool View".to_string(),
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
        let response = scroll_area()
            .padding(ui::EdgeInsets::all(16.))
            .fill_max_size()
            .build(ctx, |ctx| {
                ui::vstack().fill_max_width().build(ctx, |ctx| {
                    ui::text("Header").build(ctx);
                    ui::text("List 1").build(ctx);
                    ui::text("Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vivamus id convallis metus. Etiam mollis bibendum sapien, vel consequat ante eleifend quis. Nam a lectus in leo imperdiet dignissim sit amet non elit. Donec euismod et tortor sit amet fringilla. Aliquam tincidunt neque velit, ut placerat magna tempus eu. Quisque elementum ex quis egestas tristique. Pellentesque at mauris nisi. Interdum et malesuada fames ac ante ipsum primis in faucibus.

Maecenas ut porttitor lacus. Cras ultricies quam leo, sit amet pretium odio blandit sit amet. Quisque tincidunt consectetur est, a pellentesque lacus dignissim ut. Ut ac lectus ante. Morbi pretium ornare nunc eget fermentum. Nullam malesuada magna non tortor hendrerit, nec ultricies turpis imperdiet. Nam porta sapien ac lectus imperdiet, et placerat turpis sagittis. Etiam non elit suscipit ex dapibus blandit vel eget elit. Suspendisse in velit enim. In hac habitasse platea dictumst. Cras eleifend porttitor nisl, ut vulputate augue sagittis nec. Interdum et malesuada fames ac ante ipsum primis in faucibus. Integer bibendum ultricies urna quis mollis. Mauris quis pulvinar nulla.")
.fill_max_width()
.build(ctx);
                    ui::list_view()
                        .fill_max_width()
                        .background(
                            ui::decoration()
                                .color(ui::ColorRgba::from_hex(0xFFFF0000).with_opacity(0.2))
                                .border_radius(ui::BorderRadius::all(16.))
                                .build(ctx),
                        )
                        .items_count(10_000_000_000)
                        .item_size(64.)
                        .build(ctx, |ctx, index| {
                            ui::text(&format!("List 1: Item {index}"))
                                .background(
                                    ui::decoration()
                                        .color(
                                            ui::ColorRgba::from_hex(0xFF00FF00).with_opacity(0.2),
                                        )
                                        .border_radius(ui::BorderRadius::all(4.))
                                        .build(ctx),
                                )
                                .padding(ui::EdgeInsets::symmetric(16., 0.))
                                .margin(ui::EdgeInsets::all(4.))
                                .text_vertical_align(ui::AlignY::Center)
                                .height(56.)
                                .fill_max_width()
                                .build(ctx);
                        });
                    ui::text("List 2").build(ctx);
                    ui::list_view()
                        .fill_max_width()
                        .items_count(10)
                        .item_size(32.)
                        .build(ctx, |ctx, index| {
                            ui::text(&format!("List 2: Item {index}"))
                                .padding(ui::EdgeInsets::symmetric(16., 0.))
                                .text_vertical_align(ui::AlignY::Bottom)
                                .height(32.)
                                .fill_max_width()
                                .build(ctx);
                        });
                    ui::text("Footer").build(ctx);
                });
            });

        if response.overflow_y {
            ctx.provide(response.clone(), |ctx| {
                limur_widgets::vertical_scroll_bar().build(ctx);
            });
        }

        ui::profiler_overlay(ctx);
    }
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
