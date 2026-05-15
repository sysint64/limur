use limur_derive::{WidgetBuilder, WidgetState};
use smallvec::SmallVec;

use crate::{
    WidgetRef, WidgetType,
    layout::{DeriveWrapSize, LayoutCommand, WidgetPlacement},
    render::{PixelExtension, RenderCommand, RenderContext, ShaderBind, ShaderId, ShaderParam},
};

use super::{BuildContext, FrameBuilder, scope};

#[must_use = "widget is not rendered until .build(ctx) is called"]
#[derive(WidgetBuilder)]
pub struct BackdropFilterBuilder {
    frame: FrameBuilder,
    params: SmallVec<[(u32, ShaderParam); 4]>,
    shader_id: ShaderId,
}

pub struct BackdropFilter;

impl BackdropFilterBuilder {
    pub fn param(mut self, position: u32, param: ShaderParam) -> Self {
        self.params.push((position, param));

        self
    }

    pub fn build(self, ctx: &mut BuildContext) {
        scope(ctx.position.index).build(ctx, |ctx| {
            let id = self.frame.id.with_seed(ctx.id_seed);
            let widget_ref = WidgetRef::new(WidgetType::of::<BackdropFilter>(), id);

            let backgrounds = std::mem::take(ctx.backgrounds);
            let foregrounds = std::mem::take(ctx.foregrounds);

            if self.frame.offset_x != 0. || self.frame.offset_y != 0. {
                ctx.push_layout_command(LayoutCommand::BeginOffset {
                    offset_x: self.frame.offset_x,
                    offset_y: self.frame.offset_y,
                });
            }

            if self.frame.ignore_pointer {
                ctx.non_interactable.insert(id);
            }

            ctx.push_layout_command(LayoutCommand::Leaf {
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
                ctx.push_layout_command(LayoutCommand::EndOffset);
            }

            ctx.widgets_states.backdrop_filter.set(
                id,
                State {
                    params: self.params,
                    shader_id: self.shader_id,
                },
            );
            ctx.accessed_this_frame(id);
        });
    }
}

#[derive(WidgetState)]
pub struct State {
    params: SmallVec<[(u32, ShaderParam); 4]>,
    shader_id: ShaderId,
}

#[track_caller]
pub fn backdrop_filter(shader_id: ShaderId) -> BackdropFilterBuilder {
    BackdropFilterBuilder {
        frame: FrameBuilder::new(),
        params: SmallVec::new(),
        shader_id,
    }
}

pub fn render(ctx: &mut RenderContext, placement: &WidgetPlacement, state: &State) {
    ctx.push_command(
        placement.zindex,
        RenderCommand::BackdropFilter {
            boundary: placement.rect.px(ctx),
            shader: ShaderBind {
                id: state.shader_id,
                params: state.params.clone(),
            },
        },
    );
}
