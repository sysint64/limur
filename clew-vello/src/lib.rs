use clew::{
    Border, BorderRadius, BorderSide, ClipShape, ColorRgb, ColorRgba, Gradient, Rect, View,
    assets::Assets,
    profiler,
    render::{Fill, RenderCommand, RenderState, Renderer},
    text::{FontResources, TextsResources},
};
use cosmic_text::{Buffer, FontSystem};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{collections::HashMap, sync::Arc};
use vello::{
    AaConfig, Glyph, RenderParams, RendererOptions, Scene,
    kurbo::{Affine, RoundedRect, RoundedRectRadii, Stroke},
    peniko::{
        self, Blob, Brush, Color, Fill as VelloFill, FontData, Gradient as VelloGradient, StyleRef,
    },
    util::RenderContext,
    wgpu,
};
use vello_svg::usvg;

/// Cache for FontData to avoid repeated allocations
struct FontCache {
    cache: HashMap<cosmic_text::fontdb::ID, FontData>,
}

impl FontCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    fn get_or_insert(
        &mut self,
        font_id: cosmic_text::fontdb::ID,
        font_system: &mut FontSystem,
    ) -> Option<&FontData> {
        if let std::collections::hash_map::Entry::Vacant(e) = self.cache.entry(font_id)
            && let Some(font) = font_system.get_font(font_id)
        {
            let font_data = FontData::new(Blob::new(Arc::new(font.data().to_vec())), 0);
            e.insert(font_data);
        }

        self.cache.get(&font_id)
    }

    fn _clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}

pub struct VelloRenderer {
    render_cx: RenderContext,
    surface: Option<vello::util::RenderSurface<'static>>,
    renderer: Option<vello::Renderer>,
    scene: Scene,
    font_cache: FontCache,

    current_width: u32,
    current_height: u32,
}

impl VelloRenderer {
    pub async fn new<W>(window: Arc<W>, width: u32, height: u32) -> Self
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        let mut render_cx = RenderContext::new();

        // Create the surface
        let surface = render_cx
            .create_surface(window.clone(), width, height, wgpu::PresentMode::Fifo)
            .await
            .expect("Failed to create surface");

        #[cfg(target_os = "macos")]
        #[allow(invalid_reference_casting)]
        unsafe {
            if let Some(hal_surface) = surface.surface.as_hal::<wgpu::hal::api::Metal>() {
                let raw = (&*hal_surface) as *const wgpu::hal::metal::Surface
                    as *mut wgpu::hal::metal::Surface;
                (*raw).present_with_transaction = true;
            }
        }

        let device = &render_cx.devices[surface.dev_id].device;

        // Create Vello renderer
        let renderer = vello::Renderer::new(device, RendererOptions::default())
            .expect("Failed to create Vello renderer");

        let mut config = surface.config.clone();
        config.desired_maximum_frame_latency = 3;
        surface.surface.configure(device, &config);

