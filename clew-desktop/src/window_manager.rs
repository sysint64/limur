use std::{collections::HashMap, sync::Arc, time::Instant};

use clew::{
    ColorRgb, EdgeInsets, PhysicalSize, Rect, View, ViewId,
    render::Renderer,
    state::UiState,
    text::{StringId, TextId, TextsResources},
};
use winit::platform::{wayland::ActiveEventLoopExtWayland, x11::ActiveEventLoopExtX11};

use crate::window::Window;

#[derive(Debug, Clone)]
pub struct WindowDescriptor {
    pub title: String,
    pub name: Option<String>,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub fill_color: ColorRgb,
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        Self {
            title: "Window".to_string(),
            name: None,
            width: 800,
            height: 600,
            resizable: true,
            fill_color: ColorRgb::from_hex(0x000000),
        }
    }
}

pub(crate) struct WindowState<'a, App, Event> {
    pub(crate) window: Box<dyn Window<App, Event>>,
    pub(crate) winit_window: Arc<winit::window::Window>,
    pub(crate) texts: TextsResources<'a>,
    pub(crate) strings: HashMap<StringId, TextId>,
    pub(crate) ui_state: UiState,
    pub(crate) renderer: Box<dyn Renderer>,
    pub(crate) fill_color: ColorRgb,
    pub(crate) delta_time_timer: Instant,
    pub(crate) last_ime_rect: Rect<f32>,
    pub(crate) ime_activated: bool,
    pub(crate) ime_reset_needed: bool,
}

pub struct WindowManager<'a, App, Event> {
    pub(crate) windows: HashMap<winit::window::WindowId, WindowState<'a, App, Event>>,
    event_loop: Option<*const winit::event_loop::ActiveEventLoop>,
    renderer_factory: fn(Arc<winit::window::Window>) -> Box<dyn Renderer>,
    // TODO(sysint64): Implement proper id manager
    next_view_id: usize,
}

impl<'a, App, Event> WindowManager<'a, App, Event> {
    pub fn new(renderer_factory: fn(Arc<winit::window::Window>) -> Box<dyn Renderer>) -> Self {
        Self {
            windows: HashMap::new(),
            event_loop: None,
            renderer_factory,
            next_view_id: 0,
        }
    }

    pub fn with_event_loop<F>(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        callback: F,
    ) where
        F: FnOnce(&mut WindowManager<App, Event>),
    {
        self.event_loop = Some(event_loop);
        callback(self);
        self.event_loop = None;
    }

    /// Create a new window with the given descriptor
    pub fn spawn_window<T: Window<App, Event> + 'static>(
        &mut self,
        mut window: T,
        descriptor: WindowDescriptor,
    ) {
        if let Some(event_loop) = self.event_loop {
            let event_loop = unsafe { &*event_loop };

            let mut attributes = winit::window::WindowAttributes::default()
                .with_title(descriptor.title)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    descriptor.width,
                    descriptor.height,
                ))
                .with_resizable(descriptor.resizable);

            if let Some(name) = descriptor.name {
                if event_loop.is_wayland() {
                    use winit::platform::wayland::WindowAttributesExtWayland;

                    attributes = attributes.with_name(&name, &name);
                } else if event_loop.is_x11() {
                    use winit::platform::x11::WindowAttributesExtX11;

                    attributes = attributes.with_name(&name, &name);
                }
            }

            match event_loop.create_window(attributes) {
                Ok(winit_window) => {
                    let winit_window = Arc::new(winit_window);
                    let id = winit_window.id();
                    let scale_factor = winit_window.scale_factor();
                    let inner_size = winit_window.inner_size();
                    let renderer = (self.renderer_factory)(winit_window.clone());
                    let mut ui_state = UiState::new(View {
                        id: ViewId(self.next_view_id),
                        physical_size: PhysicalSize::new(inner_size.width, inner_size.height),
                        safe_area: EdgeInsets::ZERO,
                        scale_factor,
                    });
                    self.next_view_id += 1;

                    window.on_init(ui_state.shortcuts_registry());

                    self.windows.insert(
                        id,
                        WindowState {
                            window: Box::new(window),
                            winit_window,
                            texts: TextsResources::new(),
                            strings: HashMap::new(),
                            ui_state,
                            renderer,
                            fill_color: descriptor.fill_color,
                            delta_time_timer: Instant::now(),
                            last_ime_rect: Rect::default(),
                            ime_activated: false,
                            ime_reset_needed: false,
                        },
                    );

                    log::debug!("Created window: {id:?}");
                }
                Err(e) => {
                    log::error!("Failed to create window: {e}");
                }
            }
        } else {
            log::error!("Event loop has not been set");
        }
    }

    pub(crate) fn get_mut_window(
        &mut self,
        id: winit::window::WindowId,
    ) -> Option<&mut WindowState<'a, App, Event>> {
        self.windows.get_mut(&id)
    }

    pub fn request_view_redraw(&self, id: ViewId) {
        for window in self.windows.values() {
            if window.ui_state.view.id == id {
                window.winit_window.request_redraw();
            }
        }
    }

    pub(crate) fn request_redraw(&self, id: winit::window::WindowId) {
        if let Some(window) = self.windows.get(&id) {
            window.winit_window.request_redraw();
        }
    }

    pub fn request_redraw_all(&self) {
        for window in self.windows.values() {
            window.winit_window.request_redraw();
        }
    }
}
