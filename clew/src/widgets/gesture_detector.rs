use crate::{
    View, WidgetId, WidgetRef, WidgetType, impl_id, interaction::InteractionState, io::UserInput,
    state::WidgetState,
};
use std::any::Any;

use super::builder::BuildContext;

pub struct GestureDetectorBuilder {
    id: WidgetId,
    focusable: bool,
    clickable: bool,
    dragable: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    clicked: bool,
    is_active: bool,
    is_hot: bool,
    is_focused: bool,
    was_focused: bool,
    clickable: bool,
    dragable: bool,
    focusable: bool,
    drag_start_x: f32,
    drag_start_y: f32,
    last_x: f32,
    last_y: f32,
    drag_x: f32,
    drag_y: f32,
    drag_delta_x: f32,
    drag_delta_y: f32,
    drag_state: DragState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DragState {
    #[default]
    None,
    Start,
    Update,
    End,
}

pub struct GestureDetector;

impl WidgetState for State {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[derive(Default, Clone, PartialEq)]
pub struct GestureDetectorResponse {
    pub clicked: bool,
    pub is_active: bool,
    pub is_hot: bool,
    pub is_focused: bool,
    pub was_focused: bool,
    pub drag_start_x: f32,
    pub drag_start_y: f32,
    pub drag_x: f32,
    pub drag_y: f32,
    pub drag_delta_x: f32,
    pub drag_delta_y: f32,
    pub drag_state: DragState,
}

impl GestureDetectorResponse {
    #[inline]
    pub fn clicked(&self) -> bool {
        self.clicked
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    #[inline]
    pub fn is_hot(&self) -> bool {
        self.is_hot
    }

    #[inline]
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    #[inline]
    pub fn was_focused(&self) -> bool {
        self.was_focused
    }
}

impl GestureDetectorBuilder {
    impl_id!();

    pub fn clickable(mut self, value: bool) -> Self {
        self.clickable = value;

        self
    }

    pub fn focusable(mut self, value: bool) -> Self {
        self.focusable = value;

        self
    }

    pub fn dragable(mut self, value: bool) -> Self {
        self.dragable = value;

        self
    }

    #[profiling::function]
    pub fn build<F>(self, context: &mut BuildContext, callback: F) -> GestureDetectorResponse
    where
        F: FnOnce(&mut BuildContext),
    {
        let id = self.id.with_seed(context.id_seed);
        let widget_ref = WidgetRef::new(WidgetType::of::<GestureDetector>(), id);

        let state = context
            .widgets_states
            .gesture_detector
            .get_or_insert(id, State::default);

        state.clickable = self.clickable;
        state.dragable = self.dragable;
        state.focusable = self.focusable;

        handle_interaction(id, context.input, context.view, context.interaction, state);

        let response = GestureDetectorResponse {
            clicked: state.clicked,
            is_active: state.is_active,
            is_hot: state.is_hot,
            is_focused: state.is_focused,
            was_focused: state.was_focused,
            drag_start_x: state.drag_start_x,
            drag_start_y: state.drag_start_y,
            drag_x: state.drag_x,
            drag_y: state.drag_y,
            drag_delta_x: state.drag_delta_x,
            drag_delta_y: state.drag_delta_y,
            drag_state: state.drag_state,
        };

        context.foregrounds.push(widget_ref);
        context.provide(response.clone(), callback);

        context
            .widgets_states
            .gesture_detector
            .accessed_this_frame
            .insert(id);

        response
    }
}

#[track_caller]
pub fn gesture_detector() -> GestureDetectorBuilder {
    GestureDetectorBuilder {
        id: WidgetId::auto(),
        clickable: false,
        dragable: false,
        focusable: false,
    }
}

pub fn handle_interaction(
    id: WidgetId,
    input: &UserInput,
    view: &View,
    interaction: &mut InteractionState,
    widget_state: &mut State,
) {
    widget_state.clicked = false;

    if widget_state.dragable {
        widget_state.drag_state = match widget_state.drag_state {
            DragState::None => DragState::None,
            DragState::Start => DragState::Update,
            DragState::Update => DragState::Update,
            DragState::End => DragState::None,
        };
    }

    if widget_state.clickable || widget_state.dragable {
        if interaction.is_active(&id) {
            if input.mouse_released {
                if interaction.is_hot(&id) {
                    interaction.set_inactive(&id);
                    widget_state.clicked = widget_state.clickable;

                    if widget_state.focusable {
                        interaction.set_focused(id);
                    }
                } else {
                    interaction.set_inactive(&id);
                }

                if widget_state.dragable && widget_state.drag_state == DragState::Update {
                    widget_state.drag_state = DragState::End;
                }
            }
        } else if input.mouse_left_pressed
            && interaction.is_hot(&id)
            && interaction.active.is_none()
        {
            if widget_state.dragable && widget_state.drag_state == DragState::None {
                widget_state.drag_state = DragState::Start;
            }

            if widget_state.focusable {
                interaction.set_focused(id);
            }

            interaction.set_active(&id);
            interaction.block_hover = widget_state.dragable;
        }
    }

    if widget_state.dragable {
        match widget_state.drag_state {
            DragState::None => {
                widget_state.drag_start_x = 0.;
                widget_state.drag_start_y = 0.;
                widget_state.last_x = 0.;
                widget_state.last_y = 0.;
                widget_state.drag_delta_x = 0.;
                widget_state.drag_delta_y = 0.;
            }
            DragState::Start => {
                widget_state.drag_start_x = input.mouse_x / view.scale_factor;
                widget_state.drag_start_y = input.mouse_y / view.scale_factor;
                widget_state.last_x = input.mouse_x / view.scale_factor;
                widget_state.last_y = input.mouse_y / view.scale_factor;
                widget_state.drag_delta_x = 0.;
                widget_state.drag_delta_y = 0.;
            }
            DragState::Update => {
                widget_state.drag_x = input.mouse_x / view.scale_factor;
                widget_state.drag_y = input.mouse_y / view.scale_factor;
                widget_state.drag_delta_x = widget_state.drag_x - widget_state.last_x;
                widget_state.drag_delta_y = widget_state.drag_y - widget_state.last_y;
                widget_state.last_x = input.mouse_x / view.scale_factor;
                widget_state.last_y = input.mouse_y / view.scale_factor;
            }
            DragState::End => {
                widget_state.drag_start_x = 0.;
                widget_state.drag_start_y = 0.;
                widget_state.last_x = 0.;
                widget_state.last_y = 0.;
                widget_state.drag_delta_x = 0.;
                widget_state.drag_delta_y = 0.;
            }
        }
    }

    widget_state.is_active = interaction.is_active(&id);
    widget_state.is_hot = interaction.is_hot(&id);
    widget_state.is_focused = interaction.is_focused(&id);
    widget_state.was_focused = interaction.was_focused(&id);
}
