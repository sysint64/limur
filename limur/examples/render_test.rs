use limur::prelude::*;
use limur::{self as ui, Lerp};
use limur_desktop::{
    app::{Application, ApplicationDelegate},
    window::Window,
    window_manager::{WindowDescriptor, WindowManager},
};
use limur_tiny_skia::TinySkiaRenderer;
use limur_vello::VelloRenderer;
use limur_wgpu::WgpuRenderer;
use pollster::FutureExt;

struct ExampleApplication;

impl ApplicationDelegate<()> for ExampleApplication {
    fn on_start(
        &mut self,
        window_manager: &mut WindowManager<Self, ()>,
        _: &mut ui::ShortcutsRegistry,
    ) where
        Self: std::marker::Sized,
    {
        window_manager.spawn_window(
            MainWindow,
            WindowDescriptor {
                title: "Sub-pixel font rendering".to_string(),
                name: Some("limur-example".to_string()),
                width: 1280,
                height: 1024,
                resizable: true,
                fill_color: Some(ui::ColorRgba::from_hex(0xFFCCCCCC)),
            },
        );
    }

    fn create_renderer(window: std::sync::Arc<winit::window::Window>) -> Box<dyn ui::Renderer> {
        let vello = false;
        let tiny_skia = false;

        if tiny_skia {
            return Box::new(TinySkiaRenderer::new(window.clone(), window.clone()));
        } else if vello {
            Box::new(
                VelloRenderer::new(
                    window.clone(),
                    window.inner_size().width,
                    window.inner_size().height,
                )
                .block_on(),
            )
        } else {
            Box::new(WgpuRenderer::new(window.clone()).block_on())
        }
    }
}

pub struct MainWindow;

fn decorated_box_row<F>(ctx: &mut ui::BuildContext, builder: F)
where
    F: Fn(ui::DecoratedBoxBuilder) -> ui::DecoratedBoxBuilder,
{
    ui::hstack().spacing(0.).build(ctx, |ctx| {
        ui::hstack().spacing(8.).build(ctx, |ctx| {
            builder(
                ui::decorated_box()
                    .width(64.)
                    .height(48.)
                    .color(ui::ColorRgba::from_hex(0xff338822)),
            )
            .build(ctx);

            builder(
                ui::decorated_box()
                    .width(64.)
                    .height(48.)
                    .shape(ui::BoxShape::Oval)
                    .color(ui::ColorRgba::from_hex(0xff338822)),
            )
            .build(ctx);

            builder(
                ui::decorated_box()
                    .width(64.)
                    .height(48.)
                    .border_radius(ui::BorderRadius::all(16.))
                    .color(ui::ColorRgba::from_hex(0xff338822)),
            )
            .build(ctx);

            builder(
                ui::decorated_box()
                    .width(64.)
                    .height(48.)
                    .border_radius(ui::BorderRadius::vertical(16., 0.))
                    .color(ui::ColorRgba::from_hex(0xff338822)),
            )
            .build(ctx);

            builder(
                ui::decorated_box()
                    .width(64.)
                    .height(48.)
                    .border_radius(ui::BorderRadius::vertical(0., 16.))
                    .color(ui::ColorRgba::from_hex(0xff338822)),
            )
            .build(ctx);

            builder(
                ui::decorated_box()
                    .width(64.)
                    .height(48.)
                    .border_radius(ui::BorderRadius::new(3., 8., 16., 0.))
                    .color(ui::ColorRgba::from_hex(0xff338822)),
            )
            .build(ctx);
        });

        builder(
            ui::decorated_box()
                .width(64.)
                .height(48.)
                .color(ui::ColorRgba::from_hex(0xff338822)),
        )
        .build(ctx);
        builder(
            ui::decorated_box()
                .width(64.)
                .height(48.)
                .color(ui::ColorRgba::from_hex(0xff338822)),
        )
        .build(ctx);
    });
}

