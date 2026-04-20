use cosmic_text::Edit;

use crate::{
    BuildContext, LayoutDirection, WidgetId,
    text_history::{TextDeletionDirection, TextEditDelta},
};

use super::{
    EditableTextDelta,
    interaction::{
        ParagraphMotionDirection, decide_editable_text_direction_next,
        decide_editable_text_direction_prev, move_paragraph, on_editable_text_cursor_moved,
        on_editable_text_updated,
    },
};

pub fn select_modifier_enabled(context: &BuildContext) -> bool {
    context.input.is_shift_pressed()
}

pub fn word_modifier_enabled(context: &mut BuildContext, id: WidgetId) -> bool {
    let state = context.widgets_states.editable_text.get(id);

    if let Some(state) = state {
        if cfg!(target_os = "macos") {
            if state.macos_cmd_modifier {
                context.input.is_super_pressed()
            } else {
                context.input.is_ctrl_pressed()
            }
        } else {
            context.input.is_ctrl_pressed()
        }
    } else {
        false
    }
}

pub fn paragraph_modifier_enabled(context: &mut BuildContext, id: WidgetId) -> bool {
    let state = context.widgets_states.editable_text.get(id);

    if let Some(state) = state {
        if cfg!(target_os = "macos") {
            if state.macos_cmd_modifier {
                context.input.is_super_pressed()
            } else {
                context.input.is_ctrl_pressed()
            }
        } else {
            context.input.is_ctrl_pressed()
        }
    } else {
        false
    }
}

pub fn has_selection(context: &mut BuildContext, id: WidgetId) -> bool {
    let state = context.widgets_states.editable_text.get(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor(text_id);

        editor.selection() != cosmic_text::Selection::None
    } else {
        false
    }
}

pub fn delete(context: &mut BuildContext, id: WidgetId) {
    let has_selection = has_selection(context, id);
    let word_modifier = word_modifier_enabled(context, id);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        let (deleted_text, start, end) = if word_modifier && !has_selection {
            editor.set_selection(cosmic_text::Selection::Normal(editor.cursor()));
            editor.action(
                &mut context.fonts.font_system,
                cosmic_text::Action::Motion(cosmic_text::Motion::NextWord),
            );

            let (start, end) = editor
                .selection_bounds()
                .expect("Selection should be available");
            let text = editor
                .copy_selection()
                .expect("Selection should be available");

            editor.action(&mut context.fonts.font_system, cosmic_text::Action::Delete);
            editor.set_selection(cosmic_text::Selection::None);

            (text, start, end)
        } else if !has_selection {
            editor.set_selection(cosmic_text::Selection::Normal(editor.cursor()));
            editor.action(
                &mut context.fonts.font_system,
                cosmic_text::Action::Motion(cosmic_text::Motion::Next),
            );

            let (start, end) = editor
                .selection_bounds()
                .expect("Selection should be available");
            let text = editor
                .copy_selection()
                .expect("Selection should be available");

            editor.action(&mut context.fonts.font_system, cosmic_text::Action::Delete);
            editor.set_selection(cosmic_text::Selection::None);

            (text, start, end)
        } else {
            debug_assert!(has_selection);

            let (start, end) = editor
                .selection_bounds()
                .expect("Selection should be available");
            let text = editor
                .copy_selection()
                .expect("Selection should be available");

            editor.action(&mut context.fonts.font_system, cosmic_text::Action::Delete);
            editor.set_selection(cosmic_text::Selection::None);

            (text, start, end)
        };

        on_editable_text_updated(
            state,
            context.view_config,
            editor,
            Some(TextEditDelta::Delete {
                start,
                end,
                deleted_text,
                direction: TextDeletionDirection::Forward,
            }),
        );
    }
}

