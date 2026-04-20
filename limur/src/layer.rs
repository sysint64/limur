use rustc_hash::FxHashSet;

use crate::{
    Size, Vec2, WidgetId,
    layout::{LayoutCommand, LayoutItem, LayoutState},
    state::{STALE_THRESHOLD, TypedWidgetStates},
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

impl TypedWidgetStates<Layer> {
    /// Sweep dead layers and return the set of layer ids that survived.
    /// Layers retained by other live layers are kept alive transitively.
    pub fn sweep_layers(&mut self) -> FxHashSet<WidgetId> {
        let mut alive: FxHashSet<WidgetId> = FxHashSet::default();

        // Seed with directly-alive layers
        for i in 0..self.ids.len() {
            if self.current_frame.saturating_sub(self.last_accessed[i]) <= STALE_THRESHOLD {
                alive.insert(self.ids[i]);
            }
        }

        // Propagate: retained-by chains
        loop {
            let mut changed = false;
            for i in 0..self.ids.len() {
                if alive.contains(&self.ids[i]) {
                    continue;
                }
                if let Some(retainer) = self.retained_by[i] {
                    if alive.contains(&retainer) {
                        alive.insert(self.ids[i]);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }

        // Sweep dead layers
        let mut i = 0;
        while i < self.states.len() {
            if !alive.contains(&self.ids[i]) {
                self.id_to_index.remove(&self.ids[i]);

                self.states.swap_remove(i);
                self.ids.swap_remove(i);
                self.last_accessed.swap_remove(i);
                self.retained_by.swap_remove(i);

                if i < self.ids.len() {
                    self.id_to_index.insert(self.ids[i], i as u32);
                }
            } else {
                i += 1;
            }
        }

        alive
    }
}
