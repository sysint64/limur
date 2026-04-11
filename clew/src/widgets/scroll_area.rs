use std::any::Any;

use clew_derive::WidgetBuilder;

use crate::{
    Clip, ScrollDirection, WidgetId, WidgetRef, WidgetType,
    interaction::InteractionState,
    io::UserInput,
    layout::{ContainerKind, LayoutCommand, LayoutMeasure},
    state::WidgetState,
};

use super::{
    FrameBuilder,
    builder::{BuildContext, WidgetBuilder},
};

pub struct ScrollAreaWidget;

#[derive(WidgetBuilder)]
pub struct ScrollAreaBuilder {
    frame: FrameBuilder,
    scroll_direction: ScrollDirection,
}

#[derive(Clone, PartialEq)]
pub struct State {
    pub(crate) last_offset_x: f64,
    pub(crate) last_offset_y: f64,
    pub(crate) offset_x: f64,
    pub(crate) offset_y: f64,
    pub(crate) fraction_x: f64,
    pub(crate) fraction_y: f64,
    pub(crate) progress_x: f64,
    pub(crate) progress_y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) content_width: f64,
    pub(crate) content_height: f64,
    pub(crate) overflow_x: bool,
    pub(crate) overflow_y: bool,
    pub(crate) scroll_direction: ScrollDirection,
}

#[derive(Clone, PartialEq)]
pub struct ScrollAreaResponse {
    pub id: WidgetId,
    pub offset_x: f64,
    pub offset_y: f64,
    pub fraction_x: f64,
    pub fraction_y: f64,
    pub progress_x: f64,
    pub progress_y: f64,
    pub width: f64,
    pub height: f64,
    pub content_width: f64,
    pub content_height: f64,
    pub overflow_x: bool,
    pub overflow_y: bool,
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

impl ScrollAreaBuilder {
    pub fn scroll_direction(mut self, scroll_direction: ScrollDirection) -> Self {
        self.scroll_direction = scroll_direction;

        self
    }

    pub fn build<F>(mut self, context: &mut BuildContext, callback: F) -> ScrollAreaResponse
    where
        F: FnOnce(&mut BuildContext),
    {
        let id = self.frame.id.with_seed(context.id_seed);
        let widget_ref = WidgetRef::new(WidgetType::of::<ScrollAreaWidget>(), id);

        let (mut backgrounds, foregrounds) = context.resolve_decorators(&mut self.frame);
        backgrounds.push(widget_ref);

        let (offset_x, offset_y, response) = {
            let state = context
                .widgets_states
                .scroll_area
                .get_or_insert(id, || State {
                    last_offset_x: 0.,
                    last_offset_y: 0.,
                    offset_x: 0.,
                    offset_y: 0.,
                    overflow_x: false,
                    overflow_y: false,
                    scroll_direction: self.scroll_direction,
                    fraction_x: 0.,
                    fraction_y: 0.,
                    progress_x: 0.,
                    progress_y: 0.,
                    width: 0.,
                    height: 0.,
                    content_width: 0.,
                    content_height: 0.,
                });

            let layout_measures = context.widgets_states.layout_measures.get_mut(id);

            if let Some(layout_measures) = layout_measures {
                if !context.pre_layout {
                    handle_interaction(
                        id,
                        state,
                        context.input,
                        context.interaction,
                        layout_measures,
                        layout_measures.wrap_width,
                        layout_measures.wrap_height,
                    );
                }
            }

            state.scroll_direction = self.scroll_direction;

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

        context.push_layout_command(LayoutCommand::BeginOffset { offset_x, offset_y });
        context.provide(response.clone(), callback);
        context.push_layout_command(LayoutCommand::EndOffset);

        context.push_layout_command(LayoutCommand::EndContainer);

        context
            .widgets_states
            .scroll_area
            .accessed_this_frame
            .insert(id);
        context
            .widgets_states
            .layout_measures
            .accessed_this_frame
            .insert(id);

        response
    }
}

#[track_caller]
pub fn scroll_area() -> ScrollAreaBuilder {
    ScrollAreaBuilder {
        frame: FrameBuilder::new().clip(Clip::Rect),
        scroll_direction: ScrollDirection::Vertical,
    }
}

pub fn set_offset_x(context: &mut BuildContext, id: WidgetId, value: f64) {
    let state = context.widgets_states.scroll_area.get_mut(id);

    if let Some(state) = state {
        state.offset_x = -value;
    }
}

pub fn set_offset_y(context: &mut BuildContext, id: WidgetId, value: f64) {
    let state = context.widgets_states.scroll_area.get_mut(id);

    if let Some(state) = state {
        state.offset_y = -value;
    }
}

pub fn set_progress_x(context: &mut BuildContext, id: WidgetId, value: f64) {
    let state = context.widgets_states.scroll_area.get_mut(id);

    if let Some(state) = state {
        state.offset_x = -(state.content_width - state.width) * value;
    }
}

pub fn set_progress_y(context: &mut BuildContext, id: WidgetId, value: f64) {
    if context.pre_layout() {
        return;
    }

    let state = context.widgets_states.scroll_area.get_mut(id);

    if let Some(state) = state {
        state.offset_y = -(state.content_height - state.height) * value;
    }
}

pub fn handle_interaction(
    id: WidgetId,
    widget_state: &mut State,
    input: &UserInput,
    interaction_state: &InteractionState,
    layout_measure: &LayoutMeasure,
    wrap_width: f64,
    wrap_height: f64,
) {
    if widget_state.scroll_direction == ScrollDirection::Vertical
        || widget_state.scroll_direction == ScrollDirection::Both
    {
        if input.mouse_wheel_delta_y != 0. && interaction_state.is_hover(&id) {
            widget_state.offset_y += input.mouse_wheel_delta_y as f64;
        }

        widget_state.offset_y = widget_state
            .offset_y
            .clamp(f64::min(0., -(wrap_height - layout_measure.height)), 0.);

        widget_state.overflow_y = layout_measure.height - wrap_height <= 0.;
        widget_state.fraction_y = layout_measure.height / wrap_height;
        widget_state.height = layout_measure.height;
        widget_state.content_height = wrap_height;
        widget_state.progress_y = -widget_state.offset_y / (wrap_height - layout_measure.height);
        widget_state.progress_y = widget_state.progress_y.clamp(0., 1.);
    }

    if widget_state.scroll_direction == ScrollDirection::Horizontal
        || widget_state.scroll_direction == ScrollDirection::Both
    {
        if input.mouse_wheel_delta_x != 0. && interaction_state.is_hover(&id) {
            widget_state.offset_x += input.mouse_wheel_delta_x as f64;
        }

        widget_state.offset_x = widget_state
            .offset_x
            .clamp(f64::min(0., -(wrap_width - layout_measure.width)), 0.);

        widget_state.overflow_x = layout_measure.width - wrap_width <= 0.;
        widget_state.fraction_x = layout_measure.width / wrap_width;
        widget_state.width = layout_measure.width;
        widget_state.content_width = wrap_width;
        widget_state.progress_x = -widget_state.offset_x / (wrap_width - layout_measure.width);
        widget_state.progress_x = widget_state.progress_x.clamp(0., 1.);
    }
}
