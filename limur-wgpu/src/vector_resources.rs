use limur::{Border, BorderRadius, BorderSide, BoxShape, ColorRgba, Gradient, Rect, render::Fill};

use crate::to_wgpu_color;

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VectorData {
    // [x, y, w, h]
    boundary: [f32; 4],

    // 0: rect, 1: oval
    shape_type: u32,

    // 0: none, 1: outer, 2: inner
    box_shadow_style: u32,

    // 12 bytes — align to 16
    _pad0: [u32; 2],

    fill_color: [f32; 4],
    border_color_left: [f32; 4],
    border_color_top: [f32; 4],
    border_color_right: [f32; 4],
    border_color_bottom: [f32; 4],

    // [left, top, right, bottom]
    border_widths: [f32; 4],

    // [top left, top right, bottom right, bottom left]
    border_radii: [f32; 4],

    // [offset_x, offset_y, blur, spread]
    box_shadow: [f32; 4],

    box_shadow_color: [f32; 4],

    // [type, start_index, stop_count, pad]
    // types: 0: linear, 1: radial
    gradient_info: [u32; 4],

    // linear: [sx, sy, ex, ey] radial: [cx, cy, r, 0]
    gradient_params: [f32; 4],
}

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GradientStop {
    color: [f32; 4],
    offset: f32,
    _pad0: [u32; 3],
}

pub(crate) struct VectorResources {
    pub(crate) data: sumi::GpuVec<VectorData>,
    pub(crate) gradient_stops: sumi::GpuVec<GradientStop>,
}

pub(crate) struct GradientInfo {
    gradient_info: [u32; 4],
    gradient_params: [u32; 4],
}

impl GradientInfo {
    fn empty() -> Self {
        Self {
            gradient_info: [0; 4],
            gradient_params: [0; 4],
        }
    }
}

impl VectorResources {
    pub(crate) fn new() -> Self {
        Self {
            data: sumi::GpuVec::new(
                128,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            ),
            gradient_stops: sumi::GpuVec::new(
                128,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            ),
        }
    }

    pub(crate) fn flush(&mut self, context: &sumi::GraphicsContext) {
        self.data.flush(context);
        self.gradient_stops.flush(context);
    }

    fn push_stops(&mut self, context: &sumi::GraphicsContext, stops: &[limur::ColorStop]) {
        for stop in stops {
            self.gradient_stops.push(GradientStop {
                color: to_color(context, stop.color),
                offset: stop.offset,
                _pad0: [0; 3],
            });
        }
    }

    pub(crate) fn maybe_add_gradient(
        &mut self,
        context: &sumi::GraphicsContext,
        fill: Option<&Fill>,
    ) -> GradientInfo {
        match fill {
            Some(fill) => match fill {
                Fill::None => GradientInfo::empty(),
                Fill::Color(..) => GradientInfo::empty(),
                Fill::Gradient(gradient) => self.add_gradient(context, gradient.clone()),
            },
            None => GradientInfo::empty(),
        }
    }

    pub(crate) fn add_gradient(
        &mut self,
        context: &sumi::GraphicsContext,
        gradient: Gradient,
    ) -> GradientInfo {
        let start_index = self.gradient_stops.len() as u32;

        match &gradient {
            Gradient::Linear(gradient) => {
                self.push_stops(context, &gradient.stops);
                GradientInfo {
                    gradient_info: [0, start_index, gradient.stops.len() as u32, 0],
                    gradient_params: [
                        gradient.start.0.to_bits(),
                        gradient.start.1.to_bits(),
                        gradient.end.0.to_bits(),
                        gradient.end.1.to_bits(),
                    ],
                }
            }
            Gradient::Radial(gradient) => {
                self.push_stops(context, &gradient.stops);
                GradientInfo {
                    gradient_info: [1, start_index, gradient.stops.len() as u32, 0],
                    gradient_params: [
                        gradient.center.0.to_bits(),
                        gradient.center.1.to_bits(),
                        gradient.radius.to_bits(),
                        0,
                    ],
                }
            }
            Gradient::Sweep(_) => GradientInfo {
                gradient_info: [0, start_index, 0, 0],
                gradient_params: [0; 4],
            },
        }
    }
}

#[inline]
fn to_color(context: &sumi::GraphicsContext, color: ColorRgba) -> [f32; 4] {
    let wgpu_color = to_wgpu_color(context.surface_texture_format, color);

    [
        wgpu_color.r as f32,
        wgpu_color.g as f32,
        wgpu_color.b as f32,
        wgpu_color.a as f32,
    ]
}

impl VectorData {
    pub(crate) fn shape(
        context: &sumi::GraphicsContext,
        boundary: Rect<f32>,
        fill: Option<&Fill>,
        border_radius: Option<BorderRadius>,
        border: Option<Border>,
        shape: BoxShape,
        gradient_params: GradientInfo,
    ) -> Self {
        let side = |side: Option<BorderSide>| -> ([f32; 4], f32) {
            match side {
                Some(side) => (to_color(context, side.color), side.width),
                None => ([0.0; 4], 0.0),
            }
        };

        let fill_color = match fill {
            Some(Fill::Color(color)) => to_color(context, *color),
            _ => [0.0; 4],
        };

        let (border_color_top, border_width_top) = side(border.and_then(|it| it.top));
        let (border_color_right, border_width_right) = side(border.and_then(|it| it.right));
        let (border_color_bottom, border_width_bottom) = side(border.and_then(|it| it.bottom));
        let (border_color_left, border_width_left) = side(border.and_then(|it| it.left));

        let radii = border_radius.unwrap_or(BorderRadius::ZERO);

        let shape_type = match shape {
            BoxShape::Rect => 0,
            BoxShape::Oval => 1,
        };

        Self {
            boundary: [boundary.x, boundary.y, boundary.width, boundary.height],
            shape_type,
            box_shadow_style: 0,
            _pad0: [0; 2],
            fill_color,
            border_color_left,
            border_color_top,
            border_color_right,
            border_color_bottom,
            border_widths: [
                border_width_left,
                border_width_top,
                border_width_right,
                border_width_bottom,
            ],
            border_radii: [
                radii.top_left,
                radii.top_right,
                radii.bottom_right,
                radii.bottom_left,
            ],
            box_shadow: [0.0; 4],
            box_shadow_color: [0.0; 4],
            gradient_info: gradient_params.gradient_info,
            gradient_params: bytemuck::cast(gradient_params.gradient_params),
        }
    }
}
