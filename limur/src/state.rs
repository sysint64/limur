use std::{
    any::Any,
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(feature = "clipboard")]
use arboard::Clipboard;
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

use crate::{
    LayoutDirection, Rect, ShortcutsRegistry, View, WidgetId, WidgetRef, backdrop_filter,
    editable_text::{self, OsEvent},
    hstack,
    interaction::InteractionState,
    io::UserInput,
    layer::Layer,
    layout::{LayoutMeasure, WidgetPlacement},
    render::RenderState,
    shortcuts::ShortcutsManager,
    vstack,
    widgets::{decorated_box, gesture_detector, scroll_area, svg, text},
    zstack,
};

pub trait WidgetState: Any + Send + 'static {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

#[derive(Default, Clone, Copy)]
pub struct PerformanceMetrics {
    pub build_time_pass1: Duration,
    pub build_time_pass2: Duration,
    pub build_total: Duration,
    pub layout_pass1: Duration,
    pub layout_pass2: Duration,
    pub layout_total: Duration,
    pub render: Duration,
    pub cycle: Duration,
}

pub struct UiState {
    pub root_layer: Layer,
    pub cycle_timer: Instant,
    pub performance_metrics: PerformanceMetrics,
    pub current_event_queue: Vec<Arc<dyn Any + Send>>,
    pub next_event_queue: Vec<Arc<dyn Any + Send>>,
    pub view: View,
    pub render_state: RenderState,
    // pub layout_commands: Vec<LayoutCommand>,
    pub phase_allocator: bumpalo::Bump,
    // pub(crate) layout_state: LayoutState,
    pub(crate) widgets_states: WidgetsStates,
    pub(crate) widget_placements: Vec<WidgetPlacement>,
    // pub(crate) layout_items: Vec<LayoutItem>,
    // pub(crate) clipped_layout_items: Vec<LayoutItem>,
    pub interaction_state: InteractionState,
    pub last_interaction_state: InteractionState,
    pub user_input: UserInput,
    pub backgrounds: SmallVec<[WidgetRef; 8]>,
    pub foregrounds: SmallVec<[WidgetRef; 8]>,
    pub non_interactable: FxHashSet<WidgetId>,
    pub animations_stepped_this_frame: FxHashSet<usize>,
    // TODO(sysint64): Maybe move it to build context
    pub layout_direction: LayoutDirection,
    pub async_tx: tokio::sync::mpsc::UnboundedSender<Box<dyn Any + Send>>,
    pub async_rx: tokio::sync::mpsc::UnboundedReceiver<Box<dyn Any + Send>>,
    pub(crate) shortcuts_manager: ShortcutsManager,
    pub(crate) shortcuts_registry: ShortcutsRegistry,
    pub os_events: SmallVec<[OsEvent; 4]>,
    pub view_config: ViewConfig,
    #[cfg(feature = "clipboard")]
    pub(crate) clipboard: Option<Clipboard>,
    pub(crate) layers: TypedWidgetStates<Layer>,
}

#[derive(Default)]
pub(crate) struct WidgetsStates {
    // pub data: FxHashMap<WidgetId, Box<dyn WidgetState>>,
    // pub last: FxHashMap<WidgetId, Box<dyn WidgetState>>,
    pub(crate) layout_measures: TypedWidgetStates<LayoutMeasure>,

    // frame number of last access of layers
    pub(crate) layers_last_accessed: FxHashMap<WidgetId, u64>,

    pub(crate) decorated_box: TypedWidgetStates<decorated_box::State>,
    pub(crate) backdrop_filter: TypedWidgetStates<backdrop_filter::State>,
    pub(crate) scroll_area: TypedWidgetStates<scroll_area::State>,
    pub(crate) text: TypedWidgetStates<text::State>,
    pub(crate) vstack: TypedWidgetStates<vstack::State>,
    pub(crate) hstack: TypedWidgetStates<hstack::State>,
    pub(crate) zstack: TypedWidgetStates<zstack::State>,
    pub(crate) editable_text: TypedWidgetStates<editable_text::State>,
    pub(crate) gesture_detector: TypedWidgetStates<gesture_detector::State>,
    pub(crate) svg: TypedWidgetStates<svg::State>,
    #[allow(dead_code)]
    pub(crate) components: TypedWidgetStates<Box<dyn Any>>,
    pub(crate) custom: TypedWidgetStates<Option<Box<dyn WidgetState>>>,
    pub(crate) accessed_this_frame: FxHashSet<WidgetId>,
}

pub(crate) const STALE_THRESHOLD: u64 = 20;