        Self {
            render_cx,
            surface: Some(surface),
            renderer: Some(renderer),
            scene: Scene::new(),
            font_cache: FontCache::new(),

            current_width: width,
            current_height: height,
        }
    }

    /// Resize the renderer surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        if self.current_width == width && self.current_height == height {
            return;
        }

        self.current_width = width;
        self.current_height = height;

        if let Some(surface) = &mut self.surface {
            self.render_cx.resize_surface(surface, width, height);
        }
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self) {
        self.scene.reset();
    }

    /// End frame and present
    pub fn end_frame(&mut self, fill_color: &ColorRgb) {
        let Some(surface) = &self.surface else { return };
        let Some(renderer) = &mut self.renderer else {
            return;
        };

        let device = &self.render_cx.devices[surface.dev_id].device;
        let queue = &self.render_cx.devices[surface.dev_id].queue;

        let render_params = RenderParams {
            base_color: convert_rgb_color(fill_color),
            width: self.current_width,
            height: self.current_height,
            antialiasing_method: AaConfig::Msaa16,
        };

        {
            renderer
                .render_to_texture(
                    device,
                    queue,
                    &self.scene,
                    &surface.target_view,
                    &render_params,
                )
                .expect("Failed to render to surface");
        }

        let surface_texture = {
            surface
                .surface
                .get_current_texture()
                .expect("Failed to get surface texture")
        };

        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Surface Blit"),
            });
            surface.blitter.copy(
                device,
                &mut encoder,
                &surface.target_view,
                &surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            );
            queue.submit([encoder.finish()]);
            surface_texture.present();
        }

        device.poll(wgpu::PollType::Poll).unwrap();

        // {
        //     profiling::scope!("device_poll");
        //     device.poll(wgpu::PollType::Wait).unwrap();
        // }
    }

    /// Draw a filled rectangle with optional border
    pub fn draw_rect(
        &mut self,
        boundary: Rect<f32>,
        fill: Option<&Fill>,
        border_radius: Option<&BorderRadius>,
        border: Option<&Border>,
    ) {
        let rect = vello::kurbo::Rect::new(
            boundary.x as f64,
            boundary.y as f64,
            (boundary.x + boundary.width) as f64,
            (boundary.y + boundary.height) as f64,
        );

        let shape = if let Some(br) = border_radius {
            RoundedRect::from_rect(
                rect,
                RoundedRectRadii::new(
                    br.top_left as f64,
                    br.top_right as f64,
                    br.bottom_right as f64,
                    br.bottom_left as f64,
                ),
            )
        } else {
            RoundedRect::from_rect(rect, 0.0)
        };

        // Draw fill
        if let Some(fill) = fill
            && let Some(brush) = create_brush_from_fill(fill, boundary)
        {
            self.scene
                .fill(VelloFill::NonZero, Affine::IDENTITY, &brush, None, &shape);
        }

        // Draw border
        if let Some(border) = border {
            self.draw_border(&shape, border);
        }
    }

    /// Draw border for a shape
    fn draw_border(&mut self, shape: &RoundedRect, border: &Border) {
        // Get the maximum border width and color
        let (max_width, color) = get_border_params(border);

        if max_width > 0.0 {
            let stroke = Stroke::new(max_width as f64);
            let brush = Brush::Solid(convert_rgba_color(&color));

            self.scene
                .stroke(&stroke, Affine::IDENTITY, &brush, None, shape);
        }
    }

    /// Draw an oval/ellipse with optional border
    pub fn draw_oval(
        &mut self,
        boundary: Rect<f32>,
        fill: Option<&Fill>,
        border: Option<&BorderSide>,
    ) {
        let ellipse = vello::kurbo::Ellipse::new(
            (
                (boundary.x + boundary.width / 2.0) as f64,
                (boundary.y + boundary.height / 2.0) as f64,
            ),
            (
                (boundary.width / 2.0) as f64,
                (boundary.height / 2.0) as f64,
            ),
            0.0,
        );

        // Draw fill
        if let Some(fill) = fill
            && let Some(brush) = create_brush_from_fill(fill, boundary)
        {
            self.scene
                .fill(VelloFill::NonZero, Affine::IDENTITY, &brush, None, &ellipse);
        }

        // Draw border
        if let Some(border_side) = border
            && border_side.width > 0.0
        {
            let stroke = Stroke::new(border_side.width as f64);
            let brush = Brush::Solid(convert_rgba_color(&border_side.color));
            self.scene
                .stroke(&stroke, Affine::IDENTITY, &brush, None, &ellipse);
        }
    }

    /// Draw text from a cosmic_text Buffer
    pub fn draw_text(
        &mut self,
        font_system: &mut FontSystem,
        buffer: &Buffer,
        x: f32,
        y: f32,
        color: Color,
    ) {
        let brush = Brush::Solid(color);

        // Group glyphs by font for batched rendering
        let mut font_glyphs: HashMap<cosmic_text::fontdb::ID, Vec<(Glyph, f32)>> = HashMap::new();

        for run in buffer.layout_runs() {
            let line_y = y + run.line_y;

            for glyph in run.glyphs.iter() {
                let physical = glyph.physical((x, line_y), 1.0);
                let font_size = f32::from_bits(physical.cache_key.font_size_bits);

                let vello_glyph = Glyph {
                    id: physical.cache_key.glyph_id as u32,
                    x: physical.x as f32,
                    y: physical.y as f32,
                };

                font_glyphs
                    .entry(glyph.font_id)
                    .or_default()
                    .push((vello_glyph, font_size));
            }
        }

        // Render each font's glyphs in a batch
        for (font_id, glyphs) in font_glyphs {
            if let Some(vello_font) = self.font_cache.get_or_insert(font_id, font_system) {
                // Assuming uniform font size within a run (common case)
                let font_size = glyphs.first().map(|(_, s)| *s).unwrap_or(16.0);
                let glyph_iter = glyphs.into_iter().map(|(g, _)| g);

                self.scene
                    .draw_glyphs(vello_font)
                    .font_size(font_size)
                    .brush(&brush)
                    .draw(StyleRef::Fill(peniko::Fill::NonZero), glyph_iter);
            }
        }
    }

    /// Draw an SVG asset
    pub fn draw_svg(
        &mut self,
        tree: &usvg::Tree,
        boundary: Rect<f32>,
        tint_color: Option<ColorRgba>,
    ) {
        let sx = boundary.width / tree.size().width();
        let sy = boundary.height / tree.size().height();

        // let transform = Affine::translate((boundary.x as f64, boundary.y as f64))
        // .then_scale_non_uniform(sx as f64, sy as f64);

        let transform = Affine::scale_non_uniform(sx as f64, sy as f64)
            .then_translate((boundary.x as f64, boundary.y as f64).into());

        // Use vello_svg to render the SVG
        // vello_svg::render_tree(&mut self.scene, tree, transform);
        let svg_scene = vello_svg::render_tree(tree);

        // Note: Tinting would require post-processing or modifying the SVG tree
        // For now, tint_color is not applied
        if let Some(tint) = tint_color {
            // For tinting, we use a layer with SrcIn blend mode
            // 1. Push a layer to isolate the SVG
            // 2. Draw the SVG
            // 3. Draw a rect with the tint color using SrcIn blend
            // 4. Pop the layer

            let clip_rect = vello::kurbo::Rect::new(
                boundary.x as f64,
                boundary.y as f64,
                (boundary.x + boundary.width) as f64,
                (boundary.y + boundary.height) as f64,
            );

            // Push a layer for compositing
            self.scene.push_layer(
                peniko::BlendMode::default(),
                1.0,
                Affine::IDENTITY,
                &clip_rect,
            );

            // Draw the SVG
            self.scene.append(&svg_scene, Some(transform));

            // Draw tint color with SourceIn blend mode
            // SourceIn: shows source (tint) only where destination (SVG) has alpha
            let tint_brush = Brush::Solid(convert_rgba_color(&tint));
            self.scene.push_layer(
                peniko::BlendMode::new(peniko::Mix::Normal, peniko::Compose::SrcIn),
                1.0,
                Affine::IDENTITY,
                &clip_rect,
            );
            self.scene.fill(
                VelloFill::NonZero,
                Affine::IDENTITY,
                &tint_brush,
                None,
                &clip_rect,
            );
            self.scene.pop_layer();

            // Pop the outer layer
            self.scene.pop_layer();
        } else {
            self.scene.append(&svg_scene, Some(transform));
        }
    }
}

