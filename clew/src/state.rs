use std::{any::Any, sync::Arc};

#[cfg(feature = "clipboard")]
use arboard::Clipboard;
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

use crate::{
    LayoutDirection, Rect, ShortcutsRegistry, View, WidgetId, WidgetRef,
    editable_text::{self, OsEvent},
    interaction::InteractionState,
    io::UserInput,
    layout::{LayoutCommand, LayoutItem, LayoutMeasure, LayoutState, WidgetPlacement},
    render::RenderState,
    shortcuts::ShortcutsManager,
    widgets::{decorated_box, gesture_detector, scroll_area, svg, text},
};

pub trait WidgetState: Any + Send + 'static {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

pub struct UiState {
    pub view: View,
    pub render_state: RenderState,
    pub layout_commands: Vec<LayoutCommand>,
    pub phase_allocator: bumpalo::Bump,
    pub(crate) layout_state: LayoutState,
    pub current_event_queue: Vec<Arc<dyn Any + Send>>,
    pub next_event_queue: Vec<Arc<dyn Any + Send>>,
    pub(crate) widgets_states: WidgetsStates,
    pub(crate) widget_placements: Vec<WidgetPlacement>,
    pub(crate) layout_items: Vec<LayoutItem>,
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
}

#[derive(Default)]
pub(crate) struct WidgetsStates {
    // pub data: FxHashMap<WidgetId, Box<dyn WidgetState>>,
    // pub last: FxHashMap<WidgetId, Box<dyn WidgetState>>,
    pub(crate) layout_measures: TypedWidgetStates<LayoutMeasure>,

    pub(crate) decorated_box: TypedWidgetStates<decorated_box::State>,
    pub(crate) scroll_area: TypedWidgetStates<scroll_area::State>,
    pub(crate) text: TypedWidgetStates<text::State>,
    pub(crate) editable_text: TypedWidgetStates<editable_text::State>,
    pub(crate) gesture_detector: TypedWidgetStates<gesture_detector::State>,
    pub(crate) svg: TypedWidgetStates<svg::State>,
    pub(crate) components: TypedWidgetStates<Box<dyn Any>>,
    pub(crate) custom: TypedWidgetStates<Option<Box<dyn WidgetState>>>,
}

#[derive(Default)]
pub struct ViewConfig {
    pub ime_cursor_rect: Rect,
    pub should_use_wide_space: bool,
    pub layout_direction: LayoutDirection,
    pub should_update_cursor_each_frame: bool,
}

pub struct TypedWidgetStates<T> {
    id_to_index: FxHashMap<WidgetId, u32>,
    states: Vec<T>,
    ids: Vec<WidgetId>,
    pub accessed_this_frame: FxHashSet<WidgetId>,
}

impl<T> Default for TypedWidgetStates<T> {
    fn default() -> Self {
        Self {
            id_to_index: FxHashMap::default(),
            states: Vec::new(),
            ids: Vec::new(),
            accessed_this_frame: FxHashSet::default(),
        }
    }
}

impl<T> TypedWidgetStates<T> {
    pub fn get_or_insert(&mut self, id: WidgetId, create: impl FnOnce() -> T) -> &mut T {
        let index = *self.id_to_index.entry(id).or_insert_with(|| {
            let idx = self.states.len() as u32;
            self.states.push(create());
            self.ids.push(id);
            idx
        });
        &mut self.states[index as usize]
    }

    pub fn get_mut(&mut self, id: WidgetId) -> Option<&mut T> {
        self.id_to_index
            .get(&id)
            .map(|&idx| &mut self.states[idx as usize])
    }

    pub fn get(&self, id: WidgetId) -> Option<&T> {
        self.id_to_index
            .get(&id)
            .map(|&idx| &self.states[idx as usize])
    }

    pub fn replace(&mut self, id: WidgetId, state: T) {
        if let Some(&idx) = self.id_to_index.get(&id) {
            self.states[idx as usize] = state;
        } else {
            let idx = self.states.len() as u32;
            self.id_to_index.insert(id, idx);
            self.states.push(state);
            self.ids.push(id);
        }
    }

    pub fn set(&mut self, id: WidgetId, state: T) -> usize {
        if let Some(&idx) = self.id_to_index.get(&id) {
            self.states[idx as usize] = state;

            idx as usize
        } else {
            let idx = self.states.len() as u32;
            self.id_to_index.insert(id, idx);
            self.states.push(state);
            self.ids.push(id);

            idx as usize
        }
    }

    pub fn sweep(&mut self) {
        let mut i = 0;

        while i < self.states.len() {
            if self.accessed_this_frame.contains(&self.ids[i]) {
                i += 1;
            } else {
                // Swap-remove from both parallel arrays
                self.id_to_index.remove(&self.ids[i]);

                self.states.swap_remove(i);
                self.ids.swap_remove(i);

                // Update the index of the element that was swapped in
                if i < self.ids.len() {
                    self.id_to_index.insert(self.ids[i], i as u32);
                }
            }
        }

        self.accessed_this_frame.clear();
    }

    pub fn clear(&mut self) {
        self.id_to_index.clear();
        self.states.clear();
        self.ids.clear();
        self.accessed_this_frame.clear();
    }
}

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

        Self {
            view,
            render_state: Default::default(),
            phase_allocator,
            layout_commands: Vec::new(),
            current_event_queue: Vec::new(),
            next_event_queue: Vec::new(),
            widgets_states: WidgetsStates::default(),
            layout_state: LayoutState::default(),
            widget_placements: Vec::new(),
            layout_items: Vec::new(),
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
            clipboard,
        }
    }
}

impl WidgetsStates {
    #[profiling::function]
    pub fn get_or_insert_custom<T: WidgetState, F>(&mut self, id: WidgetId, create: F) -> &mut T
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
        let index = *self.custom.id_to_index.entry(id).or_insert_with(|| {
            let idx = self.custom.states.len() as u32;
            self.custom.states.push(Some(Box::new(create())));
            self.custom.ids.push(id);
            idx
        });

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

    #[profiling::function]
    pub fn update_last<T>(&mut self, _id: WidgetId) -> bool
    where
        T: WidgetState + Clone + PartialEq,
    {
        true

        // let current_state = self
        //     .data
        //     .get(&id)
        //     .and_then(|b| b.as_any().downcast_ref::<T>())
        //     .unwrap();

        // let last_state = self
        //     .last
        //     .get_mut(&id)
        //     .and_then(|b| b.as_any_mut().downcast_mut::<T>());

        // if let Some(last_state) = last_state {
        //     if last_state != current_state {
        //         *last_state = current_state.clone();

        //         true
        //     } else {
        //         false
        //     }
        // } else {
        //     self.last.insert(id, Box::new(current_state.clone()));

        //     true
        // }
    }

    // pub fn contains(&self, id: WidgetId) -> bool {
    //     self.data.contains_key(&id)
    // }

    #[profiling::function]
    pub fn sweep(&mut self) {
        self.decorated_box.clear();
        self.svg.clear();
        self.gesture_detector.sweep();
        self.custom.sweep();
        self.text.sweep();
        self.scroll_area.sweep();
        self.layout_measures.sweep();

        // self.data
        //     .retain(|id, _| self.accessed_this_frame.contains(id));

        // if let Some(id) = interaction.focused {
        //     if !self.accessed_this_frame.contains(&id) {
        //         interaction.focused = None;
        //     }
        // }

        // self.accessed_this_frame.clear();
    }
}