impl Window<ExampleApplication, ()> for MainWindow {
    fn build(&mut self, _: &mut ExampleApplication, ctx: &mut ui::BuildContext) {
        let response = ui::scroll_area()
            .scroll_direction(ui::ScrollDirection::Both)
            .fill_max_size()
            .build(ctx, |ctx| {
                ui::layer().fill_max_size().build(ctx, |ctx| {
                ui::vstack()
                    .fill_max_width()
                    .spacing(24.)
                    .padding(ui::EdgeInsets::all(32.))
                    .build(ctx, |ctx| {
                        section_label(ctx, "Decorated Boxes");

                        ui::hstack().spacing(24.).build(ctx, |ctx| {
                            ui::vstack().spacing(12.).build(ctx, |ctx| {
                                decorated_box_row(ctx, |decorated_box| decorated_box);
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff000000),
                                        offset: ui::Vec2::new(0., 0.),
                                        blur_radius: 5.,
                                        spread_radius: 0.,
                                    })
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff000000),
                                        offset: ui::Vec2::new(0., 0.),
                                        blur_radius: 5.,
                                        spread_radius: 5.,
                                    })
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff000000),
                                        offset: ui::Vec2::new(5., 5.),
                                        blur_radius: 5.,
                                        spread_radius: 0.,
                                    })
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff00AAAA),
                                        offset: ui::Vec2::new(-10., -10.),
                                        blur_radius: 5.,
                                        spread_radius: 0.,
                                    })
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff00AAAA),
                                        offset: ui::Vec2::new(0., 0.),
                                        blur_radius: 0.,
                                        spread_radius: 5.,
                                    })
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff00AAAA),
                                        offset: ui::Vec2::new(-5., -5.),
                                        blur_radius: 0.,
                                        spread_radius: 5.,
                                    })
                                });

                                // Inner shadow ----------------------------------------------------------------
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_inner_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff000000),
                                        offset: ui::Vec2::new(0., 0.),
                                        blur_radius: 5.,
                                        spread_radius: 0.,
                                    })
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_inner_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff000000),
                                        offset: ui::Vec2::new(0., 0.),
                                        blur_radius: 5.,
                                        spread_radius: 5.,
                                    })
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_inner_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff000000),
                                        offset: ui::Vec2::new(5., 5.),
                                        blur_radius: 5.,
                                        spread_radius: 0.,
                                    })
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_inner_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff00AAAA),
                                        offset: ui::Vec2::new(-10., -10.),
                                        blur_radius: 5.,
                                        spread_radius: 0.,
                                    })
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_inner_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff00AAAA),
                                        offset: ui::Vec2::new(0., 0.),
                                        blur_radius: 0.,
                                        spread_radius: 5.,
                                    })
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_inner_shadow(ui::BoxShadow {
                                        color: ui::ColorRgba::from_hex(0xff00AAAA),
                                        offset: ui::Vec2::new(-5., -5.),
                                        blur_radius: 0.,
                                        spread_radius: 5.,
                                    })
                                });

                                // Border ----------------------------------------------------------------
                                // All sides, thin black
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::all(ui::BorderSide::new(
                                        1.,
                                        ui::ColorRgba::from_hex(0xff000000),
                                    )))
                                });
                                // All sides, thick colored
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::all(ui::BorderSide::new(
                                        4.,
                                        ui::ColorRgba::from_hex(0xff0055FF),
                                    )))
                                });
                                // Top only
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::top(ui::BorderSide::new(
                                        3.,
                                        ui::ColorRgba::from_hex(0xffFF3300),
                                    )))
                                });
                                // Bottom only
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::bottom(ui::BorderSide::new(
                                        3.,
                                        ui::ColorRgba::from_hex(0xff00AA00),
                                    )))
                                });
                                // Left only
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::left(ui::BorderSide::new(
                                        3.,
                                        ui::ColorRgba::from_hex(0xff8800FF),
                                    )))
                                });
                                // Right only
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::right(ui::BorderSide::new(
                                        3.,
                                        ui::ColorRgba::from_hex(0xffFF8800),
                                    )))
                                });
                                // Top + bottom
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::new(
                                        Some(ui::BorderSide::new(
                                            3.,
                                            ui::ColorRgba::from_hex(0xffFF3300),
                                        )),
                                        None,
                                        Some(ui::BorderSide::new(
                                            3.,
                                            ui::ColorRgba::from_hex(0xff0033FF),
                                        )),
                                        None,
                                    ))
                                });
                                // Left + right
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::new(
                                        None,
                                        Some(ui::BorderSide::new(
                                            3.,
                                            ui::ColorRgba::from_hex(0xffFF8800),
                                        )),
                                        None,
                                        Some(ui::BorderSide::new(
                                            3.,
                                            ui::ColorRgba::from_hex(0xff00AA88),
                                        )),
                                    ))
                                });
                                // Symmetric: different H/V widths and colors
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::symmetric(
                                        ui::BorderSide::new(
                                            4.,
                                            ui::ColorRgba::from_hex(0xffAA0088),
                                        ),
                                        ui::BorderSide::new(
                                            2.,
                                            ui::ColorRgba::from_hex(0xff00AACC),
                                        ),
                                    ))
                                });
                                // All four sides with distinct widths and colors
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::new(
                                        Some(ui::BorderSide::new(
                                            2.,
                                            ui::ColorRgba::from_hex(0xffFF0000),
                                        )),
                                        Some(ui::BorderSide::new(
                                            4.,
                                            ui::ColorRgba::from_hex(0xff00CC00),
                                        )),
                                        Some(ui::BorderSide::new(
                                            6.,
                                            ui::ColorRgba::from_hex(0xff0000FF),
                                        )),
                                        Some(ui::BorderSide::new(
                                            8.,
                                            ui::ColorRgba::from_hex(0xffFFFF00),
                                        )),
                                    ))
                                });
                                // Semi-transparent borders
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::all(ui::BorderSide::new(
                                        4.,
                                        ui::ColorRgba::from_hex(0x880000FF),
                                    )))
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::all(ui::BorderSide::new(
                                        4.,
                                        ui::ColorRgba::from_hex(0x44FFFFFF),
                                    )))
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.border(ui::Border::new(
                                        Some(ui::BorderSide::new(
                                            4.,
                                            ui::ColorRgba::from_hex(0xCCFF0000),
                                        )),
                                        Some(ui::BorderSide::new(
                                            4.,
                                            ui::ColorRgba::from_hex(0x8800FF00),
                                        )),
                                        Some(ui::BorderSide::new(
                                            4.,
                                            ui::ColorRgba::from_hex(0x440000FF),
                                        )),
                                        Some(ui::BorderSide::new(
                                            4.,
                                            ui::ColorRgba::from_hex(0x22FFFF00),
                                        )),
                                    ))
                                });
                            });

                            ui::vstack().spacing(12.).build(ctx, |ctx| {
                                    ui::hstack().spacing(0.).fill_max_width().build(ctx, |ctx| {
                                        let start: ui::ColorOkLab =
                                            ui::ColorRgb::from_hex(0xFF0000).into();
                                        let end: ui::ColorOkLab =
                                            ui::ColorRgb::from_hex(0x00FFFF).into();

                                        for i in 0..100 {
                                            let t = i as f64 / 100.0;
                                            let color = start.lerp(end, t);

                                            ui::decorated_box()
                                                .fill_max_width()
                                                .height(48.)
                                                .color(color)
                                                .build(ctx)
                                        }
                                    });

                                // sRGB grey trap: red -> cyan are exact complements,
                                // midpoint = (0.5, 0.5, 0.5) = pure grey in sRGB space
                                ui::decorated_box()
                                    .fill_max_width()
                                    .height(48.)
                                    .add_linear_gradient(ui::LinearGradient::horizontal((
                                        ui::ColorRgba::from_hex(0xffFF0000),
                                        ui::ColorRgba::from_hex(0xff00FFFF),
                                    )))
                                    .build(ctx);

                                // magenta -> green: another exact complement pair
                                ui::decorated_box()
                                    .fill_max_width()
                                    .height(48.)
                                    .add_linear_gradient(ui::LinearGradient::horizontal((
                                        ui::ColorRgba::from_hex(0xffFF00FF),
                                        ui::ColorRgba::from_hex(0xff00FF00),
                                    )))
                                    .build(ctx);

                                // OKLCH shortest-path hue wrap demo:
                                // green (OKLCH hue ≈ +142°)  ->  cyan (OKLCH hue ≈ -165°)
                                // Short path = ~53° arc, stays in green -> cyan.
                                // Without wrap fix: dh = -5.36 rad  ->  detours blue -> purple -> red -> orange -> yellow (307° arc).
                                ui::decorated_box()
                                    .fill_max_width()
                                    .height(48.)
                                    .add_linear_gradient(ui::LinearGradient::horizontal((
                                        ui::ColorRgba::from_hex(0xff00FF00), // green,  OKLCH hue ≈ +142°
                                        ui::ColorRgba::from_hex(0xff00FFFF), // cyan,   OKLCH hue ≈ -165°
                                    )))
                                    .build(ctx);

                                // Linear gradients --------------------------------------------------------
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_linear_gradient(ui::LinearGradient::vertical(
                                        (
                                            ui::ColorRgba::from_hex(0xff3366FF),
                                            ui::ColorRgba::from_hex(0xffFF6633),
                                        ),
                                    ))
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_linear_gradient(
                                        ui::LinearGradient::horizontal((
                                            ui::ColorRgba::from_hex(0xff22BBAA),
                                            ui::ColorRgba::from_hex(0xffAA22BB),
                                        )),
                                    )
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_linear_gradient(ui::LinearGradient::angled(
                                        std::f32::consts::FRAC_PI_4,
                                        (
                                            ui::ColorRgba::from_hex(0xffFFCC00),
                                            ui::ColorRgba::from_hex(0xffFF3399),
                                        ),
                                    ))
                                });
                                // Linear with custom stops (uneven spacing)
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_linear_gradient(ui::LinearGradient::new(
                                        (0.0, 0.5),
                                        (1.0, 0.5),
                                        vec![
                                            ui::ColorStop::new(
                                                0.0,
                                                ui::ColorRgba::from_hex(0xffFF2200),
                                            ),
                                            ui::ColorStop::new(
                                                0.2,
                                                ui::ColorRgba::from_hex(0xffFFAA00),
                                            ),
                                            ui::ColorStop::new(
                                                0.5,
                                                ui::ColorRgba::from_hex(0xffFFFF00),
                                            ),
                                            ui::ColorStop::new(
                                                0.8,
                                                ui::ColorRgba::from_hex(0xff00CC44),
                                            ),
                                            ui::ColorStop::new(
                                                1.0,
                                                ui::ColorRgba::from_hex(0xff0044FF),
                                            ),
                                        ],
                                    ))
                                });

                                // Radial gradients --------------------------------------------------------
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_radial_gradient(ui::RadialGradient::circle(
                                        vec![
                                            ui::ColorRgba::from_hex(0xffFFFFAA),
                                            ui::ColorRgba::from_hex(0xff884400),
                                        ],
                                    ))
                                });
                                // Radial with custom center + stops
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_radial_gradient(ui::RadialGradient {
                                        center: (0.3, 0.3),
                                        radius: 0.7,
                                        focal: None,
                                        focal_radius: None,
                                        stops: vec![
                                            ui::ColorStop::new(
                                                0.0,
                                                ui::ColorRgba::from_hex(0xffFFFFFF),
                                            ),
                                            ui::ColorStop::new(
                                                0.4,
                                                ui::ColorRgba::from_hex(0xff8800FF),
                                            ),
                                            ui::ColorStop::new(
                                                1.0,
                                                ui::ColorRgba::from_hex(0xff110022),
                                            ),
                                        ],
                                        tile_mode: ui::TileMode::Clamp,
                                    })
                                });

                                // Sweep gradients ---------------------------------------------------------
                                // Shifted center (top-left quadrant)
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_gradient(ui::Gradient::Sweep(
                                        ui::SweepGradient::new(
                                            (0.2, 0.2),
                                            0.0,
                                            std::f32::consts::TAU,
                                            vec![
                                                ui::ColorStop::new(
                                                    0.0,
                                                    ui::ColorRgba::from_hex(0xffFF0000),
                                                ),
                                                ui::ColorStop::new(
                                                    0.33,
                                                    ui::ColorRgba::from_hex(0xff00FF00),
                                                ),
                                                ui::ColorStop::new(
                                                    0.66,
                                                    ui::ColorRgba::from_hex(0xff0000FF),
                                                ),
                                                ui::ColorStop::new(
                                                    1.0,
                                                    ui::ColorRgba::from_hex(0xffFF0000),
                                                ),
                                            ],
                                        ),
                                    ))
                                });
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_gradient(ui::Gradient::Sweep(
                                        ui::SweepGradient::full(vec![
                                            ui::ColorRgba::from_hex(0xffFF0000),
                                            ui::ColorRgba::from_hex(0xffFFFF00),
                                            ui::ColorRgba::from_hex(0xff00FF00),
                                            ui::ColorRgba::from_hex(0xff00FFFF),
                                            ui::ColorRgba::from_hex(0xff0000FF),
                                            ui::ColorRgba::from_hex(0xffFF00FF),
                                            ui::ColorRgba::from_hex(0xffFF0000),
                                        ]),
                                    ))
                                });
                                // Sweep with custom stops (partial arc, uneven)
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box.add_gradient(ui::Gradient::Sweep(
                                        ui::SweepGradient::new(
                                            (0.5, 0.5),
                                            0.0,
                                            std::f32::consts::PI,
                                            vec![
                                                ui::ColorStop::new(
                                                    0.0,
                                                    ui::ColorRgba::from_hex(0xffFFAA00),
                                                ),
                                                ui::ColorStop::new(
                                                    0.5,
                                                    ui::ColorRgba::from_hex(0xffFF4400),
                                                ),
                                                ui::ColorStop::new(
                                                    1.0,
                                                    ui::ColorRgba::from_hex(0xff880000),
                                                ),
                                            ],
                                        ),
                                    ))
                                });

                                // Border + gradient -------------------------------------------------------
                                // Thin uniform border over vertical linear gradient
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box
                                        .add_linear_gradient(ui::LinearGradient::vertical((
                                            ui::ColorRgba::from_hex(0xff3366FF),
                                            ui::ColorRgba::from_hex(0xffFF6633),
                                        )))
                                        .border(ui::Border::all(ui::BorderSide::new(
                                            2.,
                                            ui::ColorRgba::from_hex(0xff000000),
                                        )))
                                });
                                // Thick colored border over horizontal linear gradient
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box
                                        .add_linear_gradient(ui::LinearGradient::horizontal((
                                            ui::ColorRgba::from_hex(0xff22BBAA),
                                            ui::ColorRgba::from_hex(0xffAA22BB),
                                        )))
                                        .border(ui::Border::all(ui::BorderSide::new(
                                            5.,
                                            ui::ColorRgba::from_hex(0xffFFFFFF),
                                        )))
                                });
                                // Per-side colored borders over angled linear gradient (tests corner AA)
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box
                                        .add_linear_gradient(ui::LinearGradient::angled(
                                            std::f32::consts::FRAC_PI_4,
                                            (
                                                ui::ColorRgba::from_hex(0xffFFCC00),
                                                ui::ColorRgba::from_hex(0xffFF3399),
                                            ),
                                        ))
                                        .border(ui::Border::new(
                                            Some(ui::BorderSide::new(
                                                3.,
                                                ui::ColorRgba::from_hex(0xffFF0000),
                                            )),
                                            Some(ui::BorderSide::new(
                                                5.,
                                                ui::ColorRgba::from_hex(0xff00CC00),
                                            )),
                                            Some(ui::BorderSide::new(
                                                3.,
                                                ui::ColorRgba::from_hex(0xff0000FF),
                                            )),
                                            Some(ui::BorderSide::new(
                                                5.,
                                                ui::ColorRgba::from_hex(0xffFFAA00),
                                            )),
                                        ))
                                });
                                // Top+bottom border over radial gradient
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box
                                        .add_radial_gradient(ui::RadialGradient::circle(vec![
                                            ui::ColorRgba::from_hex(0xffFFFFAA),
                                            ui::ColorRgba::from_hex(0xff884400),
                                        ]))
                                        .border(ui::Border::new(
                                            Some(ui::BorderSide::new(
                                                4.,
                                                ui::ColorRgba::from_hex(0xffFF3300),
                                            )),
                                            None,
                                            Some(ui::BorderSide::new(
                                                4.,
                                                ui::ColorRgba::from_hex(0xff0033FF),
                                            )),
                                            None,
                                        ))
                                });
                                // Thick border over sweep gradient (rainbow fill, white frame)
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box
                                        .add_gradient(ui::Gradient::Sweep(
                                            ui::SweepGradient::full(vec![
                                                ui::ColorRgba::from_hex(0xffFF0000),
                                                ui::ColorRgba::from_hex(0xffFFFF00),
                                                ui::ColorRgba::from_hex(0xff00FF00),
                                                ui::ColorRgba::from_hex(0xff00FFFF),
                                                ui::ColorRgba::from_hex(0xff0000FF),
                                                ui::ColorRgba::from_hex(0xffFF00FF),
                                                ui::ColorRgba::from_hex(0xffFF0000),
                                            ]),
                                        ))
                                        .border(ui::Border::all(ui::BorderSide::new(
                                            6.,
                                            ui::ColorRgba::from_hex(0xffFFFFFF),
                                        )))
                                });
                                // Semi-transparent border over vertical gradient
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box
                                        .add_linear_gradient(ui::LinearGradient::vertical((
                                            ui::ColorRgba::from_hex(0xff3366FF),
                                            ui::ColorRgba::from_hex(0xffFF6633),
                                        )))
                                        .border(ui::Border::all(ui::BorderSide::new(
                                            4.,
                                            ui::ColorRgba::from_hex(0x88000000),
                                        )))
                                });
                                // Semi-transparent white border over horizontal gradient
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box
                                        .add_linear_gradient(ui::LinearGradient::horizontal((
                                            ui::ColorRgba::from_hex(0xff22BBAA),
                                            ui::ColorRgba::from_hex(0xffAA22BB),
                                        )))
                                        .border(ui::Border::all(ui::BorderSide::new(
                                            5.,
                                            ui::ColorRgba::from_hex(0x66FFFFFF),
                                        )))
                                });
                                // Per-side varying alpha over radial gradient
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box
                                        .add_radial_gradient(ui::RadialGradient::circle(vec![
                                            ui::ColorRgba::from_hex(0xffFFFFAA),
                                            ui::ColorRgba::from_hex(0xff884400),
                                        ]))
                                        .border(ui::Border::new(
                                            Some(ui::BorderSide::new(
                                                4.,
                                                ui::ColorRgba::from_hex(0xCCFF0000),
                                            )),
                                            Some(ui::BorderSide::new(
                                                4.,
                                                ui::ColorRgba::from_hex(0x8800FF00),
                                            )),
                                            Some(ui::BorderSide::new(
                                                4.,
                                                ui::ColorRgba::from_hex(0x440000FF),
                                            )),
                                            Some(ui::BorderSide::new(
                                                4.,
                                                ui::ColorRgba::from_hex(0x22000000),
                                            )),
                                        ))
                                });
                                // Semi-transparent border over sweep gradient
                                decorated_box_row(ctx, |decorated_box| {
                                    decorated_box
                                        .add_gradient(ui::Gradient::Sweep(
                                            ui::SweepGradient::full(vec![
                                                ui::ColorRgba::from_hex(0xffFF0000),
                                                ui::ColorRgba::from_hex(0xffFFFF00),
                                                ui::ColorRgba::from_hex(0xff00FF00),
                                                ui::ColorRgba::from_hex(0xff00FFFF),
                                                ui::ColorRgba::from_hex(0xff0000FF),
                                                ui::ColorRgba::from_hex(0xffFF00FF),
                                                ui::ColorRgba::from_hex(0xffFF0000),
                                            ]),
                                        ))
                                        .border(ui::Border::all(ui::BorderSide::new(
                                            6.,
                                            ui::ColorRgba::from_hex(0x88000000),
                                        )))
                                });
                            });
                        });

                        section_label(ctx, "Text");

                        ui::text("Hello world! 👋\nThis is rendered with 🦅 glyphon 🦁\nThe text below should be partially clipped.\na b c d e f g h i j k l m n o p q r s t u v w x y z")
                            .color(ui::ColorRgb::from_hex(0x000000))
                            .font_size(24.0)
                            .build(ctx);

                        section_label(
                            ctx,
                            "Text size comparison (subpixel AA matters most at small sizes)",
                        );

                        ui::vstack()
                            .spacing(4.)
                            .padding(ui::EdgeInsets::all(16.))
                            .background(
                                ui::decoration()
                                    .color(ui::ColorRgb::from_hex(0x313244).with_alpha(1.0))
                                    .border_radius(ui::BorderRadius::all(8.))
                                    .build(ctx),
                            )
                            .build(ctx, |ctx| {
                                sized_text(
                                    ctx,
                                    10.,
                                    "10px: The quick brown fox jumps (subpixel helps most here)",
                                );
                                sized_text(
                                    ctx,
                                    12.,
                                    "12px: The quick brown fox jumps over the lazy dog",
                                );
                                sized_text(ctx, 14., "14px: The quick brown fox jumps over");
                                sized_text(ctx, 18., "18px: The quick brown fox");
                                sized_text(ctx, 24., "24px: Quick brown fox");
                            });

                        section_label(ctx, "Interactive - hover to change bg under subpixel text");

                        ui::hstack().spacing(8.).build(ctx, |ctx| {
                            for label in &["Hover me", "And me", "Me too", "Also me"] {
                                hover_button(ctx, label);
                            }
                        });

                        section_label(ctx, "Long Text");

                        ui::text(include_str!("assets/latin_text.txt"))
                            .fill_max_width()
                            .color(ui::ColorRgb::from_hex(0x000000))
                            .font_size(24.0)
                            .build(ctx);

                        // ui::decorated_box().color(ui::ColorRgb::from_hex(0x000000)).fill_max_width().height(48.).build(ctx);
                    });
            });
        });

        if response.overflow_x {
            ctx.provide(response.clone(), |ctx| {
                limur_widgets::horizontal_scroll_bar().build(ctx);
            });
        }

        if response.overflow_y {
            ctx.provide(response.clone(), |ctx| {
                limur_widgets::vertical_scroll_bar().build(ctx);
            });
        }

        ui::backdrop_filter(ui::ShaderId::FrostedGlass)
            .param(0, ui::ShaderParam::Float(10.))
            .param(
                1,
                ui::ShaderParam::Color(ui::ColorRgba::from_hex(0xFF0000FF).with_opacity(0.2)),
            )
            .offset(256., 256.)
            .width(300.)
            .height(200.)
            .build(ctx);

        ui::backdrop_filter(ui::ShaderId::FrostedGlass)
            .param(0, ui::ShaderParam::Float(80.))
            .param(
                1,
                ui::ShaderParam::Color(ui::ColorRgba::from_hex(0x00000000)),
            )
            .offset(600., 600.)
            .width(300.)
            .height(200.)
            .clip(ui::Clip::Oval)
            .build(ctx);

        // ui::backdrop_filter(ui::ShaderId::FrostedGlass)
        //     .param(0, ui::ShaderParam::Float(10.))
        //     .param(
        //         1,
        //         ui::ShaderParam::Color(ui::ColorRgba::from_hex(0xFF00FF00).with_opacity(0.5)),
        //     )
        //     .offset(400., 300.)
        //     .width(300.)
        //     .height(200.)
        //     .clip(ui::Clip::Oval)
        //     .build(ctx);

        // ui::backdrop_filter(ui::ShaderId::FrostedGlass)
        //     .param(0, ui::ShaderParam::Float(10.))
        //     .param(
        //         1,
        //         ui::ShaderParam::Color(ui::ColorRgba::from_hex(0xFF00FF00).with_opacity(0.5)),
        //     )
        //     .offset(400., 300.)
        //     .width(300.)
        //     .height(200.)
        //     .clip(ui::Clip::Oval)
        //     .build(ctx);

        // ui::backdrop_filter(ui::ShaderId::FrostedGlass)
        //     .param(0, ui::ShaderParam::Float(15.))
        //     .param(
        //         1,
        //         ui::ShaderParam::Color(ui::ColorRgba::from_hex(0xFFFFFFFF).with_opacity(0.5)),
        //     )
        //     .offset(700., 700.)
        //     .width(300.)
        //     .height(200.)
        //     .clip(ui::Clip::Oval)
        //     .build(ctx);

        // Liquid glass — default refraction params, slight blur, subtle tint
        ui::backdrop_filter(ui::ShaderId::LiquidGlass)
            .param(0, ui::ShaderParam::Float(12.))   // blur_radius
            .param(1, ui::ShaderParam::Color(        // tint
                ui::ColorRgba::from_hex(0xFFFFFFFF).with_opacity(0.08),
            ))
            .param(2, ui::ShaderParam::Float(3.0))   // power_factor (squircle)
            .param(3, ui::ShaderParam::Float(1.0))   // f_power
            .param(4, ui::ShaderParam::Float(0.06))  // noise
            .param(5, ui::ShaderParam::Float(0.25))  // glow_weight
            .param(6, ui::ShaderParam::Float(0.7))   // a
            .param(7, ui::ShaderParam::Float(2.3))   // b
            .param(8, ui::ShaderParam::Float(5.2))   // c
            .param(9, ui::ShaderParam::Float(6.9))   // d
            .offset(970., 256.)
            .width(280.)
            .height(200.)
            .build(ctx);

        // Liquid glass — stronger refraction, no blur
        ui::backdrop_filter(ui::ShaderId::LiquidGlass)
            .param(0, ui::ShaderParam::Float(0.))    // no blur
            .param(1, ui::ShaderParam::Color(        // tint
                ui::ColorRgba::from_hex(0xFFFFFFFF).with_opacity(0.12),
            ))
            .param(2, ui::ShaderParam::Float(4.0))   // power_factor (more square)
            .param(3, ui::ShaderParam::Float(2.0))   // f_power (stronger refraction)
            .param(4, ui::ShaderParam::Float(0.04))  // noise
            .param(5, ui::ShaderParam::Float(0.4))   // glow_weight
            .param(6, ui::ShaderParam::Float(0.5))   // a
            .param(7, ui::ShaderParam::Float(3.0))   // b
            .param(8, ui::ShaderParam::Float(5.0))   // c
            .param(9, ui::ShaderParam::Float(8.0))   // d
            .offset(970., 490.)
            .width(280.)
            .height(200.)
            .build(ctx);

        ui::profiler_overlay(ctx);
    }
}

