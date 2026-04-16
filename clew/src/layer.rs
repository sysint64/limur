use rustc_hash::FxHashSet;

use crate::{
    Vec2, WidgetId,
    layout::{LayoutCommand, LayoutItem, LayoutState},
};

pub struct Layer {
    pub parent_layer_id: Option<WidgetId>,
    pub is_dirty: bool,
    pub wrap_size: Vec2,
    pub origin_position: Vec2,
    pub layout_commands: Vec<LayoutCommand>,
    pub(crate) layout_items: Vec<LayoutItem>,
    pub(crate) layout_state: LayoutState,
    pub(crate) accessed_this_frame: FxHashSet<WidgetId>,
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            parent_layer_id: None,
            is_dirty: true,
            wrap_size: Default::default(),
            layout_commands: Default::default(),
            layout_items: Default::default(),
            layout_state: Default::default(),
            accessed_this_frame: FxHashSet::default(),
            origin_position: Vec2::ZERO,
        }
    }
}

impl Layer {
    pub fn new(parent_layer_id: WidgetId, bound_size: Vec2) -> Self {
        Self {
            parent_layer_id: Some(parent_layer_id),
            is_dirty: true,
            wrap_size: bound_size,
            layout_commands: Vec::new(),
            layout_items: Vec::new(),
            layout_state: LayoutState::default(),
            accessed_this_frame: FxHashSet::default(),
            origin_position: Vec2::ZERO,
        }
    }
}
