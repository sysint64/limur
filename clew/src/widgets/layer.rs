use clew_derive::WidgetBuilder;
use smallvec::SmallVec;

use crate::{
    Clip, Constraints, EdgeInsets, Size,
    layer::Layer,
    layout::{ContainerKind, LayoutCommand},
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
        let id = self.frame.id.with_seed(context.id_seed);
        let layer = context.layers.get_or_insert(id, Layer::default);

        if layer.is_dirty {
            layer.parent_layer_id = context.layer_id;
            let last_layer_id = context.layer_id;
            context.layer_id = Some(id);

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
                kind: ContainerKind::Layer { id },
                size: self.frame.size,
                constraints: self.frame.constraints,
                clip: self.frame.clip,
            });

            let layer = context.layers.get_mut(id).unwrap();

            layer.accessed_this_frame.clear();
            layer.layout_items.clear();
            layer.is_dirty = false;

            scope(id).build(context, |context| {
                context.handle_decoration_defer(callback);
            });

            context.push_layout_command(LayoutCommand::EndContainer);

            if self.frame.offset_x != 0. || self.frame.offset_y != 0. {
                context.push_layout_command(LayoutCommand::EndOffset);
            }

            context.layer_id = last_layer_id;
        } else {
            let layer = context.layers.get_mut(id).unwrap();
            let size = Size::fixed(layer.wrap_size.x, layer.wrap_size.y);

            // context.push_layout_command(LayoutCommand::BeginContainer {
            //     backgrounds: SmallVec::new(),
            //     foregrounds: SmallVec::new(),
            //     zindex: 0,
            //     padding: EdgeInsets::ZERO,
            //     margin: EdgeInsets::ZERO,
            //     kind: ContainerKind::None,
            //     size,
            //     constraints: Constraints::exact_size(size),
            //     clip: Clip::None,
            // });
            context.push_layout_command(LayoutCommand::Layer { id });
            // context.push_layout_command(LayoutCommand::EndContainer);

            context.push_layer_state(id);
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
