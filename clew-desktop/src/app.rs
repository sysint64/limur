use std::any::{Any, TypeId};
use std::sync::Arc;
use std::time::Instant;

use clew::assets::Assets;
use clew::editable_text::OsEvent;
use clew::interaction::handle_interaction_before_build;
use clew::io::{Cursor, TextInputAction};
use clew::keyboard::{KeyCode, KeyModifiers};
use clew::lifecycle::{finalize_cycle, init_cycle};
use clew::render::Renderer;
use clew::shortcuts::ShortcutsManager;
use clew::text::{FontResources, StringInterner};
use clew::widgets::builder::{ApplicationEvent, ApplicationEventLoopProxy, BuildContext};
use clew::{PhysicalSize, ShortcutsRegistry};

use crate::keyboard::{from_winit_key_code, from_winit_modifiers};
use crate::window_manager::WindowManager;
use crate::window_manager::WindowState;
#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;

pub trait ApplicationDelegate<Event> {
    fn init_assets(&mut self, _assets: &mut Assets) {}

    fn on_start(
        &mut self,
        window_manager: &mut WindowManager<Self, Event>,
        shortcuts_registry: &mut ShortcutsRegistry,
    ) where
        Self: std::marker::Sized;

    fn on_shortcut(&mut self, _shortcuts_manager: &ShortcutsManager) {}

    fn on_event(&mut self, _window_manager: &mut WindowManager<Self, Event>, _event: &Event)
    where
        Self: std::marker::Sized,
    {
    }

    fn create_renderer(window: Arc<winit::window::Window>) -> Box<dyn Renderer>;
}

pub struct Application<'a, T: ApplicationDelegate<Event>, Event = ()> {
    app: T,
    window_manager: WindowManager<'a, T, Event>,
    fonts: FontResources,
    assets: Assets<'a>,
    string_interner: StringInterner,
    last_cursor: Cursor,
    modifiers: Option<KeyModifiers>,
    key_code: Option<KeyCode>,
    key_code_repeat: Option<KeyCode>,
    key_event_handled: bool,
    broadcast_event_queue: Vec<Arc<dyn Any + Send>>,
    broadcast_async_tx: tokio::sync::mpsc::UnboundedSender<Box<dyn Any + Send>>,
    broadcast_async_rx: tokio::sync::mpsc::UnboundedReceiver<Box<dyn Any + Send>>,
    event_loop_proxy: Arc<WinitEventLoopProxy>,
    force_redraw: bool,
    needs_redraw: bool,
    shortcuts_registry: ShortcutsRegistry,
}

pub struct WinitEventLoopProxy {
    proxy: winit::event_loop::EventLoopProxy<ApplicationEvent>,
}

impl ApplicationEventLoopProxy for WinitEventLoopProxy {
    fn send_event(&self, event: ApplicationEvent) {
        let _ = self.proxy.send_event(event);
    }
}

#[allow(clippy::too_many_arguments)]
fn build<'a, T: ApplicationDelegate<Event>, Event: 'static>(
    app: &mut T,
    fonts: &mut FontResources,
    assets: &Assets,
    string_interner: &mut StringInterner,
    broadcast_event_queue: &mut Vec<Arc<dyn Any + Send>>,
    broadcast_async_tx: &mut tokio::sync::mpsc::UnboundedSender<Box<dyn Any + Send>>,
    window_state: &mut WindowState<'a, T, Event>,
    event_loop_proxy: Arc<WinitEventLoopProxy>,
    force_redraw: bool,
) -> bool {
    init_cycle(&mut window_state.ui_state);

    for event_box in window_state.ui_state.current_event_queue.iter() {
        // Skip event processing for () type
        if TypeId::of::<Event>() != TypeId::of::<()>()
            && let Some(event) = event_box.downcast_ref::<Event>()
        {
            window_state.window.on_event(app, event);
        }
    }

    for event_box in broadcast_event_queue.iter() {
        window_state
            .ui_state
            .current_event_queue
            .push(event_box.clone());
    }

    broadcast_event_queue.clear();

    handle_interaction_before_build(
        &mut window_state.ui_state.user_input,
        &window_state.ui_state.view,
    );

    let mut build_context = BuildContext::new(
        &mut window_state.ui_state,
        &mut window_state.texts,
        fonts,
        broadcast_event_queue,
        broadcast_async_tx,
        event_loop_proxy.clone(),
        window_state.delta_time_timer.elapsed().as_secs_f64(),
        true,
    );

    window_state.delta_time_timer = Instant::now();

    window_state.window.build(app, &mut build_context);

    clew::pre_layout(
        &mut window_state.ui_state,
        &mut window_state.texts,
        fonts,
        assets,
    );

    window_state.ui_state.layout_commands.clear();

    let mut build_context = BuildContext::new(
        &mut window_state.ui_state,
        &mut window_state.texts,
        fonts,
        broadcast_event_queue,
        broadcast_async_tx,
        event_loop_proxy,
        window_state.delta_time_timer.elapsed().as_secs_f64(),
        false,
    );

    window_state.window.build(app, &mut build_context);

    let redraw = clew::layout_and_render(
        &mut window_state.ui_state,
        &mut window_state.texts,
        fonts,
        assets,
        string_interner,
        &mut window_state.strings,
        force_redraw,
    );

    finalize_cycle(&mut window_state.ui_state);

    redraw
}

