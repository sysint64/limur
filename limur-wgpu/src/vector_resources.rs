use limur::{
    Border, BorderRadius, BorderSide, BoxShadow, BoxShape, ColorRgba, Gradient, Rect, View,
    render::Fill,
    text::{FontResources, TextId},
};

use crate::{
    GraphicsContext,
    gpu_vec::GpuVec,
    text::{
        Bounds, ContentType, GetGlyphImageResult, GlyphBounds, GlyphMetadata, GlyphSystem,
        TextResources, prepare_glyph,
    },
};

pub(crate) struct VectorResources {
    pub(crate) swash_cache: cosmic_text::SwashCache,
    pub(crate) data: GpuVec<VectorData>,
    pub(crate) gradient_stops: GpuVec<GradientStop>,
}

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VectorData {
    // [x, y, w, h]
    pub(crate) boundary: [f32; 4],

    // 0: rect
    // 1: oval
    // 2: rect outer box shadow
    // 3: rect inner box shadow
    // 4: oval outer box shadow
    // 5: oval inner box shadow
    // 6: glyph
    pub(crate) shape_type: u32,

    pub(crate) _pad0: [u32; 3],

    pub(crate) fill_color: [f32; 4],
    pub(crate) border_color_left: [f32; 4],
    pub(crate) border_color_top: [f32; 4],
    pub(crate) border_color_right: [f32; 4],
    pub(crate) border_color_bottom: [f32; 4],

    // [left, top, right, bottom]
    pub(crate) border_widths: [f32; 4],

    // [top left, top right, bottom right, bottom left]
    pub(crate) border_radii: [f32; 4],

    // [offset_x, offset_y, blur, spread]
    pub(crate) box_shadow: [f32; 4],

    // [type, start_index, stop_count, pad]
    // types: 0: none, 1: linear, 2: radial, 3: sweep
    pub(crate) gradient_info: [u32; 4],

    // linear: [sx, sy, ex, ey]
    // radial: [cx, cy, r, _]
    // sweep: [cx, cy, start_angle, end_angle]
    pub(crate) gradient_params: [f32; 4],

    pub(crate) text_uv: [f32; 2],
    pub(crate) text_content_type_with_srgb: [u16; 2],

    pub(crate) _pad3: u32,
}

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GradientStop {
    color: [f32; 4],
    offset: f32,
    _pad0: [u32; 3],
}

pub(crate) struct GradientInfo {
    gradient_info: [u32; 4],
    gradient_params: [f32; 4],
}

impl GradientInfo {
    fn empty() -> Self {
        Self {
            gradient_info: [0; 4],
            gradient_params: [0.; 4],
        }
    }
}

impl VectorResources {
    pub(crate) fn new() -> Self {
        Self {
            data: GpuVec::new(
                8096,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            ),
            gradient_stops: GpuVec::new(
                128,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            ),
            swash_cache: cosmic_text::SwashCache::new(),
        }
    }

    pub(crate) fn flush(&mut self, context: &GraphicsContext) {
        self.data.flush(context);
        self.gradient_stops.flush(context);
    }

    pub fn take_buffer_resized(&mut self) -> bool {
        self.data.take_buffer_resized() || self.gradient_stops.take_buffer_resized()
    }

    fn push_stops(&mut self, stops: &[limur::ColorStop]) {
        for stop in stops {
            self.gradient_stops.push(GradientStop {
                color: to_shader_color(stop.color),
                offset: stop.offset,
                _pad0: [0; 3],
            });
        }
    }

    pub(crate) fn maybe_add_gradient(&mut self, fill: Option<&Fill>) -> GradientInfo {
        match fill {
            Some(fill) => match fill {
                Fill::None => GradientInfo::empty(),
                Fill::Color(..) => GradientInfo::empty(),
                Fill::Gradient(gradient) => self.add_gradient(gradient.clone()),
            },
            None => GradientInfo::empty(),
        }
    }

