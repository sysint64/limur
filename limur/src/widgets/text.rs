use limur_derive::WidgetBuilder;
use std::any::Any;

use crate::{
    AlignY, ColorRgba, TextAlign, Vec2, WidgetRef, WidgetType,
    layout::{DeriveWrapSize, LayoutCommand, WidgetPlacement},
    profiler,
    render::{PixelExtension, RenderCommand, RenderContext},
    state::WidgetState,
    text::TextId,
};

use super::{FrameBuilder, builder::BuildContext, scope};

pub struct TextWidget;

#[derive(WidgetBuilder)]
pub struct TextBuilder<'a> {
    frame: FrameBuilder,
    text: &'a str,
    color: ColorRgba,
    text_align: TextAlign,
    font_size: f32,
    line_height: f32,
    vertical_align: AlignY,
}

#[derive(Clone, PartialEq)]
pub struct State {
    pub(crate) text_id: TextId,
    pub(crate) text_data: String,
    pub(crate) color: ColorRgba,
    pub(crate) text_align: TextAlign,
    pub(crate) font_size: f32,
    pub(crate) line_height: f32,
    pub(crate) vertical_align: AlignY,
    pub(crate) scale_factor: f64,
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
    pub fn color<T: Into<ColorRgba>>(mut self, color: T) -> Self {
        self.color = color.into();

        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;

        self
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;

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

    pub fn build(mut self, context: &mut BuildContext) {
        let scope_name = if context.pre_layout {
            "text::build::pass1"
        } else {
            "text::build::pass2"
        };
        let _g = profiler::scope_named(scope_name);

        scope(context.position.index).build(context, |context| {
            let id = self.frame.id.with_seed(context.id_seed);

            let widget_ref = WidgetRef::new(WidgetType::of::<TextWidget>(), id);
            let state = context.widgets_states.text.get(id);
            let mut last_text_align = state.map(|it| it.text_align).unwrap_or(TextAlign::Auto);

            let (text_data, text_id) = if let Some(state) = state {
                if state.font_size != self.font_size || state.line_height != self.line_height {
                    context.text.update_text(state.text_id, |text| {
                        text.set_metrics(
                            context.view,
                            context.fonts,
                            self.font_size,
                            self.font_size * self.line_height,
                        );
                    });
                }

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
                let text_id = context.text.add_text(
                    context.view,
                    context.fonts,
                    self.font_size,
                    self.font_size * self.line_height,
                    |fonts, text_res| text_res.set_text(fonts, self.text),
                );

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
                derive_wrap_size: DeriveWrapSize::Text {
                    text_id,
                    derive_width: true,
                    derive_height: true,
                },
                clip: self.frame.clip,
            });

            context.accessed_this_frame(id);

            let state = context.widgets_states.text.get_or_insert(id, || State {
                text_id,
                text_data: text_data.clone().unwrap(),
                color: self.color,
                text_align: self.text_align,
                vertical_align: self.vertical_align,
                scale_factor: context.view.scale_factor,
                font_size: self.font_size,
                line_height: self.line_height,
            });

            if let Some(text_data) = text_data {
                state.text_data = text_data;
            }

            state.scale_factor = context.view.scale_factor;
            state.color = self.color;
            state.text_align = self.text_align;
            state.font_size = self.font_size;
            state.line_height = self.line_height;
        });
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
        line_height: 1.,
        text_align: TextAlign::Left,
    }
}

pub fn render(ctx: &mut RenderContext, placement: &WidgetPlacement, state: &State) {
    let size = placement.rect.size().px(ctx);
    let position = placement.rect.position().px(ctx);

    let text = ctx.text.get_mut(state.text_id);
    let text_size = text.calculate_size().as_f32();
    let text_position = position
        + Vec2::new(
            state
                .text_align
                .to_align_x()
                .position_f32(ctx.layout_direction, size.x, text_size.x),
            state.vertical_align.position_f32(size.y, text_size.y),
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
