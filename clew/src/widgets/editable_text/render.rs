use cosmic_text::Edit;
use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    AlignX, AlignY, ClipShape, LayoutDirection, Rect, Vec2,
    layout::WidgetPlacement,
    render::{Fill, PixelExtension, RenderCommand, RenderContext},
    state::ViewConfig,
    text::{FontResources, TextId},
};

use super::State;

pub fn render(
    ctx: &mut RenderContext,
    placement: &WidgetPlacement,
    state: &mut State,
    view_config: &mut ViewConfig,
    // First calculate scroll before render to prevent one frame delay
    calculate_scroll: bool,
) {
    let size = placement.rect.size().px(ctx);
    let position = placement.rect.position().px(ctx);

    state.boundary = placement.rect;

    let text_id = state
        .text_id
        .expect("Should be initialized during build phase");

    let text = ctx.text.get_mut(text_id);
    let text_size = text.calculate_size();
    let text_position =
        position + Vec2::new(0., state.vertical_align.position(size.y, text_size.y));

    let is_focused = state.gesture_detector_response.is_focused();

    let (need_shape, is_empty, wrap, editor_cursor_position, cursor, selection_bounds) = {
        let mut need_shape = false;
        let editor = ctx.text.editor_mut(text_id);
        let need_relayout = state.last_boundary_size != size;
        let wrap = editor.with_buffer(|buffer| buffer.wrap() != cosmic_text::Wrap::None);

        if state.recompose_text_content || need_relayout || editor.redraw() {
            if need_relayout || editor.redraw() {
                state.visible_view_updated = true;
            }

            editor.set_redraw(false);
            state.recompose_text_content = false;
            need_shape = true;
        }

        state.last_boundary_size = size;

        if calculate_scroll {
            state.was_relayout = need_relayout;
        }

        let is_current_line_empty = editor.with_buffer(|buffer| {
            let line = editor.cursor().line;

            if let Some(line) = buffer.lines.get(line) {
                line.text().is_empty()
            } else {
                buffer.lines.is_empty() || (buffer.lines.len() == 1 && buffer.lines[0].text() == "")
            }
        });

        let is_empty = ctx.input.ime_preedit.is_empty() && is_current_line_empty;

        (
            need_shape,
            is_empty,
            wrap,
            editor.cursor_position(),
            editor.cursor(),
            editor.selection_bounds(),
        )
    };

    let top = state.vertical_align.position(size.y, text_size.y);
    let mut text_direction = if state.multi_line {
        LayoutDirection::LTR
    } else {
        view_config.layout_direction
    };

    let editor = ctx.text.editor_mut(text_id);
    let is_text_rtl =
        editor.with_buffer(|buffer| buffer.layout_runs().next().is_none_or(|run| run.rtl));

    if is_text_rtl && state.auto_rtl && !state.multi_line && text_direction != LayoutDirection::RTL
    {
        text_direction = LayoutDirection::RTL;
    }

    state.text_offset = match text_direction {
        LayoutDirection::LTR => Vec2::new(state.scroll_x, top),
        LayoutDirection::RTL => Vec2::new(
            state.scroll_x + AlignX::Right.position(text_direction, size.x, text_size.x),
            top,
        ),
    };

    let text_position = position + state.text_offset;
    let mut cursor_ime_position = Vec2::ZERO;

    let clip_rect = Rect::from_pos_size(position, size).expand(10.0.px(ctx));

    if !calculate_scroll || !is_focused {
        ctx.push_command(
            placement.zindex,
            RenderCommand::PushClip {
                rect: clip_rect,
                shape: ClipShape::Rect,
            },
        );

        ctx.push_command(
            placement.zindex,
            RenderCommand::Text {
                x: text_position.x,
                y: text_position.y,
                text_id,
                tint_color: Some(state.color),
            },
        );
    }

    if is_focused {
        let mut cursor_position = if let Some((x, y)) = editor_cursor_position {
            if state.multi_line && view_config.layout_direction == LayoutDirection::RTL && is_empty
            {
                // When line is empty and it's RTL we want to draw cursor at right
                Vec2::new(position.x + size.x, text_position.y + y as f32)
            } else {
                Vec2::new(text_position.x + x as f32, text_position.y + y as f32)
            }
        } else {
            Vec2::new(0.0, 0.0)
        };

        let mut cursor_position_updated = false;

        // Draw selected rect and text
        if let Some((start, end)) = selection_bounds
            && !calculate_scroll
        {
            let mut fragments = select_range(
                ctx,
                SelectionLineExtension::FillWidth {
                    max_width: f32::max(size.x, text_size.x),
                },
                text_id,
                start,
                end,
            );

            for fragment in fragments.drain(..) {
                let rect = fragment.rect;
                let selection_rect = Rect::from_pos_size(
                    Vec2::new(text_position.x + rect.x, text_position.y + rect.y),
                    Vec2::new(rect.width, rect.height),
                );

                ctx.push_command(
                    placement.zindex,
                    RenderCommand::Rect {
                        boundary: selection_rect,
                        fill: Some(Fill::Color(state.selection_color)),
                        border_radius: None,
                        border: None,
                    },
                );

                ctx.push_command(
                    placement.zindex,
                    RenderCommand::PushClip {
                        rect: selection_rect,
                        shape: ClipShape::Rect,
                    },
                );

                ctx.push_command(
                    placement.zindex,
                    RenderCommand::Text {
                        x: text_position.x,
                        y: text_position.y,
                        text_id,
                        tint_color: Some(state.selected_text_color),
                    },
                );

                ctx.push_command(placement.zindex, RenderCommand::PopClip);
            }
        }

        // Draw IME Range highlight
        if let Some((start, end)) = ctx.input.ime_cursor_range
            && start != end
        {
            let mut fragments = calculate_ime_fragments(ctx, start, end, text_id, cursor);

            for fragment in fragments.drain(..) {
                let rect = fragment.rect;

                let selection_rect = Rect::from_pos_size(
                    Vec2::new(text_position.x + rect.x, text_position.y + rect.y),
                    Vec2::new(rect.width, rect.height),
                );

                cursor_position_updated = true;
                cursor_position.x = text_position.x + rect.x + rect.width;
                cursor_position.y = text_position.y + rect.y;
                cursor_ime_position.x = text_position.x + rect.x;
                cursor_ime_position.y = text_position.y + rect.y;

                if !calculate_scroll {
                    // TODO: ---------------------------------------------------
                    ctx.push_command(
                        placement.zindex,
                        RenderCommand::Rect {
                            boundary: selection_rect,
                            fill: Some(Fill::Color(state.ime_highlight_color)),
                            border_radius: None,
                            border: None,
                        },
                    );
                    // add_rect(ctx, selection_rect, &Vec4::new(0.2, 0.4, 0.8, 0.5));
                    // ---------------------------------------------------------
                }
            }
        }

        // Draw underline
        if !ctx.input.ime_preedit.is_empty() {
            let start = 0;
            let end = ctx.input.ime_preedit.len();

            let mut fragments = calculate_ime_fragments(ctx, start, end, text_id, cursor);

            for fragment in fragments.drain(..) {
                let rect = fragment.rect;

                if !cursor_position_updated {
                    cursor_position.x = text_position.x + rect.x + rect.width;
                    cursor_position.y = text_position.y + rect.y;

                    cursor_ime_position.x = text_position.x + rect.x;
                    cursor_ime_position.y = text_position.y + rect.y;
                }

                let rect = Rect::from_pos_size(
                    Vec2::new(
                        text_position.x + rect.x,
                        text_position.y + fragment.baseline_y + 2.0.px(ctx),
                    ),
                    Vec2::new(rect.width, 1.0.px(ctx)),
                );

                if !calculate_scroll {
                    // TODO ----------------------------------------------------
                    ctx.push_command(
                        placement.zindex,
                        RenderCommand::Rect {
                            boundary: rect,
                            fill: Some(Fill::Color(state.ime_underline_color)),
                            border_radius: None,
                            border: None,
                        },
                    );
                    // add_rect(ctx, rect, &Vec4::new(1., 1., 1., 1.));
                    // ---------------------------------------------------------
                }
            }
        }

        if cursor_ime_position == Vec2::ZERO {
            cursor_ime_position = cursor_position;
        }

        let cursor_size = Vec2::new(1.0.px(ctx), 16.0.px(ctx));
        let cursor_rect =
            Rect::from_pos_size(cursor_position + Vec2::new(0., -2.0.px(ctx)), cursor_size);

        view_config.ime_cursor_rect =
            Rect::from_pos_size(cursor_ime_position, Vec2::new(1.0.px(ctx), cursor_size.y));

        if !calculate_scroll && editor_cursor_position.is_some() {
            ctx.push_command(
                placement.zindex,
                RenderCommand::Rect {
                    boundary: cursor_rect,
                    fill: Some(Fill::Color(state.cursor_color)),
                    border_radius: None,
                    border: None,
                },
            );
        }

        let placement_rect = placement.rect.px(ctx);
        let left = placement_rect.left();
        let right = placement_rect.right();
        let inner_width = right - left;

        if calculate_scroll {
            if state.auto_scroll_to_cursor && editor_cursor_position.is_some() {
                // Horizontal ----------------------------------------------------------------------
                if !wrap {
                    if cursor_position.x > right {
                        let delta = right - cursor_position.x;

                        editable_text_horizontal_scroll(
                            state,
                            delta,
                            inner_width,
                            text_size.x,
                            text_direction,
                        );
                    }

                    if cursor_position.x < left {
                        let delta = left - cursor_position.x;

                        editable_text_horizontal_scroll(
                            state,
                            delta,
                            inner_width,
                            text_size.x,
                            text_direction,
                        );
                    }
                }
            }

            // Another way of implementing vertical scrolling
            // If current approach will be too fast we can adjust using
            // this code.
            // -------------------------------------------------------------------------------------
            // let top = placement_rect.top() + 4.0.px(ctx);
            // let bottom = placement_rect.bottom() - 4.0.px(ctx);

            // if ctx.interaction.is_active(&id) {
            //     let mouse_y = ctx.input.mouse_y as f32;

            //     if mouse_y > bottom {
            //         let mut delta = mouse_y - bottom;
            //         delta = f32::max(delta, 100.) / 10.;

            //         let editor = ctx.text.editor_mut(text_id);
            //         editable_text_vertical_scroll(ctx.fonts, editor, state, delta);
            //     }

            //     if mouse_y < top {
            //         let mut delta = mouse_y - top;
            //         delta = f32::min(delta, -100.) / 10.;

            //         let editor = ctx.text.editor_mut(text_id);
            //         editable_text_vertical_scroll(ctx.fonts, editor, state, delta);
            //     }
            // }
            // -------------------------------------------------------------------------------------

            state.auto_scroll_to_cursor = false;

            let editor = ctx.text.editor_mut(text_id);
            if ctx.input.mouse_wheel_delta_y != 0. {
                editable_text_vertical_scroll(
                    ctx.fonts,
                    editor,
                    state,
                    -ctx.input.mouse_wheel_delta_y as f32,
                );
                ctx.text
                    .shape_as_needed(text_id, &mut ctx.fonts.font_system, false);
            }

            if !wrap && ctx.input.mouse_wheel_delta_x != 0. {
                editable_text_horizontal_scroll(
                    state,
                    ctx.input.mouse_wheel_delta_x as f32,
                    inner_width,
                    text_size.x,
                    text_direction,
                );
            }

            render(ctx, placement, state, view_config, false);
        }
    }

    if !calculate_scroll || !is_focused {
        ctx.push_command(placement.zindex, RenderCommand::PopClip);
    }
}

