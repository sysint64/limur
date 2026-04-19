# clew

<img src="https://raw.githubusercontent.com/sysint64/clew/refs/heads/main/images/logo.svg" alt="Logo" width="150">

A composable UI framework for Rust with an immediate API. Inspired by Flutter, SwiftUI, and Jetpack Compose.

> ⚠️ **Early Development** — This project is in early development and cannot even be considered alpha yet. The API will change. Use at your own risk.
>
> This started as an experiment and still is and I'm primarily building it for my own applications and personal needs. I'm open-sourcing it in case others find it interesting or useful, but keep in mind it's a personal project first.

![Demo](https://raw.githubusercontent.com/sysint64/clew/refs/heads/main/images/demo.png)

## What is this?

Clew is a desktop-focused UI framework built around a declarative builder API. The main goal is to make it easy to create highly custom widgets while keeping the API clean and composable.

It uses an immediate mode approach — UI is built entirely from scratch whenever it needs to change, whether that's a layout update, animation, or simple highlighting.

The framework intentionally doesn't include common widgets like buttons or scroll bars. It's designed so that you implement those yourself with whatever appearance you want. That said, there's a `clew-widgets` crate — an opinionated set of commonly used widgets for when you don't need a custom look and just want some quick UI.

## Why yet another desktop framework

First of all — it's fun to work on such a project, second — the framework's primary goal is to bring modern mobile frameworks developer experience to Rust and focus primarily on Desktop platforms and try to implement desktop-specific features well ignoring all other platforms, e.g. this framework is not trying to support Android or iOS, even though it's technically possible. In addition to this, I decided to experiment with immediate API and it turned out as a nice way to shape the API for Rust. It has some retained mode features, but it's mostly immediate mode UI. The majority of UI frameworks in the Rust ecosystem are retained mode. There is an amazing immediate UI framework — egui — but they have a bit different goal: it's more lightweight but less customizable and has a different API philosophy.

## Tech-Stack

The framework uses `Vello` and `tiny-skia` as renderers. If you want you may implement your own favorite renderer — look at `clew-vello` and `clew-tiny-skia` to see how this can be achieved. As a text stack the framework uses `cosmic-text`, for platform integrations and window management — `winit`.

## Prerequisites

So far the framework was tested on MacOS only on Rust 1.92.0.

## Why "Clew"?

The name **Clew** draws from ancient Greek mythology and the surprising origin of a familiar English word.

In the myth of Theseus and the Minotaur, Princess Ariadne gives Theseus a *clew*—an old spelling of "clue," meaning a ball of thread. He uses it to navigate the twisting labyrinth, marking his path in and following it back out after slaying the monster.

This simple thread becomes the key to unraveling complexity, turning a confusing maze into a solvable journey.

A great user interface does the same: it provides the "thread" that guides users through intricate applications, helping them find what they need without getting lost. Clew aims to be that intuitive guide — lightweight, clear, and empowering developers to build experiences that feel effortless to navigate.

## Counter Example

```rust
impl Window<CounterApplication, ()> for MainWindow {
    fn build(&mut self, _: &mut CounterApplication, ctx: &mut ui::BuildContext) {
        ui::vstack()
            .spacing(12.)
            .cross_axis_alignment(ui::CrossAxisAlignment::Center)
            .build(ctx, |ctx| {
                ui::text(
                    &bumpalo::format!(in &ctx.phase_allocator, "Counter: {}", self.counter),
                )
                .build(ctx);

                ui::hstack().build(ctx, |ctx| {
                    if clew_widgets::button("+").build(ctx).clicked() {
                        self.counter += 1;
                    }

                    if clew_widgets::button("-").build(ctx).clicked() {
                        self.counter -= 1;
                    }
                });
            });
    }
}
```

<img src="https://raw.githubusercontent.com/sysint64/clew/refs/heads/main/images/counter.png" alt="Counter" width="347">

## Virtual List Example

Example on how to render 10 billions of rows.

```rust
ui::zstack()
    .fill_max_size()
    .margin(ui::EdgeInsets::symmetric(16., 8.))
    .build(ctx, |ctx| {
        let response = ui::virtual_list()
            .fill_max_size()
            .background(
                ui::decoration()
                    .color(ui::ColorRgba::from_hex(0xFFFF0000).with_opacity(0.2))
                    .border_radius(ui::BorderRadius::all(16.))
                    .build(ctx),
            )
            .items_count(10_000_000_000)
            .item_size(32.)
            .build(ctx, |ctx, index| {
                ui::text(
                    // Optionally - you can use bumpalo (there is not feature flag yet)
                    &bumpalo::format!(in &ctx.phase_allocator, "Item {}", index),
                )
                .padding(ui::EdgeInsets::symmetric(16., 0.))
                .height(32.)
                .fill_max_width()
                .build(ctx);
            });

        if response.overflow_y {
            // Push response to descendants
            ctx.provide(response.clone(), |ctx| {
                // Custom scroll bar implemented in clew-widgets
                ui::widget::<clew_widgets::VerticalScrollBar>().build(ctx);
            });
        }
    });
```

## Custom Stateful Widget with Events Example

You can create custom stateful widgets and not worrying about state ownership and lifetimes. However you can optionally store widget's state separately and provide a mutable reference to the state.

In addition to the simple immediate UI approach, the framework also supports event-based architecture and communication between widgets out of the box.

```rust
#[derive(Default, WidgetState)]
pub struct CounterWidget {
    counter: i32,
}

pub enum CounterEvent {
    Increment,
    Decrement,
}

impl ui::Widget for CounterWidget {
    type Event = CounterEvent;

    fn on_event(&mut self, event: &Self::Event) -> bool {
        match event {
            CounterEvent::Increment => self.counter += 1,
            CounterEvent::Decrement => self.counter -= 1,
        }

        true
    }

    fn build(&mut self, ctx: &mut ui::BuildContext) {
        ui::zstack()
            .fill_max_size()
            .align_x(ui::AlignX::Center)
            .align_y(ui::AlignY::Center)
            .build(ctx, |ctx| {
                ui::vstack()
                    .spacing(12.)
                    .cross_axis_alignment(ui::CrossAxisAlignment::Center)
                    .build(ctx, |ctx| {
                        ui::text(
                            &bumpalo::format!(in &ctx.phase_allocator, "Counter: {}", self.counter),
                        )
                        .build(ctx);

                        ui::hstack().build(ctx, |ctx| {
                            if clew_widgets::button("+").build(ctx).clicked() {
                                ctx.emit(CounterEvent::Increment);
                            }

                            if clew_widgets::button("-").build(ctx).clicked() {
                                ctx.emit(CounterEvent::Decrement);
                            }
                        });
                    });
            });
    }
}
```

**Usage:**

```rust
fn build(&mut self, _: &mut CounterApplication, ctx: &mut ui::BuildContext) {
    ui::vstack().build(ctx, |ctx| {
        // Store state in the framework's storage
        ui::widget::<CounterWidget>().build(ctx);

        // Maintain widget state yourself
        ui::widget().state(&mut self.counter).build(ctx);
    });
}
```

## Async Example

The framework also supports async. Currently it's hardcoded to the tokio runtime, but in the future it will support custom runtimes as well.

```rust
if clew_widgets::button("+").build(ctx).clicked() {
    ctx.spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;

        CounterEvent::Increment
    });
}
```

## Multi-Window Example

```rust
fn on_start(&mut self, window_manager: &mut WindowManager<DemoApplication, CounterEvent>) {
    window_manager.spawn_window(
        MainWindow::new(),
        WindowDescriptor {
            title: "Main Window".to_string(),
            width: 800,
            height: 600,
            resizable: true,
            fill_color: ColorRgb::from_hex(0x121212),
        },
    );

    window_manager.spawn_window(
        SettingsWindow::new(),
        WindowDescriptor {
            title: "Settings".to_string(),
            width: 400,
            height: 300,
            resizable: true,
            fill_color: ColorRgb::from_hex(0x121212),
        },
    );
}
```

## Broadcast Events

To open windows you need to send an event to the application level, since the application owns the window manager. You can use broadcast for this — an event that's visible to all components, windows, and the application:

```rust
if clew_widgets::button("Open Settings").build(ctx).clicked() {
    ctx.broadcast(ApplicationEvent::OpenSettings);
}
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
