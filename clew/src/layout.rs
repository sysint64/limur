use crate::{
    AlignX, AlignY, Axis, Clip, Constraints, CrossAxisAlignment, DebugBoundary, EdgeInsets,
    LayoutDirection, MainAxisAlignment, Rect, Size, SizeConstraint, Vec2, View, WidgetId,
    WidgetRef, WidgetType,
    assets::Assets,
    rects_overlap,
    state::TypedWidgetStates,
    text::{TextId, TextsResources},
};
use smallvec::SmallVec;

pub(crate) const RENDER_CONTAINER_DEBUG_BOUNDARIES: bool = false;
pub(crate) const RENDER_CHILD_DEBUG_BOUNDARIES: bool = false;

#[derive(Clone, Debug)]
pub struct WidgetPlacement {
    pub widget_ref: WidgetRef,
    pub zindex: i32,
    pub boundary: Rect,
    pub rect: Rect,
}

#[derive(Clone, Debug)]
pub enum LayoutItem {
    Placement(WidgetPlacement),
    PushClip { rect: Rect, clip: Clip, zindex: i32 },
    PopClip,
    BeginGroup { zindex: i32 },
    EndGroup,
}

#[derive(Debug, Clone)]
pub enum LayoutCommand {
    BeginContainer {
        backgrounds: SmallVec<[WidgetRef; 8]>,
        foregrounds: SmallVec<[WidgetRef; 8]>,
        kind: ContainerKind,
        constraints: Constraints,
        size: Size,
        zindex: i32,
        padding: EdgeInsets,
        margin: EdgeInsets,
        clip: Clip,
    },
    EndContainer,
    BeginOffset {
        offset_x: f64,
        offset_y: f64,
    },
    EndOffset,
    Leaf {
        widget_ref: WidgetRef,
        backgrounds: SmallVec<[WidgetRef; 8]>,
        foregrounds: SmallVec<[WidgetRef; 8]>,
        constraints: Constraints,
        padding: EdgeInsets,
        margin: EdgeInsets,
        size: Size,
        derive_wrap_size: DeriveWrapSize,
        zindex: i32,
        clip: Clip,
    },
    Spacer {
        constraints: Constraints,
        size: Size,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum DeriveWrapSize {
    Constraints,
    Text {
        text_id: TextId,
        derive_width: bool,
        derive_height: bool,
    },
    Svg(&'static str),
}

#[derive(Debug, Default, Clone, Copy)]
pub enum ContainerKind {
    #[default]
    None,
    Passthrough,
    VStack {
        spacing: f64,
        main_axis_alignment: MainAxisAlignment,
        cross_axis_alignment: CrossAxisAlignment,
        rtl_aware: bool,
    },
    HStack {
        spacing: f64,
        main_axis_alignment: MainAxisAlignment,
        cross_axis_alignment: CrossAxisAlignment,
        rtl_aware: bool,
    },
    Flow {
        spacing: f64,
        run_spacing: f64,
        rtl_aware: bool,
    },
    ZStack {
        align_x: AlignX,
        align_y: AlignY,
    },
    Measure {
        id: WidgetId,
    },
}

#[derive(Default, Debug, Clone, Copy)]
enum StackAxis {
    #[default]
    None,
    Horizontal {
        _rtl_aware: bool,
        spacing: f64,
    },
    Vertical {
        spacing: f64,
    },
}

#[derive(Default, Debug, Clone, Copy)]
enum StackAxisPass2 {
    #[default]
    None,
    Passthrough {
        stretch: Option<Axis>,
    },
    Align {
        align_x: AlignX,
        align_y: AlignY,
    },
    Horizontal {
        rtl_aware: bool,
        spacing: f64,
        _main_axis_alignment: MainAxisAlignment,
        cross_axis_alignment: CrossAxisAlignment,
    },
    Vertical {
        rtl_aware: bool,
        spacing: f64,
        _main_axis_alignment: MainAxisAlignment,
        cross_axis_alignment: CrossAxisAlignment,
    },
}

#[derive(Debug, Default, Clone, Copy)]
struct LayoutContainerCommand {
    kind: ContainerKind,
    constraints: Constraints,
    size: Size,
    insets: EdgeInsets,
}

#[derive(Debug, Default, Clone)]
struct LayoutContainer {
    idx: usize,
    axis: StackAxis,
    command: LayoutContainerCommand,
}

#[derive(Debug, Default, Clone)]
struct Pass2LayoutContainer {
    axis: StackAxisPass2,
    idx: usize,
    padding: EdgeInsets,
    clipping: bool,
    decorator_rect: Rect,
    zindex: i32,
    foregrounds: SmallVec<[WidgetRef; 8]>,
}

pub(crate) struct TextLayout {
    pub(crate) width: Option<f64>,
    pub(crate) height: Option<f64>,
    pub(crate) text_id: TextId,
}

#[derive(Debug, Clone)]
pub struct LayoutMeasure {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub wrap_width: f64,
    pub wrap_height: f64,
}

#[derive(Default)]
pub(crate) struct LayoutState {
    cursor: usize,

    wrap_sizes: Vec<Vec2>,
    flex_sizes: Vec<Vec2>,
    actual_sizes: Vec<Vec2>,
    offsets: Vec<Vec2>,
    resizes: Vec<Vec2>,
    flex_x: Vec<f32>,
    flex_y: Vec<f32>,
    flex_sum_x: Vec<f32>,
    flex_sum_y: Vec<f32>,
    constraints: Vec<Constraints>,
    margins: Vec<EdgeInsets>,

    position_cursor: usize,
    positions: Vec<Vec2>,

    containers_stack_cursor: usize,
    pass_2_containers_stack_cursor: usize,
    layout_direction: LayoutDirection,
    parent_container: LayoutContainer,
    pass2_parent_container: Pass2LayoutContainer,
    containers_stack: Vec<LayoutContainer>,
    pass_2_containers_stack: Vec<Pass2LayoutContainer>,

    offsets_stack_cursor: usize,
    offsets_stack: Vec<Vec2>,

    pub(crate) texts: Vec<TextLayout>,
}

impl LayoutState {
    #[inline]
    fn current_idx(&self) -> usize {
        self.cursor - 1
    }

    #[inline]
    fn set_constraints(&mut self, constraints: Constraints) {
        self.constraints[self.cursor - 1] = constraints;
    }

    #[inline]
    fn set_margin(&mut self, margin: EdgeInsets) {
        self.margins[self.cursor - 1] = margin;
    }

    #[inline]
    fn set_wrap_size(&mut self, value: Vec2) {
        self.wrap_sizes[self.cursor - 1] = value;
    }

    #[inline]
    fn set_actual_size(&mut self, value: Vec2) {
        self.actual_sizes[self.cursor - 1] = value;
    }

    #[inline]
    fn set_resize(&mut self, value: Vec2) {
        self.resizes[self.cursor - 1] = value;
    }

    #[inline]
    fn set_flex_x(&mut self, value: f32) {
        self.flex_x[self.cursor - 1] = value;
    }

    #[inline]
    fn set_flex_y(&mut self, value: f32) {
        self.flex_y[self.cursor - 1] = value;
    }

    #[inline]
    fn push_boundary(&mut self) {
        if self.wrap_sizes.len() <= self.cursor {
            self.wrap_sizes.push(Vec2::ZERO);
            self.flex_sizes.push(Vec2::ZERO);
            self.actual_sizes.push(Vec2::ZERO);
            self.offsets.push(Vec2::ZERO);
            self.resizes.push(Vec2::ZERO);
            self.flex_x.push(0.);
            self.flex_y.push(0.);
            self.flex_sum_x.push(0.);
            self.flex_sum_y.push(0.);
            self.constraints.push(Constraints::default());
            self.margins.push(EdgeInsets::ZERO);
        } else {
            self.wrap_sizes[self.cursor] = Vec2::ZERO;
            self.flex_sizes[self.cursor] = Vec2::ZERO;
            self.actual_sizes[self.cursor] = Vec2::ZERO;
            self.offsets[self.cursor] = Vec2::ZERO;
            self.resizes[self.cursor] = Vec2::ZERO;
            self.flex_x[self.cursor] = 0.;
            self.flex_y[self.cursor] = 0.;
            self.flex_sum_x[self.cursor] = 0.;
            self.flex_sum_y[self.cursor] = 0.;
            self.constraints[self.cursor] = Constraints::default();
            self.margins[self.cursor] = EdgeInsets::ZERO;
        }

        self.cursor += 1;
    }

    #[inline]
    fn push_position(&mut self, position: Vec2) {
        if self.positions.len() <= self.position_cursor {
            self.positions.push(position);
        } else {
            self.positions[self.position_cursor] = position;
        }

        self.position_cursor += 1;
    }

    #[inline]
    fn pop_position(&mut self) -> Vec2 {
        self.position_cursor -= 1;

        self.positions[self.position_cursor]
    }

    #[inline]
    fn push_offset(&mut self, offset: Vec2) {
        let last_offset = if self.offsets_stack_cursor > 0 {
            self.get_offset()
        } else {
            Vec2::ZERO
        };

        if self.offsets_stack.len() <= self.offsets_stack_cursor {
            self.offsets_stack.push(last_offset + offset);
        } else {
            self.offsets_stack[self.offsets_stack_cursor] = last_offset + offset;
        }

        self.offsets_stack_cursor += 1;
    }

    #[inline]
    fn pop_offset(&mut self) -> Vec2 {
        self.offsets_stack_cursor -= 1;

        self.offsets_stack[self.offsets_stack_cursor]
    }

    #[inline]
    fn get_offset(&mut self) -> Vec2 {
        self.offsets_stack[self.offsets_stack_cursor - 1]
    }

    #[inline]
    fn push_container(&mut self, container: LayoutContainer) {
        if self.containers_stack.len() <= self.containers_stack_cursor {
            self.containers_stack.push(container);
        } else {
            self.containers_stack[self.containers_stack_cursor] = container;
        }

        self.containers_stack_cursor += 1;
    }

    #[inline]
    fn pop_container(&mut self) -> LayoutContainer {
        debug_assert!(self.containers_stack_cursor > 0);

        self.containers_stack_cursor -= 1;

        self.containers_stack[self.containers_stack_cursor].clone()
    }

    #[inline]
    fn push_pass2_container(&mut self, container: Pass2LayoutContainer) {
        if self.pass_2_containers_stack.len() <= self.pass_2_containers_stack_cursor {
            self.pass_2_containers_stack.push(container);
        } else {
            self.pass_2_containers_stack[self.pass_2_containers_stack_cursor] = container;
        }

        self.pass_2_containers_stack_cursor += 1;
    }

    #[inline]
    fn pop_pass2_container(&mut self) -> Pass2LayoutContainer {
        self.pass_2_containers_stack_cursor -= 1;

        self.pass_2_containers_stack[self.pass_2_containers_stack_cursor].clone()
    }

    #[inline]
    fn clear(&mut self) {
        self.parent_container = LayoutContainer {
            idx: 0,
            axis: StackAxis::None,
            command: Default::default(),
        };
        self.cursor = 0;
        self.position_cursor = 0;
        self.containers_stack_cursor = 0;
        self.offsets_stack_cursor = 0;

        self.texts.clear();
    }

    fn add_flex_sum(&mut self, size: Size) {
        self.add_flex_sum_x(size.width);
        self.add_flex_sum_y(size.height);
    }

    fn add_flex_sum_x(&mut self, width: SizeConstraint) {
        if let SizeConstraint::Fill(flex) = width {
            self.set_flex_x(flex);

            if let StackAxis::Horizontal { .. } = self.parent_container.axis {
                self.flex_sum_x[self.parent_container.idx] += flex;
            } else {
                self.flex_sum_x[self.parent_container.idx] = 1.;
            }
        }
    }

    fn add_flex_sum_y(&mut self, height: SizeConstraint) {
        if let SizeConstraint::Fill(flex) = height {
            self.set_flex_y(flex);

            if let StackAxis::Vertical { .. } = self.parent_container.axis {
                self.flex_sum_y[self.parent_container.idx] += flex;
            } else {
                self.flex_sum_y[self.parent_container.idx] = 1.;
            }
        }
    }

    fn add_container_size(&mut self, size: Size, wrap_size: Vec2) -> Vec2 {
        Vec2::new(
            self.add_width(size.width, wrap_size.x, 0.),
            self.add_height(size.height, wrap_size.y, 0.),
        )
    }

    fn add_size(
        &mut self,
        size: Size,
        constraints: Constraints,
        wrap_size: Vec2,
        insets: EdgeInsets,
    ) -> Vec2 {
        let mut size = Vec2::new(
            self.add_width(size.width, wrap_size.x, insets.horizontal()),
            self.add_height(size.height, wrap_size.y, insets.vertical()),
        );

        size = apply_constraints(size, constraints);

        self.set_wrap_size(wrap_size);
        self.set_actual_size(size);

        size
    }

    fn add_width(&mut self, width: SizeConstraint, wrap_width: f64, insets: f64) -> f64 {
        let wrap_size = self.wrap_sizes.get_mut(self.parent_container.idx).unwrap();
        let flex_sizes = self.flex_sizes.get_mut(self.parent_container.idx).unwrap();

        match width {
            SizeConstraint::Fixed(value) => {
                let value = value + insets;

                match self.parent_container.axis {
                    StackAxis::None => {
                        wrap_size.x = wrap_size.x.max(value);
                        flex_sizes.x = flex_sizes.x.max(value);
                    }
                    StackAxis::Horizontal { spacing, .. } => {
                        wrap_size.x += value + spacing;
                        flex_sizes.x += value + spacing;
                    }
                    StackAxis::Vertical { .. } => {
                        wrap_size.x = wrap_size.x.max(value);
                        flex_sizes.x = flex_sizes.x.max(value);
                    }
                };

                value
            }
            SizeConstraint::Wrap => {
                let wrap_width = wrap_width + insets;

                match self.parent_container.axis {
                    StackAxis::None => {
                        wrap_size.x = wrap_size.x.max(wrap_width);
                        flex_sizes.x = flex_sizes.x.max(wrap_width);
                    }
                    StackAxis::Horizontal { spacing, .. } => {
                        wrap_size.x += wrap_width + spacing;
                        flex_sizes.x += wrap_width + spacing;
                    }
                    StackAxis::Vertical { .. } => {
                        wrap_size.x = wrap_size.x.max(wrap_width);
                        flex_sizes.x = flex_sizes.x.max(wrap_width);
                    }
                };

                wrap_width
            }
            SizeConstraint::Fill(_) => {
                let wrap_width = wrap_width + insets;

                match self.parent_container.axis {
                    StackAxis::None => {
                        wrap_size.x = wrap_size.x.max(wrap_width);
                    }
                    StackAxis::Horizontal { spacing, .. } => {
                        wrap_size.x += wrap_width + spacing;
                        flex_sizes.x += spacing;
                    }
                    StackAxis::Vertical { .. } => {
                        wrap_size.x = wrap_size.x.max(wrap_width);
                    }
                };

                wrap_width
            }
        }
    }

    fn add_height(&mut self, height: SizeConstraint, wrap_height: f64, insets: f64) -> f64 {
        let wrap_size = self.wrap_sizes.get_mut(self.parent_container.idx).unwrap();
        let flex_sizes = self.flex_sizes.get_mut(self.parent_container.idx).unwrap();

        match height {
            SizeConstraint::Fixed(value) => {
                let value = value + insets;

                match self.parent_container.axis {
                    StackAxis::None => {
                        wrap_size.y = wrap_size.y.max(value);
                        flex_sizes.y = flex_sizes.y.max(value);
                    }
                    StackAxis::Horizontal { .. } => {
                        wrap_size.y = wrap_size.y.max(value);
                        flex_sizes.y = flex_sizes.y.max(value);
                    }
                    StackAxis::Vertical { spacing } => {
                        wrap_size.y += value + spacing;
                        flex_sizes.y += value + spacing;
                    }
                };

                value
            }
            SizeConstraint::Wrap => {
                let wrap_height = wrap_height + insets;

                match self.parent_container.axis {
                    StackAxis::None => {
                        wrap_size.y = wrap_size.y.max(wrap_height);
                        flex_sizes.y = flex_sizes.y.max(wrap_height);
                    }
                    StackAxis::Horizontal { .. } => {
                        wrap_size.y = wrap_size.y.max(wrap_height);
                        flex_sizes.y = flex_sizes.y.max(wrap_height);
                    }
                    StackAxis::Vertical { spacing } => {
                        wrap_size.y += wrap_height + spacing;
                        flex_sizes.y += wrap_height + spacing;
                    }
                };

                wrap_height
            }
            SizeConstraint::Fill(_) => {
                let wrap_height = wrap_height + insets;

                match self.parent_container.axis {
                    StackAxis::None => {
                        wrap_size.y = wrap_size.y.max(wrap_height);
                    }
                    StackAxis::Horizontal { .. } => {
                        wrap_size.y = wrap_size.y.max(wrap_height);
                    }
                    StackAxis::Vertical { spacing } => {
                        wrap_size.y += wrap_height + spacing;
                        flex_sizes.y += spacing;
                    }
                }

                wrap_height
            }
        }
    }
}

#[inline]
fn apply_constraints_width(width: f64, constraints: Constraints) -> f64 {
    let mut width = width;

    width = width.max(constraints.min_width);
    width = width.min(constraints.max_width);

    width
}

#[inline]
fn apply_constraints_height(height: f64, constraints: Constraints) -> f64 {
    let mut height = height;

    height = height.max(constraints.min_height);
    height = height.min(constraints.max_height);

    height
}

#[inline]
fn apply_constraints(size: Vec2, constraints: Constraints) -> Vec2 {
    Vec2::new(
        size.x.max(constraints.min_width).min(constraints.max_width),
        size.y
            .max(constraints.min_height)
            .min(constraints.max_height),
    )
}

pub fn layout(
    layout_state: &mut LayoutState,
    view: &View,
    commands: &[LayoutCommand],
    layout_items: &mut Vec<LayoutItem>,
    clipped_layout_items: &mut Vec<LayoutItem>,
    layout_measures: &mut TypedWidgetStates<LayoutMeasure>,
    text: &mut TextsResources,
    assets: &Assets,
) -> Vec2 {
    layout_state.clear();

    let view_size = view.size();
    let scale_factor = view.scale_factor;

    // Pass 1 - Calculate fixed sizes and flex sum -------------------------------------------------
    // Root container
    layout_state.push_boundary();
    layout_state.actual_sizes[0] = view_size;

    for command in commands {
        match command {
            LayoutCommand::BeginContainer {
                kind,
                constraints,
                size,
                padding,
                margin,
                ..
            } => {
                layout_state.push_container(layout_state.parent_container.clone());
                layout_state.push_boundary();
                layout_state.add_flex_sum(*size);
                layout_state.set_constraints(*constraints);
                layout_state.set_margin(*margin);

                let insets = *padding + *margin;

                // layout_state.set_offset(Vec2::new(padding.left, padding.top));
                layout_state.set_resize(Vec2::new(-insets.horizontal(), -insets.vertical()));

                match kind {
                    ContainerKind::VStack { spacing, .. } => {
                        layout_state.parent_container = LayoutContainer {
                            idx: layout_state.current_idx(),
                            axis: StackAxis::Vertical { spacing: *spacing },
                            command: LayoutContainerCommand {
                                kind: *kind,
                                constraints: *constraints,
                                size: *size,
                                insets,
                            },
                        };
                    }
                    ContainerKind::HStack {
                        spacing, rtl_aware, ..
                    } => {
                        layout_state.parent_container = LayoutContainer {
                            idx: layout_state.current_idx(),
                            axis: StackAxis::Horizontal {
                                spacing: *spacing,
                                _rtl_aware: *rtl_aware,
                            },
                            command: LayoutContainerCommand {
                                kind: *kind,
                                constraints: *constraints,
                                size: *size,
                                insets,
                            },
                        };
                    }
                    ContainerKind::ZStack { .. } => {
                        layout_state.parent_container = LayoutContainer {
                            idx: layout_state.current_idx(),
                            axis: StackAxis::None,
                            command: LayoutContainerCommand {
                                kind: *kind,
                                constraints: *constraints,
                                size: *size,
                                insets,
                            },
                        };
                    }
                    ContainerKind::None => {
                        layout_state.parent_container = LayoutContainer {
                            idx: layout_state.current_idx(),
                            axis: StackAxis::None,
                            command: LayoutContainerCommand {
                                kind: *kind,
                                constraints: *constraints,
                                size: *size,
                                insets,
                            },
                        };
                    }
                    ContainerKind::Passthrough => {
                        layout_state.parent_container = LayoutContainer {
                            idx: layout_state.current_idx(),
                            axis: StackAxis::None,
                            command: LayoutContainerCommand {
                                kind: *kind,
                                constraints: *constraints,
                                size: *size,
                                insets,
                            },
                        };
                    }
                    ContainerKind::Measure { .. } => {
                        layout_state.parent_container = LayoutContainer {
                            idx: layout_state.current_idx(),
                            axis: StackAxis::None,
                            command: LayoutContainerCommand {
                                kind: *kind,
                                constraints: *constraints,
                                size: *size,
                                insets,
                            },
                        };
                    }
                    ContainerKind::Flow {
                        spacing, rtl_aware, ..
                    } => {
                        layout_state.parent_container = LayoutContainer {
                            idx: layout_state.current_idx(),
                            axis: StackAxis::Horizontal {
                                spacing: *spacing,
                                _rtl_aware: *rtl_aware,
                            },
                            command: LayoutContainerCommand {
                                kind: *kind,
                                constraints: *constraints,
                                size: *size,
                                insets,
                            },
                        };
                    }
                }
            }
            LayoutCommand::EndContainer => {
                let wrap_size = layout_state
                    .wrap_sizes
                    .get_mut(layout_state.parent_container.idx)
                    .unwrap();

                let size = layout_state.parent_container.command.size;
                let padding = layout_state.parent_container.command.insets;

                match layout_state.parent_container.command.kind {
                    ContainerKind::VStack { spacing, .. } => {
                        wrap_size.y -= spacing;
                        wrap_size.y = wrap_size.y.max(0.);
                    }
                    ContainerKind::HStack { spacing, .. } => {
                        wrap_size.x -= spacing;
                        wrap_size.x = wrap_size.x.max(0.);
                    }
                    ContainerKind::Flow { spacing, .. } => {
                        wrap_size.x -= spacing;
                        wrap_size.x = wrap_size.x.max(0.);
                        wrap_size.y -= spacing;
                        wrap_size.y = wrap_size.y.max(0.);
                    }
                    ContainerKind::ZStack { .. } => {}
                    ContainerKind::None => {}
                    ContainerKind::Passthrough => {}
                    ContainerKind::Measure { .. } => {}
                };

                wrap_size.x += padding.horizontal();
                wrap_size.y += padding.vertical();
                wrap_size.x = wrap_size.x.max(0.);
                wrap_size.y = wrap_size.y.max(0.);

                let wrap_size = *wrap_size;
                let current_container_idx = layout_state.parent_container.idx;

                let constraints = layout_state.parent_container.command.constraints;
                layout_state.parent_container = layout_state.pop_container();

                let size = layout_state.add_container_size(size, wrap_size);
                layout_state.actual_sizes[current_container_idx] =
                    apply_constraints(size, constraints);
            }
            LayoutCommand::Leaf {
                constraints,
                size,
                derive_wrap_size,
                padding,
                margin,
                ..
            } => {
                layout_state.push_boundary();
                layout_state.set_constraints(*constraints);
                layout_state.set_margin(*margin);
                layout_state.add_flex_sum_x(size.width);
                layout_state.add_flex_sum_y(size.height);

                // let wrap_size = if size.width.constrained() && size.height.constrained() {
                // Vec2::new(constraints.min_width, constraints.min_height)
                // } else {
                let wrap_size = match derive_wrap_size {
                    DeriveWrapSize::Constraints => {
                        Vec2::new(constraints.min_width, constraints.min_height)
                    }
                    DeriveWrapSize::Text { text_id, .. } => {
                        let text_size = text.get_mut(*text_id).calculate_size();

                        Vec2::new(
                            (text_size.x / scale_factor).ceil(),
                            (text_size.y / scale_factor).ceil(),
                        )
                    }
                    DeriveWrapSize::Svg(asset_id) => {
                        let tree = assets
                            .get_svg_tree(asset_id)
                            .unwrap_or_else(|| panic!("SVG with ID = {asset_id} has not found"));

                        Vec2::new(tree.size().width() as f64, tree.size().height() as f64)
                    }
                };
                // };

                layout_state.add_size(*size, *constraints, wrap_size, *padding + *margin);
            }
            LayoutCommand::Spacer { constraints, size } => {
                layout_state.push_boundary();
                layout_state.set_constraints(*constraints);
                layout_state.add_flex_sum(*size);
                layout_state.add_size(*size, *constraints, Vec2::ZERO, EdgeInsets::ZERO);
            }
            LayoutCommand::BeginOffset { .. } | LayoutCommand::EndOffset => {
                // No-op
            }
        }
    }

    debug_assert!(layout_state.containers_stack_cursor == 0);

    // Extra memory to simplify calculations
    layout_state.push_boundary();
    layout_state.flex_sum_x[layout_state.cursor - 1] = 0.;
    layout_state.flex_sum_y[layout_state.cursor - 1] = 0.;

    // Pass 2 - Widget placements ------------------------------------------------------------------
    let mut current_idx = 1; // Skip root container
    let mut current_position = Vec2::ZERO;

    layout_items.clear();
    clipped_layout_items.clear();

    layout_state.push_position(current_position);
    layout_state.pass2_parent_container = Pass2LayoutContainer {
        idx: 0,
        axis: StackAxisPass2::None,
        padding: EdgeInsets::ZERO,
        clipping: false,
        decorator_rect: Rect::ZERO,
        foregrounds: SmallVec::new(),
        zindex: i32::MIN,
    };
    layout_state.push_offset(Vec2::new(0., 0.));

    for command in commands {
        let mut go_next = true;
        let container_idx = layout_state.pass2_parent_container.idx;

        let offset = layout_state.get_offset();

        let container_position = layout_state.positions[layout_state.position_cursor - 1];
        let container_resize = layout_state.resizes[container_idx];
        let container_margin = layout_state.margins[container_idx];
        let container_size_resized = layout_state.actual_sizes[container_idx] + container_resize;
        let container_size = layout_state.actual_sizes[container_idx];
        let container_wrap_size = layout_state.wrap_sizes[container_idx];

        let flex_x = layout_state.flex_x[current_idx];
        let flex_y = layout_state.flex_y[current_idx];

        if flex_x > 0. {
            let constraints = layout_state.constraints[current_idx];
            let container_flex_size = layout_state.flex_sizes[container_idx];

            let mut size = match layout_state.pass2_parent_container.axis {
                StackAxisPass2::None | StackAxisPass2::Passthrough { .. } => {
                    container_size_resized.x
                }
                StackAxisPass2::Align { .. } => container_size_resized.x,
                StackAxisPass2::Horizontal { spacing, .. } => {
                    let flex_sum_x = layout_state.flex_sum_x[container_idx].max(1.);
                    let available_width =
                        (container_size_resized.x - container_flex_size.x + spacing).max(0.);
                    let per_flex = available_width / flex_sum_x as f64;

                    flex_x as f64 * per_flex
                }
                StackAxisPass2::Vertical { .. } => container_size_resized.x,
            };

            // let wrap_size = layout_state.wrap_sizes[current_idx].x;
            size = apply_constraints_width(size, constraints);
            layout_state.actual_sizes[current_idx].x = size;
        }

        if flex_y > 0. {
            let constraints = layout_state.constraints[current_idx];
            let container_flex_size = layout_state.flex_sizes[container_idx];

            let mut size = match layout_state.pass2_parent_container.axis {
                StackAxisPass2::None | StackAxisPass2::Passthrough { .. } => {
                    container_size_resized.y
                }
                StackAxisPass2::Align { .. } => container_size_resized.y,
                StackAxisPass2::Horizontal { .. } => container_size_resized.y,
                StackAxisPass2::Vertical { spacing, .. } => {
                    let flex_sum_y = layout_state.flex_sum_y[container_idx].max(1.);
                    let available_height =
                        (container_size_resized.y - container_flex_size.y + spacing).max(0.);
                    let per_flex = available_height / flex_sum_y as f64;

                    flex_y as f64 * per_flex
                }
            };

            // size = f32::max(size, layout_state.wrap_sizes[current_idx].y);
            size = apply_constraints_height(size, constraints);
            layout_state.actual_sizes[current_idx].y = size;
        }

        let mut widget_size = layout_state.actual_sizes[current_idx];

        let (boundary_position, boundary_size) = match layout_state.pass2_parent_container.axis {
            StackAxisPass2::None | StackAxisPass2::Passthrough { .. } => (
                container_position,
                Vec2::new(
                    container_size.x - layout_state.pass2_parent_container.padding.horizontal(),
                    container_size.y - layout_state.pass2_parent_container.padding.vertical(),
                ),
            ),
            StackAxisPass2::Align { .. } => (
                container_position,
                Vec2::new(
                    container_size.x - layout_state.pass2_parent_container.padding.horizontal(),
                    container_size.y - layout_state.pass2_parent_container.padding.vertical(),
                ),
            ),
            StackAxisPass2::Horizontal { .. } => (
                Vec2::new(current_position.x, current_position.y),
                Vec2::new(
                    widget_size.x,
                    container_size.y - layout_state.pass2_parent_container.padding.vertical(),
                ),
            ),
            StackAxisPass2::Vertical { .. } => (
                Vec2::new(current_position.x, current_position.y),
                Vec2::new(
                    container_size.x - layout_state.pass2_parent_container.padding.horizontal(),
                    widget_size.y,
                ),
            ),
        };

        let mut boundary = Rect::from_pos_size(boundary_position, boundary_size);
        let mut position = current_position;

        if let StackAxisPass2::Horizontal { rtl_aware, .. } =
            layout_state.pass2_parent_container.axis
            && rtl_aware
            && layout_state.layout_direction == LayoutDirection::RTL
        {
            position.x -= widget_size.x;
            boundary.x -= widget_size.x;
        }

        match layout_state.pass2_parent_container.axis {
            StackAxisPass2::None => {}
            StackAxisPass2::Align { .. } => {}
            StackAxisPass2::Horizontal {
                cross_axis_alignment,
                ..
            } => {
                if cross_axis_alignment == CrossAxisAlignment::Stretch {
                    let height = boundary.height;
                    widget_size.y = height.max(layout_state.actual_sizes[current_idx].y);
                }
            }
            StackAxisPass2::Vertical {
                cross_axis_alignment,
                ..
            } => {
                if cross_axis_alignment == CrossAxisAlignment::Stretch {
                    let width = boundary.width;
                    widget_size.x = width.max(layout_state.actual_sizes[current_idx].x);
                }
            }
            StackAxisPass2::Passthrough { stretch } => {
                if let Some(axis) = stretch {
                    match axis {
                        Axis::Horizontal => {
                            let height = boundary.height;
                            widget_size.y = height.max(layout_state.actual_sizes[current_idx].y);
                        }
                        Axis::Vertical => {
                            let width = boundary.width;
                            widget_size.x = width.max(layout_state.actual_sizes[current_idx].x);
                        }
                    }
                }
            }
        }

        match command {
            LayoutCommand::BeginOffset { offset_x, offset_y } => {
                layout_state.push_offset(Vec2::new(*offset_x, *offset_y));
                continue;
            }
            LayoutCommand::EndOffset => {
                layout_state.pop_offset();
                continue;
            }
            LayoutCommand::BeginContainer {
                kind,
                zindex,
                backgrounds,
                foregrounds,
                padding,
                margin,
                clip,
                ..
            } => {
                let parent_container_axis = layout_state.pass2_parent_container.axis;

                layout_state.push_position(current_position);
                layout_state.push_pass2_container(layout_state.pass2_parent_container.clone());

                layout_state.actual_sizes[current_idx] = widget_size;
                current_position = position + Vec2::new(margin.left, margin.top);
                let clipping = *clip != Clip::None;

                let align_x = match layout_state.pass2_parent_container.axis {
                    StackAxisPass2::None | StackAxisPass2::Passthrough { .. } => AlignX::Start,
                    StackAxisPass2::Align { align_x, .. } => align_x,
                    StackAxisPass2::Horizontal { .. } => AlignX::Start,
                    StackAxisPass2::Vertical {
                        rtl_aware,
                        cross_axis_alignment,
                        ..
                    } => match cross_axis_alignment {
                        CrossAxisAlignment::Start => {
                            if rtl_aware {
                                AlignX::Start
                            } else {
                                AlignX::Left
                            }
                        }
                        CrossAxisAlignment::End => {
                            if rtl_aware {
                                AlignX::End
                            } else {
                                AlignX::Right
                            }
                        }
                        CrossAxisAlignment::Center => AlignX::Center,
                        CrossAxisAlignment::Stretch => AlignX::Start,
                        CrossAxisAlignment::Baseline => {
                            panic!("Baseline align is not supported for vstack")
                        }
                    },
                };

                let align_y = match layout_state.pass2_parent_container.axis {
                    StackAxisPass2::None | StackAxisPass2::Passthrough { .. } => AlignY::Top,
                    StackAxisPass2::Align { align_y, .. } => align_y,
                    StackAxisPass2::Horizontal {
                        cross_axis_alignment,
                        ..
                    } => match cross_axis_alignment {
                        CrossAxisAlignment::Start => AlignY::Top,
                        CrossAxisAlignment::End => AlignY::Bottom,
                        CrossAxisAlignment::Center => AlignY::Center,
                        CrossAxisAlignment::Stretch => AlignY::Top,
                        CrossAxisAlignment::Baseline => {
                            todo!()
                        }
                    },
                    StackAxisPass2::Vertical { .. } => AlignY::Top,
                };

                current_position += Vec2::new(
                    align_x.position_f64(
                        layout_state.layout_direction,
                        boundary.width,
                        widget_size.x,
                    ),
                    align_y.position_f64(boundary.height, widget_size.y),
                );

                let current_container_position = current_position + offset;

                if RENDER_CONTAINER_DEBUG_BOUNDARIES {
                    clipped_layout_items.push(LayoutItem::Placement(WidgetPlacement {
                        widget_ref: WidgetRef {
                            widget_type: WidgetType::of::<DebugBoundary>(),
                            id: WidgetId::auto(),
                        },
                        zindex: i32::MAX,
                        boundary: Rect::ZERO,
                        rect: Rect::from_pos_size(current_container_position, widget_size),
                    }));

                    clipped_layout_items.push(LayoutItem::Placement(WidgetPlacement {
                        widget_ref: WidgetRef {
                            widget_type: WidgetType::of::<DebugBoundary>(),
                            id: WidgetId::auto(),
                        },
                        zindex: i32::MAX,
                        boundary: Rect::ZERO,
                        rect: Rect::from_pos_size(boundary.position() + offset, boundary.size()),
                    }));
                }

                let inside_size = widget_size - Vec2::new(margin.horizontal(), margin.vertical());
                let decorator_rect = Rect::from_pos_size(current_position + offset, inside_size);

                for widget_ref in backgrounds {
                    let item = LayoutItem::Placement(WidgetPlacement {
                        widget_ref: *widget_ref,
                        zindex: *zindex,
                        boundary: decorator_rect,
                        rect: decorator_rect,
                    });

                    layout_items.push(item.clone());

                    if rects_overlap(
                        Rect::from_pos_size(position + offset, inside_size),
                        Rect::from_pos_size(Vec2::ZERO, view_size),
                    ) {
                        clipped_layout_items.push(item);
                    }
                }

                if *clip != Clip::None {
                    layout_items.push(LayoutItem::PushClip {
                        rect: decorator_rect,
                        clip: *clip,
                        zindex: *zindex,
                    });
                    clipped_layout_items.push(LayoutItem::PushClip {
                        rect: decorator_rect,
                        clip: *clip,
                        zindex: *zindex,
                    });
                } else {
                    layout_items.push(LayoutItem::BeginGroup { zindex: *zindex });
                    clipped_layout_items.push(LayoutItem::BeginGroup { zindex: *zindex });
                }

                current_position.x += padding.left;
                current_position.y += padding.top;

                match kind {
                    ContainerKind::VStack {
                        spacing,
                        main_axis_alignment,
                        cross_axis_alignment,
                        rtl_aware,
                    } => {
                        layout_state.pass2_parent_container = Pass2LayoutContainer {
                            idx: current_idx,
                            clipping,
                            padding: *padding,
                            zindex: *zindex,
                            decorator_rect,
                            foregrounds: foregrounds.clone(),
                            axis: StackAxisPass2::Vertical {
                                spacing: *spacing,
                                rtl_aware: *rtl_aware,
                                _main_axis_alignment: *main_axis_alignment,
                                cross_axis_alignment: *cross_axis_alignment,
                            },
                        };

                        current_idx += 1;
                        go_next = false;
                    }
                    ContainerKind::HStack {
                        spacing,
                        rtl_aware,
                        main_axis_alignment,
                        cross_axis_alignment,
                    } => {
                        if *rtl_aware && layout_state.layout_direction == LayoutDirection::RTL {
                            current_position = position + Vec2::new(widget_size.x, 0.);
                        }

                        layout_state.pass2_parent_container = Pass2LayoutContainer {
                            idx: current_idx,
                            clipping,
                            padding: *padding,
                            zindex: *zindex,
                            decorator_rect,
                            foregrounds: foregrounds.clone(),
                            axis: StackAxisPass2::Horizontal {
                                spacing: *spacing,
                                rtl_aware: *rtl_aware,
                                _main_axis_alignment: *main_axis_alignment,
                                cross_axis_alignment: *cross_axis_alignment,
                            },
                        };

                        current_idx += 1;
                        go_next = false;
                    }
                    ContainerKind::Flow { .. } => todo!(),
                    ContainerKind::ZStack { align_x, align_y } => {
                        layout_state.pass2_parent_container = Pass2LayoutContainer {
                            padding: *padding,
                            zindex: *zindex,
                            clipping,
                            idx: current_idx,
                            decorator_rect,
                            foregrounds: foregrounds.clone(),
                            axis: StackAxisPass2::Align {
                                align_x: *align_x,
                                align_y: *align_y,
                            },
                        };

                        current_idx += 1;
                        go_next = false;
                    }
                    ContainerKind::None => {
                        layout_state.pass2_parent_container = Pass2LayoutContainer {
                            padding: *padding,
                            zindex: *zindex,
                            clipping,
                            idx: current_idx,
                            decorator_rect,
                            foregrounds: foregrounds.clone(),
                            axis: StackAxisPass2::None,
                        };

                        current_idx += 1;
                        go_next = false;
                    }
                    ContainerKind::Passthrough => {
                        layout_state.pass2_parent_container = Pass2LayoutContainer {
                            padding: *padding,
                            zindex: *zindex,
                            clipping,
                            idx: current_idx,
                            decorator_rect,
                            foregrounds: foregrounds.clone(),
                            axis: StackAxisPass2::Passthrough {
                                stretch: match parent_container_axis {
                                    StackAxisPass2::None
                                    | StackAxisPass2::Passthrough { .. }
                                    | StackAxisPass2::Align { .. } => None,
                                    StackAxisPass2::Horizontal {
                                        cross_axis_alignment,
                                        ..
                                    } => {
                                        if cross_axis_alignment == CrossAxisAlignment::Stretch {
                                            Some(Axis::Horizontal)
                                        } else {
                                            None
                                        }
                                    }
                                    StackAxisPass2::Vertical {
                                        cross_axis_alignment,
                                        ..
                                    } => {
                                        if cross_axis_alignment == CrossAxisAlignment::Stretch {
                                            Some(Axis::Vertical)
                                        } else {
                                            None
                                        }
                                    }
                                },
                            },
                        };

                        current_idx += 1;
                        go_next = false;
                    }
                    ContainerKind::Measure { id } => {
                        layout_state.pass2_parent_container = Pass2LayoutContainer {
                            padding: *padding,
                            zindex: *zindex,
                            clipping,
                            idx: current_idx,
                            decorator_rect,
                            foregrounds: foregrounds.clone(),
                            axis: StackAxisPass2::None,
                        };

                        layout_measures.set(
                            *id,
                            LayoutMeasure {
                                x: current_container_position.x + margin.left,
                                y: current_container_position.y + margin.top,
                                width: widget_size.x - margin.horizontal(),
                                height: widget_size.y - margin.vertical(),
                                wrap_width: container_wrap_size.x - container_margin.horizontal(),
                                wrap_height: container_wrap_size.y - container_margin.vertical(),
                            },
                        );

                        current_idx += 1;
                        go_next = false;
                    }
                }
            }
            LayoutCommand::EndContainer => {
                widget_size = container_size;
                let container = layout_state.pass2_parent_container.clone();
                layout_state.pass2_parent_container = layout_state.pop_pass2_container();
                current_position = layout_state.pop_position();

                if container.clipping {
                    layout_items.push(LayoutItem::PopClip);
                    clipped_layout_items.push(LayoutItem::PopClip);
                } else {
                    layout_items.push(LayoutItem::EndGroup);
                    clipped_layout_items.push(LayoutItem::EndGroup);
                }

                for widget_ref in &container.foregrounds {
                    let item = LayoutItem::Placement(WidgetPlacement {
                        widget_ref: *widget_ref,
                        zindex: container.zindex,
                        boundary: container.decorator_rect,
                        rect: container.decorator_rect,
                    });

                    layout_items.push(item.clone());
                    clipped_layout_items.push(item);
                }
            }
            LayoutCommand::Leaf {
                widget_ref,
                backgrounds,
                foregrounds,
                zindex,
                padding,
                margin,
                derive_wrap_size,
                clip,
                size,
                constraints,
                ..
            } => {
                let align_x = match layout_state.pass2_parent_container.axis {
                    StackAxisPass2::None | StackAxisPass2::Passthrough { .. } => AlignX::Start,
                    StackAxisPass2::Align { align_x, .. } => align_x,
                    StackAxisPass2::Horizontal { .. } => AlignX::Start,
                    StackAxisPass2::Vertical {
                        rtl_aware,
                        cross_axis_alignment,
                        ..
                    } => match cross_axis_alignment {
                        CrossAxisAlignment::Start => {
                            if rtl_aware {
                                AlignX::Start
                            } else {
                                AlignX::Left
                            }
                        }
                        CrossAxisAlignment::End => {
                            if rtl_aware {
                                AlignX::End
                            } else {
                                AlignX::Right
                            }
                        }
                        CrossAxisAlignment::Center => AlignX::Center,
                        CrossAxisAlignment::Stretch => AlignX::Start,
                        CrossAxisAlignment::Baseline => {
                            panic!("Baseline align is not supported for vstack")
                        }
                    },
                };

                let align_y = match layout_state.pass2_parent_container.axis {
                    StackAxisPass2::None | StackAxisPass2::Passthrough { .. } => AlignY::Top,
                    StackAxisPass2::Align { align_y, .. } => align_y,
                    StackAxisPass2::Horizontal {
                        cross_axis_alignment,
                        ..
                    } => match cross_axis_alignment {
                        CrossAxisAlignment::Start => AlignY::Top,
                        CrossAxisAlignment::End => AlignY::Bottom,
                        CrossAxisAlignment::Center => AlignY::Center,
                        CrossAxisAlignment::Stretch => AlignY::Top,
                        CrossAxisAlignment::Baseline => {
                            todo!()
                        }
                    },
                    StackAxisPass2::Vertical { .. } => AlignY::Top,
                };

                let decorators_rect = Rect::from_pos_size(
                    position
                        + Vec2::new(margin.left, margin.right)
                        + Vec2::new(
                            align_x.position_f64(
                                layout_state.layout_direction,
                                boundary.width,
                                widget_size.x,
                            ),
                            align_y.position_f64(boundary.height, widget_size.y),
                        )
                        + offset,
                    widget_size - Vec2::new(margin.horizontal(), margin.vertical()),
                );

                let boundary = Rect::from_pos_size(boundary.position() + offset, boundary.size());

                // Don't render anything outside the screen view
                let should_render =
                    rects_overlap(decorators_rect, Rect::from_pos_size(Vec2::ZERO, view_size));

                for widget_ref in backgrounds {
                    let item = LayoutItem::Placement(WidgetPlacement {
                        widget_ref: *widget_ref,
                        zindex: *zindex,
                        boundary: decorators_rect,
                        rect: decorators_rect,
                    });

                    layout_items.push(item.clone());

                    if should_render {
                        clipped_layout_items.push(item.clone());
                    }
                }

                let rect = Rect::from_pos_size(
                    decorators_rect.position() + Vec2::new(padding.left, padding.top),
                    decorators_rect.size() - Vec2::new(padding.horizontal(), padding.vertical()),
                );

                if *clip != Clip::None {
                    layout_items.push(LayoutItem::PushClip {
                        rect: decorators_rect,
                        clip: *clip,
                        zindex: *zindex,
                    });
                }

                layout_items.push(LayoutItem::Placement(WidgetPlacement {
                    widget_ref: *widget_ref,
                    zindex: *zindex,
                    boundary: decorators_rect,
                    rect,
                }));

                if *clip != Clip::None {
                    layout_items.push(LayoutItem::PopClip);
                }

                if should_render {
                    if *clip != Clip::None {
                        clipped_layout_items.push(LayoutItem::PushClip {
                            rect: decorators_rect,
                            clip: *clip,
                            zindex: *zindex,
                        });
                    }

                    clipped_layout_items.push(LayoutItem::Placement(WidgetPlacement {
                        widget_ref: *widget_ref,
                        zindex: *zindex,
                        boundary: decorators_rect,
                        rect,
                    }));

                    if *clip != Clip::None {
                        clipped_layout_items.push(LayoutItem::PopClip);
                    }
                }

                for widget_ref in foregrounds {
                    let item = LayoutItem::Placement(WidgetPlacement {
                        widget_ref: *widget_ref,
                        zindex: *zindex,
                        boundary: decorators_rect,
                        rect: decorators_rect,
                    });

                    layout_items.push(item.clone());

                    if should_render {
                        clipped_layout_items.push(item);
                    }
                }

                if let DeriveWrapSize::Text {
                    text_id,
                    derive_width,
                    derive_height,
                } = derive_wrap_size
                {
                    layout_state.texts.push(TextLayout {
                        width: if size.width.constrained() && *derive_width {
                            Some(rect.width * scale_factor)
                        } else if *derive_width {
                            if constraints.max_width != f64::INFINITY {
                                Some((constraints.max_width - padding.horizontal()) * scale_factor)
                            } else {
                                None
                            }
                        } else {
                            None
                        },
                        height: if size.height.constrained() && *derive_height {
                            Some(rect.height * scale_factor)
                        } else if *derive_height {
                            if constraints.max_height != f64::INFINITY {
                                Some((constraints.max_height - padding.vertical()) * scale_factor)
                            } else {
                                None
                            }
                        } else {
                            None
                        },
                        text_id: *text_id,
                    });
                };

                if RENDER_CHILD_DEBUG_BOUNDARIES {
                    clipped_layout_items.push(LayoutItem::Placement(WidgetPlacement {
                        widget_ref: WidgetRef {
                            widget_type: WidgetType::of::<DebugBoundary>(),
                            id: WidgetId::auto(),
                        },
                        zindex: i32::MAX,
                        boundary: Rect::ZERO,
                        rect: boundary,
                    }));

                    clipped_layout_items.push(LayoutItem::Placement(WidgetPlacement {
                        widget_ref: WidgetRef {
                            widget_type: WidgetType::of::<DebugBoundary>(),
                            id: WidgetId::auto(),
                        },
                        zindex: i32::MAX,
                        boundary: Rect::ZERO,
                        rect,
                    }));

                    clipped_layout_items.push(LayoutItem::Placement(WidgetPlacement {
                        widget_ref: WidgetRef {
                            widget_type: WidgetType::of::<DebugBoundary>(),
                            id: WidgetId::auto(),
                        },
                        zindex: i32::MAX,
                        boundary: Rect::ZERO,
                        rect: decorators_rect,
                    }));
                }

                current_idx += 1;
            }
            LayoutCommand::Spacer { .. } => {
                current_idx += 1;
            }
        }

        if go_next {
            match layout_state.pass2_parent_container.axis {
                StackAxisPass2::Horizontal {
                    spacing, rtl_aware, ..
                } => {
                    if rtl_aware && layout_state.layout_direction == LayoutDirection::RTL {
                        current_position.x -= widget_size.x + spacing
                    } else {
                        current_position.x += widget_size.x + spacing
                    }
                }
                StackAxisPass2::Vertical { spacing, .. } => {
                    current_position.y += widget_size.y + spacing;
                }
                StackAxisPass2::None
                | StackAxisPass2::Align { .. }
                | StackAxisPass2::Passthrough { .. } => {}
            }
        }
    }

    debug_assert!(layout_state.containers_stack_cursor == 0);

    layout_state.actual_sizes[0]
}
