use clew_derive::{WidgetBuilder, WidgetState};

use crate::{
    CrossAxisAlignment, MainAxisAlignment,
    layout::{ContainerKind, LayoutCommand},
};

use super::{FrameBuilder, builder::BuildContext, scope};

#[derive(WidgetBuilder)]
pub struct HStackBuilder {
    frame: FrameBuilder,
    rtl_aware: bool,
    spacing: f64,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
}

#[derive(WidgetState, Clone, PartialEq)]
pub struct State {
    pub(crate) children_count: u32,
}

impl HStackBuilder {
    pub fn rtl_aware(mut self, rtl_aware: bool) -> Self {
        self.rtl_aware = rtl_aware;

        self
    }

    pub fn spacing(mut self, spacing: f64) -> Self {
        self.spacing = spacing;

        self
    }

    pub fn main_axis_alignment(mut self, value: MainAxisAlignment) -> Self {
        self.main_axis_alignment = value;

        self
    }

    pub fn cross_axis_alignment(mut self, value: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = value;

        self
    }

    pub fn build<F>(mut self, context: &mut BuildContext, callback: F)
    where
        F: FnOnce(&mut BuildContext),
    {
        scope(context.position.index).build(context, |context| {
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
                kind: ContainerKind::HStack {
                    spacing: self.spacing,
                    rtl_aware: self.rtl_aware,
                    main_axis_alignment: self.main_axis_alignment,
                    cross_axis_alignment: self.cross_axis_alignment,
                },
                size: self.frame.size,
                constraints: self.frame.constraints,
                clip: self.frame.clip,
            });

            let id = self.frame.id.with_seed(context.id_seed);
            let state = context
                .widgets_states
                .hstack
                .get_or_insert(id, || State { children_count: 0 });

            context.position.count = state.children_count;
            context.handle_decoration_defer(callback);

            let state = context.widgets_states.hstack.get_mut(id).unwrap();
            state.children_count = context.position.index;

            context.push_layout_command(LayoutCommand::EndContainer);

            if self.frame.offset_x != 0. || self.frame.offset_y != 0. {
                context.push_layout_command(LayoutCommand::EndOffset);
            }
        });
    }
}

pub fn hstack() -> HStackBuilder {
    HStackBuilder {
        frame: FrameBuilder::new(),
        rtl_aware: false,
        spacing: 5.,
        main_axis_alignment: MainAxisAlignment::default(),
        cross_axis_alignment: CrossAxisAlignment::default(),
    }
}
