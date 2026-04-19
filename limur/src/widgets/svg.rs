use std::any::Any;

use limur_derive::WidgetBuilder;

use crate::{
    ColorRgba, WidgetRef, WidgetType,
    layout::{DeriveWrapSize, LayoutCommand, WidgetPlacement},
    render::{PixelExtension, RenderCommand, RenderContext},
    state::WidgetState,
};

use super::{FrameBuilder, builder::BuildContext};

pub struct SvgWidget;

#[derive(WidgetBuilder)]
pub struct SvgBuilder {
    frame: FrameBuilder,
    asset_id: &'static str,
    color: Option<ColorRgba>,
}

#[derive(Clone, PartialEq)]
pub struct State {
    pub(crate) asset_id: &'static str,
    pub(crate) color: Option<ColorRgba>,
}

impl WidgetState for State {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl SvgBuilder {
    pub fn color(mut self, color: ColorRgba) -> Self {
        self.color = Some(color);

        self
    }

    pub fn build(&self, context: &mut BuildContext) {
        let id = self.frame.id.with_seed(context.id_seed);

        let widget_ref = WidgetRef::new(WidgetType::of::<SvgWidget>(), id);
        let backgrounds = std::mem::take(context.backgrounds);
        let foregrounds = std::mem::take(context.foregrounds);

        context.push_layout_command(LayoutCommand::Leaf {
            widget_ref,
            backgrounds,
            foregrounds,
            padding: self.frame.padding,
            margin: self.frame.margin,
            constraints: self.frame.constraints,
            size: self.frame.size,
            zindex: self.frame.zindex,
            derive_wrap_size: DeriveWrapSize::Svg(self.asset_id),
            clip: self.frame.clip,
        });

        context.widgets_states.svg.set(
            id,
            State {
                asset_id: self.asset_id,
                color: self.color,
            },
        );
    }
}

#[track_caller]
pub fn svg(asset_id: &'static str) -> SvgBuilder {
    SvgBuilder {
        frame: FrameBuilder::new(),
        asset_id,
        color: None,
    }
}

pub fn render(ctx: &mut RenderContext, placement: &WidgetPlacement, state: &State) {
    ctx.push_command(
        placement.zindex,
        RenderCommand::Svg {
            boundary: placement.rect.px(ctx),
            asset_id: state.asset_id,
            tint_color: state.color,
        },
    );
}
