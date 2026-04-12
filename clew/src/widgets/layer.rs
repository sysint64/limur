use clew_derive::WidgetBuilder;
use smallvec::SmallVec;

use crate::{
    Clip, Constraints, EdgeInsets, Size, SizeConstraint, Vec2, WidgetRef, WidgetType,
    interaction::{InteractionContext, handle_interaction},
    layer::Layer,
    layout::{ContainerKind, LayoutCommand, LayoutItem},
    render::{LayoutContext, layer_layout},
};

use super::{FrameBuilder, builder::BuildContext, scope};

pub const ROOT_LAYER_WIDGET_ID: &'static str = "limur::root_layer";

#[derive(WidgetBuilder)]
pub struct LayerBuilder {
    frame: FrameBuilder,
}

impl LayerBuilder {
    pub fn build<F>(mut self, context: &mut BuildContext, callback: F)
    where
        F: FnOnce(&mut BuildContext),
    {
        let (backgrounds, foregrounds) = context.resolve_decorators(&mut self.frame);

        if self.frame.offset_x != 0. || self.frame.offset_y != 0. {
            context.push_layout_command(LayoutCommand::BeginOffset {
                offset_x: self.frame.offset_x,
                offset_y: self.frame.offset_y,
            });
        }

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

        let id = self.frame.id.with_seed(context.id_seed);
        let layer = context.layers.get_or_insert(id, Layer::default);

        if layer.is_dirty {
            layer.parent_layer_id = context.layer_id;
            let last_layer_id = context.layer_id;
            context.layer_id = Some(id);
            layer.accessed_this_frame.clear();
            layer.layout_commands.clear();
            layer.is_dirty = false;

            scope(id).build(context, |context| {
                context.handle_decoration_defer(callback);
            });

            context.layer_id = last_layer_id;

            // let layer = context.layers.get(id).unwrap();
            // println!("{}", layer.layout_commands.len());
            // println!("{}", layer.accessed_this_frame.len());
        } else {
            context.push_layer_commands(id);

            // let layer = context.layers.get(id).unwrap();
            // println!("{:?}", layer.layout_commands);
        }

        context.push_layout_command(LayoutCommand::EndContainer);

        if self.frame.offset_x != 0. || self.frame.offset_y != 0. {
            context.push_layout_command(LayoutCommand::EndOffset);
        }

        if !context.pre_layout {
            context.widgets_states.accessed_this_frame.insert(id);
        }
    }
}

pub fn layer() -> LayerBuilder {
    LayerBuilder {
        frame: FrameBuilder::new(),
    }
}