pub fn backspace(context: &mut BuildContext, id: WidgetId) {
    let has_selection = has_selection(context, id);
    let word_modifier = word_modifier_enabled(context, id);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        let (deleted_text, start, end) = if word_modifier && !has_selection {
            editor.set_selection(cosmic_text::Selection::Normal(editor.cursor()));
            editor.action(
                &mut context.fonts.font_system,
                cosmic_text::Action::Motion(cosmic_text::Motion::PreviousWord),
            );

            let (start, end) = editor
                .selection_bounds()
                .expect("Selection should be available");
            let text = editor
                .copy_selection()
                .expect("Selection should be available");

            editor.action(
                &mut context.fonts.font_system,
                cosmic_text::Action::Backspace,
            );
            editor.set_selection(cosmic_text::Selection::None);

            (text, start, end)
        } else if !has_selection {
            editor.set_selection(cosmic_text::Selection::Normal(editor.cursor()));
            editor.action(
                &mut context.fonts.font_system,
                cosmic_text::Action::Motion(cosmic_text::Motion::Previous),
            );

            let (start, end) = editor
                .selection_bounds()
                .expect("Selection should be available");
            let text = editor
                .copy_selection()
                .expect("Selection should be available");

            editor.action(
                &mut context.fonts.font_system,
                cosmic_text::Action::Backspace,
            );
            editor.set_selection(cosmic_text::Selection::None);

            (text, start, end)
        } else {
            debug_assert!(has_selection);

            let (start, end) = editor
                .selection_bounds()
                .expect("Selection should be available");
            let text = editor
                .copy_selection()
                .expect("Selection should be available");

            editor.action(
                &mut context.fonts.font_system,
                cosmic_text::Action::Backspace,
            );
            editor.set_selection(cosmic_text::Selection::None);

            (text, start, end)
        };

        on_editable_text_updated(
            state,
            context.view_config,
            editor,
            Some(TextEditDelta::Delete {
                start,
                end,
                deleted_text,
                direction: TextDeletionDirection::Backward,
            }),
        );
    }
}

pub fn move_next(context: &mut BuildContext, id: WidgetId) {
    let has_selection = has_selection(context, id);
    let select_modifier = select_modifier_enabled(context);
    let word_modifier = word_modifier_enabled(context, id);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        if !has_selection || select_modifier {
            if has_selection && !state.direction_decided {
                state.direction_decided = true;
                decide_editable_text_direction_next(state, context.view_config, editor);
            }

            if word_modifier {
                editor.action(
                    &mut context.fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::RightWord),
                );
            } else {
                editor.action(
                    &mut context.fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::Right),
                );
            }
        } else {
            let bounds = editor.selection_bounds();

            if let Some((start, end)) = bounds {
                match context.view_config.layout_direction {
                    LayoutDirection::LTR => editor.set_cursor(end),
                    LayoutDirection::RTL => editor.set_cursor(start),
                }
            }

            editor.set_selection(cosmic_text::Selection::None);

            if word_modifier {
                editor.action(
                    &mut context.fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::RightWord),
                );
            }
        }

        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

pub fn move_prev(context: &mut BuildContext, id: WidgetId) {
    let has_selection = has_selection(context, id);
    let select_modifier = select_modifier_enabled(context);
    let word_modifier = word_modifier_enabled(context, id);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        if !has_selection || select_modifier {
            if has_selection && !state.direction_decided {
                state.direction_decided = true;
                decide_editable_text_direction_prev(state, context.view_config, editor);
            }

            if word_modifier {
                editor.action(
                    &mut context.fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::LeftWord),
                );
            } else {
                editor.action(
                    &mut context.fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::Left),
                );
            }
        } else {
            let bounds = editor.selection_bounds();

            if let Some((start, end)) = bounds {
                match context.view_config.layout_direction {
                    LayoutDirection::LTR => editor.set_cursor(start),
                    LayoutDirection::RTL => editor.set_cursor(end),
                }
            }

            editor.set_selection(cosmic_text::Selection::None);

            if word_modifier {
                editor.action(
                    &mut context.fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::LeftWord),
                );
            }
        }

        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

pub fn move_start(context: &mut BuildContext, id: WidgetId) {
    let select_modifier = select_modifier_enabled(context);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        if state.multi_line {
            let cursor = editor.cursor();

            editor.action(
                &mut context.fonts.font_system,
                cosmic_text::Action::Motion(cosmic_text::Motion::SoftHome),
            );
            let home = editor.cursor();

            if cursor.line == home.line && cursor.index == home.index {
                editor.action(
                    &mut context.fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::Home),
                );
            }

            if !select_modifier {
                editor.set_selection(cosmic_text::Selection::None);
            }

            on_editable_text_cursor_moved(state, context.view_config, editor);
        } else {
            editor.action(
                &mut context.fonts.font_system,
                cosmic_text::Action::Motion(cosmic_text::Motion::Home),
            );

            if !select_modifier {
                editor.set_selection(cosmic_text::Selection::None);
            }

            on_editable_text_cursor_moved(state, context.view_config, editor);
        }
    }
}

