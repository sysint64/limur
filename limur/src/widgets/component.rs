use crate::{WidgetId, impl_id, state::WidgetState};
use std::any::TypeId;

use super::{builder::BuildContext, scope::scope};

pub struct ComponentBuilder<'a, V: Component> {
    app: &'a mut V::App,
    id: WidgetId,
}

pub struct ComponentWithStateBuilder<'a, V: Component> {
    app: &'a mut V::App,
    state: &'a mut V,
}

pub trait Component: 'static {
    type App;
    type Event;

    fn on_event(&mut self, _app: &mut Self::App, _event: &Self::Event) -> bool {
        false
    }

    fn build(&mut self, app: &mut Self::App, ctx: &mut BuildContext);
}

impl<'a, V: Component> ComponentBuilder<'a, V> {
    impl_id!();

    pub fn state(self, state: &'a mut V) -> ComponentWithStateBuilder<'a, V> {
        ComponentWithStateBuilder {
            app: self.app,
            state,
        }
    }
}

impl<'a, V: Component + Default + WidgetState> ComponentBuilder<'a, V> {
    pub fn build(&mut self, context: &mut BuildContext) {
        let id = self.id.with_seed(context.id_seed);
        let (idx, mut state) = context.widgets_states.take_or_create(id, V::default);

        // Skip event processing for () type
        if TypeId::of::<V::Event>() != TypeId::of::<()>() {
            for event_box in context.event_queue.iter() {
                if let Some(event) = event_box.downcast_ref::<V::Event>() {
                    state.on_event(self.app, event);
                }
            }
        }

        context.accessed_this_frame(id);

        scope(id).build(context, |context| {
            state.build(self.app, context);
        });

        context.widgets_states.restore(idx, state);
    }
}

impl<'a, V: Component> ComponentWithStateBuilder<'a, V> {
    pub fn build(&mut self, context: &mut BuildContext) {
        // Skip event processing for () type
        if TypeId::of::<V::Event>() != TypeId::of::<()>() {
            for event_box in context.event_queue.iter() {
                if let Some(event) = event_box.downcast_ref::<V::Event>() {
                    self.state.on_event(self.app, event);
                }
            }
        }

        self.state.build(self.app, context);
    }
}

#[track_caller]
pub fn component<'a, V: Component>(app: &'a mut V::App) -> ComponentBuilder<'a, V> {
    ComponentBuilder {
        app,
        id: WidgetId::auto(),
    }
}
