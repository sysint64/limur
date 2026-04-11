use std::time::{Duration, Instant};

use clew::{self as ui, SHORTCUTS_ROOT_SCOPE_ID};
use clew::{Tween, prelude::*};
use clew_derive::{ShortcutId, ShortcutScopeId};
use clew_desktop::{
    app::{Application, ApplicationDelegate},
    window::Window,
    window_manager::{WindowDescriptor, WindowManager},
};
use clew_vello::VelloRenderer;
use pollster::FutureExt;

struct ShortcutsApplication;

impl ApplicationDelegate<()> for ShortcutsApplication {
    fn on_start(
        &mut self,
        window_manager: &mut WindowManager<Self, ()>,
        shortcuts_registry: &mut ui::ShortcutsRegistry,
    ) where
        Self: std::marker::Sized,
    {
        // Test 1: Child shadows parent
        shortcuts_registry
            .scope(TestScopes::S1)
            .add(
                TestShortcuts::S1Bind1,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyA),
            )
            .add(
                TestShortcuts::S1Bind2,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyG),
            )
            .add_sequence(
                TestShortcuts::S1Chord1,
                &[
                    ui::KeyBinding::new(ui::keyboard::KeyCode::KeyK),
                    ui::KeyBinding::new(ui::keyboard::KeyCode::KeyC),
                ],
            );

        shortcuts_registry.scope(TestScopes::S2).add(
            TestShortcuts::S2Bind1,
            ui::KeyBinding::new(ui::keyboard::KeyCode::KeyA),
        );
        shortcuts_registry.scope(TestScopes::S3).add(
            TestShortcuts::S3Bind1,
            ui::KeyBinding::new(ui::keyboard::KeyCode::KeyA),
        );

        // Test 2: Unique shortcut on non-leaf parent (S4)
        shortcuts_registry.scope(TestScopes::S4).add(
            TestShortcuts::S4Bind1,
            ui::KeyBinding::new(ui::keyboard::KeyCode::KeyB),
        );
        // S5 is non-leaf, no shortcut registered
        shortcuts_registry
            .scope(TestScopes::S6)
            .add(
                TestShortcuts::S6Bind1,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyC),
            )
            .add(
                TestShortcuts::S6Bind2,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyE),
            );

        shortcuts_registry.scope(TestScopes::S7).add(
            TestShortcuts::S7Bind1,
            ui::KeyBinding::new(ui::keyboard::KeyCode::KeyD),
        );

        // Test 3: Multi-level fallthrough
        shortcuts_registry.scope(TestScopes::S5).add(
            TestShortcuts::S5Bind1,
            ui::KeyBinding::new(ui::keyboard::KeyCode::KeyE),
        );

        // Test 4: Root fallback
        shortcuts_registry.scope(TestScopes::S8).add(
            TestShortcuts::S8Bind1,
            ui::KeyBinding::new(ui::keyboard::KeyCode::KeyF),
        );

        // Test 5: Global fallback on root
        shortcuts_registry.scope(SHORTCUTS_ROOT_SCOPE_ID).add(
            TestShortcuts::RootBind1,
            ui::KeyBinding::new(ui::keyboard::KeyCode::KeyZ),
        );

        // Test text editing MacOS shortcuts
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

        window_manager.spawn_window(
            MainWindow {
                last_shortcut: Vec::new(),
            },
            WindowDescriptor {
                title: "Shortcut System Test".to_string(),
                name: Some("clew-example".to_string()),
                width: 1200,
                height: 800,
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

impl MainWindow {
    pub fn push_shortcut(&mut self, shortcut: &str) {
        if self.last_shortcut.len() > 6 {
            self.last_shortcut.remove(6);
        }

        let mut color = ui::Tween::new(ui::ColorRgba::from_hex(0xFFFF0000))
            .curve(ui::curves::f32::ease_out_back)
            .duration(Duration::from_secs(2));

        color.tween_to(ui::ColorRgba::from_hex(0xFF00FF00));

        self.last_shortcut.insert(
            0,
            ExecutedShortcut {
                id: Instant::now(),
                title: shortcut.to_string(),
                color,
            },
        );
    }
}

pub struct MainWindow {
    last_shortcut: Vec<ExecutedShortcut>,
}

pub struct ExecutedShortcut {
    id: Instant,
    title: String,
    color: Tween<ui::ColorRgba>,
}

impl Identifiable for ExecutedShortcut {
    type Id = Instant;

    fn id(&self) -> Self::Id {
        self.id
    }
}

#[derive(ShortcutId)]
pub enum TestShortcuts {
    S1Bind1,
    S1Bind2,
    S1Chord1,
    S2Bind1,
    S3Bind1,
    S4Bind1,
    S5Bind1,
    S6Bind1,
    S6Bind2,
    S7Bind1,
    S8Bind1,
    RootBind1,
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

impl Window<ShortcutsApplication, ()> for MainWindow {
    fn build(&mut self, _: &mut ShortcutsApplication, ctx: &mut ui::BuildContext) {
        ui::hstack()
            .spacing(20.)
            .padding(ui::EdgeInsets::all(20.))
            .fill_max_size()
            .build(ctx, |ctx| {
                // Left panel - Scope Hierarchy Tests
                ui::vstack().spacing(10.).build(ctx, |ctx| {
                    ui::text("Scope Hierarchy Tests")
                        .font_size(20.)
                        .color(ui::ColorRgba::from_hex(0xFFFFFFFF))
                        .build(ctx);

                    divider(ctx);

                    // Test 1: Simple siblings with shadowing
                    ui::vstack().spacing(5.).build(ctx, |ctx| {
                        ui::text("Test 1: Shadowing")
                            .font_size(16.)
                            .color(ui::ColorRgba::from_hex(0xFFAAFFFF))
                            .build(ctx);

                        ui::text("Press A - S2/S3 should shadow S1")
                            .font_size(12.)
                            .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                            .build(ctx);

                        ui::text("Press G - Falls through to S1")
                            .font_size(12.)
                            .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                            .build(ctx);

                        ui::text("Press K, then C - Chord test")
                            .font_size(12.)
                            .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                            .build(ctx);

                        ui::shortcut_scope(TestScopes::S1).build(ctx, |ctx| {
                            if ctx.is_shortcut_up(TestShortcuts::S1Bind1) {
                                self.push_shortcut("S1 / BIND3 (KeyA)");
                            }
                            if ctx.is_shortcut_up(TestShortcuts::S1Bind2) {
                                self.push_shortcut("S1 / BIND2 (KeyG)");
                            }
                            if ctx.is_shortcut_up(TestShortcuts::S1Chord1) {
                                self.push_shortcut("S1 / Chord K+C triggered");
                            }

                            ui::shortcut_scope(TestScopes::S2)
                                .active(true)
                                .build(ctx, |ctx| {
                                    if ctx.is_shortcut_up(TestShortcuts::S2Bind1) {
                                        self.push_shortcut("S2 / BIND1 (KeyA) - shadowed S1");
                                    }
                                    if ctx.is_shortcut_up(TestShortcuts::S1Bind2) {
                                        self.push_shortcut("S2 / BIND2 (KeyG) - from S1");
                                    }
                                });

                            ui::shortcut_scope(TestScopes::S2)
                                .active(false)
                                .build(ctx, |ctx| {
                                    if ctx.is_shortcut_up(TestShortcuts::S2Bind1) {
                                        self.push_shortcut(
                                            "Inactive S2 / BIND1 (KeyA) - should not be triggered",
                                        );
                                    }
                                    if ctx.is_shortcut_up(TestShortcuts::S1Bind2) {
                                        self.push_shortcut("Inactive S2 / BIND2 (KeyG) - from S1");
                                    }
                                });

                            ui::shortcut_scope(TestScopes::S3)
                                .active(true)
                                .build(ctx, |ctx| {
                                    if ctx.is_shortcut_up(TestShortcuts::S3Bind1) {
                                        self.push_shortcut("S3 / BIND1 (KeyA) - shadowed S1");
                                    }
                                    if ctx.is_shortcut_up(TestShortcuts::S1Bind2) {
                                        self.push_shortcut("S3 / BIND2 (KeyG) - from S1");
                                    }
                                });
                        });
                    });

                    divider(ctx);

                    // Test 2: Mix of leaf and non-leaf siblings
                    ui::vstack().spacing(5.).build(ctx, |ctx| {
                        ui::text("Test 2: Non-leaf Fallthrough")
                            .font_size(16.)
                            .color(ui::ColorRgba::from_hex(0xFFAAFFFF))
                            .build(ctx);

                        ui::text("Press B - S4 (non-leaf) handles it")
                            .font_size(12.)
                            .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                            .build(ctx);

                        ui::text("Press C - Only S6 has it")
                            .font_size(12.)
                            .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                            .build(ctx);

                        ui::text("Press D - Only S7 has it")
                            .font_size(12.)
                            .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                            .build(ctx);

                        ui::text("Press E - S6 shadows S5")
                            .font_size(12.)
                            .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                            .build(ctx);

                        ui::shortcut_scope(TestScopes::S4).build(ctx, |ctx| {
                            if ctx.is_shortcut_up(TestShortcuts::S4Bind1) {
                                self.push_shortcut("S4 / BIND1 (KeyB)");
                            }

                            ui::shortcut_scope(TestScopes::S5).build(ctx, |ctx| {
                                if ctx.is_shortcut_up(TestShortcuts::S5Bind1) {
                                    self.push_shortcut("S5 / BIND1 (KeyE)");
                                }

                                ui::shortcut_scope(TestScopes::S6).build(ctx, |ctx| {
                                    if ctx.is_shortcut_up(TestShortcuts::S6Bind1) {
                                        self.push_shortcut(
                                            "S6 / BIND1 (KeyC) - shadowed S5's KeyE",
                                        );
                                    }
                                    if ctx.is_shortcut_up(TestShortcuts::S6Bind2) {
                                        self.push_shortcut(
                                            "S6 / BIND2 (KeyE) - shadowed S5's BIND1",
                                        );
                                    }
                                });
                            });

                            ui::shortcut_scope(TestScopes::S7).build(ctx, |ctx| {
                                if ctx.is_shortcut_up(TestShortcuts::S7Bind1) {
                                    self.push_shortcut("S7 / BIND1 (KeyD)");
                                }
                            });
                        });
                    });

                    divider(ctx);

                    // Test 3: Single leaf & global fallback
                    ui::vstack().spacing(5.).build(ctx, |ctx| {
                        ui::text("Test 3: Single Leaf & Global")
                            .font_size(16.)
                            .color(ui::ColorRgba::from_hex(0xFFAAFFFF))
                            .build(ctx);

                        ui::text("Press F - Only S8 has it")
                            .font_size(12.)
                            .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                            .build(ctx);

                        ui::text("Press Z - Global fallback (root)")
                            .font_size(12.)
                            .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                            .build(ctx);

                        ui::shortcut_scope(TestScopes::S8).build(ctx, |ctx| {
                            if ctx.is_shortcut_up(TestShortcuts::S8Bind1) {
                                self.push_shortcut("S8 / BIND1 (KeyF)");
                            }
                        });

                        if ctx.is_shortcut_up(TestShortcuts::RootBind1) {
                            self.push_shortcut("ROOT / BIND1 (KeyZ) - global fallback");
                        }
                    });

                    // Status display
                    divider(ctx);
                    ui::text("Last Triggered:")
                        .font_size(14.)
                        .color(ui::ColorRgba::from_hex(0xFFFFFF00))
                        .build(ctx);

                    ui::for_each(self.last_shortcut.iter_mut()).build(ctx, |ctx, item| {
                        ui::text(&item.title)
                            .font_size(14.)
                            .color(item.color.resolve(ctx))
                            .build(ctx);
                    });
                });

                // Right panel - Text Editing Tests
                ui::vstack().spacing(10.).build(ctx, |ctx| {
                    ui::text("Text Editing Shortcuts")
                        .font_size(20.)
                        .color(ui::ColorRgba::from_hex(0xFFFFFFFF))
                        .build(ctx);

                    divider(ctx);

                    ui::shortcut_scope(ui::ShortcutScopes::TextEditing).build(ctx, |ctx| {
                        // Navigation shortcuts
                        ui::vstack().spacing(5.).build(ctx, |ctx| {
                            ui::text("Character Navigation (repeatable)")
                                .font_size(16.)
                                .color(ui::ColorRgba::from_hex(0xFFAAFFFF))
                                .build(ctx);

                            if ctx.is_shortcut_up(ui::TextEditingShortcut::MoveNext) {
                                self.push_shortcut("MoveNext (→)");
                            }
                            if ctx.is_shortcut_up(ui::TextEditingShortcut::MovePrev) {
                                self.push_shortcut("MovePrev (←)");
                            }
                            if ctx.is_shortcut_up(ui::TextEditingShortcut::MoveUp) {
                                self.push_shortcut("MoveUp (↑)");
                            }
                            if ctx.is_shortcut_up(ui::TextEditingShortcut::MoveDown) {
                                self.push_shortcut("MoveDown (↓)");
                            }

                            ui::text("Arrow keys: ← → ↑ ↓")
                                .font_size(12.)
                                .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                                .build(ctx);
                        });

                        divider(ctx);

                        // Line navigation
                        ui::vstack().spacing(5.).build(ctx, |ctx| {
                            ui::text("Line Navigation")
                                .font_size(16.)
                                .color(ui::ColorRgba::from_hex(0xFFAAFFFF))
                                .build(ctx);

                            if ctx.is_shortcut_up(ui::TextEditingShortcut::MoveStart) {
                                self.push_shortcut("MoveStart (Home)");
                            }
                            if ctx.is_shortcut_up(ui::TextEditingShortcut::MoveEnd) {
                                self.push_shortcut("MoveEnd (End)");
                            }

                            ui::text("Home: Start of line, End: End of line")
                                .font_size(12.)
                                .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                                .build(ctx);
                        });

                        divider(ctx);

                        // Buffer navigation with modifiers
                        ui::vstack().spacing(5.).build(ctx, |ctx| {
                            ui::text("Buffer Navigation (with Super)")
                                .font_size(16.)
                                .color(ui::ColorRgba::from_hex(0xFFAAFFFF))
                                .build(ctx);

                            if ctx.is_shortcut_up(ui::TextEditingShortcut::BufferStart) {
                                self.push_shortcut("BufferStart (⌘+Home)");
                            }
                            if ctx.is_shortcut_up(ui::TextEditingShortcut::BufferEnd) {
                                self.push_shortcut("BufferEnd (⌘+End)");
                            }

                            ui::text("⌘+Home: Start of document")
                                .font_size(12.)
                                .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                                .build(ctx);
                            ui::text("⌘+End: End of document")
                                .font_size(12.)
                                .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                                .build(ctx);
                        });

                        divider(ctx);

                        // Page navigation
                        ui::vstack().spacing(5.).build(ctx, |ctx| {
                            ui::text("Page Navigation (repeatable)")
                                .font_size(16.)
                                .color(ui::ColorRgba::from_hex(0xFFAAFFFF))
                                .build(ctx);

                            if ctx.is_shortcut_up(ui::TextEditingShortcut::PageUp) {
                                self.push_shortcut("PageUp");
                            }
                            if ctx.is_shortcut_up(ui::TextEditingShortcut::PageDown) {
                                self.push_shortcut("PageDown");
                            }

                            ui::text("PageUp/PageDown for scrolling")
                                .font_size(12.)
                                .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                                .build(ctx);
                        });

                        divider(ctx);

                        // Editing shortcuts
                        ui::vstack().spacing(5.).build(ctx, |ctx| {
                            ui::text("Editing (repeatable)")
                                .font_size(16.)
                                .color(ui::ColorRgba::from_hex(0xFFAAFFFF))
                                .build(ctx);

                            if ctx.is_shortcut_up(ui::TextEditingShortcut::Delete) {
                                self.push_shortcut("Delete");
                            }
                            if ctx.is_shortcut_up(ui::TextEditingShortcut::Backspace) {
                                self.push_shortcut("Backspace");
                            }
                            if ctx.is_shortcut_up(ui::TextEditingShortcut::NextLine) {
                                self.push_shortcut("NextLine (Enter)");
                            }

                            ui::text("Delete, Backspace, Enter")
                                .font_size(12.)
                                .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                                .build(ctx);
                        });

                        divider(ctx);

                        // Selection
                        ui::vstack().spacing(5.).build(ctx, |ctx| {
                            ui::text("Selection")
                                .font_size(16.)
                                .color(ui::ColorRgba::from_hex(0xFFAAFFFF))
                                .build(ctx);

                            if ctx.is_shortcut_up(ui::TextEditingShortcut::SelectAll) {
                                self.push_shortcut("SelectAll (⌘+A)");
                            }

                            ui::text("⌘+A: Select all")
                                .font_size(12.)
                                .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                                .build(ctx);
                        });

                        divider(ctx);

                        // Modifiers
                        ui::vstack().spacing(5.).build(ctx, |ctx| {
                            ui::text("Modifiers (hold keys)")
                                .font_size(16.)
                                .color(ui::ColorRgba::from_hex(0xFFAAFFFF))
                                .build(ctx);

                            let mut active_modifiers = Vec::new();

                            if ctx.has_modifier(ui::TextInputModifier::Select) {
                                active_modifiers.push("Select (⇧ Shift)");
                            }
                            if ctx.has_modifier(ui::TextInputModifier::Word) {
                                active_modifiers.push("Word (⌘ Super)");
                            }
                            if ctx.has_modifier(ui::TextInputModifier::Paragraph) {
                                active_modifiers.push("Paragraph (⌘ Super)");
                            }

                            if !active_modifiers.is_empty() {
                                self.push_shortcut(&format!(
                                    "Modifiers: {}",
                                    active_modifiers.join(", ")
                                ));
                            }

                            ui::text("⇧ Shift: Select modifier")
                                .font_size(12.)
                                .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                                .build(ctx);
                            ui::text("⌘ Super: Word/Paragraph modifier")
                                .font_size(12.)
                                .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                                .build(ctx);
                            ui::text("Combine: ⇧+→ = Move + Select")
                                .font_size(12.)
                                .color(ui::ColorRgba::from_hex(0xFFAAAAAA))
                                .build(ctx);
                        });
                    });
                });
            });
    }
}

fn divider(ctx: &mut ui::BuildContext) {
    ui::decorated_box()
        .fill_max_width()
        .color(ui::ColorRgba::from_hex(0xFFCCCCCC))
        .height(1.)
        .build(ctx);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracy_client::Client::start();

    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .init();

    log::info!("Starting Shortcut System Test");
    Application::run_application(ShortcutsApplication)?;

    Ok(())
}
