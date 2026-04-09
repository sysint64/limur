use std::time::Instant;

#[cfg(feature = "clipboard")]
use arboard::Clipboard;

use cosmic_text::Edit;
use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    BuildContext, GestureDetectorResponse, LayoutDirection, Rect, ShortcutId, ShortcutsManager,
    View, WidgetId,
    io::{Cursor, TextInputAction, UserInput},
    keyboard::KeyCode,
    state::ViewConfig,
    text::{FontResources, TextsResources},
    text_history::{TextDeletionDirection, TextEditDelta},
};

use super::{CommonShortcut, EditableTextDelta, OsEvent, State, TextEditingShortcut, commands};

pub(crate) fn handle_commands(context: &mut BuildContext, id: WidgetId) {
    let is_multi_line = context
        .widgets_states
        .editable_text
        .get(id)
        .map(|it| it.multi_line)
        .unwrap_or(false);

    if context.input.key_pressed == Some(KeyCode::ArrowRight) {
        commands::move_next(context, id);
    }

    if context.input.key_pressed == Some(KeyCode::ArrowLeft) {
        commands::move_prev(context, id);
    }

    if context.input.key_pressed == Some(KeyCode::Delete) {
        commands::delete(context, id);
    }

    if context.input.key_pressed == Some(KeyCode::Backspace) {
        commands::backspace(context, id);
    }

    if cfg!(target_os = "macos") {
        // TODO
    } else {
        if context.input.key_pressed == Some(KeyCode::Home) {
            if context.input.is_ctrl_pressed() && is_multi_line {
                commands::buffer_start(context, id);
            } else {
                commands::move_start(context, id);
            }
        }

        if context.input.key_pressed == Some(KeyCode::End) {
            if context.input.is_ctrl_pressed() && is_multi_line {
                commands::buffer_end(context, id);
            } else {
                commands::move_end(context, id);
            }
        }

        if is_multi_line {
            if context.input.key_pressed == Some(KeyCode::ArrowUp) {
                commands::move_up(context, id);
            }

            if context.input.key_pressed == Some(KeyCode::ArrowDown) {
                commands::move_down(context, id);
            }

            if context.input.key_pressed == Some(KeyCode::PageUp) {
                commands::page_up(context, id);
            }

            if context.input.key_pressed == Some(KeyCode::PageDown) {
                commands::page_down(context, id);
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::collapsible_else_if)]
pub(crate) fn handle_interaction(
    _id: WidgetId,
    user_input: &mut UserInput,
    view: &View,
    gesture_response: &GestureDetectorResponse,
    state: &mut State,
    os_events: &mut SmallVec<[OsEvent; 4]>,
    text: &mut TextsResources,
    fonts: &mut FontResources,
    view_config: &mut ViewConfig,
    shortcuts_manager: &mut ShortcutsManager,
    #[cfg(feature = "clipboard")] clipboard: Option<&mut Clipboard>,
    boundary: Rect,
) {
    if gesture_response.is_hot() || gesture_response.is_active() {
        user_input.cursor = Cursor::Text;
    }

    if gesture_response.is_active() {
        if user_input.mouse_released && gesture_response.is_hot() {
            os_events.push(OsEvent::FocusWindow);
        }
    } else if user_input.mouse_left_pressed && gesture_response.is_hot() {
        os_events.push(OsEvent::FocusWindow);
    }

    if gesture_response.is_focused() {
        // Important to do this when mouse released just for convinience so we
        // can properly handle was_focused branch without conflicting with
        // other text editing widgets.
        if user_input.mouse_released {
            os_events.push(OsEvent::ActivateIme);
        }

        view_config.should_update_cursor_each_frame = true;

        let select_modifier = user_input.is_shift_pressed();
        let word_modifier = if cfg!(target_os = "macos") {
            if state.macos_cmd_modifier {
                user_input.is_super_pressed()
            } else {
                user_input.is_ctrl_pressed()
            }
        } else {
            user_input.is_ctrl_pressed()
        };
        let paragraph_modifier = if cfg!(target_os = "macos") {
            if state.macos_cmd_modifier {
                user_input.is_super_pressed()
            } else {
                user_input.is_ctrl_pressed()
            }
        } else {
            user_input.is_ctrl_pressed()
        };

        // List of shortcuts that modifies text
        let edit_shortcuts: &[ShortcutId] = &[
            TextEditingShortcut::Delete.into(),
            TextEditingShortcut::Backspace.into(),
            TextEditingShortcut::NextLine.into(),
        ];

        if let Some(shortcut) = shortcuts_manager.active_shortcut_id()
            && edit_shortcuts.contains(&shortcut)
            && let Some(id) = state.text_id
        {
            let editor = text.editor_mut(id);
            normalize_editable_text_selection(state, view_config, editor, true);
        }

        let has_selection = if let Some(id) = state.text_id {
            let editor = text.editor_mut(id);

            editor.selection() != cosmic_text::Selection::None
        } else {
            false
        };

        if let Some(id) = state.text_id {
            let editor = text.editor_mut(id);

            if select_modifier && !has_selection {
                editor.set_selection(cosmic_text::Selection::Normal(editor.cursor()));
            }
        };

        if shortcuts_manager.is_shortcut(TextEditingShortcut::NextLine) && state.multi_line {
            user_input.text_input.push('\n');
            user_input.text_input_actions.push(TextInputAction::Insert);

            // If shortcut_id is not None then actions won't be processed
            shortcuts_manager.reset();
        }

        let prevent_insert = /* !shortcuts_manager.last_sequence.is_empty()
            && shortcuts_manager.candidates > 0
            || */
        //shortcuts_manager.last_active_shortcut_id().is_some();
            word_modifier || paragraph_modifier;

        if shortcuts_manager.is_shortcut(TextEditingShortcut::SelectAll)
            && let Some(id) = state.text_id
        {
            let editor = text.editor_mut(id);
            editor.set_selection(cosmic_text::Selection::None);
            editor.action(
                &mut fonts.font_system,
                cosmic_text::Action::Motion(cosmic_text::Motion::BufferStart),
            );
            editor.set_selection(cosmic_text::Selection::Normal(editor.cursor()));
            editor.action(
                &mut fonts.font_system,
                cosmic_text::Action::Motion(cosmic_text::Motion::BufferEnd),
            );
            on_editable_text_cursor_moved(state, view_config, editor);
        }

        #[cfg(feature = "clipboard")]
        if let Some(clipboard) = clipboard {
            if shortcuts_manager.is_shortcut(CommonShortcut::Copy)
                && let Some(id) = state.text_id
            {
                let editor = text.editor_mut(id);
                let text = editor.copy_selection();

                if let Some(text) = text
                    && let Err(err) = clipboard.set_text(text)
                {
                    log::error!("Failed to copy to clipboard: {err}");
                }
            }

            if shortcuts_manager.is_shortcut(CommonShortcut::Cut)
                && let Some(id) = state.text_id
                && has_selection
            {
                let editor = text.editor_mut(id);
                let text = editor.copy_selection();

                if let Some(text) = text {
                    match clipboard.set_text(text.clone()) {
                        Ok(_) => {
                            let (start, end) = editor
                                .selection_bounds()
                                .expect("Selection should be available");
                            editor.delete_selection();

                            on_editable_text_updated(
                                state,
                                view_config,
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
            }

            if shortcuts_manager.is_shortcut(CommonShortcut::Paste)
                && let Some(id) = state.text_id
            {
                let editor = text.editor_mut(id);

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

                        on_editable_text_updated(state, view_config, editor, Some(delta));
                    }
                    Err(err) => {
                        log::error!("Failed to paste from clipboard: {err}");
                    }
                }
            }
        }

        if shortcuts_manager.is_shortcut(CommonShortcut::Undo)
            && let Some(id) = state.text_id
        {
            let editor = text.editor_mut(id);
            let delta = state.history_manager.undo(editor).cloned();

            on_editable_text_updated(state, view_config, editor, None);

            if let Some(delta) = delta {
                state.deltas.push(EditableTextDelta::Undo(delta));
            }
        }

        if shortcuts_manager.is_shortcut(CommonShortcut::Redo)
            && let Some(id) = state.text_id
        {
            let editor = text.editor_mut(id);
            let delta = state.history_manager.redo(editor).cloned();

            on_editable_text_updated(state, view_config, editor, None);

            if let Some(delta) = delta {
                state.deltas.push(EditableTextDelta::Apply(delta));
            }
        }

        for text_input_action in &user_input.text_input_actions {
            match text_input_action {
                TextInputAction::None => {}
                TextInputAction::ImePreedit => {
                    if let Some(id) = state.text_id {
                        let editor = text.editor_mut(id);

                        if editor.selection() != cosmic_text::Selection::None {
                            editor.delete_selection();
                        }

                        editor.delete_range(editor.cursor(), state.ime_cursor_end);
                        on_editable_text_updated(state, view_config, editor, None);

                        if !user_input.ime_preedit.is_empty() {
                            state.ime_cursor_end =
                                editor.insert_at(editor.cursor(), &user_input.ime_preedit, None);
                        }
                    }
                }
                TextInputAction::ImeEnable => {}
                TextInputAction::ImeDisable | TextInputAction::ImeCommit => {
                    if let Some(id) = state.text_id {
                        let editor = text.editor_mut(id);
                        editor.delete_range(editor.cursor(), state.ime_cursor_end);
                        on_editable_text_updated(state, view_config, editor, None);
                    }
                }
                TextInputAction::Insert => {
                    if !user_input.text_input.is_empty()
                        && shortcuts_manager.active_shortcut_id().is_none()
                        && !prevent_insert
                        && let Some(id) = state.text_id
                    {
                        let editor = text.editor_mut(id);
                        let text = user_input.text_input.clone();

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
                        on_editable_text_updated(state, view_config, editor, Some(delta));
                    }
                }
            }
        }

        user_input.text_input_actions.clear();

        let mouse_dx = state.last_mouse_x - user_input.mouse_x;
        let mouse_dy = state.last_mouse_y - user_input.mouse_y;

        state.last_mouse_x = user_input.mouse_x;
        state.last_mouse_y = user_input.mouse_y;

        let drag_trigger = 4.0 * view.scale_factor;

        if gesture_response.is_active() {
            state.mouse_path_x += mouse_dx.abs();
            state.mouse_path_y += mouse_dy.abs();

            if let Some(id) = state.text_id {
                let editor = text.editor_mut(id);

                let relative_mouse_x = user_input.mouse_x as f32
                    - (boundary.x * view.scale_factor) as f32
                    - state.text_offset.x;
                let relative_mouse_y = user_input.mouse_y as f32
                    - (boundary.y * view.scale_factor) as f32
                    - state.text_offset.y;

                let relative_mouse_x = relative_mouse_x.floor() as i32;
                let relative_mouse_y = relative_mouse_y.floor() as i32;

                if user_input.mouse_left_pressed {
                    user_input.ime_preedit.clear();
                    os_events.push(OsEvent::CommitIme);

                    if user_input.mouse_left_click_count == 1 {
                        if select_modifier {
                            editor.action(
                                &mut fonts.font_system,
                                cosmic_text::Action::Drag {
                                    x: relative_mouse_x,
                                    y: relative_mouse_y,
                                },
                            );
                        } else {
                            editor.set_selection(cosmic_text::Selection::None);

                            // HACK: Invalidate buffer by invoking Home motion
                            // Don't know why, but it makes click position
                            // more consistent when text buffer updates from
                            // the build phase
                            editor.action(
                                &mut fonts.font_system,
                                cosmic_text::Action::Motion(cosmic_text::Motion::Home),
                            );
                            editor.action(
                                &mut fonts.font_system,
                                cosmic_text::Action::Click {
                                    x: relative_mouse_x,
                                    y: relative_mouse_y,
                                },
                            );
                        }
                    } else if user_input.mouse_left_click_count.is_multiple_of(2) {
                        editor.action(
                            &mut fonts.font_system,
                            cosmic_text::Action::DoubleClick {
                                x: relative_mouse_x,
                                y: relative_mouse_y,
                            },
                        );
                    } else {
                        editor.action(
                            &mut fonts.font_system,
                            cosmic_text::Action::TripleClick {
                                x: relative_mouse_x,
                                y: relative_mouse_y,
                            },
                        );
                    }

                    user_input.last_click_time = Some(Instant::now());
                    state.direction_decided = false;
                    on_editable_text_cursor_moved(state, view_config, editor);
                } else if let Some(last_click_time) = user_input.last_click_time
                    && last_click_time.elapsed().as_millis() > 17
                    && (state.mouse_path_x > drag_trigger || state.mouse_path_y > drag_trigger)
                {
                    let height = boundary.height * view.scale_factor;
                    let scroll_area_size = 8.0 * view.scale_factor;
                    let relative_mouse_y_f64 = relative_mouse_y as f64;
                    let at_top = relative_mouse_y_f64 <= scroll_area_size;
                    let at_bottom = relative_mouse_y_f64 >= height - scroll_area_size;

                    // Adjust overscroll speed
                    if at_top || at_bottom {
                        let mut distance = if at_top {
                            (relative_mouse_y_f64 - scroll_area_size).abs()
                        } else {
                            (relative_mouse_y_f64 - height + scroll_area_size).abs()
                        };

                        distance = f64::min(distance, 200.0);
                        let normalized = distance / 200.0; // 0.0 to 1.0

                        // non-linear curve - x^2 for more natural feel
                        let interval = (40.0 * (1.0 - normalized * normalized)).ceil() as u128;

                        if let Some(last_drag) = state.last_drag {
                            if last_drag.elapsed().as_millis() > interval {
                                editor.action(
                                    &mut fonts.font_system,
                                    cosmic_text::Action::Drag {
                                        x: relative_mouse_x,
                                        y: relative_mouse_y,
                                    },
                                );
                                state.last_drag = Some(Instant::now());
                            }
                        } else {
                            editor.action(
                                &mut fonts.font_system,
                                cosmic_text::Action::Drag {
                                    x: relative_mouse_x,
                                    y: relative_mouse_y,
                                },
                            );
                            state.last_drag = Some(Instant::now());
                        }
                    } else {
                        editor.action(
                            &mut fonts.font_system,
                            cosmic_text::Action::Drag {
                                x: relative_mouse_x,
                                y: relative_mouse_y,
                            },
                        );
                        state.last_drag = Some(Instant::now());
                    }

                    let bounds = editor.selection_bounds();

                    if let Some((start, end)) = bounds
                        && start == end
                    {
                        editor.set_selection(cosmic_text::Selection::None);
                    }

                    state.direction_decided = false;
                    on_editable_text_cursor_moved(state, view_config, editor);
                }
            }
        } else {
            state.mouse_path_x = 0.;
            state.mouse_path_y = 0.;
        }

        if let Some(id) = state.text_id {
            let editor = text.editor_mut(id);
            if user_input.mouse_released {
                normalize_editable_text_selection(state, view_config, editor, true);
            }
        }
    } else if gesture_response.was_focused() {
        user_input.ime_preedit.clear();
        os_events.push(OsEvent::CommitIme);
        view_config.should_update_cursor_each_frame = false;

        os_events.push(OsEvent::DeactivateIme);

        state.history_manager.clear();
        state.scroll_x = 0.;

        if let Some(id) = state.text_id {
            let editor = text.editor_mut(id);
            editor.set_selection(cosmic_text::Selection::None);
            editor.action(
                &mut fonts.font_system,
                cosmic_text::Action::Motion(cosmic_text::Motion::Home),
            );

            on_editable_text_cursor_moved(state, view_config, editor);
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) enum ParagraphMotionDirection {
    Up,
    Down,
}

pub(crate) fn move_paragraph(
    fonts: &mut FontResources,
    editor: &mut cosmic_text::Editor,
    direction: ParagraphMotionDirection,
) {
    let mut reached_boundary = true;
    let delta: i32 = match direction {
        ParagraphMotionDirection::Up => -1,
        ParagraphMotionDirection::Down => 1,
    };
    let mut cursor_line = editor.cursor().line as i32 + delta;

    editor.with_buffer(|buffer| {
        let mut line = buffer.lines.get(i32::max(0, cursor_line) as usize);

        while line.is_some() && cursor_line >= 0 {
            if line.unwrap().text().trim().is_empty() {
                reached_boundary = false;
                break;
            } else {
                cursor_line += delta;
                line = buffer.lines.get(i32::max(0, cursor_line) as usize);
            }
        }
    });

    editor.set_cursor(cosmic_text::Cursor::new(
        i32::max(0, cursor_line) as usize,
        0,
    ));

    if reached_boundary {
        editor.action(
            &mut fonts.font_system,
            cosmic_text::Action::Motion(match direction {
                ParagraphMotionDirection::Up => cosmic_text::Motion::BufferStart,
                ParagraphMotionDirection::Down => cosmic_text::Motion::BufferEnd,
            }),
        );
    }
}

pub(crate) fn on_editable_text_updated(
    state: &mut State,
    view_config: &mut ViewConfig,
    editor: &mut cosmic_text::Editor,
    delta: Option<TextEditDelta>,
) {
    state.ime_cursor_end = editor.cursor();

    if let Some(delta) = &delta {
        state.deltas.push(EditableTextDelta::Apply(delta.clone()));
    }

    state.recompose_text_content = true;
    state.direction_decided = true;
    state.auto_scroll_to_cursor = true;

    if let Some(delta) = delta
        && state.save_history
    {
        state.history_manager.push(delta);
    }

    update_should_use_wide_space(view_config, editor);
}

#[allow(clippy::collapsible_else_if)]
pub(crate) fn normalize_editable_text_selection(
    state: &mut State,
    view_config: &mut ViewConfig,
    editor: &mut cosmic_text::Editor,
    handle_cursor_moved: bool,
) {
    let normalize_selection = match editor.selection() {
        cosmic_text::Selection::None => false,
        cosmic_text::Selection::Normal(..) => true,
        cosmic_text::Selection::Line(..) => true,
        cosmic_text::Selection::Word(..) => true,
    };

    if normalize_selection {
        let bounds = editor.selection_bounds();

        if let Some((start, end)) = bounds {
            if start.line == end.line && start.index == end.index {
                editor.set_selection(cosmic_text::Selection::None);
                state.direction_decided = true;
            } else {
                let cursor = editor.cursor();

                if cursor.line == start.line && cursor.line == end.line {
                    if end > start {
                        if cursor.index - start.index < end.index - cursor.index {
                            editor.set_cursor(start);
                            editor.set_selection(cosmic_text::Selection::Normal(end));
                        } else {
                            editor.set_cursor(end);
                            editor.set_selection(cosmic_text::Selection::Normal(start));
                        }
                    } else {
                        if cursor.index - end.index < start.index - cursor.index {
                            editor.set_cursor(end);
                            editor.set_selection(cosmic_text::Selection::Normal(start));
                        } else {
                            editor.set_cursor(start);
                            editor.set_selection(cosmic_text::Selection::Normal(end));
                        }
                    }
                } else {
                    if end > start {
                        if cursor.line == start.line {
                            editor.set_cursor(start);
                            editor.set_selection(cosmic_text::Selection::Normal(end));
                        } else {
                            editor.set_cursor(end);
                            editor.set_selection(cosmic_text::Selection::Normal(start));
                        }
                    } else {
                        if cursor.line == start.line {
                            editor.set_cursor(end);
                            editor.set_selection(cosmic_text::Selection::Normal(start));
                        } else {
                            editor.set_cursor(start);
                            editor.set_selection(cosmic_text::Selection::Normal(end));
                        }
                    }
                }

                if handle_cursor_moved {
                    on_editable_text_cursor_moved(state, view_config, editor);
                }
            }
        } else {
            editor.set_selection(cosmic_text::Selection::None);
        }
    }
}

pub(crate) fn on_editable_text_cursor_moved(
    state: &mut State,
    view_config: &mut ViewConfig,
    editor: &mut cosmic_text::Editor,
) {
    normalize_editable_text_selection(state, view_config, editor, false);

    state.ime_cursor_end = editor.cursor();
    state.direction_decided =
        state.direction_decided || editor.selection() == cosmic_text::Selection::None;
    state.auto_scroll_to_cursor = true;

    update_should_use_wide_space(view_config, editor);
}

fn update_should_use_wide_space(view_config: &mut ViewConfig, editor: &cosmic_text::Editor) {
    let cursor = editor.cursor();

    editor.with_buffer(|buffer| {
        let line = &buffer.lines[cursor.line];
        let grapheme = grapheme_before_cursor(line.text(), cursor.index);

        if let Some(grapheme) = grapheme {
            view_config.should_use_wide_space = UnicodeWidthStr::width_cjk(grapheme) >= 2;
        } else {
            view_config.should_use_wide_space = false;
        }
    });
}

fn grapheme_before_cursor(text: &str, byte_index: usize) -> Option<&str> {
    // Handle out of bounds
    if byte_index > text.len() {
        return None;
    }

    // Find the nearest valid UTF-8 character boundary at or before byte_index
    let mut valid_index = byte_index;

    while valid_index > 0 && !text.is_char_boundary(valid_index) {
        valid_index -= 1;
    }

    if byte_index == 0 {
        // Return the first grapheme if index is 0
        text.graphemes(true).next()
    } else {
        // Return the grapheme before the index
        let prefix = &text[..valid_index];
        prefix.graphemes(true).next_back()
    }
}

pub(crate) fn decide_editable_text_direction_next(
    state: &mut State,
    view_config: &mut ViewConfig,
    editor: &mut cosmic_text::Editor,
) {
    let bounds = editor.selection_bounds();

    if let Some((start, end)) = bounds {
        if start == end {
            editor.set_selection(cosmic_text::Selection::None);
        } else {
            match view_config.layout_direction {
                LayoutDirection::LTR => {
                    editor.set_cursor(end);
                    editor.set_selection(cosmic_text::Selection::Normal(start));
                }
                LayoutDirection::RTL => {
                    editor.set_cursor(start);
                    editor.set_selection(cosmic_text::Selection::Normal(end));
                }
            }

            on_editable_text_cursor_moved(state, view_config, editor);
        }
    }
}

pub(crate) fn decide_editable_text_direction_prev(
    state: &mut State,
    view_config: &mut ViewConfig,
    editor: &mut cosmic_text::Editor,
) {
    let bounds = editor.selection_bounds();

    if let Some((start, end)) = bounds {
        if start == end {
            editor.set_selection(cosmic_text::Selection::None);
            state.direction_decided = true;
        } else {
            match view_config.layout_direction {
                LayoutDirection::LTR => {
                    editor.set_cursor(start);
                    editor.set_selection(cosmic_text::Selection::Normal(end));
                }
                LayoutDirection::RTL => {
                    editor.set_cursor(end);
                    editor.set_selection(cosmic_text::Selection::Normal(start));
                }
            }

            on_editable_text_cursor_moved(state, view_config, editor);
        }
    }
}
