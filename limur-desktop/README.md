# clew-desktop

Desktop application runner for the [clew](https://github.com/sysint64/clew) UI framework.

Provides window management, input handling, and event loop integration using `winit`.

## Usage

```rust
use clew_desktop::{Application, ApplicationDelegate, WindowManager, WindowDescriptor};

struct MyApp;

impl ApplicationDelegate<()> for MyApp {
    fn on_start(&mut self, window_manager: &mut WindowManager<Self, ()>) {
        window_manager.spawn_window(
            MainWindow::new(),
            WindowDescriptor {
                title: "My App".to_string(),
                width: 800,
                height: 600,
                resizable: true,
                fill_color: ColorRgb::from_hex(0x121212),
            },
        );
    }

    fn create_renderer(window: Arc<winit::window::Window>) -> Box<dyn Renderer> {
        // Use clew-vello or clew-tiny-skia
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

fn main() -> anyhow::Result<()> {
    Application::run_application(MyApp)
}
```

## Platform Support

Currently tested on macOS only.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
