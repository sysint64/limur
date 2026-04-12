use std::{
    any::Any,
    hash::{Hash, Hasher},
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "clipboard")]
use arboard::Clipboard;

use rustc_hash::{FxHashSet, FxHasher};
use smallvec::SmallVec;

use crate::{
    Animation, Constraints, ShortcutId, ShortcutModifierId, ShortcutsManager, ShortcutsRegistry,
    Size, Value, Vec2, View, ViewId, WidgetId, WidgetRef,
    assets::Assets,
    interaction::InteractionState,
    io::UserInput,
    layer::Layer,
    layout::{LayoutCommand, LayoutItem},
    profiler,
    state::{PerformanceMetrics, TypedWidgetStates, UiState, ViewConfig, WidgetsStates},
    text::{FontResources, TextsResources},
};

use super::{
    FrameBuilder, decorated_box::DecorationBuilder, editable_text::OsEvent,
    frame::FrameBuilderFlags,
};

pub struct PositionedChildMeta {
    pub index: u32,
    pub count: u32,
    pub is_first: bool,
    pub is_last: bool,
}

pub(crate) type DecorationDeferFn =
    Box<dyn Fn(&BuildContext, PositionedChildMeta) -> DecorationBuilder>;

#[derive(Debug)]
pub enum ApplicationEvent {
    Wake { view_id: ViewId },
}

pub trait ApplicationEventLoopProxy: Send + Sync {
    fn send_event(&self, event: ApplicationEvent);
}

pub struct UserDataStack<'a> {
    data: &'a (dyn Any + Send),
    parent: Option<&'a UserDataStack<'a>>,
}

pub struct MutUserDataStack<'a> {
    data: &'a mut (dyn Any + Send),
    parent: Option<&'a mut MutUserDataStack<'a>>,
}

pub struct BuildContext<'a, 'b, 'c> {
    pub(crate) root_layer: &'a mut Layer,
    pub(crate) bound_size: Vec2,
    pub(crate) performance_metrics: PerformanceMetrics,
    pub(crate) pre_layout: bool,
    pub(crate) ignore_pointer: bool,
    pub(crate) layer_id: Option<WidgetId>,
    // pub(crate) layout_commands: &'a mut Vec<LayoutCommand>,
    pub(crate) widgets_states: &'a mut WidgetsStates,
    pub(crate) event_queue: &'a mut Vec<Arc<dyn Any + Send>>,
    pub(crate) next_event_queue: &'a mut Vec<Arc<dyn Any + Send>>,
    pub(crate) broadcast_event_queue: &'a mut Vec<Arc<dyn Any + Send>>,
    pub(crate) text: &'a mut TextsResources<'b>,
    pub(crate) fonts: &'a mut FontResources,
    pub(crate) view: &'a View,
    pub(crate) async_tx: &'a mut tokio::sync::mpsc::UnboundedSender<Box<dyn Any + Send>>,
    pub(crate) broadcast_async_tx: &'a mut tokio::sync::mpsc::UnboundedSender<Box<dyn Any + Send>>,
    pub(crate) event_loop_proxy: Arc<dyn ApplicationEventLoopProxy>,
    pub(crate) id_seed: Option<u64>,
    // pub(crate) user_data: Vec<Box<dyn Any + Send>>,
    pub(crate) user_data: Option<&'a UserDataStack<'a>>,
    pub(crate) scoped_user_data: Option<&'a mut MutUserDataStack<'a>>,
    pub(crate) backgrounds: &'a mut SmallVec<[WidgetRef; 8]>,
    pub(crate) foregrounds: &'a mut SmallVec<[WidgetRef; 8]>,
    pub(crate) non_interactable: &'a mut FxHashSet<WidgetId>,
    pub(crate) phase_allocator: &'a bumpalo::Bump,
    pub(crate) input: &'a mut UserInput,
    pub(crate) interaction: &'a mut InteractionState,
    pub(crate) delta_time: f64,
    pub(crate) animations_stepped_this_frame: &'a mut FxHashSet<usize>,
    pub(crate) child_index: u32,
    pub(crate) child_index_stack: Vec<u32>,
    pub(crate) decoration_defer: Vec<(WidgetId, u32, DecorationDeferFn)>,
    pub(crate) decoration_defer_start_stack: Vec<usize>,
    pub(crate) shortcuts_manager: &'a mut ShortcutsManager,
    pub(crate) shortcuts_registry: &'a mut ShortcutsRegistry,
    pub(crate) os_events: &'a mut SmallVec<[OsEvent; 4]>,
    #[cfg(feature = "clipboard")]
    pub(crate) clipboard: &'a mut Option<Clipboard>,
    pub(crate) view_config: &'a mut ViewConfig,
    pub(crate) assets: &'a Assets<'c>,
    pub(crate) clipped_layout_items: &'a mut Vec<LayoutItem>,
    pub(crate) layers: &'a mut TypedWidgetStates<Layer>,
    pub(crate) last_interaction_state: &'a mut InteractionState,
}

