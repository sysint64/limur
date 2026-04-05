use clew_derive::WidgetBuilder;

use crate::{
    CrossAxisAlignment, MainAxisAlignment,
    layout::{ContainerKind, LayoutCommand},
};

use super::{FrameBuilder, builder::BuildContext};

#[derive(WidgetBuilder)]
pub struct VStackBuilder {
    frame: FrameBuilder,
    rtl_aware: bool,
    spacing: f64,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
}

impl VStackBuilder {
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

    #[profiling::function]
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
            kind: ContainerKind::VStack {
                spacing: self.spacing,
                rtl_aware: self.rtl_aware,
                main_axis_alignment: self.main_axis_alignment,
                cross_axis_alignment: self.cross_axis_alignment,
            },
            size: self.frame.size,
            constraints: self.frame.constraints,
            clip: self.frame.clip,
        });

        context.handle_decoration_defer(callback);

        if self.frame.offset_x != 0. || self.frame.offset_y != 0. {
            context.push_layout_command(LayoutCommand::EndOffset);
        }

        context.push_layout_command(LayoutCommand::EndContainer);
    }
}

pub fn vstack() -> VStackBuilder {
    VStackBuilder {
        frame: FrameBuilder::new(),
        spacing: 5.,
        rtl_aware: false,
        main_axis_alignment: MainAxisAlignment::default(),
        cross_axis_alignment: CrossAxisAlignment::default(),
    }
}