#[derive(Default)]
pub struct ViewConfig {
    pub ime_cursor_rect: Rect<f32>,
    pub should_use_wide_space: bool,
    pub layout_direction: LayoutDirection,
    pub should_update_cursor_each_frame: bool,
}

pub struct TypedWidgetStates<T> {
    pub(crate) id_to_index: FxHashMap<WidgetId, u32>,
    pub(crate) states: Vec<T>,
    pub(crate) retained_by: Vec<Option<WidgetId>>,
    pub(crate) ids: Vec<WidgetId>,
    // frame number of last access
    pub(crate) last_accessed: Vec<u64>,
    // where we left off last frame
    pub(crate) sweep_cursor: usize,
    pub(crate) current_frame: u64,
    pub(crate) current_layer: Option<WidgetId>,
}

impl<T> Default for TypedWidgetStates<T> {
    fn default() -> Self {
        Self {
            id_to_index: FxHashMap::default(),
            states: Vec::new(),
            ids: Vec::new(),
            last_accessed: Vec::new(),
            retained_by: Vec::new(),
            sweep_cursor: 0,
            current_frame: 0,
            current_layer: None,
        }
    }
}

impl<T> TypedWidgetStates<T> {
    /// Stamp last_accessed and update retained_by based on current_layer.
    fn touch_index(&mut self, index: u32) {
        let i = index as usize;
        self.last_accessed[i] = self.current_frame;
        self.retained_by[i] = self.current_layer;
    }

    pub fn touch(&mut self, index: u32) {
        self.touch_index(index);
    }

    pub fn touch_if_present(&mut self, id: WidgetId) {
        if let Some(&idx) = self.id_to_index.get(&id) {
            self.touch_index(idx);
        }
    }

    pub fn get_or_insert(&mut self, id: WidgetId, create: impl FnOnce() -> T) -> &mut T {
        let states = &mut self.states;
        let ids = &mut self.ids;
        let last_accessed = &mut self.last_accessed;
        let retained_by = &mut self.retained_by;
        let current_frame = self.current_frame;
        let current_layer = self.current_layer;

        let index = *self.id_to_index.entry(id).or_insert_with(|| {
            let idx = states.len() as u32;
            states.push(create());
            ids.push(id);
            last_accessed.push(current_frame);
            retained_by.push(current_layer);
            idx
        });

        let i = index as usize;
        self.last_accessed[i] = self.current_frame;
        self.retained_by[i] = self.current_layer;
        &mut self.states[i]
    }

    pub fn get_mut(&mut self, id: WidgetId) -> Option<&mut T> {
        self.id_to_index
            .get(&id)
            .copied()
            .map(|idx| &mut self.states[idx as usize])

        // self.id_to_index.get(&id).copied().map(|idx| {
        //     self.touch_index(idx);
        //     &mut self.states[idx as usize]
        // })
    }

    pub fn get(&self, id: WidgetId) -> Option<&T> {
        self.id_to_index
            .get(&id)
            .map(|&idx| &self.states[idx as usize])
    }

    // pub fn peek_mut(&mut self, id: WidgetId) -> Option<&mut T> {
    //     self.id_to_index
    //         .get(&id)
    //         .copied()
    //         .map(|idx| &mut self.states[idx as usize])
    // }

    // pub fn peek(&self, id: WidgetId) -> Option<&T> {
    //     self.id_to_index
    //         .get(&id)
    //         .map(|&idx| &self.states[idx as usize])
    // }

    pub fn replace(&mut self, id: WidgetId, state: T) {
        if let Some(&idx) = self.id_to_index.get(&id) {
            self.states[idx as usize] = state;
            self.touch_index(idx);
        } else {
            let idx = self.states.len() as u32;
            self.id_to_index.insert(id, idx);
            self.states.push(state);
            self.ids.push(id);
            self.last_accessed.push(self.current_frame);
            self.retained_by.push(self.current_layer);
        }
    }

    pub fn set(&mut self, id: WidgetId, state: T) -> usize {
        if let Some(&idx) = self.id_to_index.get(&id) {
            self.states[idx as usize] = state;
            self.touch_index(idx);
            idx as usize
        } else {
            let idx = self.states.len() as u32;
            self.id_to_index.insert(id, idx);
            self.states.push(state);
            self.ids.push(id);
            self.last_accessed.push(self.current_frame);
            self.retained_by.push(self.current_layer);
            idx as usize
        }
    }

