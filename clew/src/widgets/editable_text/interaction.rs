use std::time::Instant;

#[cfg(feature = "clipboard")]
use arboard::Clipboard;

use cosmic_text::Edit;
use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    GestureDetectorResponse, LayoutDirection, Rect, ShortcutId, ShortcutsManager, View, WidgetId,
    interaction::InteractionState,
    io::{Cursor, TextInputAction, UserInput},
    state::ViewConfig,
    text::{FontResources, TextsResources},
    text_history::{TextDeletionDirection, TextEditDelta},
};

use super::{
    CommonShortcut, EditableTextDelta, OsEvent, State, TextEditingShortcut, TextInputModifier,
};

pub(crate) fn handle_interaction(
    id: WidgetId,
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
        if user_input.mouse_released {
            if gesture_response.is_hot() {
                // interaction.set_inactive(&id);
                // interaction.focused = Some(id);
                os_events.push(OsEvent::FocusWindow);
            } else {
                // interaction.set_inactive(&id);
            }
        }
    } else if user_input.mouse_left_pressed && gesture_response.is_hot() {
        // interaction.set_active(&id);
        // interaction.focused = Some(id);
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

        let select_modifier = shortcuts_manager.has_modifier(TextInputModifier::Select);
        let word_modifier = shortcuts_manager.has_modifier(TextInputModifier::Word);
        let paragraph_modifier = shortcuts_manager.has_modifier(TextInputModifier::Paragraph);

        // List of shortcuts that modifies text
        let edit_shortcuts: &[ShortcutId] = &[
            TextEditingShortcut::Delete.into(),
            TextEditingShortcut::Backspace.into(),
            TextEditingShortcut::NextLine.into(),
        ];

        if let Some(shortcut) = shortcuts_manager.active_shortcut_id()
            && edit_shortcuts.contains(&shortcut)
        {
            if let Some(id) = state.text_id {
                let editor = text.editor_mut(id);
                normalize_editable_text_selection(state, view_config, editor);
            }
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

        if shortcuts_manager.is_shortcut(TextEditingShortcut::Delete) {
            if let Some(id) = state.text_id {
                let editor = text.editor_mut(id);

                let (deleted_text, start, end) = if word_modifier && !has_selection {
                    editor.set_selection(cosmic_text::Selection::Normal(editor.cursor()));
                    editor.action(
                        &mut fonts.font_system,
                        cosmic_text::Action::Motion(cosmic_text::Motion::NextWord),
                    );

                    let (start, end) = editor
                        .selection_bounds()
                        .expect("Selection should be available");
                    let text = editor
                        .copy_selection()
                        .expect("Selection should be available");

                    editor.action(&mut fonts.font_system, cosmic_text::Action::Delete);
                    editor.set_selection(cosmic_text::Selection::None);

                    (text, start, end)
                } else if !has_selection {
                    editor.set_selection(cosmic_text::Selection::Normal(editor.cursor()));
                    editor.action(
                        &mut fonts.font_system,
                        cosmic_text::Action::Motion(cosmic_text::Motion::Next),
                    );

                    let (start, end) = editor
                        .selection_bounds()
                        .expect("Selection should be available");
                    let text = editor
                        .copy_selection()
                        .expect("Selection should be available");

                    editor.action(&mut fonts.font_system, cosmic_text::Action::Delete);
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

                    editor.action(&mut fonts.font_system, cosmic_text::Action::Delete);
                    editor.set_selection(cosmic_text::Selection::None);

                    (text, start, end)
                };

                on_editable_text_updated(
                    state,
                    view_config,
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

        if shortcuts_manager.is_shortcut(TextEditingShortcut::Backspace) {
            if let Some(id) = state.text_id {
                let editor = text.editor_mut(id);

                let (deleted_text, start, end) = if word_modifier && !has_selection {
                    editor.set_selection(cosmic_text::Selection::Normal(editor.cursor()));
                    editor.action(
                        &mut fonts.font_system,
                        cosmic_text::Action::Motion(cosmic_text::Motion::PreviousWord),
                    );

                    let (start, end) = editor
                        .selection_bounds()
                        .expect("Selection should be available");
                    let text = editor
                        .copy_selection()
                        .expect("Selection should be available");

                    editor.action(&mut fonts.font_system, cosmic_text::Action::Backspace);
                    editor.set_selection(cosmic_text::Selection::None);

                    (text, start, end)
                } else if !has_selection {
                    editor.set_selection(cosmic_text::Selection::Normal(editor.cursor()));
                    editor.action(
                        &mut fonts.font_system,
                        cosmic_text::Action::Motion(cosmic_text::Motion::Previous),
                    );

                    let (start, end) = editor
                        .selection_bounds()
                        .expect("Selection should be available");
                    let text = editor
                        .copy_selection()
                        .expect("Selection should be available");

                    editor.action(&mut fonts.font_system, cosmic_text::Action::Backspace);
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

                    editor.action(&mut fonts.font_system, cosmic_text::Action::Backspace);
                    editor.set_selection(cosmic_text::Selection::None);

                    (text, start, end)
                };

                on_editable_text_updated(
                    state,
                    view_config,
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

        if shortcuts_manager.is_shortcut(TextEditingShortcut::NextLine) && state.multi_line {
            user_input.text_input.push('\n');
            user_input.text_input_actions.push(TextInputAction::Insert);

            // If shortcut_id is not None then actions won't be processed
            shortcuts_manager.reset();
        }

        if shortcuts_manager.is_shortcut(TextEditingShortcut::MoveStart)
            && let Some(id) = state.text_id
        {
            if state.multi_line {
                let editor = text.editor_mut(id);
                let cursor = editor.cursor();

                editor.action(
                    &mut fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::SoftHome),
                );
                let home = editor.cursor();

                if cursor.line == home.line && cursor.index == home.index {
                    editor.action(
                        &mut fonts.font_system,
                        cosmic_text::Action::Motion(cosmic_text::Motion::Home),
                    );
                }

                if !select_modifier {
                    editor.set_selection(cosmic_text::Selection::None);
                }

                on_editable_text_cursor_moved(state, view_config, editor);
            } else {
                let editor = text.editor_mut(id);
                editor.action(
                    &mut fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::Home),
                );

                if !select_modifier {
                    editor.set_selection(cosmic_text::Selection::None);
                }

                on_editable_text_cursor_moved(state, view_config, editor);
            }
        }

        if shortcuts_manager.is_shortcut(TextEditingShortcut::MoveEnd) {
            if let Some(id) = state.text_id {
                let editor = text.editor_mut(id);
                editor.action(
                    &mut fonts.font_system,
                    cosmic_text::Action::Motion(cosmic_text::Motion::ParagraphEnd),
                );

                if !select_modifier {
                    editor.set_selection(cosmic_text::Selection::None);
                }

                on_editable_text_cursor_moved(state, view_config, editor);
            }
        }

        if state.multi_line {
            if shortcuts_manager.is_shortcut(TextEditingShortcut::MoveUp) {
                if let Some(id) = state.text_id {
                    let editor = text.editor_mut(id);

                    if !has_selection || select_modifier {
                        if paragraph_modifier {
                            move_paragraph(fonts, editor, ParagraphMotionDirection::Up);
                        } else {
                            editor.action(
                                &mut fonts.font_system,
                                cosmic_text::Action::Motion(cosmic_text::Motion::Up),
                            );
                        }
                    } else {
                        if let Some((start, _)) = editor.selection_bounds() {
                            editor.set_cursor(start);
                        }

                        editor.set_selection(cosmic_text::Selection::None);
                        editor.action(
                            &mut fonts.font_system,
                            cosmic_text::Action::Motion(cosmic_text::Motion::Up),
                        );
                    }

                    on_editable_text_cursor_moved(state, view_config, editor);
                }
            }

            if shortcuts_manager.is_shortcut(TextEditingShortcut::MoveDown) {
                if let Some(id) = state.text_id {
                    let editor = text.editor_mut(id);

                    if !has_selection || select_modifier {
                        if paragraph_modifier {
                            move_paragraph(fonts, editor, ParagraphMotionDirection::Down);
                        } else {
                            editor.action(
                                &mut fonts.font_system,
                                cosmic_text::Action::Motion(cosmic_text::Motion::Down),
                            );
                        }
                    } else {
                        if let Some((_, end)) = editor.selection_bounds() {
                            editor.set_cursor(end);
                        }

                        editor.set_selection(cosmic_text::Selection::None);
                        editor.action(
                            &mut fonts.font_system,
                            cosmic_text::Action::Motion(cosmic_text::Motion::Down),
                        );
                    }

                    on_editable_text_cursor_moved(state, view_config, editor);
                }
            }

            if shortcuts_manager.is_shortcut(TextEditingShortcut::PageUp) {
                if let Some(id) = state.text_id {
                    let editor = text.editor_mut(id);

                    if !select_modifier {
                        editor.set_selection(cosmic_text::Selection::None);
                    }

                    editor.action(
                        &mut fonts.font_system,
                        cosmic_text::Action::Motion(cosmic_text::Motion::PageUp),
                    );

                    on_editable_text_cursor_moved(state, view_config, editor);
                }
            }

            if shortcuts_manager.is_shortcut(TextEditingShortcut::PageDown) {
                if let Some(id) = state.text_id {
                    let editor = text.editor_mut(id);

                    if !select_modifier {
                        editor.set_selection(cosmic_text::Selection::None);
                    }

                    editor.action(
                        &mut fonts.font_system,
                        cosmic_text::Action::Motion(cosmic_text::Motion::PageDown),
                    );

                    on_editable_text_cursor_moved(state, view_config, editor);
                }
            }

            if shortcuts_manager.is_shortcut(TextEditingShortcut::BufferStart) {
                if let Some(id) = state.text_id {
                    let editor = text.editor_mut(id);

                    if !select_modifier {
                        editor.set_selection(cosmic_text::Selection::None);
                    }

                    editor.action(
                        &mut fonts.font_system,
                        cosmic_text::Action::Motion(cosmic_text::Motion::BufferStart),
                    );

                    on_editable_text_cursor_moved(state, view_config, editor);
                }
            }

            if shortcuts_manager.is_shortcut(TextEditingShortcut::BufferEnd) {
                if let Some(id) = state.text_id {
                    let editor = text.editor_mut(id);

                    if !select_modifier {
                        editor.set_selection(cosmic_text::Selection::None);
                    }

                    editor.action(
                        &mut fonts.font_system,
                        cosmic_text::Action::Motion(cosmic_text::Motion::BufferEnd),
                    );

                    on_editable_text_cursor_moved(state, view_config, editor);
                }
            }
        }

        if shortcuts_manager.is_shortcut(TextEditingShortcut::MovePrev) {
            if let Some(id) = state.text_id {
                let editor = text.editor_mut(id);

                if !has_selection || select_modifier {
                    if has_selection && !state.direction_decided {
                        state.direction_decided = true;
                        decide_editable_text_direction_prev(state, view_config, editor);
                    }

                    if word_modifier {
                        editor.action(
                            &mut fonts.font_system,
                            cosmic_text::Action::Motion(cosmic_text::Motion::LeftWord),
                        );
                    } else {
                        editor.action(
                            &mut fonts.font_system,
                            cosmic_text::Action::Motion(cosmic_text::Motion::Left),
                        );
                    }
                } else {
                    let bounds = editor.selection_bounds();

                    if let Some((start, end)) = bounds {
                        match view_config.layout_direction {
                            LayoutDirection::LTR => editor.set_cursor(start),
                            LayoutDirection::RTL => editor.set_cursor(end),
                        }
                    }

                    editor.set_selection(cosmic_text::Selection::None);

                    if word_modifier {
                        editor.action(
                            &mut fonts.font_system,
                            cosmic_text::Action::Motion(cosmic_text::Motion::LeftWord),
                        );
                    }
                }

                on_editable_text_cursor_moved(state, view_config, editor);
            }
        }

        if shortcuts_manager.is_shortcut(TextEditingShortcut::MoveNext) {
            if let Some(id) = state.text_id {
                let editor = text.editor_mut(id);

                if !has_selection || select_modifier {
                    if has_selection && !state.direction_decided {
                        state.direction_decided = true;
                        decide_editable_text_direction_next(state, view_config, editor);
                    }

                    if word_modifier {
                        editor.action(
                            &mut fonts.font_system,
                            cosmic_text::Action::Motion(cosmic_text::Motion::RightWord),
                        );
                    } else {
                        editor.action(
                            &mut fonts.font_system,
                            cosmic_text::Action::Motion(cosmic_text::Motion::Right),
                        );
                    }
                } else {
                    let bounds = editor.selection_bounds();

                    if let Some((start, end)) = bounds {
                        match view_config.layout_direction {
                            LayoutDirection::LTR => editor.set_cursor(end),
                            LayoutDirection::RTL => editor.set_cursor(start),
                        }
                    }

                    editor.set_selection(cosmic_text::Selection::None);

                    if word_modifier {
                        editor.action(
                            &mut fonts.font_system,
                            cosmic_text::Action::Motion(cosmic_text::Motion::RightWord),
                        );
                    }
                }

                on_editable_text_cursor_moved(state, view_config, editor);
            }
        }

        if shortcuts_manager.is_shortcut(TextEditingShortcut::SelectAll) {
            if let Some(id) = state.text_id {
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
        }

        #[cfg(feature = "clipboard")]
        if let Some(clipboard) = clipboard {
            if shortcuts_manager.is_shortcut(CommonShortcut::Copy) {
                if let Some(id) = state.text_id {
                    let editor = text.editor_mut(id);
                    let text = editor.copy_selection();

                    if let Some(text) = text {
                        if let Err(err) = clipboard.set_text(text) {
                            log::error!("Failed to copy to clipboard: {err}");
                        }
                    }
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

            if shortcuts_manager.is_shortcut(CommonShortcut::Paste) {
                if let Some(id) = state.text_id {
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
                                    text_before: selected_text
                                        .expect("Selection should be available"),
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
        }

        if shortcuts_manager.is_shortcut(CommonShortcut::Undo) {
            if let Some(id) = state.text_id {
                let editor = text.editor_mut(id);
                let delta = state.history_manager.undo(editor).cloned();

                on_editable_text_updated(state, view_config, editor, None);

                if let Some(delta) = delta {
                    state.deltas.push(EditableTextDelta::Undo(delta));
                }
            }
        }

        if shortcuts_manager.is_shortcut(CommonShortcut::Redo) {
            if let Some(id) = state.text_id {
                let editor = text.editor_mut(id);
                let delta = state.history_manager.redo(editor).cloned();

                on_editable_text_updated(state, view_config, editor, None);

                if let Some(delta) = delta {
                    state.deltas.push(EditableTextDelta::Apply(delta));
                }
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
                    {
                        if let Some(id) = state.text_id {
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
                                    text_before: selected_text
                                        .expect("Selection should be available"),
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
                    - boundary.x * view.scale_factor.ceil()
                    - state.text_offset.x;
                let relative_mouse_y = user_input.mouse_y as f32
                    - boundary.y * view.scale_factor.ceil()
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
                    } else if user_input.mouse_left_click_count % 2 == 0 {
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
                    let height = boundary.height * view.scale_factor.ceil();
                    let scroll_area_size = 8.0 * view.scale_factor.ceil();
                    let relative_mouse_y_f32 = relative_mouse_y as f32;
                    let at_top = relative_mouse_y_f32 <= scroll_area_size;
                    let at_bottom = relative_mouse_y_f32 >= height - scroll_area_size;

                    // Adjust overscroll speed
                    if at_top || at_bottom {
                        let mut distance = if at_top {
                            (relative_mouse_y_f32 - scroll_area_size).abs()
                        } else {
                            (relative_mouse_y_f32 - height + scroll_area_size).abs()
                        };

                        distance = f32::min(distance, 200.0);
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

                    if let Some((start, end)) = bounds {
                        if start == end {
                            editor.set_selection(cosmic_text::Selection::None);
                        }
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
                normalize_editable_text_selection(state, view_config, editor);
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
enum ParagraphMotionDirection {
    Up,
    Down,
}

fn move_paragraph(
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
            if start == end {
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

                on_editable_text_cursor_moved(state, view_config, editor);
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
