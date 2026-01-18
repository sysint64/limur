pub(crate) mod interaction;
pub(crate) mod render;

pub(crate) use render::render;

use std::time::Instant;

use clew_derive::{ShortcutId, ShortcutModifierId, ShortcutScopeId, WidgetBuilder, WidgetState};
use cosmic_text::Edit;

use crate::{
    AlignY, ColorRgba, Rect, TextAlign, Vec2, WidgetId, WidgetInteractionState, WidgetRef,
    WidgetType,
    layout::{DeriveWrapSize, LayoutCommand},
    text::{Text, TextId},
    text_data::TextData,
    text_history::{TextEditDelta, TextEditHistoryManager},
};

use super::{BuildContext, FrameBuilder, GestureDetectorResponse};

pub struct EditableTextWidget;

#[derive(WidgetBuilder)]
pub struct EditableTextBuilder<'a> {
    frame: FrameBuilder,
    color: ColorRgba,
    text_align: TextAlign,
    vertical_align: AlignY,
    text: &'a mut TextData,
    gesture_response: Option<GestureDetectorResponse>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EditableTextDelta {
    Undo(TextEditDelta),
    Apply(TextEditDelta),
}

#[derive(Clone, Debug)]
pub enum OsEvent {
    FocusWindow,
    CommitIme,
    ActivateIme,
    DeactivateIme,
}

#[derive(WidgetState, Clone, PartialEq)]
pub(crate) struct State {
    pub(crate) text_id: Option<TextId>,
    pub(crate) deltas: Vec<EditableTextDelta>,
    pub(crate) save_history: bool,
    pub(crate) scroll_x: f32,
    pub(crate) auto_scroll_to_cursor: bool,
    pub(crate) reached_end: bool,
    pub(crate) was_relayout: bool,
    pub(crate) recompose_text_content: bool,
    pub(crate) last_boundary_size: Vec2,
    pub(crate) ime_cursor_end: cosmic_text::Cursor,
    pub(crate) direction_decided: bool,
    pub(crate) text_offset: Vec2,
    pub(crate) history_manager: TextEditHistoryManager,
    pub(crate) multi_line: bool,
    pub(crate) auto_rtl: bool,
    pub(crate) visible_view_updated: bool,
    pub(crate) last_mouse_x: f32,
    pub(crate) last_mouse_y: f32,
    pub(crate) mouse_path_x: f32,
    pub(crate) mouse_path_y: f32,
    pub(crate) last_drag: Option<Instant>,
    pub(crate) color: ColorRgba,
    pub(crate) cursor_color: ColorRgba,
    pub(crate) selection_color: ColorRgba,
    pub(crate) selected_text_color: ColorRgba,
    pub(crate) ime_highlight_color: ColorRgba,
    pub(crate) ime_underline_color: ColorRgba,
    pub(crate) vertical_align: AlignY,
    pub(crate) gesture_detector_response: GestureDetectorResponse,
    pub(crate) boundary: Rect,
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            text_id: None,
            save_history: true,
            auto_scroll_to_cursor: false,
            reached_end: false,
            was_relayout: false,
            visible_view_updated: false,
            scroll_x: 0.0,
            recompose_text_content: true,
            ime_cursor_end: cosmic_text::Cursor::default(),
            direction_decided: false,
            text_offset: Vec2::ZERO,
            history_manager: TextEditHistoryManager::new(20, true),
            multi_line: true,
            auto_rtl: false,
            last_boundary_size: Vec2::ZERO,
            last_mouse_x: 0.,
            last_mouse_y: 0.,
            mouse_path_x: 0.,
            mouse_path_y: 0.,
            last_drag: None,
            deltas: vec![],
            color: ColorRgba::from_hex(0xFFFFFFFF),
            cursor_color: ColorRgba::from_hex(0xFF3366CC),
            selection_color: ColorRgba::from_hex(0xFF3366CC),
            selected_text_color: ColorRgba::from_hex(0xFF000000),
            ime_highlight_color: ColorRgba::from_hex(0x803366CC),
            ime_underline_color: ColorRgba::from_hex(0xFFFFFFFF),
            vertical_align: AlignY::Top,
            gesture_detector_response: GestureDetectorResponse::default(),
            boundary: Rect::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, ShortcutScopeId)]
pub enum ShortcutScopes {
    TextEditing,
}

#[derive(Debug, Clone, Copy, ShortcutId)]
pub enum CommonShortcut {
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, ShortcutId)]
pub enum TextEditingShortcut {
    Delete,
    Backspace,
    MoveStart,
    MoveEnd,
    MoveNext,
    MovePrev,
    MoveUp,
    MoveDown,
    NextLine,
    PageUp,
    PageDown,
    BufferStart,
    BufferEnd,
    SelectAll,
}

