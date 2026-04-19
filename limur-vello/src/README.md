# clew-vello

GPU-accelerated renderer backend for the [clew](https://github.com/sysint64/clew) UI framework using `vello` and `wgpu`.

## Overview

This crate provides hardware-accelerated 2D rendering. Recommended for most desktop applications.

## Usage

```rust
use clew_vello::VelloRenderer;
use std::sync::Arc;

impl ApplicationDelegate<()> for MyApp {
    fn create_renderer(window: Arc<winit::window::Window>) -> Box<dyn Renderer> {
        Box::new(
            VelloRenderer::new(
                window.clone(),
                window.inner_size().width,
                window.inner_size().height,
            )
            .block_on(),
        )
    }
}
```

## Platform Notes

Uses Metal backend on macOS with `present_with_transaction` for smoother frame pacing.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
