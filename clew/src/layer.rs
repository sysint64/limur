use clew_derive::WidgetState;

use crate::{
    Vec2, WidgetId,
    layout::{LayoutCommand, LayoutItem, LayoutState},
    state::WidgetsStates,
};

pub struct Layer {
    pub parent_layer_id: Option<WidgetId>,
    pub is_dirty: bool,
    pub bound_size: Vec2,
    pub layout_commands: Vec<LayoutCommand>,
    pub(crate) layout_items: Vec<LayoutItem>,
    pub(crate) layout_state: LayoutState,
    pub(crate) widgets_state: WidgetsStates,
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            parent_layer_id: None,
            is_dirty: true,
            bound_size: Default::default(),
            layout_commands: Default::default(),
            layout_items: Default::default(),
            layout_state: Default::default(),
            widgets_state: Default::default(),
        }
    }
}

impl Layer {
    pub fn new(parent_layer_id: WidgetId, bound_size: Vec2) -> Self {
        Self {
            parent_layer_id: Some(parent_layer_id),
            is_dirty: true,
            bound_size,
            layout_commands: Vec::new(),
            layout_items: Vec::new(),
            layout_state: LayoutState::default(),
            widgets_state: Default::default(),
        }
    }
}
