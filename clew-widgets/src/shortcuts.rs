use clew::ShortcutsRegistry;
use clew::{self as ui, SHORTCUTS_ROOT_SCOPE_ID};

pub fn init_shortcuts(shortcuts_registry: &mut ShortcutsRegistry) {
    // let scope = shortcuts_registry
    //     .scope(ui::ShortcutScopes::GestureDetector)
    //     .add(
    //         ui::GestureDetectorShortcut::Activate,
    //         ui::KeyBinding::down(ui::keyboard::KeyCode::Space),
    //     )
    //     .add(
    //         ui::GestureDetectorShortcut::Click,
    //         ui::KeyBinding::up(ui::keyboard::KeyCode::Space),
    //     )
    //     .add(
    //         ui::GestureDetectorShortcut::Activate,
    //         ui::KeyBinding::down(ui::keyboard::KeyCode::Enter),
    //     )
    //     .add(
    //         ui::GestureDetectorShortcut::Click,
    //         ui::KeyBinding::up(ui::keyboard::KeyCode::Enter),
    //     );

    let scope = shortcuts_registry
        .scope(ui::ShortcutScopes::TextEditing)
        .add(
            ui::TextEditingShortcut::Delete,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::Delete),
        )
        .add(
            ui::TextEditingShortcut::Delete,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::Delete),
        )
        .add(
            ui::TextEditingShortcut::Backspace,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::Backspace),
        )
        .add(
            ui::TextEditingShortcut::MoveNext,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::ArrowRight),
        )
        .add(
            ui::TextEditingShortcut::MovePrev,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::ArrowLeft),
        )
        .add(
            ui::TextEditingShortcut::MoveUp,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::ArrowUp),
        )
        .add(
            ui::TextEditingShortcut::MoveDown,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::ArrowDown),
        )
        .add(
            ui::TextEditingShortcut::NextLine,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::Enter),
        )
        .add(
            ui::TextEditingShortcut::MoveStart,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::Home),
        )
        .add(
            ui::TextEditingShortcut::MoveEnd,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::End),
        )
        .add(
            ui::TextEditingShortcut::PageUp,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::PageUp),
        )
        .add(
            ui::TextEditingShortcut::PageDown,
            ui::KeyBinding::repeat(ui::keyboard::KeyCode::PageDown),
        )
        .add_modifier(
            ui::TextInputModifier::Select,
            ui::keyboard::KeyModifiers::shift(),
        );

    if cfg!(target_os = "macos") {
        scope
            .add(
                ui::TextEditingShortcut::BufferStart,
                ui::KeyBinding::repeat(ui::keyboard::KeyCode::Home).with_super(),
            )
            .add(
                ui::TextEditingShortcut::BufferEnd,
                ui::KeyBinding::new(ui::keyboard::KeyCode::End).with_super(),
            )
            .add(
                ui::TextEditingShortcut::SelectAll,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyA).with_super(),
            )
            .add_modifier(
                ui::TextInputModifier::Word,
                ui::keyboard::KeyModifiers::alt(),
            )
            .add_modifier(
                ui::TextInputModifier::Paragraph,
                ui::keyboard::KeyModifiers::alt(),
            )
            .add(
                ui::TextEditingShortcut::MoveStart,
                ui::KeyBinding::repeat(ui::keyboard::KeyCode::Home).with_super(),
            )
            .add(
                ui::TextEditingShortcut::MoveEnd,
                ui::KeyBinding::repeat(ui::keyboard::KeyCode::End).with_super(),
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
            .add(
                ui::CommonShortcut::Paste,
                ui::KeyBinding::repeat(ui::keyboard::KeyCode::KeyV).with_super(),
            )
            .add(
                ui::CommonShortcut::Undo,
                ui::KeyBinding::repeat(ui::keyboard::KeyCode::KeyZ).with_super(),
            )
            .add(
                ui::CommonShortcut::Redo,
                ui::KeyBinding::repeat(ui::keyboard::KeyCode::KeyZ)
                    .with_super()
                    .with_shift(),
            );
    } else {
        scope
            .add(
                ui::TextEditingShortcut::BufferStart,
                ui::KeyBinding::repeat(ui::keyboard::KeyCode::Home).with_ctrl(),
            )
            .add(
                ui::TextEditingShortcut::BufferEnd,
                ui::KeyBinding::new(ui::keyboard::KeyCode::End).with_ctrl(),
            )
            // .add_sequence(
            //     ui::TextEditingShortcut::SelectAll,
            //     &[
            //         ui::KeyBinding::new(ui::keyboard::KeyCode::KeyA).with_ctrl(),
            //         ui::KeyBinding::new(ui::keyboard::KeyCode::KeyA),
            //     ],
            // )
            .add(
                ui::TextEditingShortcut::SelectAll,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyA).with_ctrl(),
            )
            .add_modifier(
                ui::TextInputModifier::Word,
                ui::keyboard::KeyModifiers::ctrl(),
            )
            .add_modifier(
                ui::TextInputModifier::Paragraph,
                ui::keyboard::KeyModifiers::ctrl(),
            );

        shortcuts_registry
            .scope(SHORTCUTS_ROOT_SCOPE_ID)
            .add(
                ui::CommonShortcut::Copy,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyC).with_ctrl(),
            )
            .add(
                ui::CommonShortcut::Cut,
                ui::KeyBinding::new(ui::keyboard::KeyCode::KeyX).with_ctrl(),
            )
            .add(
                ui::CommonShortcut::Paste,
                ui::KeyBinding::repeat(ui::keyboard::KeyCode::KeyV).with_ctrl(),
            )
            .add(
                ui::CommonShortcut::Undo,
                ui::KeyBinding::repeat(ui::keyboard::KeyCode::KeyZ).with_ctrl(),
            )
            .add(
                ui::CommonShortcut::Redo,
                ui::KeyBinding::repeat(ui::keyboard::KeyCode::KeyZ)
                    .with_ctrl()
                    .with_shift(),
            );
    }
}
