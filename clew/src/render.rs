use std::collections::HashMap;

use crate::{
    Border, BorderRadius, BorderSide, ClipShape, ColorRgb, ColorRgba, DebugBoundary, Gradient,
    LayoutDirection, Rect, Vec2, View, WidgetType,
    assets::Assets,
    interaction::{InteractionState, handle_interaction},
    io::UserInput,
    layout::{LayoutItem, WidgetPlacement, layout},
    state::UiState,
    text::{FontResources, StringId, StringInterner, TextId, TextsResources},
    widgets,
};

#[derive(Debug, Default)]
pub struct RenderState {
    pub(crate) commands: Vec<RenderCommand>,
    pub(crate) unsorted_commands: Vec<RenderCommandUnsorted>,
}

impl RenderState {
    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }
}

pub trait Renderer {
    fn upload_svg(&mut self, _name: &'static str, _tree: &usvg::Tree) {}

    fn on_scale_factor_update(&mut self, _scale_factor: f32) {}

    fn process_commands(
        &mut self,
        view: &View,
        state: &RenderState,
        fill_color: ColorRgb,
        fonts: &mut FontResources,
        text: &mut TextsResources,
        assets: &Assets,
    );
}

pub struct RenderContext<'a, 'b> {
    pub interaction: &'a InteractionState,
    pub input: &'a UserInput,
    pub view: &'a View,
    pub text: &'a mut TextsResources<'b>,
    pub fonts: &'a mut FontResources,
    pub string_interner: &'a mut StringInterner,
    pub strings: &'a mut HashMap<StringId, TextId>,
    pub layout_direction: LayoutDirection,
    unsorted_commands: &'a mut Vec<RenderCommandUnsorted>,
}

impl RenderContext<'_, '_> {
    pub fn push_command(&mut self, zindex: i32, command: RenderCommand) {
        self.unsorted_commands
            .push(RenderCommandUnsorted::RenderCommand { zindex, command });
    }
}

#[derive(Debug, Clone)]
pub enum RenderCommand {
    Rect {
        boundary: Rect<f32>,
        fill: Option<Fill>,
        border_radius: Option<BorderRadius>,
        border: Option<Border>,
    },
    Oval {
        boundary: Rect<f32>,
        fill: Option<Fill>,
        border: Option<BorderSide>,
    },
    Text {
        x: f32,
        y: f32,
        text_id: TextId,
        tint_color: Option<ColorRgba>,
    },
    Svg {
        boundary: Rect<f32>,
        asset_id: &'static str,
        tint_color: Option<ColorRgba>,
    },
    PushClip {
        rect: Rect<f32>,
        shape: ClipShape,
    },
    PopClip,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum RenderCommandUnsorted {
    RenderCommand { zindex: i32, command: RenderCommand },
    BeginGroup { zindex: i32 },
    EndGroup,
}

impl RenderCommandUnsorted {
    pub fn zindex(&self) -> i32 {
        match self {
            RenderCommandUnsorted::RenderCommand { zindex, .. } => *zindex,
            RenderCommandUnsorted::BeginGroup { zindex, .. } => *zindex,
            RenderCommandUnsorted::EndGroup => {
                unreachable!("End markers should not be sorted independently")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Fill {
    None,
    Color(ColorRgba),
    Gradient(Gradient),
}

pub trait PixelExtension<T> {
    fn px(self, ctx: &RenderContext) -> T;
}

impl PixelExtension<f32> for f32 {
    fn px(self, ctx: &RenderContext) -> f32 {
        (self as f64 * ctx.view.scale_factor) as f32
    }
}

impl PixelExtension<f32> for f64 {
    fn px(self, ctx: &RenderContext) -> f32 {
        (self * ctx.view.scale_factor) as f32
    }
}

impl PixelExtension<Vec2<f32>> for Vec2<f64> {
    fn px(self, ctx: &RenderContext) -> Vec2<f32> {
        Vec2::new(self.x.px(ctx), self.y.px(ctx))
    }
}

// impl PixelExtension<Rect<f32>> for Rect<f64> {
//     fn px(self, ctx: &RenderContext) -> Rect<f32> {
//         let scaled = self * ctx.view.scale_factor;

//         Rect {
//             x: scaled.x.clamp(-ctx.view.size().x, ctx.view.size().x * 2.) as f32,
//             y: scaled.y.clamp(-ctx.view.size().y, ctx.view.size().y * 2.) as f32,
//             width: scaled
//                 .width
//                 .clamp(-ctx.view.size().x, ctx.view.size().x * 4.) as f32,
//             height: scaled
//                 .height
//                 .clamp(-ctx.view.size().y, ctx.view.size().y * 4.) as f32,
//         }
//     }
// }

impl PixelExtension<Rect<f32>> for Rect<f64> {
    fn px(self, ctx: &RenderContext) -> Rect<f32> {
        let scaled = self * ctx.view.scale_factor;
        let vw = ctx.view.physical_size.width as f64;
        let vh = ctx.view.physical_size.height as f64;

        let pad = ctx.view.scale_factor.ceil();

        let left = scaled.x.max(-pad);
        let top = scaled.y.max(-pad);
        let right = (scaled.x + scaled.width).min(vw + pad);
        let bottom = (scaled.y + scaled.height).min(vh + pad);

        Rect {
            x: left as f32,
            y: top as f32,
            width: (right - left).max(0.0) as f32,
            height: (bottom - top).max(0.0) as f32,
        }
    }
}

impl Rect<f64> {
    pub fn px_with_radius(
        self,
        ctx: &RenderContext,
        border_radius: Option<&BorderRadius>,
    ) -> Rect<f32> {
        let scaled = self * ctx.view.scale_factor;
        let vw = ctx.view.physical_size.width as f64;
        let vh = ctx.view.physical_size.height as f64;

        let pad = border_radius.map_or(0.0, |br| {
            br.top_left
                .max(br.top_right)
                .max(br.bottom_left)
                .max(br.bottom_right) as f64
        }) + ctx.view.scale_factor.ceil();

        let left = scaled.x.max(-pad);
        let top = scaled.y.max(-pad);
        let right = (scaled.x + scaled.width).min(vw + pad);
        let bottom = (scaled.y + scaled.height).min(vh + pad);

        Rect {
            x: left as f32,
            y: top as f32,
            width: (right - left).max(0.0) as f32,
            height: (bottom - top).max(0.0) as f32,
        }
    }
}

impl PixelExtension<BorderRadius> for BorderRadius {
    fn px(self, ctx: &RenderContext) -> BorderRadius {
        BorderRadius {
            top_left: (self.top_left as f64 * ctx.view.scale_factor) as f32,
            top_right: (self.top_right as f64 * ctx.view.scale_factor) as f32,
            bottom_left: (self.bottom_left as f64 * ctx.view.scale_factor) as f32,
            bottom_right: (self.bottom_right as f64 * ctx.view.scale_factor) as f32,
        }
    }
}

impl PixelExtension<BorderSide> for BorderSide {
    fn px(self, ctx: &RenderContext) -> BorderSide {
        BorderSide {
            width: (self.width as f64 * ctx.view.scale_factor) as f32,
            color: self.color,
        }
    }
}

impl PixelExtension<Border> for Border {
    fn px(self, ctx: &RenderContext) -> Border {
        Border {
            top: self.top.map(|border_side| border_side.px(ctx)),
            right: self.right.map(|border_side| border_side.px(ctx)),
            bottom: self.bottom.map(|border_side| border_side.px(ctx)),
            left: self.left.map(|border_side| border_side.px(ctx)),
        }
    }
}

impl PixelExtension<ClipShape> for ClipShape {
    fn px(self, ctx: &RenderContext) -> ClipShape {
        match self {
            ClipShape::Rect => self,
            ClipShape::RoundedRect { border_radius } => ClipShape::RoundedRect {
                border_radius: border_radius.px(ctx),
            },
            ClipShape::Oval => self,
        }
    }
}

#[derive(Clone, Copy)]
enum GroupKind {
    Clip,
    Group,
}

impl GroupKind {
    fn matches_end(&self, cmd: &RenderCommandUnsorted) -> bool {
        matches!(
            (self, cmd),
            (
                GroupKind::Clip,
                RenderCommandUnsorted::RenderCommand {
                    command: RenderCommand::PopClip,
                    ..
                }
            ) | (GroupKind::Group, RenderCommandUnsorted::EndGroup)
        )
    }
}

fn get_zindex(cmd: &RenderCommandUnsorted) -> i32 {
    match cmd {
        RenderCommandUnsorted::RenderCommand { zindex, .. } => *zindex,
        RenderCommandUnsorted::BeginGroup { zindex } => *zindex,
        RenderCommandUnsorted::EndGroup => {
            unreachable!("EndGroup should not be queried for zindex")
        }
    }
}

fn group_start(cmd: &RenderCommandUnsorted) -> Option<GroupKind> {
    match cmd {
        RenderCommandUnsorted::RenderCommand {
            command: RenderCommand::PushClip { .. },
            ..
        } => Some(GroupKind::Clip),
        RenderCommandUnsorted::BeginGroup { .. } => Some(GroupKind::Group),
        _ => None,
    }
}

fn group_end(cmd: &RenderCommandUnsorted) -> Option<GroupKind> {
    match cmd {
        RenderCommandUnsorted::RenderCommand {
            command: RenderCommand::PopClip,
            ..
        } => Some(GroupKind::Clip),
        RenderCommandUnsorted::EndGroup => Some(GroupKind::Group),
        _ => None,
    }
}

pub fn sort_render_commands(
    commands: &mut Vec<RenderCommandUnsorted>,
    output: &mut Vec<RenderCommand>,
) {
    let len = commands.len();
    sort_segment(commands, 0, len);

    output.clear();

    for cmd in commands.drain(..) {
        if let RenderCommandUnsorted::RenderCommand { command, .. } = cmd {
            output.push(command);
        }
    }
}

fn sort_segment(commands: &mut [RenderCommandUnsorted], start: usize, end: usize) {
    if start >= end {
        return;
    }

    // First pass: identify items and groups
    let mut items: Vec<(usize, usize, i32, bool)> = Vec::new();
    //                  ^^^^^  ^^^^^  ^^^  ^^^^
    //                  start  end    z    is_group)

    let mut i = start;

    while i < end {
        if let Some(kind) = group_start(&commands[i]) {
            let group_start_idx = i;
            let group_zindex = get_zindex(&commands[i]);
            let mut depth = 1;
            i += 1;

            while i < end && depth > 0 {
                if group_start(&commands[i]).is_some() {
                    depth += 1;
                } else if kind.matches_end(&commands[i]) {
                    depth -= 1;
                }
                i += 1;
            }

            items.push((group_start_idx, i, group_zindex, true));
        } else if group_end(&commands[i]).is_some() {
            break;
        } else {
            items.push((i, i + 1, get_zindex(&commands[i]), false));
            i += 1;
        }
    }

    items.sort_by_key(|&(start, _, z, _)| (z, start));

    // Rearrange and track new positions
    let original: Vec<RenderCommandUnsorted> = commands[start..end].to_vec();
    let base = start;

    let mut write_pos = start;
    // Store the new positions of each group for recursion
    let mut group_ranges: Vec<(usize, usize)> = Vec::new();

    for &(item_start, item_end, _, is_group) in &items {
        let src_start = item_start - base;
        let src_end = item_end - base;
        let len = src_end - src_start;

        let new_start = write_pos;
        commands[write_pos..write_pos + len].clone_from_slice(&original[src_start..src_end]);
        write_pos += len;

        if is_group {
            // Content is between the group_start marker and the group_end marker
            let content_start = new_start + 1;
            let content_end = new_start + len - 1;
            if content_start < content_end {
                group_ranges.push((content_start, content_end));
            }
        }
    }

    // Recurse using the tracked positions, not a re-scan
    for (content_start, content_end) in group_ranges {
        sort_segment(commands, content_start, content_end);
    }
}

// fn sort_segment(commands: &mut [RenderCommandUnsorted], start: usize, end: usize) {
//     if start >= end {
//         return;
//     }

//     let mut items: Vec<(usize, usize, i32)> = Vec::new();
//     let mut i = start;

//     while i < end {
//         if let Some(kind) = group_start(&commands[i]) {
//             let group_start_idx = i;
//             let group_zindex = get_zindex(&commands[i]);
//             let mut depth = 1;
//             i += 1;

//             while i < end && depth > 0 {
//                 if group_start(&commands[i]).is_some() {
//                     depth += 1;
//                 } else if kind.matches_end(&commands[i]) {
//                     depth -= 1;
//                 }
//                 i += 1;
//             }

//             items.push((group_start_idx, i, group_zindex));
//         } else if group_end(&commands[i]).is_some() {
//             break;
//         } else {
//             items.push((i, i + 1, get_zindex(&commands[i])));
//             i += 1;
//         }
//     }

//     items.sort_by_key(|&(start, _, z)| (z, start));

//     let original: Vec<RenderCommandUnsorted> = commands[start..end].to_vec();
//     let base = start;

//     let mut write_pos = start;
//     for (item_start, item_end, _) in &items {
//         let src_start = item_start - base;
//         let src_end = item_end - base;
//         let len = src_end - src_start;

//         commands[write_pos..write_pos + len].clone_from_slice(&original[src_start..src_end]);
//         write_pos += len;
//     }

//     // Recursively sort inside each group
//     let mut i = start;
//     while i < end {
//         if let Some(kind) = group_start(&commands[i]) {
//             let content_start = i + 1;
//             let mut depth = 1;
//             i += 1;

//             while i < end && depth > 0 {
//                 if group_start(&commands[i]).is_some() {
//                     depth += 1;
//                 } else if kind.matches_end(&commands[i]) {
//                     depth -= 1;
//                 }
//                 i += 1;
//             }

//             sort_segment(commands, content_start, i - 1);
//         } else {
//             i += 1;
//         }
//     }
// }

pub fn pre_layout(
    state: &mut UiState,
    text_resources: &mut TextsResources,
    fonts: &mut FontResources,
    assets: &Assets,
) {
    profiling::scope!("clew :: PreLayout");

    layout(
        &mut state.layout_state,
        &state.view,
        &state.layout_commands,
        &mut state.layout_items,
        &mut state.widgets_states.layout_measures,
        text_resources,
        assets,
    );

    for layout_text in &state.layout_state.texts {
        let text = text_resources.get_mut(layout_text.text_id);

        text.with_buffer_mut(|buffer| {
            buffer.set_size(
                &mut fonts.font_system,
                layout_text.width.map(|it| it as f32),
                layout_text.height.map(|it| it as f32),
            );
        });

        text_resources.shape_as_needed(layout_text.text_id, &mut fonts.font_system, false);
    }

    layout(
        &mut state.layout_state,
        &state.view,
        &state.layout_commands,
        &mut state.layout_items,
        &mut state.widgets_states.layout_measures,
        text_resources,
        assets,
    );
}

pub fn layout_and_render(
    state: &mut UiState,
    text_resources: &mut TextsResources,
    fonts: &mut FontResources,
    assets: &Assets,
    string_interner: &mut StringInterner,
    strings: &mut HashMap<StringId, TextId>,
    force_redraw: bool,
) -> bool {
    let mut need_to_redraw = false;

    // let layout_time = std::time::Instant::now();
    {
        profiling::scope!("clew :: Layout");

        layout(
            &mut state.layout_state,
            &state.view,
            &state.layout_commands,
            &mut state.layout_items,
            &mut state.widgets_states.layout_measures,
            text_resources,
            assets,
        );

        for layout_text in &state.layout_state.texts {
            let text = text_resources.get_mut(layout_text.text_id);

            text.with_buffer_mut(|buffer| {
                buffer.set_size(
                    &mut fonts.font_system,
                    layout_text.width.map(|it| it as f32),
                    layout_text.height.map(|it| it as f32),
                );
            });

            text_resources.shape_as_needed(layout_text.text_id, &mut fonts.font_system, false);
        }

        layout(
            &mut state.layout_state,
            &state.view,
            &state.layout_commands,
            &mut state.layout_items,
            &mut state.widgets_states.layout_measures,
            text_resources,
            assets,
        );
    }

    tracy_client::plot!(
        "clew :: Layout commands",
        state.layout_commands.len() as f64
    );

    {
        profiling::scope!("clew :: Interaction");

        need_to_redraw = need_to_redraw
            || handle_interaction(
                &mut state.user_input,
                &mut state.interaction_state,
                &state.non_interactable,
                &state.view,
                text_resources,
                fonts,
                &state.layout_items,
            );

        need_to_redraw = need_to_redraw || state.interaction_state != state.last_interaction_state;
        state.last_interaction_state = state.interaction_state.clone();
    }

    state.render_state.unsorted_commands.clear();

    if force_redraw || need_to_redraw {
        profiling::scope!("clew :: Collect Render Commands");

        for layout_item in &state.layout_items {
            let mut render_context = RenderContext {
                interaction: &state.interaction_state,
                input: &state.user_input,
                view: &state.view,
                text: text_resources,
                fonts,
                string_interner,
                strings,
                layout_direction: state.layout_direction,
                unsorted_commands: &mut state.render_state.unsorted_commands,
            };

            match layout_item {
                LayoutItem::Placement(placement) => {
                    if placement.widget_ref.widget_type
                        == WidgetType::of::<widgets::text::TextWidget>()
                    {
                        widgets::text::render(
                            &mut render_context,
                            placement,
                            state
                                .widgets_states
                                .text
                                .get(placement.widget_ref.id)
                                .unwrap(),
                        );
                    }

                    if placement.widget_ref.widget_type
                        == WidgetType::of::<widgets::decorated_box::DecoratedBox>()
                    {
                        widgets::decorated_box::render(
                            &mut render_context,
                            placement,
                            state
                                .widgets_states
                                .decorated_box
                                .get(placement.widget_ref.id)
                                .unwrap(),
                            // state
                            //     .widgets_states
                            //     .get_mut::<widgets::decorated_box::State>(placement.widget_ref.id)
                            //     .unwrap(),
                        );
                    }

                    if placement.widget_ref.widget_type
                        == WidgetType::of::<widgets::svg::SvgWidget>()
                    {
                        widgets::svg::render(
                            &mut render_context,
                            placement,
                            state
                                .widgets_states
                                .svg
                                .get(placement.widget_ref.id)
                                .unwrap(),
                            // state
                            //     .widgets_states
                            //     .get_mut::<widgets::svg::State>(placement.widget_ref.id)
                            //     .unwrap(),
                        );
                    }

                    if placement.widget_ref.widget_type
                        == WidgetType::of::<widgets::editable_text::EditableTextWidget>()
                    {
                        widgets::editable_text::render(
                            &mut render_context,
                            placement,
                            state
                                .widgets_states
                                .editable_text
                                .get_mut(placement.widget_ref.id)
                                .unwrap(),
                            &mut state.view_config,
                            true,
                        );
                    }

                    if placement.widget_ref.widget_type == WidgetType::of::<DebugBoundary>() {
                        render_debug_boundary(&mut render_context, placement);
                    }
                }
                LayoutItem::PushClip { rect, clip, zindex } => {
                    let shape = clip
                        .to_shape()
                        .expect("Cannot push clip without a shape")
                        .px(&render_context);

                    let rect = rect.px(&render_context);

                    state.render_state.unsorted_commands.push(
                        RenderCommandUnsorted::RenderCommand {
                            zindex: *zindex,
                            command: RenderCommand::PushClip { rect, shape },
                        },
                    )
                }
                LayoutItem::PopClip => {
                    state.render_state.unsorted_commands.push(
                        RenderCommandUnsorted::RenderCommand {
                            zindex: 0,
                            command: RenderCommand::PopClip,
                        },
                    );
                }
                LayoutItem::BeginGroup { zindex } => {
                    state
                        .render_state
                        .unsorted_commands
                        .push(RenderCommandUnsorted::BeginGroup { zindex: *zindex });
                }
                LayoutItem::EndGroup => {
                    // state.render_state.commands.push(RenderCommand::EndGroup);

                    state
                        .render_state
                        .unsorted_commands
                        .push(RenderCommandUnsorted::EndGroup);
                }
            }
        }

        tracy_client::plot!("clew :: Layout Items", state.layout_items.len() as f64);

        tracy_client::plot!(
            "clew :: Render Commands",
            state.render_state.unsorted_commands.len() as f64
        );
    }

    state.widgets_states.sweep();
    state.user_input.clear_frame_events();

    {
        profiling::scope!("clew :: Sort commands by zindex");

        // println!("Before sort:");
        // for (i, cmd) in state.render_state.commands.iter().enumerate() {
        //     println!("  {}: {:?}", i, cmd);
        // }

        sort_render_commands(
            &mut state.render_state.unsorted_commands,
            &mut state.render_state.commands,
        );

        // println!("After sort:");
        // for (i, cmd) in state.render_state.commands.iter().enumerate() {
        //     println!("  {}: {:?}", i, cmd);
        // }

        // sort_render_commands(&mut state.render_state.commands);
        // state
        //     .render_state
        //     .commands
        //     .sort_by_key(|cmd| cmd.zindex().unwrap_or(i32::MAX));
    }

    {
        profiling::scope!("clew :: Reset phase allocator");
        state.phase_allocator.reset();
    }

    force_redraw || need_to_redraw
}

fn render_debug_boundary(ctx: &mut RenderContext, placement: &WidgetPlacement) {
    ctx.push_command(
        placement.zindex,
        RenderCommand::Rect {
            boundary: placement.rect.shrink(2.).px(ctx),
            fill: None,
            border_radius: None,
            border: Some(Border::all(BorderSide::new(
                2.,
                ColorRgba::from_hex(0xFFFF0000),
            ))),
        },
    );
}
