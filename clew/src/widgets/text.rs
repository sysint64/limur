use clew_derive::WidgetBuilder;
use std::any::Any;

use crate::{
    AlignY, ColorRgba, TextAlign, Vec2, WidgetRef, WidgetType,
    layout::{DeriveWrapSize, LayoutCommand, WidgetPlacement},
    render::{PixelExtension, RenderCommand, RenderContext},
    state::WidgetState,
    text::TextId,
};

use super::{FrameBuilder, builder::BuildContext};

pub struct TextWidget;

#[derive(WidgetBuilder)]
pub struct TextBuilder<'a> {
    frame: FrameBuilder,
    text: &'a str,
    color: ColorRgba,
    text_align: TextAlign,
    font_size: f32,
    vertical_align: AlignY,
}

#[derive(Clone, PartialEq)]
pub struct State {
    pub(crate) text_id: TextId,
    pub(crate) text_data: String,
    pub(crate) color: ColorRgba,
    pub(crate) text_align: TextAlign,
    pub(crate) vertical_align: AlignY,
    pub(crate) scale_factor: f32,
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

impl<'a> TextBuilder<'a> {
    pub fn color(mut self, color: ColorRgba) -> Self {
        self.color = color;

        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;

        self
    }

    pub fn text_align(mut self, text_align: TextAlign) -> Self {
        self.text_align = text_align;

        self
    }

    pub fn text_vertical_align(mut self, align_y: AlignY) -> Self {
        self.vertical_align = align_y;

        self
    }

    #[profiling::function]
    pub fn build(mut self, context: &mut BuildContext) {
        let id = self.frame.id.with_seed(context.id_seed);

        let widget_ref = WidgetRef::new(WidgetType::of::<TextWidget>(), id);
        let state = context.widgets_states.text.get(id);
        let mut last_text_align = state.map(|it| it.text_align).unwrap_or(TextAlign::Auto);

        let (text_data, text_id) = if let Some(state) = state {
            if state.text_data != self.text || context.view.scale_factor != state.scale_factor {
                context.text.update_text(state.text_id, |text| {
                    text.set_text(context.fonts, self.text);
                });

                last_text_align = TextAlign::Auto;

                // Reset wrap size calculation during layout.
                if !self.frame.size.width.constrained() {
                    let text = context.text.get_mut(state.text_id);
                    text.with_buffer_mut(|buffer| {
                        buffer.set_size(&mut context.fonts.font_system, None, None);

                        for line in buffer.lines.iter_mut() {
                            line.set_align(None);
                        }
                    });
                }

                (Some(self.text.to_string()), state.text_id)
            } else {
                (None, state.text_id)
            }
        } else {
            let text_id =
                context
                    .text
                    .add_text(context.view, context.fonts, 12., 12., |fonts, text_res| {
                        text_res.set_text(fonts, self.text)
                    });

            (Some(self.text.to_string()), text_id)
        };

        if last_text_align != self.text_align {
            let text = context.text.get_mut(text_id);
            if self.frame.size.width.constrained() {
                text.with_buffer_mut(|buffer| {
                    for line in buffer.lines.iter_mut() {
                        line.set_align(match self.text_align {
                            TextAlign::Auto => None,
                            TextAlign::Left => Some(cosmic_text::Align::Left),
                            TextAlign::Right => Some(cosmic_text::Align::Right),
                            TextAlign::End => Some(cosmic_text::Align::End),
                            TextAlign::Center => Some(cosmic_text::Align::Center),
                            TextAlign::Justified => Some(cosmic_text::Align::Justified),
                        });
                    }
                });
            }
        }

        let (backgrounds, foregrounds) = context.resolve_decorators(&mut self.frame);

        context.push_layout_command(LayoutCommand::Leaf {
            widget_ref,
            backgrounds,
            foregrounds,
            padding: self.frame.padding,
            margin: self.frame.margin,
            constraints: self.frame.constraints,
            size: self.frame.size,
            zindex: self.frame.zindex,
            derive_wrap_size: DeriveWrapSize::Text(text_id),
            clip: self.frame.clip,
        });

        context.widgets_states.text.accessed_this_frame.insert(id);

        let state = context.widgets_states.text.get_or_insert(id, || State {
            text_id,
            text_data: text_data.clone().unwrap(),
            color: self.color,
            text_align: self.text_align,
            vertical_align: self.vertical_align,
            scale_factor: context.view.scale_factor,
        });

        if let Some(text_data) = text_data {
            state.text_data = text_data;
        }

        state.scale_factor = context.view.scale_factor;
        state.color = self.color;
        state.text_align = self.text_align;
    }
}

#[track_caller]
pub fn text(text: &str) -> TextBuilder<'_> {
    TextBuilder {
        frame: FrameBuilder::new(),
        text,
        color: ColorRgba::from_hex(0xFFFFFFFF),
        vertical_align: AlignY::Top,
        font_size: 12.,
        text_align: TextAlign::Left,
    }
}

pub fn render(ctx: &mut RenderContext, placement: &WidgetPlacement, state: &State) {
    let size = placement.rect.size().px(ctx);
    let position = placement.rect.position().px(ctx);

    let text = ctx.text.get_mut(state.text_id);
    let text_size = text.calculate_size();
    let text_position = position
        + Vec2::new(
            state
                .text_align
                .to_align_x()
                .position(ctx.layout_direction, size.x, text_size.x),
            state.vertical_align.position(size.y, text_size.y),
        );

    ctx.push_command(
        placement.zindex,
        RenderCommand::Text {
            x: text_position.x,
            y: text_position.y,
            text_id: state.text_id,
            tint_color: Some(state.color),
        },
    );
}