impl Renderer for VelloRenderer {
    fn process_commands(
        &mut self,
        view: &View,
        state: &RenderState,
        fill_color: ColorRgb,
        fonts: &mut FontResources,
        text: &mut TextsResources,
        assets: &Assets,
    ) {
        let _g = profiler::scope_named("vello::render");

        let width = view.physical_size.width;
        let height = view.physical_size.height;

        self.resize(width, height);
        self.begin_frame();

        for command in state.commands() {
            match command {
                RenderCommand::Rect {
                    boundary,
                    fill,
                    border_radius,
                    border,
                    ..
                } => {
                    self.draw_rect(
                        *boundary,
                        fill.as_ref(),
                        border_radius.as_ref(),
                        border.as_ref(),
                    );
                }
                RenderCommand::Oval {
                    boundary,
                    fill,
                    border,
                    ..
                } => {
                    self.draw_oval(*boundary, fill.as_ref(), border.as_ref());
                }
                RenderCommand::Text {
                    x,
                    y,
                    text_id,
                    tint_color,
                    ..
                } => {
                    let color = tint_color
                        .map(|c| convert_rgba_color(&c))
                        .unwrap_or_else(|| Color::from_rgba8(0, 0, 0, 255));

                    text.get_mut(*text_id).with_buffer_mut(|buffer| {
                        let brush = Brush::Solid(color);

                        for run in buffer.layout_runs() {
                            let line_y = y + run.line_y.round();

                            // Group by font
                            let mut font_glyphs: HashMap<
                                cosmic_text::fontdb::ID,
                                Vec<(Glyph, f32)>,
                            > = HashMap::new();

                            for glyph in run.glyphs.iter() {
                                let physical = glyph.physical((*x, line_y), 1.0);
                                let font_size = f32::from_bits(physical.cache_key.font_size_bits);

                                // Use raw floating-point positions for smooth subpixel rendering
                                // This prevents jiggling with justified text during resize
                                let vello_glyph = Glyph {
                                    id: physical.cache_key.glyph_id as u32,
                                    x: x + glyph.x + glyph.x_offset,
                                    y: glyph.y - glyph.y_offset + line_y,
                                };

                                font_glyphs
                                    .entry(glyph.font_id)
                                    .or_default()
                                    .push((vello_glyph, font_size));
                            }

                            // Render glyphs for each font
                            for (font_id, glyphs) in font_glyphs {
                                if let Some(vello_font) = self
                                    .font_cache
                                    .get_or_insert(font_id, &mut fonts.font_system)
                                {
                                    let font_size = glyphs
                                        .first()
                                        .map(|(_, s)| *s)
                                        .unwrap_or((12.0 * view.scale_factor) as f32);
                                    let glyph_iter = glyphs.into_iter().map(|(g, _)| g);

                                    self.scene
                                        .draw_glyphs(vello_font)
                                        .font_size(font_size)
                                        .brush(&brush)
                                        .draw(StyleRef::Fill(peniko::Fill::NonZero), glyph_iter);
                                }
                            }
                        }
                    });
                }
                RenderCommand::PushClip { rect, shape, .. } => match shape {
                    ClipShape::Rect => {
                        self.scene.push_clip_layer(
                            Affine::IDENTITY,
                            &vello::kurbo::Rect::new(
                                rect.x as f64,
                                rect.y as f64,
                                (rect.x + rect.width) as f64,
                                (rect.y + rect.height) as f64,
                            ),
                        );
                    }
                    ClipShape::RoundedRect { border_radius } => self.scene.push_clip_layer(
                        Affine::IDENTITY,
                        &vello::kurbo::RoundedRect::new(
                            rect.x as f64,
                            rect.y as f64,
                            (rect.x + rect.width) as f64,
                            (rect.y + rect.height) as f64,
                            RoundedRectRadii {
                                top_left: border_radius.top_left as f64,
                                top_right: border_radius.top_right as f64,
                                bottom_right: border_radius.bottom_right as f64,
                                bottom_left: border_radius.bottom_left as f64,
                            },
                        ),
                    ),
                    ClipShape::Oval => {
                        let center = vello::kurbo::Point::new(
                            (rect.x + rect.width / 2.0) as f64,
                            (rect.y + rect.height / 2.0) as f64,
                        );
                        let radii = vello::kurbo::Vec2::new(
                            (rect.width / 2.0) as f64,
                            (rect.height / 2.0) as f64,
                        );

                        self.scene.push_clip_layer(
                            Affine::IDENTITY,
                            &vello::kurbo::Ellipse::new(center, radii, 0.0),
                        );
                    }
                },
                RenderCommand::PopClip => {
                    self.scene.pop_layer();
                }
                RenderCommand::Svg {
                    boundary,
                    asset_id,
                    tint_color,
                    ..
                } => {
                    if let Some(tree) = assets.get_svg_tree(asset_id) {
                        self.draw_svg(tree, *boundary, *tint_color);
                    } else {
                        log::warn!("SVG with ID = {} not found", asset_id);
                    }
                }
            }
        }

        self.end_frame(&fill_color);
    }
}

