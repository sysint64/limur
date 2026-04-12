use clew_derive::WidgetBuilder;
use smallvec::SmallVec;

use crate::{
    Clip, Constraints, EdgeInsets, Size, SizeConstraint, Vec2, WidgetRef, WidgetType,
    interaction::{InteractionContext, handle_interaction},
    layer::Layer,
    layout::{ContainerKind, LayoutCommand, LayoutItem},
    render::{LayoutContext, layer_layout},
};

use super::{FrameBuilder, builder::BuildContext};

pub const ROOT_LAYER_WIDGET_ID: &'static str = "limur::root_layer";

#[derive(WidgetBuilder)]
pub struct LayerBuilder {
    frame: FrameBuilder,
}

impl LayerBuilder {
    pub fn build<F>(mut self, context: &mut BuildContext, mut callback: F)
    where
        F: FnMut(&mut BuildContext),
    {
        let id = self.frame.id.with_seed(context.id_seed);
        let layer = context.layers.get_or_insert(id, Layer::default);

        let layout_measures = context.widgets_states.layout_measures.get_mut(id);

        layer.parent_layer_id = context.layer_id;
        layer.layout_commands.clear();

        let offset = if let Some(layout_measures) = layout_measures {
            Vec2::new(
                self.frame.offset_x + layout_measures.x,
                self.frame.offset_y + layout_measures.y,
            )
        } else {
            Vec2::new(self.frame.offset_x, self.frame.offset_y)
        };

        let last_layer_id = context.layer_id;
        context.layer_id = id;

        //

        if offset.x != 0. || offset.y != 0. {
            context.push_layout_command(LayoutCommand::BeginOffset {
                offset_x: offset.x,
                offset_y: offset.y,
            });
        }


        let (backgrounds, foregrounds) = context.resolve_decorators(&mut self.frame);

        context.push_layout_command(LayoutCommand::BeginContainer {
            backgrounds,
            foregrounds,
            zindex: self.frame.zindex,
            padding: self.frame.padding,
            margin: self.frame.margin,
            kind: ContainerKind::None,
            size: self.frame.size,
            constraints: self.frame.constraints,
            clip: self.frame.clip,
        });
        callback(context);
        context.push_layout_command(LayoutCommand::EndContainer);

        if offset.x != 0. || offset.y != 0. {
            context.push_layout_command(LayoutCommand::EndOffset);
        }

        let mut layout_ctx = LayoutContext {
            text_resources: context.text,
            fonts: context.fonts,
            assets: context.assets,
            view: context.view,
            clipped_layout_items: context.clipped_layout_items,
            widgets_states: context.widgets_states,
        };

        let layer = context.layers.get_mut(id).unwrap();
        layer.bound_size = context.bound_size;

        let size = layer_layout(&mut layout_ctx, layer);

        // let mut interaction_context = InteractionContext {
        //     user_input: context.input,
        //     view: context.view,
        //     interaction_state: context.interaction,
        //     last_interaction_state: context.last_interaction_state,
        //     layout_items: context.clipped_layout_items,
        //     non_interactable: context.non_interactable,
        //     widgets_states: context.widgets_states,
        // };

        // handle_interaction(&mut interaction_context);

        // layer.layout_commands.clear();
        // context.pre_layout = false;

        // let (backgrounds, foregrounds) = context.resolve_decorators(&mut self.frame);

        // if offset.x != 0. || offset.y != 0. {
        //     context.push_layout_command(LayoutCommand::BeginOffset {
        //         offset_x: offset.x,
        //         offset_y: offset.y,
        //     });
        // }

        // context.push_layout_command(LayoutCommand::BeginContainer {
        //     backgrounds,
        //     foregrounds,
        //     zindex: self.frame.zindex,
        //     padding: self.frame.padding,
        //     margin: self.frame.margin,
        //     kind: ContainerKind::None,
        //     size: self.frame.size,
        //     constraints: self.frame.constraints,
        //     clip: self.frame.clip,
        // });
        // callback(context);
        // context.push_layout_command(LayoutCommand::EndContainer);

        // if offset.x != 0. || offset.y != 0. {
        //     context.push_layout_command(LayoutCommand::EndOffset);
        // }

        // let layer = context.layers.get_mut(id).unwrap();

        // let mut layout_ctx = LayoutContext {
        //     text_resources: context.text,
        //     fonts: context.fonts,
        //     assets: context.assets,
        //     view: context.view,
        //     clipped_layout_items: context.clipped_layout_items,
        //     widgets_states: context.widgets_states,
        // };

        // let size = layer_layout(&mut layout_ctx, layer);

        context.layer_id = last_layer_id;

        context.push_layout_command(LayoutCommand::BeginContainer {
            backgrounds: SmallVec::new(),
            foregrounds: SmallVec::new(),
            zindex: 0,
            padding: EdgeInsets::ZERO,
            margin: EdgeInsets::ZERO,
            kind: ContainerKind::None,
            size: Size::fixed(size.x, size.y),
            constraints: Constraints::exact_size(size),
            clip: self.frame.clip,
        });
        context.push_layout_command(LayoutCommand::EndContainer);
    }
}

pub fn layer() -> LayerBuilder {
    LayerBuilder {
        frame: FrameBuilder::new(),
    }
}
