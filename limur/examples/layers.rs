use limur as ui;
use limur::prelude::*;
use limur_desktop::{
    app::{Application, ApplicationDelegate},
    window::Window,
    window_manager::{WindowDescriptor, WindowManager},
};
use limur_vello::VelloRenderer;
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
        limur_widgets::init_shortcuts(shortcuts_registry);

        window_manager.spawn_window(
            MainWindow {
                height_fraction: 0.25,
                width_fraction: 0.25,
                center: false,
            },
            WindowDescriptor {
                title: "100k Non Virtualized Buttons".to_string(),
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

pub struct MainWindow {
    width_fraction: f64,
    height_fraction: f64,
    center: bool,
}

impl Window<TodoApplication, ()> for MainWindow {
    fn build(&mut self, _: &mut TodoApplication, ctx: &mut ui::BuildContext) {
        ui::vstack().fill_max_size().build(ctx, |ctx| {
            ui::gap().height(164.).build(ctx);

            ui::hstack().spacing(4.).build(ctx, |ctx| {
                if limur_widgets::button("rotate width").build(ctx).clicked() {
                    self.width_fraction = match self.width_fraction {
                        0.25 => 0.5,
                        0.5 => 0.75,
                        0.75 => 1.,
                        1. => 0.76,
                        0.76 => 0.51,
                        0.51 => 0.25,
                        _ => 0.25,
                    };
                }

                if limur_widgets::button("rotate height").build(ctx).clicked() {
                    self.height_fraction = match self.height_fraction {
                        0.25 => 0.5,
                        0.5 => 0.75,
                        0.75 => 1.,
                        1. => 0.76,
                        0.76 => 0.51,
                        0.51 => 0.25,
                        _ => 0.25,
                    };
                }

                if limur_widgets::button("toggle center").build(ctx).clicked() {
                    self.center = !self.center;
                }
            });

            ui::zstack()
                .height((ctx.view().size().y - 164. - 42.) * self.height_fraction)
                .fill_max_width()
                .build(ctx, |ctx| {
                    ui::vstack()
                        .fill_max_height()
                        .width(ctx.view().size().x * self.width_fraction)
                        .cross_axis_alignment(if self.center {
                            ui::CrossAxisAlignment::Center
                        } else {
                            ui::CrossAxisAlignment::Start
                        })
                        .build(ctx, |ctx| {
                            for li in 0..2 {
                                ui::hstack().fill_max_size().build(ctx, |ctx| {
                                    for lj in 0..2 {
                                        ui::layer()
                                            .id(li * 2 + lj)
                                            .build(ctx, layer_body);
                                    }
                                });
                                // ui::layer()
                                //     // ui::zstack()
                                //     //     .clip(ui::Clip::Rect)
                                //     // .id(99999)
                                //     .width(64.)
                                //     .height(64.)
                                //     .build(ctx, layer_body);
                                // ui::zstack().height(32.).width(32.).build(ctx, |ctx| {
                                //     layer_body(ctx);
                                // });
                            }
                        });
                });
        });

        ui::profiler_overlay(ctx);
    }
}

fn layer_body(ctx: &mut ui::BuildContext) {
    ui::vstack()
        .background(
            ui::decoration()
                .color(ui::ColorRgba::from_hex(0x00000000))
                .border(ui::Border::all(ui::BorderSide::new(
                    2.,
                    ui::ColorRgba::from_hex(0xFF00FF00),
                )))
                .build(ctx),
        )
        .clip(ui::Clip::Oval)
        // .fill_max_size()
        .width(64.)
        .height(64.)
        .spacing(4.)
        .build(ctx, |ctx| {
            for i in 0..2 {
                ui::hstack().fill_max_size().spacing(4.).build(ctx, |ctx| {
                    for j in 0..2 {
                        ui::decorated_box()
                            .color(ui::ColorRgba::from_hex(0xFFCCCCCC))
                            .id(i * 2 + j)
                            .fill_max_size()
                            .build(ctx);
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