pub fn move_end(context: &mut BuildContext, id: WidgetId) {
    let select_modifier = select_modifier_enabled(context);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        editor.action(
            &mut context.fonts.font_system,
            cosmic_text::Action::Motion(cosmic_text::Motion::ParagraphEnd),
        );

        if !select_modifier {
            editor.set_selection(cosmic_text::Selection::None);
        }

        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

pub fn buffer_start(context: &mut BuildContext, id: WidgetId) {
    let select_modifier = select_modifier_enabled(context);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        if !select_modifier {
            editor.set_selection(cosmic_text::Selection::None);
        }

        editor.action(
            &mut context.fonts.font_system,
            cosmic_text::Action::Motion(cosmic_text::Motion::BufferStart),
        );

        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

pub fn buffer_end(context: &mut BuildContext, id: WidgetId) {
    let select_modifier = select_modifier_enabled(context);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        if !select_modifier {
            editor.set_selection(cosmic_text::Selection::None);
        }

        editor.action(
            &mut context.fonts.font_system,
            cosmic_text::Action::Motion(cosmic_text::Motion::BufferEnd),
        );

        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

pub fn move_up(context: &mut BuildContext, id: WidgetId) {
    let has_selection = has_selection(context, id);
    let select_modifier = select_modifier_enabled(context);
    let paragraph_modifier = paragraph_modifier_enabled(context, id);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        if !has_selection || select_modifier {
            if paragraph_modifier {
                move_paragraph(context.fonts, editor, ParagraphMotionDirection::Up);
            } else {
                editor.action(
                    &mut context.fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::Up),
                );
            }
        } else {
            if let Some((start, _)) = editor.selection_bounds() {
                editor.set_cursor(start);
            }

            editor.set_selection(cosmic_text::Selection::None);
            editor.action(
                &mut context.fonts.font_system,
                cosmic_text::Action::Motion(cosmic_text::Motion::Up),
            );
        }

        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

pub fn move_down(context: &mut BuildContext, id: WidgetId) {
    let has_selection = has_selection(context, id);
    let select_modifier = select_modifier_enabled(context);
    let paragraph_modifier = paragraph_modifier_enabled(context, id);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        if !has_selection || select_modifier {
            if paragraph_modifier {
                move_paragraph(context.fonts, editor, ParagraphMotionDirection::Down);
            } else {
                editor.action(
                    &mut context.fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::Down),
                );
            }
        } else {
            if let Some((_, end)) = editor.selection_bounds() {
                editor.set_cursor(end);
            }

            editor.set_selection(cosmic_text::Selection::None);
            editor.action(
                &mut context.fonts.font_system,
                cosmic_text::Action::Motion(cosmic_text::Motion::Down),
            );
        }

        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

pub fn move_paragraph_up(context: &mut BuildContext, id: WidgetId) {
    let select_modifier = select_modifier_enabled(context);
    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        if !select_modifier {
            editor.set_selection(cosmic_text::Selection::None);
        }

        move_paragraph(context.fonts, editor, ParagraphMotionDirection::Up);
        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

pub fn move_paragraph_down(context: &mut BuildContext, id: WidgetId) {
    let select_modifier = select_modifier_enabled(context);
    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        if !select_modifier {
            editor.set_selection(cosmic_text::Selection::None);
        }

        move_paragraph(context.fonts, editor, ParagraphMotionDirection::Down);
        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

pub fn page_up(context: &mut BuildContext, id: WidgetId) {
    let select_modifier = select_modifier_enabled(context);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        if !select_modifier {
            editor.set_selection(cosmic_text::Selection::None);
        }

        editor.action(
            &mut context.fonts.font_system,
            cosmic_text::Action::Motion(cosmic_text::Motion::PageUp),
        );

        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

pub fn page_down(context: &mut BuildContext, id: WidgetId) {
    let select_modifier = select_modifier_enabled(context);

    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        if !select_modifier {
            editor.set_selection(cosmic_text::Selection::None);
        }

        editor.action(
            &mut context.fonts.font_system,
            cosmic_text::Action::Motion(cosmic_text::Motion::PageDown),
        );

        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

pub fn insert_new_line(context: &mut BuildContext, id: WidgetId) {
    // TODO(sysint64): Handle auto indentation
    insert_text(context, id, "\n".to_string());
}

pub fn insert_text(context: &mut BuildContext, id: WidgetId, text: String) {
    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        let bounds = editor.selection_bounds();
        let selected_text = editor.copy_selection();

        let after_start = if let Some((before_start, _)) = bounds {
            before_start
        } else {
            editor.cursor()
        };

        editor.insert_string(&text, None);
        let after_end = editor.cursor();

        let delta = if let Some((before_start, before_end)) = bounds
            && before_start != before_end
        {
            TextEditDelta::Replace {
                range_before: (before_start, before_end),
                range_after: (after_start, after_end),
                text_before: selected_text.expect("Selection should be available"),
                text_after: text,
            }
        } else {
            TextEditDelta::Insert {
                cursor_before: after_start,
                cursor_after: after_end,
                text,
            }
        };

        editor.set_selection(cosmic_text::Selection::None);
        on_editable_text_updated(state, context.view_config, editor, Some(delta));
    }
}

pub fn select_all(context: &mut BuildContext, id: WidgetId) {
    let state = context.widgets_states.editable_text.get_mut(id);

    if let Some(state) = state
        && let Some(text_id) = state.text_id
    {
        let editor = context.text.editor_mut(text_id);

        editor.set_selection(cosmic_text::Selection::None);
        editor.action(
            &mut context.fonts.font_system,
            cosmic_text::Action::Motion(cosmic_text::Motion::BufferStart),
        );
        editor.set_selection(cosmic_text::Selection::Normal(editor.cursor()));
        editor.action(
            &mut context.fonts.font_system,
            cosmic_text::Action::Motion(cosmic_text::Motion::BufferEnd),
        );
        on_editable_text_cursor_moved(state, context.view_config, editor);
    }
}

#[cfg(feature = "clipboard")]
pub fn copy_selected(context: &mut BuildContext, id: WidgetId) {
    if !has_selection(context, id) {
        return;
    }

    let Some(clipboard) = context.clipboard else {
        return;
    };

    let Some(state) = context.widgets_states.editable_text.get_mut(id) else {
        return;
    };
    let Some(text_id) = state.text_id else { return };

    let editor = context.text.editor_mut(text_id);
    let text = editor.copy_selection();

    if let Some(text) = text
        && let Err(err) = clipboard.set_text(text)
    {
        log::error!("Failed to copy to clipboard: {err}");
    }
}

#[cfg(feature = "clipboard")]
pub fn cut_selected(context: &mut BuildContext, id: WidgetId) {
    if !has_selection(context, id) {
        return;
    }

    let Some(clipboard) = context.clipboard else {
        return;
    };

    let Some(state) = context.widgets_states.editable_text.get_mut(id) else {
        return;
    };
    let Some(text_id) = state.text_id else { return };

    let editor = context.text.editor_mut(text_id);
    let text = editor.copy_selection();

    let Some(text) = text else { return };

    match clipboard.set_text(text.clone()) {
        Ok(_) => {
            let (start, end) = editor
                .selection_bounds()
                .expect("Selection should be available");
            editor.delete_selection();

            on_editable_text_updated(
                state,
                context.view_config,
                editor,
                Some(TextEditDelta::Delete {
                    start,
                    end,
                    deleted_text: text,
                    direction: TextDeletionDirection::Backward,
                }),
            );
        }
        Err(err) => {
            log::error!("Failed to copy to clipboard: {err}");
        }
    }
}

#[cfg(feature = "clipboard")]
pub fn paste(context: &mut BuildContext, id: WidgetId) {
    let Some(clipboard) = context.clipboard else {
        return;
    };

    let Some(state) = context.widgets_states.editable_text.get_mut(id) else {
        return;
    };
    let Some(text_id) = state.text_id else { return };

    let editor = context.text.editor_mut(text_id);

    match clipboard.get_text() {
        Ok(text) => {
            let bounds = editor.selection_bounds();
            let selected_text = editor.copy_selection();

            let after_start = if let Some((before_start, _)) = bounds {
                before_start
            } else {
                editor.cursor()
            };

            editor.insert_string(&text, None);
            let after_end = editor.cursor();

            let delta = if let Some((before_start, before_end)) = bounds {
                TextEditDelta::Replace {
                    range_before: (before_start, before_end),
                    range_after: (after_start, after_end),
                    text_before: selected_text.expect("Selection should be available"),
                    text_after: text,
                }
            } else {
                TextEditDelta::Insert {
                    cursor_before: after_start,
                    cursor_after: after_end,
                    text,
                }
            };

            on_editable_text_updated(state, context.view_config, editor, Some(delta));
        }
        Err(err) => {
            log::error!("Failed to paste from clipboard: {err}");
        }
    }
}

pub fn undo(context: &mut BuildContext, id: WidgetId) {
    let Some(state) = context.widgets_states.editable_text.get_mut(id) else {
        return;
    };
    let Some(text_id) = state.text_id else { return };
    let editor = context.text.editor_mut(text_id);

    let delta = state.history_manager.undo(editor).cloned();

    on_editable_text_updated(state, context.view_config, editor, None);

    if let Some(delta) = delta {
        state.deltas.push(EditableTextDelta::Undo(delta));
    }
}

pub fn redo(context: &mut BuildContext, id: WidgetId) {
    let Some(state) = context.widgets_states.editable_text.get_mut(id) else {
        return;
    };
    let Some(text_id) = state.text_id else { return };
    let editor = context.text.editor_mut(text_id);

    let delta = state.history_manager.redo(editor).cloned();

    on_editable_text_updated(state, context.view_config, editor, None);

    if let Some(delta) = delta {
        state.deltas.push(EditableTextDelta::Apply(delta));
    }
}