fn editable_text_horizontal_scroll(
    state: &mut State,
    delta: f32,
    inner_width: f32,
    text_width: f32,
    text_direction: LayoutDirection,
) {
    state.scroll_x += delta;
    state.scroll_x = match text_direction {
        LayoutDirection::LTR => state
            .scroll_x
            .clamp(f32::min(-(text_width - inner_width), 0.0), 0.0),
        LayoutDirection::RTL => state
            .scroll_x
            .clamp(0.0, f32::max(0.0, text_width - inner_width)),
    };

    state.recompose_text_content = false;
}

fn editable_text_vertical_scroll(
    fonts: &mut FontResources,
    editor: &mut cosmic_text::Editor,
    state: &mut State,
    delta: f32,
) {
    state.reached_end = false;
    state.visible_view_updated = true;

    editor.with_buffer_mut(|buffer| {
        let mut scroll = buffer.scroll();
        scroll.vertical += delta;

        buffer.set_scroll(scroll);
    });
}

#[derive(Copy, Clone)]
enum SelectionLineExtension {
    None,
    FillWidth { max_width: f32 },
    Padded { padding: f32 },
}

struct SelectionFragment {
    rect: Rect,
    baseline_y: f32,
}

fn calculate_ime_fragments(
    ctx: &mut RenderContext,
    start: usize,
    end: usize,
    text_id: TextId,
    cursor: cosmic_text::Cursor,
) -> SmallVec<[SelectionFragment; 4]> {
    select_range(
        ctx,
        SelectionLineExtension::Padded {
            padding: 4.0.px(ctx),
        },
        text_id,
        cosmic_text::Cursor {
            line: cursor.line,
            index: start + cursor.index,
            affinity: cosmic_text::Affinity::After,
        },
        cosmic_text::Cursor {
            line: cursor.line,
            index: end + cursor.index,
            affinity: cosmic_text::Affinity::Before,
        },
    )
}