pub trait Resolve<V> {
    fn resolve(&mut self, ctx: &mut BuildContext) -> V;
}

impl<V, A> Resolve<V> for A
where
    A: Animation + Value<V>,
{
    /// Advances the animation by the current frame's delta time (if not already
    /// advanced this frame) and returns the resolved value for this frame.
    ///
    /// Calling this multiple times in the same frame will not advance time
    /// multiple times.
    fn resolve(&mut self, ctx: &mut BuildContext) -> V {
        ctx.step_animation(self);

        self.value()
    }
}

impl<'a, 'b, 'c> BuildContext<'a, 'b, 'c> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ui_state: &'a mut UiState,
        texts: &'a mut TextsResources<'b>,
        fonts: &'a mut FontResources,
        broadcast_event_queue: &'a mut Vec<Arc<dyn Any + Send>>,
        broadcast_async_tx: &'a mut tokio::sync::mpsc::UnboundedSender<Box<dyn Any + Send>>,
        event_loop_proxy: Arc<dyn ApplicationEventLoopProxy>,
        delta_time: f64,
        assets: &'a Assets<'c>,
        pre_layout: bool,
    ) -> BuildContext<'a, 'b, 'c> {
        BuildContext {
            pre_layout,
            root_layer: &mut ui_state.root_layer,
            performance_metrics: ui_state.performance_metrics,
            bound_size: ui_state.view.size(),
            child_index: 0,
            ignore_pointer: false,
            layer_id: None,
            widgets_states: &mut ui_state.widgets_states,
            event_queue: &mut ui_state.current_event_queue,
            next_event_queue: &mut ui_state.next_event_queue,
            text: texts,
            fonts,
            view: &ui_state.view,
            async_tx: &mut ui_state.async_tx,
            broadcast_event_queue,
            broadcast_async_tx,
            event_loop_proxy,
            id_seed: None,
            user_data: None,
            scoped_user_data: None,
            phase_allocator: &mut ui_state.phase_allocator,
            backgrounds: &mut ui_state.backgrounds,
            input: &mut ui_state.user_input,
            interaction: &mut ui_state.interaction_state,
            delta_time,
            animations_stepped_this_frame: &mut ui_state.animations_stepped_this_frame,
            foregrounds: &mut ui_state.foregrounds,
            non_interactable: &mut ui_state.non_interactable,
            child_index_stack: Vec::new(),
            decoration_defer: Vec::new(),
            decoration_defer_start_stack: Vec::new(),
            shortcuts_manager: &mut ui_state.shortcuts_manager,
            shortcuts_registry: &mut ui_state.shortcuts_registry,
            os_events: &mut ui_state.os_events,
            #[cfg(feature = "clipboard")]
            clipboard: &mut ui_state.clipboard,
            view_config: &mut ui_state.view_config,
            assets: assets,
            clipped_layout_items: &mut ui_state.clipped_layout_items,
            layers: &mut ui_state.layers,
            last_interaction_state: &mut ui_state.last_interaction_state,
        }
    }

    pub fn performance_metrics(&self) -> PerformanceMetrics {
        self.performance_metrics
    }

    /// Advances an animation by the current frame's delta time.
    ///
    /// This method updates the animation's internal state and status
    /// based on the elapsed time since the previous frame.
    ///
    /// Each animation is guaranteed to be stepped at most once per frame.
    /// Calling this method multiple times with the same animation in the
    /// same frame has no additional effect.
    ///
    /// This is typically called explicitly for long-lived animations, or
    /// indirectly via `resolve(ctx)` when retrieving an animated value.
    pub fn step_animation<T: Animation>(&mut self, animation: &mut T) {
        if animation.in_progress() {
            let id = animation as *mut T as usize;

            if self.animations_stepped_this_frame.insert(id) {
                animation.step(self.delta_time)
            }
        }
    }

    pub fn pre_layout(&self) -> bool {
        self.pre_layout
    }

    pub fn child_index(&self) -> u32 {
        self.child_index
    }

    pub fn phase_allocator(&self) -> &bumpalo::Bump {
        self.phase_allocator
    }

    pub fn input(&self) -> &UserInput {
        self.input
    }

    pub fn view(&self) -> &View {
        self.view
    }

    pub fn accessed_this_frame(&mut self, id: WidgetId) {
        if let Some(layer_id) = self.layer_id
            && let Some(layer) = self.layers.get_mut(layer_id)
        {
            layer.accessed_this_frame.insert(id);
        }

        self.widgets_states.accessed_this_frame.insert(id);
    }

    #[inline]
    pub fn handle_decoration_defer<F>(&mut self, callback: F)
    where
        F: FnOnce(&mut Self),
    {
        let start = self.decoration_defer.len();
        self.decoration_defer_start_stack.push(start);

        callback(self);

        let start = self.decoration_defer_start_stack.pop().unwrap();
        let end = self.decoration_defer.len();
        let count = self.child_index.saturating_sub(1);

        for i in start..end {
            let (id, child_index, defer) = &self.decoration_defer[i];

            let child_meta = PositionedChildMeta {
                index: *child_index,
                count,
                is_first: *child_index == 0,
                is_last: *child_index == count,
            };

            let mut builder = defer(self, child_meta);
            let state = self
                .widgets_states
                .decorated_box
                .get_mut(*id)
                .expect("Decoration state should be initialized for defered");

            if builder.border_radius.is_some() {
                state.border_radius = builder.border_radius;
            }

            if builder.color.is_some() {
                state.color = builder.color;
            }

            state.gradients.append(&mut builder.gradients);

            if builder.border.is_some() {
                state.border = builder.border;
            }

            if let Some(shape) = builder.shape {
                state.shape = shape;
            }
        }

        self.decoration_defer.truncate(start);
    }

    pub fn provide<F, T: Any + Send>(&mut self, data: T, callback: F)
    where
        F: FnOnce(&mut Self),
    {
        // Store as raw pointer to avoid lifetime issues
        let data_ref: &(dyn Any + Send) = &data;
        let node = UserDataStack {
            data: unsafe { &*(data_ref as *const _) },
            parent: self.user_data.take(),
        };

        self.user_data = Some(unsafe { &*(&node as *const _) });

        callback(self);

        // Restore parent, dropping our node's reference
        self.user_data = node.parent;
    }

    pub fn scoped<F, T: Any + Send>(&mut self, data: &mut T, callback: F)
    where
        F: FnOnce(&mut Self),
    {
        // Store as raw pointer to avoid lifetime issues
        let data_ref: &mut (dyn Any + Send) = data;
        let mut node = MutUserDataStack {
            data: unsafe { &mut *(data_ref as *mut _) },
            parent: self.scoped_user_data.take(),
        };

        self.scoped_user_data = Some(unsafe { &mut *(&mut node as *mut _) });

        callback(self);

        // Restore parent, dropping our node's reference
        self.scoped_user_data = node.parent;
    }

    pub fn of<T: 'static>(&self) -> Option<&T> {
        let mut current = self.user_data;
        while let Some(node) = current {
            if let Some(data) = node.data.downcast_ref::<T>() {
                return Some(data);
            }
            current = node.parent;
        }
        None
    }

    pub fn is_shortcut<T: Into<ShortcutId>>(&self, shortcut_id: T) -> bool {
        self.shortcuts_manager.is_shortcut(shortcut_id)
    }

    pub fn has_modifier<T: Into<ShortcutModifierId>>(&self, modifier_id: T) -> bool {
        self.shortcuts_manager.has_modifier(modifier_id)
    }

    // pub fn of_mut<T: 'static>(&mut self) -> Option<&mut T> {
    //     let mut current = self.scoped_user_data;
    //     while let Some(node) = current {
    //         if let Some(data) = node.data.downcast_mut::<T>() {
    //             return Some(data);
    //         }
    //         current = node.parent;
    //     }
    //     None
    // }

    pub fn of_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let mut current = self.scoped_user_data.as_mut();
        while let Some(node) = current {
            if (*node.data).is::<T>() {
                return Some(unsafe { &mut *(node.data as *mut dyn Any as *mut T) });
            }
            current = node.parent.as_mut();
        }
        None
    }

    // pub fn with_user_data<F, T: Any + Send + 'static>(&mut self, data: T, callback: F)
    // where
    //     F: FnOnce(&mut BuildContext),
    // {
    //     self.user_data.push(Box::new(data));
    //     callback(self);
    //     self.user_data.pop();
    // }

    // pub fn of<T: 'static>(&self) -> Option<&T> {
    //     for data in self.user_data.iter().rev() {
    //         let data = data.downcast_ref::<T>();

    //         if data.is_some() {
    //             return data;
    //         }
    //     }

    //     None
    // }

    pub fn push_layer_commands(&mut self, layer_id: WidgetId) {
        let _g = profiler::scope();

        let layer = self.layers.get(layer_id).unwrap();

        // self.root_layer
        //     .layout_commands
        //     .extend(layer.layout_commands.iter().cloned());

        self.root_layer
            .layout_commands
            .extend_from_slice(&layer.layout_commands);

        self.widgets_states
            .accessed_this_frame
            .extend(layer.accessed_this_frame.iter());
    }

    pub fn push_layout_command(&mut self, command: LayoutCommand) {
        match command {
            LayoutCommand::BeginContainer { .. } => {
                self.child_index += 1;
                self.child_index_stack.push(self.child_index);
                self.child_index = 0;
            }
            LayoutCommand::EndContainer => {
                self.child_index = self.child_index_stack.pop().unwrap_or(0);
            }
            LayoutCommand::Leaf { .. } => self.child_index += 1,
            _ => {}
        }

        if let Some(layer_id) = self.layer_id {
            let layer = self.layers.get_mut(layer_id);

            if let Some(layer) = layer {
                self.root_layer.layout_commands.push(command.clone());
                layer.layout_commands.push(command);
            } else {
                self.root_layer.layout_commands.push(command);
            }
        } else {
            self.root_layer.layout_commands.push(command);
        }
    }

    pub fn scope<F, T>(&mut self, key: impl Hash, callback: F) -> T
    where
        F: FnOnce(&mut BuildContext) -> T,
    {
        let mut hasher = FxHasher::default();
        key.hash(&mut hasher);

        self.with_id_seed(hasher.finish(), callback)
    }

    pub fn with_id_seed<F, T>(&mut self, seed: u64, callback: F) -> T
    where
        F: FnOnce(&mut BuildContext) -> T,
    {
        let last_id_seed = self.id_seed;

        // Combine with parent seed, to support nested scopes
        self.id_seed = Some(match last_id_seed {
            Some(parent) => {
                let mut hasher = FxHasher::default();
                parent.hash(&mut hasher);
                seed.hash(&mut hasher);
                hasher.finish()
            }
            None => seed,
        });

        let result = callback(self);
        self.id_seed = last_id_seed;

        result
    }

    #[inline]
    pub(crate) fn resolve_decorators(
        &mut self,
        frame: &mut FrameBuilder,
    ) -> (SmallVec<[WidgetRef; 8]>, SmallVec<[WidgetRef; 8]>) {
        self.scope(frame.id, |ctx| {
            let mut backgrounds = std::mem::take(ctx.backgrounds);
            backgrounds.append(&mut frame.backgrounds);

            let mut foregrounds = std::mem::take(ctx.foregrounds);
            foregrounds.append(&mut frame.foregrounds);

            (backgrounds, foregrounds)
        })
    }

    pub fn emit<E: Any + Send + 'static>(&mut self, event: E) {
        self.next_event_queue.push(Arc::new(event));
    }

    pub fn spawn<E: Any + Send + 'static, F>(&self, future: F)
    where
        F: Future<Output = E> + Send + 'static,
    {
        let tx = self.async_tx.clone();
        let event_loop_proxy = self.event_loop_proxy.clone();
        let view_id = self.view.id;

        tokio::spawn(async move {
            let event = future.await;
            let _ = tx.send(Box::new(event));
            event_loop_proxy.send_event(ApplicationEvent::Wake { view_id });
        });
    }

    pub fn broadcast<E: Any + Send + 'static>(&mut self, event: E) {
        self.broadcast_event_queue.push(Arc::new(event));
    }

    pub fn spawn_broadcast<E: Any + Send + 'static, F>(&self, future: F)
    where
        F: Future<Output = E> + Send + 'static,
    {
        let tx = self.broadcast_async_tx.clone();
        let event_loop_proxy = self.event_loop_proxy.clone();
        let view_id = self.view.id;

        tokio::spawn(async move {
            let event = future.await;
            let _ = tx.send(Box::new(event));
            event_loop_proxy.send_event(ApplicationEvent::Wake { view_id });
        });
    }
}

