use limur as ui;
use limur::prelude::*;
use limur_desktop::{
    app::{Application, ApplicationDelegate},
    window::Window,
    window_manager::{WindowDescriptor, WindowManager},
};
use limur_vello::VelloRenderer;
use pollster::FutureExt;

struct CounterApplication;

impl ApplicationDelegate<()> for CounterApplication {
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
                title: "Counter".to_string(),
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
    counter: i32,
}

impl Window<CounterApplication, ()> for MainWindow {
    fn build(&mut self, _: &mut CounterApplication, ctx: &mut ui::BuildContext) {
        ui::zstack()
            .fill_max_size()
            .align_x(ui::AlignX::Center)
            .align_y(ui::AlignY::Center)
            .build(ctx, |ctx| {
                ui::vstack()
                    .spacing(12.)
                    .cross_axis_alignment(ui::CrossAxisAlignment::Center)
                    .build(ctx, |ctx| {
                        ui::text(&format!("Counter: {}", self.counter)).build(ctx);

                        ui::hstack().build(ctx, |ctx| {
                            if limur_widgets::button("+").build(ctx).clicked() {
                                self.counter += 1;
                            }

                            if limur_widgets::button("-").build(ctx).clicked() {
                                self.counter -= 1;
                            }
                        });
                    });
            });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .init();

    log::info!("Starting app");
    Application::run_application(CounterApplication)?;

    Ok(())
}
