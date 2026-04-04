use clew::prelude::*;
use clew::{self as ui};
use clew_derive::{ShortcutId, ShortcutScopeId};
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
            MainWindow {
                task_name: ui::TextData::from("Test"),
            },
            WindowDescriptor {
                title: "Todo List".to_string(),
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

pub struct MainWindow {
    task_name: ui::TextData,
}

#[derive(ShortcutId)]
pub enum TestShortcuts {
    Bind1,
    Bind2,
    Chord1,
}

#[derive(ShortcutScopeId)]
pub enum TestScopes {
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
}

impl Window<TodoApplication, ()> for MainWindow {
    fn build(&mut self, _: &mut TodoApplication, ctx: &mut ui::BuildContext) {
        ui::zstack()
            .fill_max_size()
            .align_x(ui::AlignX::Center)
            .align_y(ui::AlignY::Center)
            .build(ctx, |ctx| {
                ui::gesture_detector()
                    .clickable(true)
                    .focusable(true)
                    .build(ctx, |ctx| {
                        let response = ctx.of::<ui::GestureDetectorResponse>().unwrap();

                        ui::editable_text(&mut self.task_name)
                            .gesture_response(response.clone())
                            // .text_vertical_align(ui::AlignY::Center)
                            .padding(ui::EdgeInsets::symmetric(8., 4.))
                            .truncate_lines(false)
                            .multi_line(true)
                            .virtualize(false)
                            .background(
                                ui::decoration()
                                    .border_radius(ui::BorderRadius::all(3.))
                                    .color(ui::ColorRgba::from_hex(0xFF000000))
                                    .border(ui::Border::all(ui::BorderSide::new(
                                        1.,
                                        if response.is_focused() {
                                            ui::ColorRgba::from_hex(0xFF357CCE)
                                        } else {
                                            ui::ColorRgba::from_hex(0xFF414141)
                                        },
                                    )))
                                    .build(ctx),
                            )
                            // .width(200.)
                            // .height(200.)
                            .min_width(50.)
                            .min_height(20.)
                            .max_height(12. * 4.)
                            .max_width(200.)
                            .build(ctx);
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
    Application::run_application(TodoApplication)?;

    Ok(())
}