#[macro_export]
macro_rules! impl_size_methods {
    () => {
        pub fn size<T: Into<Size>>(mut self, size: T) -> Self {
            self.size = size.into();
            self
        }

        pub fn width<T: Into<SizeConstraint>>(mut self, size: T) -> Self {
            self.size.width = size.into();
            self
        }

        pub fn height<T: Into<SizeConstraint>>(mut self, size: T) -> Self {
            self.size.height = size.into();
            self
        }

        pub fn fill_max_width(mut self) -> Self {
            self.size.width = SizeConstraint::Fill(1.);
            self
        }

        pub fn fill_max_height(mut self) -> Self {
            self.size.height = SizeConstraint::Fill(1.);
            self
        }

        pub fn fill_max_size(mut self) -> Self {
            self.size.width = SizeConstraint::Fill(1.);
            self.size.height = SizeConstraint::Fill(1.);
            self
        }

        pub fn constraints(mut self, constraints: Constraints) -> Self {
            self.constraints = constraints;
            self
        }

        pub fn max_width(mut self, value: f64) -> Self {
            self.constraints.max_width = value;
            self
        }

        pub fn max_height(mut self, value: f64) -> Self {
            self.constraints.max_height = value;
            self
        }

        pub fn min_width(mut self, value: f64) -> Self {
            self.constraints.min_width = value;
            self
        }

        pub fn min_height(mut self, value: f64) -> Self {
            self.constraints.min_height = value;
            self
        }
    };
}

