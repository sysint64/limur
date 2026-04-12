use clew_derive::WidgetState;

use crate::{
    Vec2, WidgetId,
    layout::{LayoutCommand, LayoutItem, LayoutState},
};

#[derive(WidgetState, Default)]
pub struct Layer {
    pub parent_layer_id: WidgetId,
    pub is_dirty: bool,
    pub bound_size: Vec2,
    pub layout_commands: Vec<LayoutCommand>,
    pub(crate) layout_items: Vec<LayoutItem>,
    pub(crate) layout_state: LayoutState,
}

impl Layer {
    pub fn new(parent_layer_id: WidgetId, bound_size: Vec2) -> Self {
        Self {
            parent_layer_id,
            is_dirty: true,
            bound_size,
            layout_commands: Vec::new(),
            layout_items: Vec::new(),
            layout_state: LayoutState::default(),
        }
    }
}
