use crate::{
    View, WidgetId, WidgetRef, WidgetType, impl_id,
    interaction::{InteractionContext, InteractionState},
    io::UserInput,
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
    layer_id: Option<WidgetId>,
    clicked: bool,
    is_active: bool,
    is_hot: bool,
    is_focused: bool,
    was_focused: bool,
    clickable: bool,
    dragable: bool,
    focusable: bool,
    drag_start_x: f64,
    drag_start_y: f64,
    last_x: f64,
    last_y: f64,
    drag_x: f64,
    drag_y: f64,
    drag_delta_x: f64,
    drag_delta_y: f64,
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
    pub id: WidgetId,
    pub clicked: bool,
    pub is_active: bool,
    pub is_hot: bool,
    pub is_focused: bool,
    pub was_focused: bool,
    pub drag_start_x: f64,
    pub drag_start_y: f64,
    pub drag_x: f64,
    pub drag_y: f64,
    pub drag_delta_x: f64,
    pub drag_delta_y: f64,
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

        state.layer_id = context.layer_id;
        state.clickable = self.clickable;
        state.dragable = self.dragable;
        state.focusable = self.focusable;

        let response = GestureDetectorResponse {
            id,
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
        context.accessed_this_frame(id);

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

pub fn set_active(context: &mut BuildContext, id: WidgetId, value: bool) {
    let state = context.widgets_states.gesture_detector.get_mut(id);

    if let Some(state) = state {
        state.is_active = value;
    }
}

pub fn set_clicked(context: &mut BuildContext, id: WidgetId, value: bool) {
    let state = context.widgets_states.gesture_detector.get_mut(id);

    if let Some(state) = state {
        state.clicked = value;
    }
}

pub fn handle_interaction(ctx: &mut InteractionContext, id: WidgetId) -> bool {
    // let Some(widget_state) = ctx.widgets_states.gesture_detector.get_mut(id) else {
    // return false;
    // };
    let widget_state = ctx.widgets_states.gesture_detector.get_mut(id).unwrap();

    let state = widget_state.clone();

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
        if ctx.interaction_state.is_active(&id) {
            if ctx.user_input.mouse_released {
                if ctx.interaction_state.is_hot(&id) {
                    ctx.interaction_state.set_inactive(&id);
                    widget_state.clicked = widget_state.clickable;

                    if widget_state.focusable {
                        ctx.interaction_state.set_focused(id);
                    }
                } else {
                    ctx.interaction_state.set_inactive(&id);
                }

                if widget_state.dragable && widget_state.drag_state == DragState::Update {
                    widget_state.drag_state = DragState::End;
                }
            }
        } else if ctx.user_input.mouse_left_pressed
            && ctx.interaction_state.is_hot(&id)
            && ctx.interaction_state.active.is_none()
        {
            if widget_state.dragable && widget_state.drag_state == DragState::None {
                widget_state.drag_state = DragState::Start;
            }

            if widget_state.focusable {
                ctx.interaction_state.set_focused(id);
            }

            ctx.interaction_state.set_active(&id);
            ctx.interaction_state.block_hover = widget_state.dragable;
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
                widget_state.drag_start_x = ctx.user_input.mouse_x / ctx.view.scale_factor;
                widget_state.drag_start_y = ctx.user_input.mouse_y / ctx.view.scale_factor;
                widget_state.last_x = ctx.user_input.mouse_x / ctx.view.scale_factor;
                widget_state.last_y = ctx.user_input.mouse_y / ctx.view.scale_factor;
                widget_state.drag_delta_x = 0.;
                widget_state.drag_delta_y = 0.;
            }
            DragState::Update => {
                widget_state.drag_x = ctx.user_input.mouse_x / ctx.view.scale_factor;
                widget_state.drag_y = ctx.user_input.mouse_y / ctx.view.scale_factor;
                widget_state.drag_delta_x = widget_state.drag_x - widget_state.last_x;
                widget_state.drag_delta_y = widget_state.drag_y - widget_state.last_y;
                widget_state.last_x = ctx.user_input.mouse_x / ctx.view.scale_factor;
                widget_state.last_y = ctx.user_input.mouse_y / ctx.view.scale_factor;
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

    widget_state.is_active = ctx.interaction_state.is_active(&id);
    widget_state.is_hot = ctx.interaction_state.is_hot(&id);
    widget_state.is_focused = ctx.interaction_state.is_focused(&id);
    widget_state.was_focused = ctx.interaction_state.was_focused(&id);

    let has_changed = state != *widget_state;

    if has_changed {
        let mut layer_id_option: Option<WidgetId> = widget_state.layer_id;

        while let Some(layer_id) = layer_id_option {
            let layer = ctx.layers.get_mut(layer_id).unwrap();
            layer.is_dirty = true;
            layer_id_option = layer.parent_layer_id;
        }
    }

    has_changed
}