#[macro_export]
macro_rules! impl_id {
    () => {
        #[track_caller]
        pub fn id(mut self, id: impl std::hash::Hash) -> Self {
            self.id = WidgetId::auto_with_seed(id);

            self
        }
    };
}

#[macro_export]
macro_rules! impl_width_methods {
    () => {
        pub fn width<T: Into<SizeConstraint>>(mut self, size: T) -> Self {
            self.width = size.into();
            self
        }

        pub fn fill_max_width(mut self) -> Self {
            self.width = SizeConstraint::Fill(1.);
            self
        }

        pub fn max_width(mut self, value: f64) -> Self {
            self.constraints.max_width = value;
            self
        }

        pub fn min_width(mut self, value: f64) -> Self {
            self.constraints.min_width = value;
            self
        }
    };
}

#[macro_export]
macro_rules! impl_position_methods {
    () => {
        pub fn zindex(mut self, zindex: i32) -> Self {
            self.zindex = zindex;
            self
        }
    };
}

#[derive(Default)]
pub struct Layout {
    pub size: Size,
    pub constraints: Constraints,
}

impl WidgetBuilder for FrameBuilder {
    fn frame_mut(&mut self) -> &mut FrameBuilder {
        self
    }
}

pub trait WidgetBuilder {
    fn frame_mut(&mut self) -> &mut FrameBuilder;

