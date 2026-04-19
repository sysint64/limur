use std::{any::TypeId, marker::PhantomData};

use limur_derive::WidgetBuilder;

use super::FrameBuilder;
use super::builder::BuildContext;
use crate::state::WidgetState;

pub trait StatefulWidgetBuilder: crate::widgets::builder::WidgetBuilder {
    fn build(self, context: &mut BuildContext);
}

#[derive(WidgetBuilder)]
pub struct StatefulWidgetAutoStateBuilder<T> {
    frame: FrameBuilder,
    phantom_data: PhantomData<T>,
}

#[derive(WidgetBuilder)]
pub struct StatefulWidgetWithStateBuilder<'a, T> {
    frame: FrameBuilder,
    state: &'a mut T,
}

pub trait StatefulWidget: 'static {
    type Event;

    fn on_event(&mut self, _event: &Self::Event) -> bool {
        false
    }

    fn build(&mut self, ctx: &mut BuildContext, frame: FrameBuilder);
}

impl<T> StatefulWidgetAutoStateBuilder<T> {
    pub fn state<'b>(self, state: &'b mut T) -> StatefulWidgetWithStateBuilder<'b, T> {
        StatefulWidgetWithStateBuilder {
            frame: self.frame,
            state,
        }
    }

    pub fn frame(mut self, frame: FrameBuilder) -> Self {
        self.frame = frame;
        self
    }
}

impl<'a, T> StatefulWidgetWithStateBuilder<'a, T> {
    pub fn frame(mut self, frame: FrameBuilder) -> Self {
        self.frame = frame;
        self
    }
}

impl<T: WidgetState + StatefulWidget + Default> StatefulWidgetBuilder
    for StatefulWidgetAutoStateBuilder<T>
{
    fn build(self, context: &mut BuildContext) {
        let id = self.frame.id.with_seed(context.id_seed);
        let (idx, mut state) = context.widgets_states.take_or_create(id, T::default);

        // Skip event processing for () type
        if TypeId::of::<T::Event>() != TypeId::of::<()>() {
            for event_box in context.event_queue.iter() {
                if let Some(event) = event_box.downcast_ref::<T::Event>() {
                    state.on_event(event);
                }
            }
        }

        context.accessed_this_frame(id);
        state.build(context, self.frame);

        context.widgets_states.restore(idx, state);
    }
}

impl<T: WidgetState + StatefulWidget + Default> StatefulWidgetAutoStateBuilder<T> {
    pub fn update_state_and_build<F>(self, context: &mut BuildContext, update_state: F)
    where
        F: FnOnce(&mut T),
    {
        let id = self.frame.id.with_seed(context.id_seed);
        let (idx, mut state) = context.widgets_states.take_or_create(id, T::default);

        update_state(&mut state);

        // Skip event processing for () type
        if TypeId::of::<T::Event>() != TypeId::of::<()>() {
            for event_box in context.event_queue.iter() {
                if let Some(event) = event_box.downcast_ref::<T::Event>() {
                    state.on_event(event);
                }
            }
        }

        context.accessed_this_frame(id);
        state.build(context, self.frame);

        context.widgets_states.restore(idx, state);
    }
}

impl<'a, T: WidgetState + StatefulWidget + Default> StatefulWidgetBuilder
    for StatefulWidgetWithStateBuilder<'a, T>
{
    fn build(self, context: &mut BuildContext) {
        let id = self.frame.id.with_seed(context.id_seed);

        // Skip event processing for () type
        if TypeId::of::<T::Event>() != TypeId::of::<()>() {
            for event_box in context.event_queue.iter() {
                if let Some(event) = event_box.downcast_ref::<T::Event>() {
                    self.state.on_event(event);
                }
            }
        }

        context.accessed_this_frame(id);
        self.state.build(context, self.frame);
    }
}

#[track_caller]
pub fn stateful<T: WidgetState>() -> StatefulWidgetAutoStateBuilder<T> {
    StatefulWidgetAutoStateBuilder {
        frame: FrameBuilder::default(),
        phantom_data: PhantomData,
    }
}