fn section_label(ctx: &mut ui::BuildContext, label: &str) {
    ui::text(label)
        .font_size(14.)
        .color(ui::ColorRgba::from_hex(0xFF000000))
        .build(ctx);
}

fn text_on_solid(ctx: &mut ui::BuildContext, label: &str, bg: u32, fg: u32) {
    ui::text(label)
        .font_size(16.)
        .color(ui::ColorRgb::from_hex(fg))
        .text_align(ui::TextAlign::Center)
        .text_vertical_align(ui::AlignY::Center)
        .padding(ui::EdgeInsets::symmetric(32., 16.))
        .background(
            ui::decoration()
                .color(ui::ColorRgb::from_hex(bg))
                .border_radius(ui::BorderRadius::all(8.))
                .build(ctx),
        )
        .build(ctx);
}

fn sized_text(ctx: &mut ui::BuildContext, size: f32, label: &str) {
    ui::text(label)
        .font_size(size)
        .color(ui::ColorRgba::from_hex(0xCDD6F4FF))
        .build(ctx);
}

fn hover_button(ctx: &mut ui::BuildContext, label: &str) {
    ui::gesture_detector().clickable(true).build(ctx, |ctx| {
        let response = ctx.of::<ui::GestureDetectorResponse>().unwrap().clone();

        let bg_color = if response.is_hot() {
            0xFF475aFF
        } else {
            0xFF3244FF
        };

        ui::text(label)
            .font_size(16.)
            .color(ui::ColorRgba::from_hex(0xCDD6F4FF))
            .text_align(ui::TextAlign::Center)
            .text_vertical_align(ui::AlignY::Center)
            .padding(ui::EdgeInsets::symmetric(24., 12.))
            .background(
                ui::decoration()
                    .color(ui::ColorRgba::from_hex(bg_color))
                    .border_radius(ui::BorderRadius::vertical(8., 0.))
                    .add_shadow(ui::BoxShadow {
                        color: ui::ColorRgba::from_hex(0x1400FF00),
                        offset: ui::Vec2::new(0., 5.),
                        blur_radius: 0.,
                        spread_radius: 5.,
                    })
                    .build(ctx),
            )
            .build(ctx);
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .init();

    log::info!("Starting Subpixel AA Layer Test");
    Application::run_application(ExampleApplication)?;

    Ok(())
}