    pub(crate) fn add_gradient(&mut self, gradient: Gradient) -> GradientInfo {
        let start_index = self.gradient_stops.len() as u32;

        match &gradient {
            Gradient::Linear(gradient) => {
                self.push_stops(&gradient.stops);
                GradientInfo {
                    gradient_info: [1, start_index, gradient.stops.len() as u32, 0],
                    gradient_params: [
                        gradient.start.0,
                        gradient.start.1,
                        gradient.end.0,
                        gradient.end.1,
                    ],
                }
            }
            Gradient::Radial(gradient) => {
                self.push_stops(&gradient.stops);
                GradientInfo {
                    gradient_info: [2, start_index, gradient.stops.len() as u32, 0],
                    gradient_params: [gradient.center.0, gradient.center.1, gradient.radius, 0.0],
                }
            }
            Gradient::Sweep(gradient) => {
                self.push_stops(&gradient.stops);
                GradientInfo {
                    gradient_info: [3, start_index, gradient.stops.len() as u32, 0],
                    gradient_params: [
                        gradient.center.0,
                        gradient.center.1,
                        gradient.start_angle,
                        gradient.end_angle,
                    ],
                }
            }
        }
    }

    pub(crate) fn push_shape(
        &mut self,
        boundary: Rect<f32>,
        fill: Option<&Fill>,
        border_radius: Option<BorderRadius>,
        border: Option<Border>,
        shape: BoxShape,
        gradient_params: GradientInfo,
    ) {
        let side = |side: Option<BorderSide>| -> ([f32; 4], f32) {
            match side {
                Some(side) => (to_shader_color(side.color), side.width),
                None => ([0.0; 4], 0.0),
            }
        };

        let fill_color = match fill {
            Some(Fill::Color(color)) => to_shader_color(*color),
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

        self.data.push(VectorData {
            boundary: [boundary.x, boundary.y, boundary.width, boundary.height],
            shape_type,
            fill_color,
            border_color_left,
            border_color_top,
            border_color_right,
            border_color_bottom,
            border_widths: [
                border_width_left.round(),
                border_width_top.round(),
                border_width_right.round(),
                border_width_bottom.round(),
            ],
            border_radii: [
                radii.top_left,
                radii.top_right,
                radii.bottom_right,
                radii.bottom_left,
            ],
            gradient_info: gradient_params.gradient_info,
            gradient_params: gradient_params.gradient_params,
            ..Default::default()
        });
    }

    pub(crate) fn push_outer_shadow(
        &mut self,
        boundary: Rect<f32>,
        box_shadow: BoxShadow,
        border_radius: Option<BorderRadius>,
        shape: BoxShape,
    ) {
        let radii = border_radius.unwrap_or(BorderRadius::ZERO);

        let shape_type = match shape {
            BoxShape::Rect => 2,
            BoxShape::Oval => 4,
        };

        self.data.push(VectorData {
            boundary: [boundary.x, boundary.y, boundary.width, boundary.height],
            shape_type,
            fill_color: to_shader_color(box_shadow.color),
            border_radii: [
                radii.top_left,
                radii.top_right,
                radii.bottom_right,
                radii.bottom_left,
            ],
            box_shadow: [
                box_shadow.offset.x as f32,
                box_shadow.offset.y as f32,
                box_shadow.blur_radius as f32,
                box_shadow.spread_radius as f32,
            ],
            ..Default::default()
        })
    }

    pub(crate) fn push_inner_shadow(
        &mut self,
        boundary: Rect<f32>,
        box_shadow: BoxShadow,
        border_radius: Option<BorderRadius>,
        shape: BoxShape,
    ) {
        let radii = border_radius.unwrap_or(BorderRadius::ZERO);

        let shape_type = match shape {
            BoxShape::Rect => 3,
            BoxShape::Oval => 5,
        };

        self.data.push(VectorData {
            boundary: [boundary.x, boundary.y, boundary.width, boundary.height],
            shape_type,
            fill_color: to_shader_color(box_shadow.color),
            border_radii: [
                radii.top_left,
                radii.top_right,
                radii.bottom_right,
                radii.bottom_left,
            ],
            box_shadow: [
                box_shadow.offset.x as f32,
                box_shadow.offset.y as f32,
                box_shadow.blur_radius as f32,
                box_shadow.spread_radius as f32,
            ],
            ..Default::default()
        });
    }

    pub(crate) fn push_text(
        &mut self,
        context: &GraphicsContext,
        fonts: &mut FontResources,
        text: &mut limur::text::TextsResources,
        text_resrouces: &mut TextResources,
        view: &View,
        id: TextId,
        boundary: Rect<f32>,
        x: f32,
        y: f32,
        tint_color: Option<ColorRgba>,
    ) {
        let is_run_visible = |run: &cosmic_text::LayoutRun| {
            let start_y_physical = (y + run.line_top) as i32;
            let end_y_physical = start_y_physical + run.line_height as i32;

            start_y_physical <= boundary.bottom().ceil() as i32
                && boundary.top().floor() as i32 <= end_y_physical
        };

        let buffer = text.get(id).buffer();
        let layout_runs = buffer
            .layout_runs()
            .skip_while(|run| !is_run_visible(run))
            .take_while(is_run_visible);

        for run in layout_runs {
            for glyph in run.glyphs.iter() {
                // x, y is the scroll-adjusted text render origin; boundary is clip-only.
                let physical_glyph = glyph.physical((x, y), 1.0);

                let color = match glyph.color_opt {
                    Some(color) => ColorRgba {
                        r: color.r() as f32 / 255.0,
                        g: color.g() as f32 / 255.0,
                        b: color.b() as f32 / 255.0,
                        a: color.a() as f32 / 255.0,
                    },
                    None => tint_color.unwrap_or(ColorRgba::from_hex(0xFF000000)),
                };

                let mut system = GlyphSystem {
                    resources: text_resrouces,
                    cache: &mut self.swash_cache,
                    font_system: &mut fonts.font_system,
                };

                let bounds = GlyphBounds {
                    x: Bounds {
                        min: boundary.left().max(0.0) as i32,
                        max: boundary.right().min(view.physical_size.width as f32) as i32,
                    },
                    y: Bounds {
                        min: boundary.top().max(0.0) as i32,
                        max: boundary.bottom().min(view.physical_size.height as f32) as i32,
                    },
                };

                if let Ok(Some(glyph_to_render)) = prepare_glyph(
                    &context,
                    &mut system,
                    GlyphMetadata {
                        x: physical_glyph.x,
                        y: physical_glyph.y,
                        line_y: run.line_y,
                        color,
                        metadata: glyph.metadata,
                        cache_key: physical_glyph.cache_key,
                        scale_factor: 1.0,
                    },
                    bounds,
                    |system| -> Option<GetGlyphImageResult> {
                        let image = system
                            .cache
                            .get_image_uncached(system.font_system, physical_glyph.cache_key)?;

                        let content_type = match image.content {
                            cosmic_text::SwashContent::Color => ContentType::Color,
                            cosmic_text::SwashContent::Mask => ContentType::Mask,
                            cosmic_text::SwashContent::SubpixelMask => ContentType::SubpixelMask,
                        };

                        Some(GetGlyphImageResult {
                            content_type,
                            top: image.placement.top as i16,
                            left: image.placement.left as i16,
                            width: image.placement.width as u16,
                            height: image.placement.height as u16,
                            data: image.data,
                        })
                    },
                ) {
                    self.data.push(glyph_to_render);
                }
            }
        }
    }
}

#[inline]
pub(crate) fn to_shader_color(color: ColorRgba) -> [f32; 4] {
    // Compositor textures store linear light (Rgba16Float). Always linearize.
    [
        srgb_to_linear(color.r as f64) as f32,
        srgb_to_linear(color.g as f64) as f32,
        srgb_to_linear(color.b as f64) as f32,
        color.a as f32,
    ]
}

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
