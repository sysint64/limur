use rustc_hash::FxHashSet;

use crate::{
    Vec2, View, WidgetId, WidgetType,
    io::UserInput,
    layer::Layer,
    layout::LayoutItem,
    point_with_rect_hit_test,
    state::{TypedWidgetStates, UiState, WidgetsStates},
    widgets,
};

#[derive(Default, Clone, PartialEq)]
pub struct InteractionState {
    pub(crate) hover: FxHashSet<WidgetId>,
    pub(crate) hot: Option<WidgetId>,
    pub(crate) active: Option<WidgetId>,
    focused: Option<WidgetId>,
    pub(crate) was_focused: Option<WidgetId>,
    pub(crate) block_hover: bool,
}

#[derive(Default, Clone, PartialEq)]
pub struct WidgetInteractionState {
    pub is_hover: bool,
    pub is_hot: bool,
    pub is_active: bool,
    pub is_focused: bool,
    pub was_focused: bool,
}

impl InteractionState {
    pub fn is_hover(&self, id: &WidgetId) -> bool {
        self.hover.contains(id)
    }

    #[allow(dead_code)]
    pub(crate) fn clear_focused(&mut self) {
        if let Some(was_focused_id) = self.focused {
            self.was_focused = Some(was_focused_id);
        }

        self.focused = None;
    }

    pub(crate) fn set_focused(&mut self, id: WidgetId) {
        if let Some(was_focused_id) = self.focused
            && was_focused_id != id
        {
            self.was_focused = Some(was_focused_id);
        }

        self.focused = Some(id);
    }

    pub(crate) fn is_hot(&self, id: &WidgetId) -> bool {
        self.hot == Some(*id)
    }

    pub(crate) fn is_active(&self, id: &WidgetId) -> bool {
        self.active == Some(*id)
    }

    pub(crate) fn is_focused(&self, id: &WidgetId) -> bool {
        self.focused == Some(*id)
    }

    pub(crate) fn was_focused(&self, id: &WidgetId) -> bool {
        self.was_focused == Some(*id)
    }

    pub(crate) fn set_active(&mut self, id: &WidgetId) {
        self.active = Some(*id);
    }

    pub(crate) fn set_inactive(&mut self, id: &WidgetId) {
        if self.is_active(id) {
            self.active = None;
            self.block_hover = false;
        }
    }
}

pub fn handle_interaction_before_build(user_input: &mut UserInput, view: &View) {
    if user_input.mouse_left_pressed {
        user_input.mouse_left_click_count = user_input.mouse_left_click_tracker.on_click(
            user_input.mouse_x,
            user_input.mouse_y,
            view.scale_factor,
        );
    }
}

pub struct InteractionContext<'a> {
    pub(crate) user_input: &'a mut UserInput,
    pub(crate) view: &'a View,
    pub(crate) interaction_state: &'a mut InteractionState,
    pub(crate) last_interaction_state: &'a mut InteractionState,
    pub(crate) layout_items: &'a [LayoutItem],
    pub(crate) non_interactable: &'a FxHashSet<WidgetId>,
    pub(crate) widgets_states: &'a mut WidgetsStates,
    pub(crate) layers: &'a mut TypedWidgetStates<Layer>,
    pub(crate) root_layer: &'a mut Layer,
}

impl<'a> InteractionContext<'a> {
    pub fn new(state: &'a mut UiState) -> Self {
        Self {
            user_input: &mut state.user_input,
            view: &state.view,
            interaction_state: &mut state.interaction_state,
            last_interaction_state: &mut state.last_interaction_state,
            layout_items: &state.clipped_layout_items,
            non_interactable: &state.non_interactable,
            widgets_states: &mut state.widgets_states,
            layers: &mut state.layers,
            root_layer: &mut state.root_layer,
        }
    }
}

pub fn handle_interaction(ctx: &mut InteractionContext) -> bool {
    let unscaled_mouse_x = ctx.user_input.mouse_x / ctx.view.scale_factor;
    let unscaled_mouse_y = ctx.user_input.mouse_y / ctx.view.scale_factor;

    let mouse_point = Vec2::new(unscaled_mouse_x, unscaled_mouse_y);

    ctx.interaction_state.hot = None;
    ctx.interaction_state.hover.clear();

    for layout_item in ctx.layout_items.iter() {
        if let LayoutItem::Placement(placement) = layout_item
            && point_with_rect_hit_test(mouse_point, placement.boundary)
        {
            ctx.interaction_state.hover.insert(placement.widget_ref.id);
        }
    }

    for layout_item in ctx.layout_items.iter().rev() {
        if let LayoutItem::Placement(placement) = layout_item
            && !ctx.non_interactable.contains(&placement.widget_ref.id)
            && (!ctx.interaction_state.block_hover
                || ctx.interaction_state.active.is_none()
                || ctx.interaction_state.active == Some(placement.widget_ref.id))
            && point_with_rect_hit_test(mouse_point, placement.boundary)
        {
            ctx.interaction_state.hot = Some(placement.widget_ref.id);
            break;
        }
    }

    let mut is_dirty = false;

    for layout_item in ctx.layout_items.iter() {
        match layout_item {
            LayoutItem::Placement(placement) => {
                if placement.widget_ref.widget_type
                    == WidgetType::of::<widgets::gesture_detector::GestureDetector>()
                {
                    is_dirty = is_dirty
                        || widgets::gesture_detector::handle_interaction(
                            ctx,
                            placement.widget_ref.id,
                        );
                }
            }
            _ => {}
        }
    }

    let state_updated = ctx.interaction_state != ctx.last_interaction_state;
    *ctx.last_interaction_state = ctx.interaction_state.clone();

    ctx.user_input.reset();
    ctx.user_input.clear_frame_events();

    is_dirty || state_updated
}
