use clew_derive::WidgetBuilder;

use crate::{
    Axis, Clip, Size, SizeConstraint, Vec2, WidgetRef, WidgetType,
    layout::{ContainerKind, LayoutCommand},
    scroll_area::ScrollAreaWidget,
    widgets::scope::scope,
};

use super::{
    FrameBuilder,
    builder::{BuildContext, WidgetBuilder},
    scroll_area::ScrollAreaResponse,
};

#[derive(WidgetBuilder)]
pub struct ListViewBuilder {
    frame: FrameBuilder,
    item_size: f64,
    items_count: u64,
    axis: Axis,
}

impl ListViewBuilder {
    pub fn item_size(mut self, size: f64) -> Self {
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

    #[profiling::function]
    pub fn build<F>(mut self, context: &mut BuildContext, item_build: F)
    where
        F: Fn(&mut BuildContext, u64),
    {
        let id = self.frame.id.with_seed(context.id_seed);
        let widget_ref = WidgetRef::new(WidgetType::of::<ScrollAreaWidget>(), id);

        let (mut backgrounds, foregrounds) = context.resolve_decorators(&mut self.frame);
        backgrounds.push(widget_ref);

        let scroll_area_response = context
            .of::<ScrollAreaResponse>()
            .expect("List view should be inside scroll area")
            .clone();

        let layout_measures = context.widgets_states.layout_measures.get_mut(id);

        let list_offset = if let Some(layout_measures) = layout_measures {
            Some(Vec2::new(layout_measures.x, layout_measures.y))
        } else {
            None
        };

        match self.axis {
            Axis::Horizontal => {
                let width = self.item_size * self.items_count as f64;

                context.push_layout_command(LayoutCommand::BeginContainer {
                    backgrounds,
                    foregrounds,
                    zindex: self.frame.zindex,
                    padding: self.frame.padding,
                    margin: self.frame.margin,
                    kind: ContainerKind::Measure { id },
                    size: Size::new(SizeConstraint::Fixed(width), self.frame.size.height),
                    constraints: self.frame.constraints,
                    clip: self.frame.clip,
                });

                todo!();
            }
            Axis::Vertical => {
                let height = self.item_size * self.items_count as f64;

                context.push_layout_command(LayoutCommand::BeginContainer {
                    backgrounds,
                    foregrounds,
                    zindex: self.frame.zindex,
                    padding: self.frame.padding,
                    margin: self.frame.margin,
                    kind: ContainerKind::Measure { id },
                    size: Size::new(self.frame.size.width, SizeConstraint::Fixed(height)),
                    constraints: self.frame.constraints,
                    clip: self.frame.clip,
                });

                if !context.pre_layout {
                    if let Some(list_offset) = list_offset {
                        let scroll_area_viewport_height = if scroll_area_response.height == 0. {
                            context.view.physical_size.height as f64
                        } else {
                            scroll_area_response.height
                        };

                        let list_viewport_height = (scroll_area_viewport_height - list_offset.y)
                            .clamp(0., scroll_area_viewport_height);

                        if list_offset.y < scroll_area_viewport_height {
                            let first_visible = (-list_offset.y / self.item_size).floor() as u64;
                            let visible_count =
                                (list_viewport_height / self.item_size).ceil() as u64 + 1;
                            let last_visible =
                                (first_visible + visible_count).min(self.items_count);
                            let item_size = self.item_size;

                            for i in first_visible..last_visible {
                                context.push_layout_command(LayoutCommand::BeginOffset {
                                    offset_x: 0.,
                                    offset_y: (i as f64) * item_size,
                                });
                                scope(i).build(context, |ctx| item_build(ctx, i));
                                context.push_layout_command(LayoutCommand::EndOffset);
                            }
                        }
                    }
                }
            }
        }

        context.push_layout_command(LayoutCommand::EndContainer);

        context
            .widgets_states
            .layout_measures
            .accessed_this_frame
            .insert(id);
    }
}

#[track_caller]
pub fn list_view() -> ListViewBuilder {
    ListViewBuilder {
        frame: FrameBuilder::new().clip(Clip::Rect),
        axis: Axis::Vertical,
        item_size: 32.,
        items_count: 0,
    }
}
