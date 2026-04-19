use std::any::Any;

use limur_derive::WidgetBuilder;
use smallvec::{SmallVec, smallvec};

use crate::{
    Border, BorderRadius, BorderSide, BoxShape, ColorRgba, Gradient, LinearGradient,
    RadialGradient, WidgetId, WidgetRef, WidgetType, impl_id,
    layout::{DeriveWrapSize, LayoutCommand, WidgetPlacement},
    render::{Fill, PixelExtension, RenderCommand, RenderContext},
    state::WidgetState,
};

use super::{
    FrameBuilder,
    builder::{BuildContext, DecorationDeferFn, PositionedChildMeta},
    scope,
};

pub struct DecoratedBox;

#[must_use = "widget is not rendered until .build(ctx) is called"]
#[derive(WidgetBuilder)]
pub struct DecoratedBoxBuilder {
    frame: FrameBuilder,
    color: Option<ColorRgba>,
    gradients: SmallVec<[Gradient; 4]>,
    border_radius: Option<BorderRadius>,
    border: Option<Border>,
    shape: BoxShape,
}

pub struct DecorationBuilder {
    pub(crate) id: WidgetId,
    pub(crate) color: Option<ColorRgba>,
    pub(crate) gradients: SmallVec<[Gradient; 4]>,
    pub(crate) border_radius: Option<BorderRadius>,
    pub(crate) border: Option<Border>,
    pub(crate) defer: Option<DecorationDeferFn>,
    pub(crate) shape: Option<BoxShape>,
}

#[derive(Clone, PartialEq)]
pub struct State {
    pub(crate) shape: BoxShape,
    pub(crate) color: Option<ColorRgba>,
    pub(crate) gradients: SmallVec<[Gradient; 4]>,
    pub(crate) border_radius: Option<BorderRadius>,
    pub(crate) border: Option<Border>,
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

impl DecorationBuilder {
    impl_id!();

    pub fn color(mut self, color: ColorRgba) -> Self {
        self.color = Some(color);

        self
    }

    pub fn border_radius(mut self, border_radius: BorderRadius) -> Self {
        self.border_radius = Some(border_radius);

        self
    }

    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);

        self
    }

    pub fn add_gradient(mut self, gradient: Gradient) -> Self {
        self.gradients.push(gradient);

        self
    }

    pub fn add_linear_gradient(mut self, gradient: LinearGradient) -> Self {
        self.gradients.push(Gradient::Linear(gradient));

        self
    }

    pub fn add_radial_gradient(mut self, gradient: RadialGradient) -> Self {
        self.gradients.push(Gradient::Radial(gradient));

        self
    }

    pub fn shape(mut self, shape: BoxShape) -> Self {
        self.shape = Some(shape);

        self
    }

    #[deprecated(note = "Use BuildContext::position instead")]
    pub fn when_positioned<F>(mut self, f: F) -> Self
    where
        F: Fn(&BuildContext, PositionedChildMeta) -> DecorationBuilder + 'static,
    {
        self.defer = Some(Box::new(f));
        self
    }

    pub fn build(self, context: &mut BuildContext) -> WidgetRef {
        scope(context.position.index).build(context, |context| {
            let id = self.id.with_seed(context.id_seed);
            context.accessed_this_frame(id);

            context.widgets_states.decorated_box.set(
                id,
                State {
                    color: self.color,
                    shape: self.shape.unwrap_or(BoxShape::Rect),
                    gradients: self.gradients,
                    border_radius: self.border_radius,
                    border: self.border,
                },
            );

            if let Some(defer) = self.defer {
                context
                    .decoration_defer
                    .push((id, context.position.index, defer));
            }

            WidgetRef::new(WidgetType::of::<DecoratedBox>(), id)
        })
    }
}

impl DecoratedBoxBuilder {
    pub fn color(mut self, color: ColorRgba) -> Self {
        self.color = Some(color);

        self
    }

