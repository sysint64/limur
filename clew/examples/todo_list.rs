use clew::prelude::*;
use clew::{self as ui, SHORTCUTS_ROOT_SCOPE_ID};
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
        shortcuts_registry
            .scope(ui::ShortcutScopes::TextEditing)
            .add_repeat(
                ui::TextEditingShortcut::Delete,
                ui::KeyBinding::new(ui::keyboard::KeyCode::Delete),
            )
            .add_repeat(
                ui::TextEditingShortcut::Backspace,
                ui::KeyBinding::new(ui::keyboard::KeyCode::Backspace),
            )
            .add_repeat(
                ui::TextEditingShortcut::MoveNext,
                ui::KeyBinding::new(ui::keyboard::KeyCode::ArrowRight),
            )
            .add_repeat(
                ui::TextEditingShortcut::MovePrev,
                ui::KeyBinding::new(ui::keyboard::KeyCode::ArrowLeft),
            )
            .add_repeat(
                ui::TextEditingShortcut::MoveUp,
                ui::KeyBinding::new(ui::keyboard::KeyCode::ArrowUp),
            )
            .add_repeat(
                ui::TextEditingShortcut::MoveDown,
                ui::KeyBinding::new(ui::keyboard::KeyCode::ArrowDown),
            )
            .add_repeat(
                ui::TextEditingShortcut::NextLine,
                ui::KeyBinding::new(ui::keyboard::KeyCode::Enter),
            )
            .add_repeat(
                ui::TextEditingShortcut::MoveStart,
                ui::KeyBinding::new(ui::keyboard::KeyCode::Home),
            )
            .add(
                ui::TextEditingShortcut::MoveEnd,
                ui::KeyBinding::new(ui::keyboard::KeyCode::End),
            )
            .add_repeat(
                ui::TextEditingShortcut::BufferStart,
                ui::KeyBinding::new(ui::keyboard::KeyCode::Home).with_super(),
            )
            .add(
                ui::TextEditingShortcut::BufferEnd,
                ui::KeyBinding::new(ui::keyboard::KeyCode::End).with_super(),
            )
            .add_repeat(
                ui::TextEditingShortcut::PageUp,
                ui::KeyBinding::new(ui::keyboard::KeyCode::PageUp),
            )
            .add_repeat(
                ui::TextEditingShortcut::PageDown,
                ui::KeyBinding::new(ui::keyboard::KeyCode::PageDown),
            )
            .add(
                ui::TextEditingShortcut::SelectAll,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyA).with_super(),
            )
            .add_modifier(
                ui::TextInputModifier::Select,
                ui::keyboard::KeyModifiers::shift(),
            )
            .add_modifier(
                ui::TextInputModifier::Word,
                ui::keyboard::KeyModifiers::super_key(),
            )
            .add_modifier(
                ui::TextInputModifier::Paragraph,
                ui::keyboard::KeyModifiers::super_key(),
            );

        shortcuts_registry
            .scope(SHORTCUTS_ROOT_SCOPE_ID)
            .add(
                ui::CommonShortcut::Copy,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyC).with_super(),
            )
            .add(
                ui::CommonShortcut::Cut,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyX).with_super(),
            )
            .add_repeat(
                ui::CommonShortcut::Paste,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyV).with_super(),
            )
            .add_repeat(
                ui::CommonShortcut::Undo,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyZ).with_super(),
            )
            .add_repeat(
                ui::CommonShortcut::Redo,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyZ)
                    .with_super()
                    .with_shift(),
            );

        window_manager.spawn_window(
            MainWindow {
                task_name: ui::TextData::from("Test"),
            },
            WindowDescriptor {
                title: "Todo List".to_string(),
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
                            .multi_line(false)
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
                            .width(200.)
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
