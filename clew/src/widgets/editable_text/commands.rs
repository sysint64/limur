use cosmic_text::Edit;

use crate::{
    BuildContext, LayoutDirection, WidgetId,
    text_history::{TextDeletionDirection, TextEditDelta},
};

use super::interaction::{
    ParagraphMotionDirection, decide_editable_text_direction_next,
    decide_editable_text_direction_prev, move_paragraph, on_editable_text_cursor_moved,
    on_editable_text_updated,
};

pub fn select_modifier_enabled(context: &BuildContext) -> bool {
    context.input.is_shift_pressed()
}

pub fn word_modifier_enabled(context: &BuildContext, id: WidgetId) -> bool {
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

pub fn paragraph_modifier_enabled(context: &BuildContext, id: WidgetId) -> bool {
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

pub fn has_selection(context: &BuildContext, id: WidgetId) -> bool {
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
