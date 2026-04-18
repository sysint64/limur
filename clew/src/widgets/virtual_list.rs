use clew_derive::WidgetBuilder;

use crate::{
    Axis, Clip, WidgetRef, WidgetType,
    layout::{ContainerKind, LayoutCommand},
    scroll_area::ScrollAreaWidget,
    widgets::{scope::scope, scroll_area},
};

use super::{
    FrameBuilder,
    builder::{BuildContext, WidgetBuilder},
    scroll_area::ScrollAreaResponse,
};

#[derive(WidgetBuilder)]
pub struct VirtualListBuilder {
    frame: FrameBuilder,
    item_size: f32,
    items_count: u64,
    axis: Axis,
}

impl VirtualListBuilder {
    pub fn item_size(mut self, size: f32) -> Self {
        self.item_size = size;

        self
    }

    pub fn items_count(mut self, count: u64) -> Self {
        self.items_count = count;

        self
    }

    pub fn scroll_direction(mut self, axis: Axis) -> Self {
        self.axis = axis;

        self
    }

    pub fn build<F>(mut self, context: &mut BuildContext, item_build: F) -> ScrollAreaResponse
    where
        F: Fn(&mut BuildContext, u64),
    {
        let id = self.frame.id.with_seed(context.id_seed);
        let widget_ref = WidgetRef::new(WidgetType::of::<ScrollAreaWidget>(), id);

        let (mut backgrounds, foregrounds) = context.resolve_decorators(&mut self.frame);
        backgrounds.push(widget_ref);

        let (offset_x, offset_y, response) = {
            let state =
                context
                    .widgets_states
                    .scroll_area
                    .get_or_insert(id, || scroll_area::State {
                        last_offset_x: 0.,
                        last_offset_y: 0.,
                        offset_x: 0.,
                        offset_y: 0.,
                        overflow_x: false,
                        overflow_y: false,
                        scroll_direction: self.axis.to_scroll_direction(),
                        fraction_x: 0.,
                        fraction_y: 0.,
                        progress_x: 0.,
                        progress_y: 0.,
                        width: 0.,
                        height: 0.,
                        content_width: 0.,
                        content_height: 0.,
                    });

            // let layout_measures = context.widgets_states.layout_measures.get_mut(id);
            // let wrap_size = self.item_size as f64 * (self.items_count as f64);

            // if let Some(layout_measures) = layout_measures {
            //     scroll_area::handle_interaction(
            //         id,
            //         state,
            //         context.input,
            //         context.interaction,
            //         layout_measures,
            //         match self.axis {
            //             Axis::Horizontal => wrap_size,
            //             Axis::Vertical => 0.,
            //         },
            //         match self.axis {
            //             Axis::Horizontal => 0.,
            //             Axis::Vertical => wrap_size,
            //         },
            //     );
            // }

            state.scroll_direction = self.axis.to_scroll_direction();

            (
                state.offset_x,
                state.offset_y,
                ScrollAreaResponse {
                    id,
                    offset_x: state.offset_x,
                    offset_y: state.offset_y,
                    overflow_x: state.overflow_x,
                    overflow_y: state.overflow_y,
                    fraction_x: state.fraction_x,
                    fraction_y: state.fraction_y,
                    progress_x: state.progress_x,
                    progress_y: state.progress_y,
                    width: state.width,
                    height: state.height,
                    content_width: state.content_width,
                    content_height: state.content_height,
                },
            )
        };

        context.push_layout_command(LayoutCommand::BeginContainer {
            backgrounds,
            foregrounds,
            zindex: self.frame.zindex,
            padding: self.frame.padding,
            margin: self.frame.margin,
            kind: ContainerKind::Measure { id },
            size: self.frame.size,
            constraints: self.frame.constraints,
            clip: self.frame.clip,
        });

        match self.axis {
            Axis::Horizontal => {
                let viewport_width = if response.width == 0. {
                    context.view.physical_size.width as f32
                } else {
                    response.width as f32
                };

                let scroll_offset = -offset_x;

                let first_visible = (scroll_offset / self.item_size as f64).floor() as u64;
                let visible_count = (viewport_width / self.item_size).ceil() as u64 + 1;
                let last_visible = (first_visible + visible_count).min(self.items_count);
                let item_size = self.item_size as f64;

                for i in first_visible..last_visible {
                    // Position relative to viewport top
                    let relative_x = ((i - first_visible) as f64) * item_size;

                    // Adjust for partial scroll (how much of first item is scrolled off)
                    let first_item_offset = scroll_offset % item_size;
                    let final_x = relative_x - first_item_offset;

                    context.push_layout_command(LayoutCommand::BeginOffset {
                        offset_x: final_x,
                        offset_y: 0.,
                    });
                    scope(i).build(context, |ctx| item_build(ctx, i));
                    context.push_layout_command(LayoutCommand::EndOffset);
                }
            }
            Axis::Vertical => {
                let viewport_height = if response.height == 0. {
                    context.view.physical_size.height as f32
                } else {
                    response.height as f32
                };

                let scroll_offset = -offset_y;

                let first_visible = (scroll_offset / self.item_size as f64).floor() as u64;
                let visible_count = (viewport_height / self.item_size).ceil() as u64 + 1;
                let last_visible = (first_visible + visible_count).min(self.items_count);
                let item_size = self.item_size as f64;

                for i in first_visible..last_visible {
                    // Position relative to viewport top
                    let relative_y = ((i - first_visible) as f64) * item_size;

                    // Adjust for partial scroll (how much of first item is scrolled off)
                    let first_item_offset = scroll_offset % item_size;
                    let final_y = relative_y - first_item_offset;

                    context.push_layout_command(LayoutCommand::BeginOffset {
                        offset_x: 0.,
                        offset_y: final_y,
                    });
                    scope(i).build(context, |ctx| item_build(ctx, i));
                    context.push_layout_command(LayoutCommand::EndOffset);
                }
            }
        }

        context.push_layout_command(LayoutCommand::EndContainer);

        context.accessed_this_frame(id);
        response
    }
}

#[track_caller]
pub fn virtual_list() -> VirtualListBuilder {
    VirtualListBuilder {
        frame: FrameBuilder::new().clip(Clip::Rect),
        axis: Axis::Vertical,
        item_size: 32.,
        items_count: 0,
    }
}