    pub fn border_radius(mut self, border_radius: BorderRadius) -> Self {
        self.border_radius = Some(border_radius);

        self
    }

    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);

        self
    }

    pub fn add_gradient(mut self, gradient: Gradient) -> Self {
        self.gradients.push(gradient);

        self
    }

    pub fn add_linear_gradient(mut self, gradient: LinearGradient) -> Self {
        self.gradients.push(Gradient::Linear(gradient));

        self
    }

    pub fn add_radial_gradient(mut self, gradient: RadialGradient) -> Self {
        self.gradients.push(Gradient::Radial(gradient));

        self
    }

    pub fn shape(mut self, shape: BoxShape) -> Self {
        self.shape = shape;

        self
    }

    pub fn build(self, context: &mut BuildContext) {
        let id = self.frame.id.with_seed(context.id_seed);
        let widget_ref = WidgetRef::new(WidgetType::of::<DecoratedBox>(), id);
        let backgrounds = std::mem::take(context.backgrounds);
        let foregrounds = std::mem::take(context.foregrounds);

        if self.frame.offset_x != 0. || self.frame.offset_y != 0. {
            context.push_layout_command(LayoutCommand::BeginOffset {
                offset_x: self.frame.offset_x,
                offset_y: self.frame.offset_y,
            });
        }

        if self.frame.ignore_pointer {
            context.non_interactable.insert(id);
        }

        context.push_layout_command(LayoutCommand::Leaf {
            widget_ref,
            backgrounds,
            foregrounds,
            padding: self.frame.padding,
            margin: self.frame.margin,
            constraints: self.frame.constraints,
            size: self.frame.size,
            zindex: self.frame.zindex,
            derive_wrap_size: DeriveWrapSize::Constraints,
            clip: self.frame.clip,
        });

        if self.frame.offset_x != 0. || self.frame.offset_y != 0. {
            context.push_layout_command(LayoutCommand::EndOffset);
        }

        context.widgets_states.decorated_box.set(
            id,
            State {
                color: self.color,
                shape: self.shape,
                gradients: self.gradients.clone(),
                border_radius: self.border_radius,
                border: self.border,
            },
        );
        context.accessed_this_frame(id);
    }
}

#[track_caller]
pub fn decorated_box() -> DecoratedBoxBuilder {
    DecoratedBoxBuilder {
        frame: FrameBuilder::new(),
        color: None,
        gradients: smallvec![],
        border_radius: None,
        border: None,
        shape: BoxShape::Rect,
    }
}

#[track_caller]
pub fn decoration() -> DecorationBuilder {
    DecorationBuilder {
        id: WidgetId::auto(),
        color: None,
        gradients: smallvec![],
        border_radius: None,
        border: None,
        shape: None,
        defer: None,
    }
}

pub fn render(ctx: &mut RenderContext, placement: &WidgetPlacement, state: &State) {
    match state.shape {
        BoxShape::Rect => {
            if let Some(color) = state.color {
                ctx.push_command(
                    placement.zindex,
                    RenderCommand::Rect {
                        boundary: placement
                            .rect
                            .px_with_radius(ctx, state.border_radius.as_ref()),
                        fill: Some(Fill::Color(color)),
                        border_radius: state.border_radius.map(|it| it.px(ctx)),
                        border: None,
                    },
                );
            }

            for gradient in &state.gradients {
                ctx.push_command(
                    placement.zindex,
                    RenderCommand::Rect {
                        boundary: placement
                            .rect
                            .px_with_radius(ctx, state.border_radius.as_ref()),
                        fill: Some(Fill::Gradient(gradient.clone())),
                        border_radius: state.border_radius.map(|it| it.px(ctx)),
                        border: None,
                    },
                );
            }

            if let Some(border) = state.border {
                ctx.push_command(
                    placement.zindex,
                    RenderCommand::Rect {
                        boundary: placement
                            .rect
                            .px_with_radius(ctx, state.border_radius.as_ref()),
                        fill: None,
                        border_radius: state.border_radius.map(|it| it.px(ctx)),
                        border: Some(border.px(ctx)),
                    },
                );
            }
        }
        BoxShape::Oval => {
            let border = state.border.map(|it| it.px(ctx)).map(|it| {
                it.top
                    .or(it.bottom)
                    .or(it.left)
                    .or(it.right)
                    .unwrap_or(BorderSide::default())
            });

            if let Some(color) = state.color {
                ctx.push_command(
                    placement.zindex,
                    RenderCommand::Oval {
                        boundary: placement.rect.px(ctx),
                        fill: Some(Fill::Color(color)),
                        border: None,
                    },
                );
            }

            for gradient in &state.gradients {
                ctx.push_command(
                    placement.zindex,
                    RenderCommand::Oval {
                        boundary: placement.rect.px(ctx),
                        fill: Some(Fill::Gradient(gradient.clone())),
                        border: None,
                    },
                );
            }

            if state.border.is_some() {
                ctx.push_command(
                    placement.zindex,
                    RenderCommand::Oval {
                        boundary: placement.rect.px(ctx),
                        fill: None,
                        border,
                    },
                );
            }
        }
    }
}
