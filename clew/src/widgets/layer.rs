use clew_derive::WidgetBuilder;

use crate::{
    WidgetRef, WidgetType,
    interaction::{InteractionContext, handle_interaction},
    layer::Layer,
    layout::LayoutCommand,
    render::LayoutContext,
};

use super::{FrameBuilder, builder::BuildContext};

#[derive(WidgetBuilder)]
pub struct LayerBuilder {
    frame: FrameBuilder,
}

impl LayerBuilder {
    pub fn build<F>(mut self, context: &mut BuildContext, mut callback: F)
    where
        F: FnMut(&mut BuildContext),
    {
        // let id = self.frame.id.with_seed(context.id_seed);
        // let widget_ref = WidgetRef::new(WidgetType::of::<Layer>(), id);

        // let layer = context.layers.get_or_insert(id, Layer::default);

        // context.layout_commands.clear();

        // callback(context);

        // let mut layout_ctx = LayoutContext {
        //     text_resources: context.text,
        //     fonts: context.fonts,
        //     assets: context.assets,
        //     view: context.view,
        //     clipped_layout_items: context.clipped_layout_items,
        //     widgets_states: context.widgets_states,
        // };

        // let layer = context.layers.get_mut(id).unwrap();
        // layer.bound_size = context.bound_size;

        // clew::layer_layout(&mut layout_ctx, layer, context.layout_commands);

        // // let mut interaction_context = InteractionContext {
        // //     user_input: context.input,
        // //     view: context.view,
        // //     interaction_state: context.interaction,
        // //     last_interaction_state: context.last_interaction_state,
        // //     layout_items: context.clipped_layout_items,
        // //     non_interactable: context.non_interactable,
        // //     widgets_states: context.widgets_states,
        // // };

        // // handle_interaction(&mut interaction_context);

        // // println!("REACEHD THIS");
        // // context.layout_commands.clear();

        // // callback(context);

        // // let layer = context.layers.get_mut(id).unwrap();

        // // let mut layout_ctx = LayoutContext {
        // //     text_resources: context.text,
        // //     fonts: context.fonts,
        // //     assets: context.assets,
        // //     view: context.view,
        // //     clipped_layout_items: context.clipped_layout_items,
        // //     widgets_states: context.widgets_states,
        // // };

        // // clew::layer_layout(&mut layout_ctx, layer, context.layout_commands);
        // // println!("REACEHD THIS 2");
    }
}

pub fn layer() -> LayerBuilder {
    LayerBuilder {
        frame: FrameBuilder::new(),
    }
}