fn select_range(
    ctx: &mut RenderContext,
    line_extension: SelectionLineExtension,
    text_id: TextId,
    start: cosmic_text::Cursor,
    end: cosmic_text::Cursor,
) -> SmallVec<[SelectionFragment; 4]> {
    let mut ranges = SmallVec::new();

    ctx.text.get(text_id).with_buffer(|buffer| {
        for run in buffer.layout_runs() {
            let line_i = run.line_i;
            let line_top = run.line_top;

            if line_i >= start.line && line_i <= end.line {
                let mut range_opt = None;
                for glyph in run.glyphs.iter() {
                    // Guess x offset based on characters
                    let cluster = &run.text[glyph.start..glyph.end];
                    let total = cluster.grapheme_indices(true).count();
                    let mut c_x = glyph.x;
                    let c_w = glyph.w / total as f32;
                    for (i, c) in cluster.grapheme_indices(true) {
                        let c_start = glyph.start + i;
                        let c_end = glyph.start + i + c.len();
                        if (start.line != line_i || c_end > start.index)
                            && (end.line != line_i || c_start < end.index)
                        {
                            range_opt = match range_opt.take() {
                                Some((min, max)) => Some((
                                    std::cmp::min(min, c_x as i32),
                                    std::cmp::max(max, (c_x + c_w) as i32),
                                )),
                                None => Some((c_x as i32, (c_x + c_w) as i32)),
                            };
                        } else if let Some((min, max)) = range_opt.take() {
                            let selection_height = run.line_height + 4.0.px(ctx);
                            let delta = AlignY::Center.position(run.line_height, selection_height);

                            ranges.push(SelectionFragment {
                                rect: Rect::from_pos_size(
                                    Vec2::new(min as f32, line_top + delta),
                                    Vec2::new(std::cmp::max(0, max - min) as f32, selection_height),
                                ),
                                baseline_y: run.line_y,
                            });
                        }
                        c_x += c_w;
                    }
                }

                let max_width = match line_extension {
                    SelectionLineExtension::None => run.line_w,
                    SelectionLineExtension::FillWidth { max_width } => max_width,
                    SelectionLineExtension::Padded { padding } => run.line_w + padding,
                };

                if run.glyphs.is_empty() && end.line > line_i {
                    // Highlight all of internal empty lines
                    range_opt = Some((0, max_width as i32));
                }

                if let Some((mut min, mut max)) = range_opt.take() {
                    if end.line > line_i {
                        // Draw to end of line
                        if run.rtl {
                            min = 0;
                        } else {
                            max = max_width as i32;
                        }
                    }

                    let selection_height = run.line_height + 4.0.px(ctx);
                    let delta = AlignY::Center.position(run.line_height, selection_height);

                    ranges.push(SelectionFragment {
                        rect: Rect::from_pos_size(
                            Vec2::new(min as f32, line_top + delta),
                            Vec2::new(std::cmp::max(0, max - min) as f32, selection_height),
                        ),
                        baseline_y: run.line_y,
                    });
                }
            }
        }
    });

    ranges
}