// Helper functions

fn convert_rgba_color(color: &ColorRgba) -> Color {
    Color::from_rgba8(
        (color.r * 255.) as u8,
        (color.g * 255.) as u8,
        (color.b * 255.) as u8,
        (color.a * 255.) as u8,
    )
}

fn convert_rgb_color(color: &ColorRgb) -> Color {
    Color::from_rgb8(
        (color.r * 255.) as u8,
        (color.g * 255.) as u8,
        (color.b * 255.) as u8,
    )
}

fn create_brush_from_fill(fill: &Fill, rect: Rect<f32>) -> Option<Brush> {
    match fill {
        Fill::None => None,
        Fill::Color(color) => Some(Brush::Solid(convert_rgba_color(color))),
        Fill::Gradient(gradient) => create_gradient_brush(gradient, rect),
    }
}

fn create_gradient_brush(gradient: &Gradient, rect: Rect<f32>) -> Option<Brush> {
    match gradient {
        Gradient::Linear(linear) => {
            let start_x = rect.x + linear.start.0 * rect.width;
            let start_y = rect.y + linear.start.1 * rect.height;
            let end_x = rect.x + linear.end.0 * rect.width;
            let end_y = rect.y + linear.end.1 * rect.height;

            let stops: Vec<peniko::ColorStop> = linear
                .stops
                .iter()
                .map(|stop| peniko::ColorStop {
                    offset: stop.offset,
                    color: convert_rgba_color(&stop.color).into(),
                })
                .collect();

            let grad = VelloGradient::new_linear(
                (start_x as f64, start_y as f64),
                (end_x as f64, end_y as f64),
            )
            .with_stops(stops.as_slice());

            Some(Brush::Gradient(grad))
        }
        Gradient::Radial(radial) => {
            let center_x = rect.x + radial.center.0 * rect.width;
            let center_y = rect.y + radial.center.1 * rect.height;
            let radius = radial.radius * rect.width.max(rect.height);

            let stops: Vec<peniko::ColorStop> = radial
                .stops
                .iter()
                .map(|stop| peniko::ColorStop {
                    offset: stop.offset,
                    color: convert_rgba_color(&stop.color).into(),
                })
                .collect();

            let grad = VelloGradient::new_radial((center_x, center_y), radius)
                .with_stops(stops.as_slice());

            Some(Brush::Gradient(grad))
        }
        Gradient::Sweep(sweep) => {
            let center_x = rect.x + sweep.center.0 * rect.width;
            let center_y = rect.y + sweep.center.1 * rect.height;

            let stops: Vec<peniko::ColorStop> = sweep
                .stops
                .iter()
                .map(|stop| peniko::ColorStop {
                    offset: stop.offset,
                    color: convert_rgba_color(&stop.color).into(),
                })
                .collect();

            let grad =
                VelloGradient::new_sweep((center_x, center_y), sweep.start_angle, sweep.end_angle)
                    .with_stops(stops.as_slice());

            Some(Brush::Gradient(grad))
        }
    }
}

fn get_border_params(border: &Border) -> (f32, ColorRgba) {
    let max_width = [
        border.top.as_ref().map(|s| s.width).unwrap_or(0.0),
        border.right.as_ref().map(|s| s.width).unwrap_or(0.0),
        border.bottom.as_ref().map(|s| s.width).unwrap_or(0.0),
        border.left.as_ref().map(|s| s.width).unwrap_or(0.0),
    ]
    .into_iter()
    .fold(0.0f32, f32::max);

    let color = border
        .top
        .as_ref()
        .or(border.right.as_ref())
        .or(border.bottom.as_ref())
        .or(border.left.as_ref())
        .map(|s| s.color)
        .unwrap_or(ColorRgba::TRANSPARENT);

    (max_width, color)
}
