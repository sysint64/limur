# clew-tiny-skia

Software renderer backend for the [clew](https://github.com/sysint64/clew) UI framework using `tiny-skia` and `softbuffer`.

## Overview

This crate provides a CPU-based renderer that works without GPU acceleration. Useful for compatibility, debugging, screenshot testing, or environments where GPU access is limited.

> ⚠️ **Important** — This renderer is less developed and messier than `clew-vello` and is still in active development.

## Usage

```rust
use clew_tiny_skia::TinySkiaRenderer;

impl ApplicationDelegate<()> for MyApp {
    fn create_renderer(window: Arc<winit::window::Window>) -> Box<dyn Renderer> {
        Box::new(TinySkiaRenderer::new(
            window.clone(),
            window,
        ))
    }
}
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