impl<T: ApplicationDelegate<Event>, Event: 'static>
    winit::application::ApplicationHandler<ApplicationEvent> for Application<'_, T, Event>
{
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        self.window_manager
            .with_event_loop(event_loop, |window_manager| {
                self.app
                    .on_start(window_manager, &mut self.shortcuts_registry);
            });

        for window in self.window_manager.windows.values_mut() {
            let scale = window.winit_window.scale_factor();
            window.ui_state.view.scale_factor = scale;
            window
                .texts
                .update_view(&window.ui_state.view, &mut self.fonts);
            window
                .ui_state
                .shortcuts_registry()
                .merge_with(&self.shortcuts_registry);
        }
    }

    fn user_event(&mut self, _: &winit::event_loop::ActiveEventLoop, event: ApplicationEvent) {
        match event {
            ApplicationEvent::Wake { view_id } => {
                self.window_manager.request_view_redraw(view_id);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        // Request redraw for all windows that need it
        for (_, window) in self.window_manager.windows.iter_mut() {
            // if self.needs_redraw {
            window.winit_window.request_redraw();
            handle_os_events(window);
            // }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // Collect async events
        while let Ok(event) = self.broadcast_async_rx.try_recv() {
            self.broadcast_event_queue.push(event.into());
        }

        for event_box in self.broadcast_event_queue.iter() {
            if let Some(event) = event_box.downcast_ref::<Event>() {
                self.app.on_event(&mut self.window_manager, event);

                for window in self.window_manager.windows.values_mut() {
                    window.window.on_event(&mut self.app, event);
                }
            }
        }

        if !matches!(event, winit::event::WindowEvent::RedrawRequested) {
            self.broadcast_event_queue.clear();
        }

        let Some(window) = self.window_manager.get_mut_window(window_id) else {
            return;
        };

        let input_cursor = window.ui_state.user_input.cursor;

        if self.last_cursor != input_cursor
        /* || ui_state.parameters.should_update_cursor_each_frame*/
        {
            let cursor = match input_cursor {
                Cursor::Default => winit::window::CursorIcon::Default,
                Cursor::Pointer => winit::window::CursorIcon::Pointer,
                Cursor::Text => winit::window::CursorIcon::Text,
                Cursor::EwResize => winit::window::CursorIcon::EwResize,
                Cursor::NsResize => winit::window::CursorIcon::NsResize,
                Cursor::NeswResize => winit::window::CursorIcon::NeswResize,
                Cursor::NwseResize => winit::window::CursorIcon::NwseResize,
            };

            window
                .winit_window
                .set_cursor(winit::window::Cursor::Icon(cursor));
            self.last_cursor = input_cursor;
        }

        match event {
            winit::event::WindowEvent::CloseRequested => {
                self.window_manager.windows.remove(&window_id);

                if self.window_manager.windows.is_empty() {
                    event_loop.exit();
                }
            }
            winit::event::WindowEvent::Resized(size) => {
                window.ui_state.view.physical_size = PhysicalSize::new(size.width, size.height);
                self.force_redraw = true;

                window.ui_state.user_input.mouse_left_pressed = false;
                window.ui_state.user_input.mouse_right_pressed = false;
                window.ui_state.user_input.mouse_middle_pressed = false;
                window.ui_state.user_input.mouse_left_released = false;
                window.ui_state.user_input.mouse_right_released = false;
                window.ui_state.user_input.mouse_middle_released = false;
                window.ui_state.user_input.mouse_pressed = false;
                window.ui_state.user_input.mouse_released = false;
                window.ui_state.user_input.mouse_x = -1.;
                window.ui_state.user_input.mouse_y = -1.;
                window.ui_state.user_input.mouse_wheel_delta_x = 0.;
                window.ui_state.user_input.mouse_wheel_delta_y = 0.;
                window.ui_state.user_input.mouse_left_click_count = 0;

                self.window_manager.request_redraw(window_id);
            }
            winit::event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                window.ui_state.view.scale_factor = scale_factor;
                window
                    .texts
                    .update_view(&window.ui_state.view, &mut self.fonts);

                self.force_redraw = true;
                self.window_manager.request_redraw(window_id);
            }
            winit::event::WindowEvent::RedrawRequested => {
                self.key_code_repeat = None;
                self.key_event_handled = true;
                self.key_code = None;

                let need_to_redraw = build(
                    &mut self.app,
                    &mut self.fonts,
                    &self.assets,
                    &mut self.string_interner,
                    &mut self.broadcast_event_queue,
                    &mut self.broadcast_async_tx,
                    window,
                    self.event_loop_proxy.clone(),
                    self.force_redraw,
                );

                window.ui_state.user_input.text_input.clear();
                window.ui_state.user_input.keys_pressed.clear();
                window.ui_state.user_input.keys_pressed_repeat.clear();

                if need_to_redraw {
                    window.renderer.process_commands(
                        &window.ui_state.view,
                        &window.ui_state.render_state,
                        window.fill_color,
                        &mut self.fonts,
                        &mut window.texts,
                        &self.assets,
                    );

                    window.winit_window.request_redraw();
                    self.force_redraw = false;
                }
            }
            winit::event::WindowEvent::MouseInput {
                state: btn_state,
                button,
                ..
            } => {
                // window.winit_window.request_redraw();
                self.needs_redraw = true;

                window.ui_state.user_input.mouse_pressed =
                    btn_state == winit::event::ElementState::Pressed;
                window.ui_state.user_input.mouse_released =
                    btn_state == winit::event::ElementState::Released;

                match button {
                    winit::event::MouseButton::Left => {
                        window.ui_state.user_input.mouse_left_pressed =
                            window.ui_state.user_input.mouse_pressed;
                        window.ui_state.user_input.mouse_left_released =
                            window.ui_state.user_input.mouse_released;
                    }
                    winit::event::MouseButton::Right => {
                        window.ui_state.user_input.mouse_right_pressed =
                            window.ui_state.user_input.mouse_pressed;
                        window.ui_state.user_input.mouse_right_released =
                            window.ui_state.user_input.mouse_released;
                    }
                    winit::event::MouseButton::Middle => {
                        window.ui_state.user_input.mouse_middle_pressed =
                            window.ui_state.user_input.mouse_pressed;
                        window.ui_state.user_input.mouse_middle_released =
                            window.ui_state.user_input.mouse_released;
                    }
                    _ => {}
                }
            }

            // Mouse wheel scrolling
            winit::event::WindowEvent::MouseWheel { delta, .. } => {
                self.needs_redraw = true;
                // window.winit_window.request_redraw();

                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        // Scale line delta
                        window.ui_state.user_input.mouse_wheel_delta_x = x * 20.;
                        window.ui_state.user_input.mouse_wheel_delta_y = y * 20.;
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        window.ui_state.user_input.mouse_wheel_delta_x = pos.x as f32;
                        window.ui_state.user_input.mouse_wheel_delta_y = pos.y as f32;
                    }
                }
            }

            // Mouse movement
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                // window.winit_window.request_redraw();
                self.needs_redraw = true;

                window.ui_state.user_input.mouse_x = position.x;
                window.ui_state.user_input.mouse_y = position.y;
            }

            // Focus events
            winit::event::WindowEvent::Focused(focused) => {
                window.winit_window.request_redraw();

                if !focused {
                    // Clear input state when window loses focus
                    // input.keys_pressed.clear();
                    window.ui_state.user_input.mouse_left_pressed = false;
                    window.ui_state.user_input.mouse_right_pressed = false;
                    window.ui_state.user_input.mouse_middle_pressed = false;

                    window.winit_window.set_cursor(winit::window::Cursor::Icon(
                        winit::window::CursorIcon::Default,
                    ));
                    self.last_cursor = Cursor::Default;
                    window.ui_state.user_input.cursor = Cursor::Default;
                }
            }
            winit::event::WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = from_winit_modifiers(new_modifiers.state());
                window.ui_state.user_input.modifiers = self.modifiers;
            }
            winit::event::WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(code),
                        logical_key,
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                window.ui_state.user_input.is_key_pressed =
                    state == winit::event::ElementState::Pressed;
                window.ui_state.user_input.is_key_released =
                    state == winit::event::ElementState::Released;

                if state == winit::event::ElementState::Pressed {
                    window.ui_state.user_input.key_pressed = from_winit_key_code(code);

                    if !repeat {
                        self.key_code = from_winit_key_code(code);
                    } else {
                        self.key_code_repeat = from_winit_key_code(code);
                    }
                } else if state == winit::event::ElementState::Released {
                    window.ui_state.user_input.key_released = from_winit_key_code(code);
                }

                match logical_key {
                    winit::keyboard::Key::Character(ref text) => {
                        if state.is_pressed() {
                            println!("char: {text}");
                            window.ui_state.user_input.text_input.push_str(text);
                            window
                                .ui_state
                                .user_input
                                .text_input_actions
                                .push(TextInputAction::Insert);
                        }
                    }
                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) => {
                        if state.is_pressed() {
                            if window.ui_state.view_config.should_use_wide_space {
                                window.ui_state.user_input.text_input.push('\u{3000}');
                            } else {
                                window.ui_state.user_input.text_input.push(' ');
                            }

                            window
                                .ui_state
                                .user_input
                                .text_input_actions
                                .push(TextInputAction::Insert);
                        }
                    }
                    _ => {}
                }

                if let (winit::keyboard::KeyCode::Escape, true) = (code, state.is_pressed()) {
                    event_loop.exit()
                }

                let _ = state;

                // self.handle_key_events(window_id);
                window
                    .ui_state
                    .user_input
                    .keys_pressed
                    .push((self.modifiers, self.key_code));

                window
                    .ui_state
                    .user_input
                    .keys_pressed_repeat
                    .push((self.modifiers, self.key_code_repeat));
            }
            // Text input events (handles regular text and IME composition)
            winit::event::WindowEvent::Ime(ime_event) => {
                use winit::event::Ime;

                match ime_event {
                    // IME composition events
                    Ime::Preedit(text, cursor_range) => {
                        window.ui_state.user_input.ime_preedit = text;
                        window.ui_state.user_input.ime_cursor_range = cursor_range;
                        window
                            .ui_state
                            .user_input
                            .text_input_actions
                            .push(TextInputAction::ImePreedit);
                    }

                    // Final committed text from IME
                    Ime::Commit(text) => {
                        window.ui_state.user_input.text_input.push_str(&text);
                        window.ui_state.user_input.ime_preedit.clear();
                        window.ui_state.user_input.ime_cursor_range = None;

                        window
                            .ui_state
                            .user_input
                            .text_input_actions
                            .push(TextInputAction::ImeCommit);
                        window
                            .ui_state
                            .user_input
                            .text_input_actions
                            .push(TextInputAction::Insert);

                        window.ime_reset_needed = true;
                    }

                    // IME enabled/disabled
                    Ime::Enabled => {
                        window
                            .ui_state
                            .user_input
                            .text_input_actions
                            .push(TextInputAction::ImeEnable);
                    }
                    Ime::Disabled => {
                        window.ui_state.user_input.ime_preedit.clear();
                        window.ui_state.user_input.ime_cursor_range = None;

                        window
                            .ui_state
                            .user_input
                            .text_input_actions
                            .push(TextInputAction::ImeDisable);
                    }
                }
            }
            _ => (),
        }
    }
}

