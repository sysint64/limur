# Limur

A composable UI framework for Rust with an immediate API.

> ⚠️ **Early Development** - This project is in early development and cannot even be considered alpha yet. The API will change. Use at your own risk.

Limur (**L**ayered **I**mmediate **M**ode **U**I for **R**ust) is a UI framework built around widget composition. Instead of shipping a fixed set of styled controls, Limur gives you layout and rendering primitives so you can build your own design system from scratch.

The API is immediate-mode - you describe your UI as plain function calls, so state synchronization is straightforward and you never fight the borrow checker over widget handles. Unlike traditional immediate-mode frameworks that rebuild every frame, Limur only re-renders when widget state actually changes. Under the hood it mixes in retained-mode techniques for performance, e.g. for large static subtrees, you can opt into [Layers](#layers) - a retained-mode optimization that caches widget trees and only rebuilds them when marked dirty.

**Design goals:**

- **Composition over configuration.** Build complex widgets by composing simple ones.
- **Immediate API, no stale state.** Correct widget metrics (sizes, positions) are available from frame 0 - no one-frame-delay surprises. The UI only rebuilds when state changes, not every frame.
- **Maximum customizability.** The core crate ships no buttons, scroll bars, or opinionated styling - it's designed for building rich, expressive UIs from the ground up. Limur provides the primitives - layout, rendering, hit testing, animations etc. - so you can build your own UI kit with widgets, theming, and design guidelines that make sense for your application. Whether it's a game editor, a custom HUD, or a full desktop app - you define the look and feel, Limur provides the building blocks.

## Quick Example

The classic counter - state lives on your struct, UI is just function calls:

```rust
impl Window<CounterApplication, ()> for MainWindow {
    fn build(&mut self, _: &mut CounterApplication, ctx: &mut ui::BuildContext) {
        ui::vstack()
            .spacing(12.)
            .cross_axis_alignment(ui::CrossAxisAlignment::Center)
            .build(ctx, |ctx| {
                ui::text(&format!("Counter: {}", self.counter)).build(ctx);

                ui::hstack().build(ctx, |ctx| {
                    if limur_widgets::button("+").build(ctx).clicked() {
                        self.counter += 1;
                    }

                    if limur_widgets::button("-").build(ctx).clicked() {
                        self.counter -= 1;
                    }
                });
            });
    }
}
```

<img src="./images/counter.png" alt="Counter example" width="347">

## Layers

Layers are an opt-in optimization for large, mostly-static subtrees. A layer retains its widget tree and only rebuilds when its state is marked dirty. This keeps the expressive immediate-mode API while avoiding unnecessary rebuilds.

```rust
for i in 0..4 {
    ui::hstack().fill_max_size().build(ctx, |ctx| {
        for j in 0..4 {
            ui::layer()
                .margin(ui::EdgeInsets::all(4.))
                .padding(ui::EdgeInsets::all(8.))
                .build(ctx, |ctx| {
                    // 1024 buttons per layer, only rebuilt when dirty
                    layer_body(ctx, i * 4 + j);
                });
        }
    });
}
```

## Crates

| Crate | Description |
|---|---|
| `limur` | Core framework |
| `limur-widgets` | Opinionated set of common widgets (buttons, scroll bars, etc.) for quick prototyping |
| `limur-desktop` | Desktop application shell - window management, event loop, application lifecycle |
| `limur-vello` | Renderer backend using [Vello](https://github.com/linebender/vello) |
| `limur-tiny-skia` | Renderer backend using [tiny-skia](https://github.com/nickel-org/tiny-skia) |

## Getting Started

### Prerequisites

Tested on macOS and Linux. Requires Rust 1.94.1 or later.

### Running the examples

```sh
cargo run --example counter
```

## Inspiration

Limur draws ideas from Flutter, SwiftUI, and Jetpack Compose.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
