use limur::prelude::*;
use limur::{self as ui};
use limur_derive::{ShortcutId, ShortcutScopeId, WidgetBuilder, WidgetState};

mod shortcuts;
pub use shortcuts::*;

#[derive(WidgetBuilder)]
pub struct ButtonBuilder<'a> {
    frame: ui::FrameBuilder,
    text: &'a str,
}

pub struct ButtonResponse {
    clicked: bool,
}

impl ButtonResponse {
    pub fn clicked(&self) -> bool {
        self.clicked
    }
}

#[derive(ShortcutScopeId)]
pub struct ShortcutScopeButton;

#[derive(ShortcutId)]
pub enum ButtonShortcut {
    Activate,
    Click,
}

impl<'a> ButtonBuilder<'a> {
    pub fn build(mut self, ctx: &mut ui::BuildContext) -> ButtonResponse {
        let layout = self.frame.take_layout();

        let response = self.frame.build(ctx, |ctx| {
            ui::gesture_detector()
                .clickable(true)
                .focusable(true)
                .build(ctx, |ctx| {
                    let response = ctx.of::<ui::GestureDetectorResponse>().unwrap();

                    let gradient = {
                        if response.is_active() && response.is_hot() {
                            ui::LinearGradient::vertical((
                                ui::ColorRgba::from_hex(0xFF1C1C1C),
                                ui::ColorRgba::from_hex(0xFF212121),
                            ))
                        } else if response.is_hot() {
                            ui::LinearGradient::vertical((
                                ui::ColorRgba::from_hex(0xFF383838),
                                ui::ColorRgba::from_hex(0xFF2E2E2E),
                            ))
                        } else {
                            ui::LinearGradient::vertical((
                                ui::ColorRgba::from_hex(0xFF2F2F2F),
                                ui::ColorRgba::from_hex(0xFF272727),
                            ))
                        }
                    };

                    let border_color = if response.is_focused() {
                        ui::ColorRgba::from_hex(0xFF357CCE)
                    } else if response.is_active() && response.is_hot() {
                        ui::ColorRgba::from_hex(0xFF414141)
                    } else if response.is_hot() {
                        ui::ColorRgba::from_hex(0xFF616161)
                    } else {
                        ui::ColorRgba::from_hex(0xFF414141)
                    };

                    let gesture_id = response.id;

                    ui::shortcut_scope(ShortcutScopeButton)
                        .active(response.is_focused())
                        .build(ctx, |ctx| {
                            ui::text(self.text)
                                .background(
                                    ui::decoration()
                                        .border_radius(ui::BorderRadius::all(3.))
                                        .add_linear_gradient(gradient)
                                        .border(ui::Border::all(ui::BorderSide::new(
                                            1.,
                                            border_color,
                                        )))
                                        .build(ctx),
                                )
                                .text_align(ui::TextAlign::Center)
                                .text_vertical_align(ui::AlignY::Center)
                                .size(layout.size)
                                .constraints(layout.constraints)
                                .padding(ui::EdgeInsets::symmetric(12., 8.))
                                .build(ctx);
                        });
                })
        });

        ButtonResponse {
            clicked: response.clicked(),
        }
    }
}

#[track_caller]
pub fn button(text: &str) -> ButtonBuilder<'_> {
    ButtonBuilder {
        frame: ui::FrameBuilder::new().constraints(ui::Constraints {
            min_width: 20.,
            min_height: 0.,
            max_width: f64::INFINITY,
            max_height: f64::INFINITY,
        }),
        text,
    }
}

#[derive(WidgetState, Default)]
pub struct HorizontalScrollBar {
    offset: f64,
    last_offset: f64,
}

impl ui::StatefulWidget for HorizontalScrollBar {
    type Event = ();