fn handle_os_events<App, Event>(window: &mut WindowState<App, Event>) {
    let ime_widget_rect = window.ui_state.view_config.ime_cursor_rect;

    if window.ime_reset_needed {
        window.winit_window.set_ime_cursor_area(
            winit::dpi::PhysicalPosition::new(ime_widget_rect.x, ime_widget_rect.y),
            winit::dpi::PhysicalSize::new(ime_widget_rect.width, ime_widget_rect.height),
        );
        window.ime_reset_needed = false;

        // #[cfg(target_os = "macos")]
        // macos_invalidate_ime_coordinates(&state.window);
    }

    if window.ime_activated && window.last_ime_rect != ime_widget_rect {
        window.last_ime_rect = ime_widget_rect;

        window.winit_window.set_ime_cursor_area(
            winit::dpi::PhysicalPosition::new(ime_widget_rect.x, ime_widget_rect.y),
            winit::dpi::PhysicalSize::new(ime_widget_rect.width, ime_widget_rect.height),
        );

        // #[cfg(target_os = "macos")]
        // macos_invalidate_ime_coordinates(&state.window);
    }

    let mut clear_ime = false;
    let commit_ime = false;

    for event in window.ui_state.os_events.drain(..) {
        match event {
            OsEvent::FocusWindow => window.winit_window.focus_window(),
            OsEvent::CommitIme => {
                // #[cfg(target_os = "macos")]
                // {
                //     macos_cancel_ime_for_winit(&state.window);
                //     commit_ime = true;
                // }

                window.ime_reset_needed = true;
            }
            OsEvent::ActivateIme => {
                if window.ime_activated {
                    return;
                }

                clear_ime = true;

                window.ime_activated = true;
                window.last_ime_rect = ime_widget_rect;

                window.winit_window.set_ime_allowed(true);
                window.winit_window.set_ime_cursor_area(
                    winit::dpi::PhysicalPosition::new(ime_widget_rect.x, ime_widget_rect.y),
                    winit::dpi::PhysicalSize::new(ime_widget_rect.width, ime_widget_rect.height),
                );
            }
            OsEvent::DeactivateIme => {
                if !window.ime_activated {
                    return;
                }

                clear_ime = true;

                window.ime_activated = false;
                window.winit_window.set_ime_allowed(false);
                println!("DEACTIVATE");
            }
        }
    }

    if commit_ime {
        let preedit = window.ui_state.user_input.ime_preedit.clone();

        window.ui_state.user_input.text_input.push_str(&preedit);
        window.ui_state.user_input.ime_preedit.clear();
        window.ui_state.user_input.ime_cursor_range = None;

        window
            .ui_state
            .user_input
            .text_input_actions
            .push(TextInputAction::ImeCommit);
        window
            .ui_state
            .user_input
            .text_input_actions
            .push(TextInputAction::Insert);
    }

    if clear_ime {
        window.ui_state.user_input.ime_preedit.clear();
        window.ui_state.user_input.ime_cursor_range = None;
    }
}