    pub fn sweep(&mut self, is_retainer_alive: impl Fn(WidgetId) -> bool) {
        let mut i = 0;

        while i < self.states.len() {
            let is_stale =
                self.current_frame.saturating_sub(self.last_accessed[i]) > STALE_THRESHOLD;

            let is_retained =
                self.retained_by[i].map_or(false, |layer_id| is_retainer_alive(layer_id));

            if is_stale && !is_retained {
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
    }

    /// Incrementally sweep up to `budget` entries, removing any state that
    /// hasn't been accessed within the last `STALE_THRESHOLD` frames and
    /// is not retained by a live layer.
    // pub fn incremental_sweep(
    //     &mut self,
    //     budget: usize,
    //     is_retainer_alive: impl Fn(WidgetId) -> bool,
    // ) {
    //     if self.states.is_empty() {
    //         self.sweep_cursor = 0;
    //         return;
    //     }

    //     debug_assert_eq!(self.states.len(), self.last_accessed.len());
    //     debug_assert_eq!(self.states.len(), self.retained_by.len());

    //     let mut remaining = budget;

    //     while remaining > 0 && !self.states.is_empty() {
    //         if self.sweep_cursor >= self.states.len() {
    //             self.sweep_cursor = 0;
    //             break;
    //         }

    //         let i = self.sweep_cursor;

    //         let is_stale =
    //             self.current_frame.saturating_sub(self.last_accessed[i]) > STALE_THRESHOLD;

    //         let is_retained =
    //             self.retained_by[i].map_or(false, |layer_id| is_retainer_alive(layer_id));

    //         if is_stale && !is_retained {
    //             self.id_to_index.remove(&self.ids[i]);

    //             self.states.swap_remove(i);
    //             self.ids.swap_remove(i);
    //             self.last_accessed.swap_remove(i);
    //             self.retained_by.swap_remove(i);

    //             if i < self.ids.len() {
    //                 self.id_to_index.insert(self.ids[i], i as u32);
    //             }
    //         } else {
    //             self.sweep_cursor += 1;
    //         }

    //         remaining -= 1;
    //     }
    // }

    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn clear(&mut self) {
        self.id_to_index.clear();
        self.states.clear();
        self.ids.clear();
        self.last_accessed.clear();
        self.retained_by.clear();
        self.sweep_cursor = 0;
    }
}

// impl<T> TypedWidgetStates<T> {
// pub fn get_or_insert(&mut self, id: WidgetId, create: impl FnOnce() -> T) -> &mut T {
//     let index = *self.id_to_index.entry(id).or_insert_with(|| {
//         let idx = self.states.len() as u32;
//         self.states.push(create());
//         self.ids.push(id);
//         idx
//     });
//     &mut self.states[index as usize]
// }

// pub fn get_mut(&mut self, id: WidgetId) -> Option<&mut T> {
//     self.id_to_index
//         .get(&id)
//         .map(|&idx| &mut self.states[idx as usize])
// }

// pub fn get(&self, id: WidgetId) -> Option<&T> {
//     self.id_to_index
//         .get(&id)
//         .map(|&idx| &self.states[idx as usize])
// }

// pub fn replace(&mut self, id: WidgetId, state: T) {
//     if let Some(&idx) = self.id_to_index.get(&id) {
//         self.states[idx as usize] = state;
//     } else {
//         let idx = self.states.len() as u32;
//         self.id_to_index.insert(id, idx);
//         self.states.push(state);
//         self.ids.push(id);
//     }
// }

// pub fn set(&mut self, id: WidgetId, state: T) -> usize {
//     if let Some(&idx) = self.id_to_index.get(&id) {
//         self.states[idx as usize] = state;

//         idx as usize
//     } else {
//         let idx = self.states.len() as u32;
//         self.id_to_index.insert(id, idx);
//         self.states.push(state);
//         self.ids.push(id);

//         idx as usize
//     }
// }

// pub fn sweep(&mut self, accessed_this_frame: &FxHashSet<WidgetId>) {
//     let mut i = 0;

//     while i < self.states.len() {
//         if accessed_this_frame.contains(&self.ids[i]) {
//             i += 1;
//         } else {
//             // Swap-remove from both parallel arrays
//             self.id_to_index.remove(&self.ids[i]);

//             self.states.swap_remove(i);
//             self.ids.swap_remove(i);

//             // Update the index of the element that was swapped in
//             if i < self.ids.len() {
//                 self.id_to_index.insert(self.ids[i], i as u32);
//             }
//         }
//     }
// }

// pub fn clear(&mut self) {
//     self.id_to_index.clear();
//     self.states.clear();
//     self.ids.clear();
// }
// }

impl UiState {
    pub fn shortcuts_manager(&mut self) -> &mut ShortcutsManager {
        &mut self.shortcuts_manager
    }

    pub fn shortcuts_registry(&mut self) -> &mut ShortcutsRegistry {
        &mut self.shortcuts_registry
    }

    pub fn new(view: View) -> Self {
        let (async_tx, async_rx) = tokio::sync::mpsc::unbounded_channel();

        let phase_allocator = bumpalo::Bump::with_capacity(16 * 1024 * 1024);

        #[cfg(feature = "clipboard")]
        let clipboard = {
            let clipboard = Clipboard::new();

            match clipboard {
                Ok(clipboard) => {
                    log::info!("Successfully initialized clipboard");
                    Some(clipboard)
                }
                Err(err) => {
                    log::error!("Failed to initialize clipboard: {err}");
                    None
                }
            }
        };

        let root_layer = Layer::default();

        Self {
            view,
            render_state: Default::default(),
            phase_allocator,
            // layout_commands: Vec::new(),
            current_event_queue: Vec::new(),
            next_event_queue: Vec::new(),
            widgets_states: WidgetsStates::default(),
            // layout_state: LayoutState::default(),
            widget_placements: Vec::new(),
            // layout_items: Vec::new(),
            backgrounds: SmallVec::new(),
            foregrounds: SmallVec::new(),
            interaction_state: InteractionState::default(),
            last_interaction_state: InteractionState::default(),
            user_input: UserInput::default(),
            layout_direction: LayoutDirection::LTR,
            non_interactable: FxHashSet::default(),
            animations_stepped_this_frame: FxHashSet::default(),
            async_tx,
            async_rx,
            shortcuts_manager: ShortcutsManager::default(),
            shortcuts_registry: ShortcutsRegistry::default(),
            os_events: SmallVec::new(),
            view_config: ViewConfig::default(),
            #[cfg(feature = "clipboard")]
            clipboard,
            layers: TypedWidgetStates::default(),
            cycle_timer: Instant::now(),
            performance_metrics: PerformanceMetrics::default(),
            root_layer,
        }
    }
}

impl WidgetsStates {
    pub fn _get_or_insert_custom<T: WidgetState, F>(&mut self, id: WidgetId, create: F) -> &mut T
    where
        F: FnOnce() -> T,
    {
        let index = *self.custom.id_to_index.entry(id).or_insert_with(|| {
            let idx = self.custom.states.len() as u32;
            self.custom.states.push(Some(Box::new(create())));
            self.custom.ids.push(id);
            idx
        });

        self.custom.states[index as usize]
            .as_mut()
            .unwrap()
            .as_any_mut()
            .downcast_mut::<T>()
            .unwrap()

        // self.data
        //     .entry(id)
        //     .or_insert_with(|| Box::new(create()))
        //     .as_any_mut()
        //     .downcast_mut::<T>()
        //     .unwrap()
    }

    // pub fn take_or_create<T: WidgetState, F>(&mut self, id: WidgetId, create: F) -> (u32, T)
    // where
    //     F: Fn() -> T,
    // {
    //     let index = *self.custom.id_to_index.entry(id).or_insert_with(|| {
    //         let idx = self.custom.states.len() as u32;
    //         self.custom.states.push(Box::new(create()));
    //         self.custom.ids.push(id);
    //         idx
    //     });

    //     let boxed = std::mem::replace(&mut self.custom.states[index as usize], Box::new(create()));
    //     let concrete = *boxed
    //         .into_any()
    //         .downcast::<T>()
    //         .expect("Type mismatch in widget state");

    //     (index, concrete)
    // }

    // pub fn restore<T: WidgetState>(&mut self, index: u32, state: T) {
    //     self.custom.states[index as usize] = Box::new(state);
    // }

    pub fn take_or_create<T: WidgetState, F>(&mut self, id: WidgetId, create: F) -> (u32, Box<T>)
    where
        F: FnOnce() -> T,
    {
        let current_frame = self.custom.current_frame;
        let current_layer = self.custom.current_layer;

        let index = *self.custom.id_to_index.entry(id).or_insert_with(|| {
            let idx = self.custom.states.len() as u32;
            self.custom.states.push(Some(Box::new(create())));
            self.custom.ids.push(id);
            self.custom.last_accessed.push(current_frame);
            self.custom.retained_by.push(current_layer);
            idx
        });

        self.custom.last_accessed[index as usize] = current_frame;
        self.custom.retained_by[index as usize] = current_layer;

        let boxed = self.custom.states[index as usize]
            .take()
            .expect("State already taken");

        let concrete: Box<T> = boxed
            .into_any()
            .downcast::<T>()
            .expect("Type mismatch in widget state");

        (index, concrete)
    }

    pub fn restore<T: WidgetState>(&mut self, index: u32, state: Box<T>) {
        self.custom.states[index as usize] = Some(state as Box<dyn WidgetState>);
    }

    // #[profiling::function]
    // pub fn replace<T: WidgetState>(&mut self, id: WidgetId, state: T) {
    //     match self.data.entry(id) {
    //         std::collections::hash_map::Entry::Occupied(mut entry) => {
    //             // Try to reuse existing allocation
    //             if let Some(existing) = entry.get_mut().as_any_mut().downcast_mut::<T>() {
    //                 *existing = state;
    //             } else {
    //                 entry.insert(Box::new(state));
    //             }
    //         }
    //         std::collections::hash_map::Entry::Vacant(entry) => {
    //             entry.insert(Box::new(state));
    //         }
    //     }

    //     // self.data.insert(id, Box::new(state));

    //     // self.data.entry(id).or_insert(|| Box::new(create()));
    //     // self.accessed_this_frame.insert(id);
    //     // self.data.entry(id).or_insert_with(|| Box::new(create()));

    //     // self.data
    //     //     .get_mut(&id)
    //     //     .unwrap()
    //     //     .as_any_mut()
    //     //     .downcast_mut::<T>()
    //     //     .unwrap()
    // }

    // #[profiling::function]
    // pub fn get_mut<T: WidgetState>(&mut self, id: WidgetId) -> Option<&mut T> {
    //     self.data
    //         .get_mut(&id)
    //         .and_then(|b| b.as_any_mut().downcast_mut::<T>())
    // }

    // pub fn _update_last<T>(&mut self, _id: WidgetId) -> bool
    // where
    //     T: WidgetState + Clone + PartialEq,
    // {
    //     true

    //     // let current_state = self
    //     //     .data
    //     //     .get(&id)
    //     //     .and_then(|b| b.as_any().downcast_ref::<T>())
    //     //     .unwrap();

    //     // let last_state = self
    //     //     .last
    //     //     .get_mut(&id)
    //     //     .and_then(|b| b.as_any_mut().downcast_mut::<T>());

    //     // if let Some(last_state) = last_state {
    //     //     if last_state != current_state {
    //     //         *last_state = current_state.clone();

    //     //         true
    //     //     } else {
    //     //         false
    //     //     }
    //     // } else {
    //     //     self.last.insert(id, Box::new(current_state.clone()));

    //     //     true
    //     // }
    // }

    // pub fn contains(&self, id: WidgetId) -> bool {
    //     self.data.contains_key(&id)
    // }

    pub fn next_frame(&mut self) {
        self.decorated_box.current_frame += 1;
        self.backdrop_filter.current_frame += 1;
        self.svg.current_frame += 1;
        self.gesture_detector.current_frame += 1;
        self.custom.current_frame += 1;
        self.text.current_frame += 1;
        self.scroll_area.current_frame += 1;
        self.layout_measures.current_frame += 1;
    }

    pub fn sweep(&mut self, is_retainer_alive: impl Fn(WidgetId) -> bool) {
        // let _g = profiler::scope();

        self.decorated_box.sweep(&is_retainer_alive);
        self.backdrop_filter.sweep(&is_retainer_alive);
        self.svg.sweep(&is_retainer_alive);
        self.gesture_detector.sweep(&is_retainer_alive);
        self.custom.sweep(&is_retainer_alive);
        self.text.sweep(&is_retainer_alive);
        self.scroll_area.sweep(&is_retainer_alive);
        self.layout_measures.sweep(&is_retainer_alive);

        self.accessed_this_frame.clear();

        // self.data
        //     .retain(|id, _| self.accessed_this_frame.contains(id));

        // if let Some(id) = interaction.focused {
        //     if !self.accessed_this_frame.contains(&id) {
        //         interaction.focused = None;
        //     }
        // }
    }

    pub(crate) fn set_current_layer(&mut self, layer_id: Option<WidgetId>) {
        self.decorated_box.current_layer = layer_id;
        self.backdrop_filter.current_layer = layer_id;
        self.svg.current_layer = layer_id;
        self.gesture_detector.current_layer = layer_id;
        self.custom.current_layer = layer_id;
        self.text.current_layer = layer_id;
        self.scroll_area.current_layer = layer_id;
        self.layout_measures.current_layer = layer_id;
    }
}
