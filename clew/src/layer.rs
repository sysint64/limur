use rustc_hash::FxHashSet;

use crate::{
    Size, Vec2, WidgetId,
    layout::{LayoutCommand, LayoutItem, LayoutState},
};

pub struct Layer {
    pub parent_layer_id: Option<WidgetId>,
    pub is_dirty: bool,
    pub invalidate: bool,
    pub wrap_size: Vec2,
    pub actual_size: Vec2,
    pub size: Size,
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
            invalidate: false,
            is_dirty: true,
            size: Size::default(),
            actual_size: Vec2::ZERO,
            wrap_size: Default::default(),
            layout_commands: Default::default(),
            layout_items: Default::default(),
            layout_state: Default::default(),
            accessed_this_frame: FxHashSet::default(),
            origin_position: Vec2::ZERO,
        }
    }
}
