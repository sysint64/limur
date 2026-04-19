# limur-widgets

Opinionated widget collection for the [limur](https://github.com/sysint64/limur) UI framework.

## Overview

Limur is designed for building custom widgets from scratch. This crate provides ready-to-use widgets for when you don't need highly customized look and just want some quick UI.

## Widgets

### Button

Simple text button with hover, active, and focus states.

```rust
if limur_widgets::button("Click me").build(ctx).clicked() {
    println!("Button clicked!");
}
```

### VerticalScrollBar / HorizontalScrollBar

Draggable scroll bars that integrate with limur's `ScrollAreaResponse`.

```rust
let response = ui::scroll_area().build(ctx, |ctx| {
    // content
});

if response.overflow_y {
    ctx.provide(response, |ctx| {
        widget::<limur_widgets::VerticalScrollBar>().build(ctx);
    });
}
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