    fn build(&mut self, ctx: &mut ui::BuildContext, frame: ui::FrameBuilder) {
        frame.fill_max_size().build(ctx, |ctx| {
            ui::zstack()
                .fill_max_size()
                .align_y(ui::AlignY::Bottom)
                .build(ctx, |ctx| {
                    ui::gesture_detector().dragable(true).build(ctx, |ctx| {
                        let gesture = ctx.of::<ui::GestureDetectorResponse>().unwrap().clone();

                        let color = ui::ColorRgba::from_hex(0xFFFFFFFF).with_opacity(
                            if gesture.is_hot() || gesture.is_active() {
                                0.5
                            } else {
                                0.4
                            },
                        );

                        let response = ctx.of::<ui::ScrollAreaResponse>().unwrap().clone();
                        let horizontal_padding = 16.;
                        let mut scroll_area_width = response.width - horizontal_padding;

                        if response.overflow_y {
                            scroll_area_width -= 8.;
                        }

                        let bar_width = f64::max(16., scroll_area_width * response.fraction_x);

                        if gesture.drag_state == ui::DragState::None
                            || gesture.drag_state == ui::DragState::End
                        {
                            self.offset = (scroll_area_width - bar_width) * response.progress_x;
                        } else if gesture.drag_state == ui::DragState::Start {
                            self.last_offset = self.offset;
                        } else {
                            self.offset = self.last_offset + gesture.drag_x - gesture.drag_start_x;
                            self.offset = self.offset.clamp(0., scroll_area_width - bar_width);

                            let progress_x = self.offset / (scroll_area_width - bar_width);

                            ui::scroll_area::set_progress_x(ctx, response.id, progress_x);
                        }

                        ui::decorated_box()
                            .color(color)
                            .border_radius(ui::BorderRadius::all(if gesture.is_active() {
                                0.
                            } else {
                                2.
                            }))
                            .width(bar_width)
                            .height(if gesture.is_active() { 8. } else { 4. })
                            .offset_x(self.offset)
                            .padding(if gesture.is_active() {
                                ui::EdgeInsets::symmetric(8., 6.)
                            } else {
                                ui::EdgeInsets::all(8.)
                            })
                            .build(ctx);
                    });
                });
        });
    }
}

pub fn horizontal_scroll_bar() -> impl StatefulWidgetBuilder {
    ui::stateful::<HorizontalScrollBar>()
}

#[derive(WidgetBuilder)]
pub struct VerticalScrollBarBuilder {
    frame: ui::FrameBuilder,
    thinkness: f32,
}

pub fn vertical_scroll_bar() -> VerticalScrollBarBuilder {
    VerticalScrollBarBuilder {
        frame: ui::FrameBuilder::new(),
        thinkness: 4.,
    }
}

impl VerticalScrollBarBuilder {
    pub fn thinkness(mut self, thinkness: f32) -> Self {
        self.thinkness = thinkness;
        self
    }

    pub fn build(self, ctx: &mut ui::BuildContext) {
        ui::stateful::<VerticalScrollBar>()
            .frame(self.frame)
            .update_state_and_build(ctx, |state| state.thinkness = self.thinkness);
    }
}

#[derive(WidgetState, Default)]
pub struct VerticalScrollBar {
    offset: f64,
    last_offset: f64,
    thinkness: f32,
}

impl ui::StatefulWidget for VerticalScrollBar {
    type Event = ();

    fn build(&mut self, ctx: &mut ui::BuildContext, frame: ui::FrameBuilder) {
        frame.fill_max_size().build(ctx, |ctx| {
            ui::zstack()
                .fill_max_size()
                .align_x(ui::AlignX::Right)
                .build(ctx, |ctx| {
                    ui::gesture_detector().dragable(true).build(ctx, |ctx| {
                        let gesture = ctx.of::<ui::GestureDetectorResponse>().unwrap().clone();

                        let color = ui::ColorRgba::from_hex(0xFFFFFFFF).with_opacity(
                            if gesture.is_hot() || gesture.is_active() {
                                0.5
                            } else {
                                0.4
                            },
                        );

                        let response = ctx.of::<ui::ScrollAreaResponse>().unwrap().clone();
                        let vertical_padding = 16.;
                        let mut scroll_area_height = response.height - vertical_padding;

                        if response.overflow_x {
                            scroll_area_height -= 8.;
                        }

                        let bar_height = f64::max(16., scroll_area_height * response.fraction_y);

                        if gesture.drag_state == ui::DragState::None
                            || gesture.drag_state == ui::DragState::End
                        {
                            self.offset = (scroll_area_height - bar_height) * response.progress_y;
                        } else if gesture.drag_state == ui::DragState::Start {
                            self.last_offset = self.offset;
                        } else {
                            self.offset = self.last_offset + gesture.drag_y - gesture.drag_start_y;
                            self.offset = self.offset.clamp(0., scroll_area_height - bar_height);

                            let progress_y = self.offset / (scroll_area_height - bar_height);

                            ui::scroll_area::set_progress_y(ctx, response.id, progress_y);
                        }

                        ui::decorated_box()
                            .color(color)
                            .border_radius(ui::BorderRadius::all(if gesture.is_active() {
                                0.
                            } else {
                                2.
                            }))
                            .width(if gesture.is_active() { 8. } else { 4. })
                            .height(bar_height)
                            .offset_y(self.offset)
                            .padding(if gesture.is_active() {
                                ui::EdgeInsets::symmetric(6., 8.)
                            } else {
                                ui::EdgeInsets::all(8.)
                            })
                            .build(ctx);
                    });
                });
        });
    }
}