    #[track_caller]
    fn id(mut self, id: impl std::hash::Hash) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().id = ::clew::WidgetId::auto_with_seed(id);
        self.frame_mut().flags |= FrameBuilderFlags::ID;
        self
    }

    fn size<T: Into<::clew::Size>>(mut self, size: T) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().size = size.into();
        self.frame_mut().flags |= FrameBuilderFlags::SIZE;
        self
    }

    fn width<T: Into<::clew::SizeConstraint>>(mut self, size: T) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().size.width = size.into();
        self.frame_mut().flags |= FrameBuilderFlags::SIZE;
        self
    }

    fn height<T: Into<::clew::SizeConstraint>>(mut self, size: T) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().size.height = size.into();
        self.frame_mut().flags |= FrameBuilderFlags::SIZE;
        self
    }

    fn fill_max_width(mut self) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().size.width = ::clew::SizeConstraint::Fill(1.);
        self.frame_mut().flags |= FrameBuilderFlags::SIZE;
        self
    }

    fn fill_max_height(mut self) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().size.height = ::clew::SizeConstraint::Fill(1.);
        self.frame_mut().flags |= FrameBuilderFlags::SIZE;
        self
    }

    fn fill_max_size(mut self) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().size.width = ::clew::SizeConstraint::Fill(1.);
        self.frame_mut().size.height = ::clew::SizeConstraint::Fill(1.);
        self.frame_mut().flags |= FrameBuilderFlags::SIZE;
        self
    }

    fn constraints(mut self, constraints: ::clew::Constraints) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().constraints = constraints;
        self.frame_mut().flags |= FrameBuilderFlags::CONSTRAINTS;
        self
    }

    fn max_width(mut self, value: f64) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().constraints.max_width = value;
        self.frame_mut().flags |= FrameBuilderFlags::CONSTRAINTS;
        self
    }

    fn max_height(mut self, value: f64) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().constraints.max_height = value;
        self.frame_mut().flags |= FrameBuilderFlags::CONSTRAINTS;
        self
    }

    fn min_width(mut self, value: f64) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().constraints.min_width = value;
        self.frame_mut().flags |= FrameBuilderFlags::CONSTRAINTS;
        self
    }

    fn min_height(mut self, value: f64) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().constraints.min_height = value;
        self.frame_mut().flags |= FrameBuilderFlags::CONSTRAINTS;
        self
    }

    fn clip(mut self, clip: ::clew::Clip) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().clip = clip;
        self.frame_mut().flags |= FrameBuilderFlags::CLIP;
        self
    }

    fn offset(mut self, x: f64, y: f64) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().offset_x = x;
        self.frame_mut().offset_y = y;
        self.frame_mut().flags |= FrameBuilderFlags::OFFSET;
        self
    }

    fn offset_x(mut self, offset: f64) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().offset_x = offset;
        self.frame_mut().flags |= FrameBuilderFlags::OFFSET;
        self
    }

    fn offset_y(mut self, offset: f64) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().offset_y = offset;
        self.frame_mut().flags |= FrameBuilderFlags::OFFSET;
        self
    }

    fn zindex(mut self, zindex: i32) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().zindex = zindex;
        self.frame_mut().flags |= FrameBuilderFlags::ZINDEX;
        self
    }

    fn padding(mut self, padding: ::clew::EdgeInsets) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().padding = padding;
        self.frame_mut().flags |= FrameBuilderFlags::PADDING;
        self
    }

    fn margin(mut self, margin: ::clew::EdgeInsets) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().margin = margin;
        self.frame_mut().flags |= FrameBuilderFlags::MARGIN;
        self
    }

    fn background(mut self, decorator: ::clew::WidgetRef) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().backgrounds.push(decorator);
        self.frame_mut().flags |= FrameBuilderFlags::BACKGROUNDS;
        self
    }

    fn foreground(mut self, decorator: ::clew::WidgetRef) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().foregrounds.push(decorator);
        self.frame_mut().flags |= FrameBuilderFlags::FOREGROUNDS;
        self
    }

    fn ignore_pointer(mut self, value: bool) -> Self
    where
        Self: Sized,
    {
        self.frame_mut().ignore_pointer = value;
        self.frame_mut().flags |= FrameBuilderFlags::IGNORE_POINTER;
        self
    }
}