impl<T: ApplicationDelegate<Event>, Event: 'static> Application<'_, T, Event> {
    pub fn run_application(mut delegate: T) -> anyhow::Result<()> {
        let (broadcast_async_tx, broadcast_async_rx) = tokio::sync::mpsc::unbounded_channel();

        let mut assets = Assets::new();

        delegate.init_assets(&mut assets);

        let fonts = assets.create_font_resources();

        #[cfg(target_os = "macos")]
        let event_loop = winit::event_loop::EventLoop::with_user_event()
            .with_activation_policy(winit::platform::macos::ActivationPolicy::Regular)
            .build()?;

        #[cfg(not(target_os = "macos"))]
        let event_loop = winit::event_loop::EventLoop::with_user_event().build()?;

        let event_proxy = event_loop.create_proxy();

        let mut application = Application {
            app: delegate,
            window_manager: WindowManager::new(T::create_renderer),
            fonts,
            string_interner: StringInterner::new(),
            last_cursor: Cursor::Default,
            broadcast_event_queue: Vec::new(),
            broadcast_async_rx,
            broadcast_async_tx,
            force_redraw: false,
            needs_redraw: false,
            event_loop_proxy: Arc::new(WinitEventLoopProxy { proxy: event_proxy }),
            assets,
            shortcuts_registry: ShortcutsRegistry::default(),
            modifiers: None,
            key_code: None,
            key_code_repeat: None,
            key_event_handled: false,
        };

        event_loop.run_app(&mut application)?;

        Ok(())
    }
}