#[derive(Debug, Clone, Copy, ShortcutModifierId)]
pub enum TextInputModifier {
    Select,
    Word,
    Paragraph,
}

impl<'a> EditableTextBuilder<'a> {
    pub fn color(mut self, color: ColorRgba) -> Self {
        self.color = color;

        self
    }

    pub fn gesture_response(mut self, response: GestureDetectorResponse) -> Self {
        self.gesture_response = Some(response);

        self
    }

    pub fn text_align(mut self, text_align: TextAlign) -> Self {
        self.text_align = text_align;

        self
    }

    pub fn text_vertical_align(mut self, align_y: AlignY) -> Self {
        self.vertical_align = align_y;

        self
    }

    #[profiling::function]
    pub fn build(mut self, context: &mut BuildContext) {
        context
            .shortcuts_manager
            .push_scope(ShortcutScopes::TextEditing);

        let id = self.frame.id.with_seed(context.id_seed);
        let widget_ref = WidgetRef::new(WidgetType::of::<EditableTextWidget>(), id);

        let mut state = context
            .widgets_states
            .editable_text
            .get_or_insert(id, || State::new());

        let text_id = match self.text.text_id(id) {
            Some(text_id) => text_id,
            None => {
                let text_id = context.text.add_editor(
                    context.view,
                    context.fonts,
                    12.,
                    12.,
                    |fonts, text| text.set_text(fonts, &self.text.get_text()),
                );
                self.text.set_text_id(id, text_id);

                text_id
            }
        };

        state.text_id = self.text.text_id(id);
        state.color = self.color;
        state.vertical_align = self.vertical_align;

        if let Some(gesture_response) = self.gesture_response {
            state.gesture_detector_response = gesture_response.clone();

            interaction::handle_interaction(
                id,
                context.input,
                context.view,
                &gesture_response,
                state,
                context.os_events,
                context.text,
                context.fonts,
                context.view_config,
                context.shortcuts_manager,
                #[cfg(feature = "clipboard")]
                context.clipboard.as_mut(),
                state.boundary,
            );

            context
                .widgets_states
                .editable_text
                .accessed_this_frame
                .insert(id);
        } else {
        }

        let mut state = context
            .widgets_states
            .editable_text
            .get_or_insert(id, || State::new());

        context
            .text
            .shape_as_needed(text_id, &mut context.fonts.font_system, false);

        if !state.deltas.is_empty() {
            for delta in state.deltas.drain(..) {
                self.text.apply_delta(context.text, id, &delta);
            }

            if !self.frame.size.width.constrained() {
                let text = context.text.get_mut(text_id);

                text.with_buffer_mut(|buffer| {
                    buffer.set_size(&mut context.fonts.font_system, None, None);
                });

                context
                    .text
                    .shape_as_needed(text_id, &mut context.fonts.font_system, false);
            }
        } else if self.text.replace_buffer.contains(&id) {
            state.recompose_text_content = true;
            state.history_manager.clear();

            self.text.replace_buffer.remove(&id);

            let text = context.text.get_mut(text_id);
            let data = self.text.get_text();

            text.set_text(context.fonts, &data);

            if !self.frame.size.width.constrained() {
                text.with_buffer_mut(|buffer| {
                    buffer.set_size(&mut context.fonts.font_system, None, None);
                });
            }

            let editor = match text {
                Text::Buffer { .. } => panic!("Provided text id is not editor"),
                Text::Editor { editor, .. } => editor,
            };

            editor.set_cursor(cosmic_text::Cursor::default());

            interaction::on_editable_text_cursor_moved(
                &mut state,
                &mut context.view_config,
                editor,
            );
        }

        if self.text.dirty.contains(&id) {
            state.recompose_text_content = true;
            self.text.mark_as_not_dirty(&id);
        }

        let (backgrounds, foregrounds) = context.resolve_decorators(&mut self.frame);

        context.push_layout_command(LayoutCommand::Leaf {
            widget_ref,
            backgrounds,
            foregrounds,
            padding: self.frame.padding,
            margin: self.frame.margin,
            constraints: self.frame.constraints,
            size: self.frame.size,
            zindex: self.frame.zindex,
            derive_wrap_size: DeriveWrapSize::Text(text_id),
            clip: self.frame.clip,
        });

        context
            .shortcuts_manager
            .pop_scope(context.input, context.shortcuts_registry);
    }
}

#[track_caller]
pub fn editable_text(text: &mut TextData) -> EditableTextBuilder<'_> {
    EditableTextBuilder {
        frame: FrameBuilder::new(),
        text,
        color: ColorRgba::from_hex(0xFFFFFFFF),
        vertical_align: AlignY::Top,
        text_align: TextAlign::Left,
        gesture_response: None,
    }
}
