use std::time::Instant;

use crate::{
    WidgetId,
    io::Cursor,
    profiler,
    state::{STALE_THRESHOLD, UiState},
};

pub fn init_cycle(state: &mut UiState) {
    profiler::start_cycle();

    state.root_layer.layout_commands.clear();
    state.animations_stepped_this_frame.clear();
    state.render_state.commands.clear();
    state.widget_placements.clear();
    state.non_interactable.clear();
    state.user_input.cursor = Cursor::Default;
    state.cycle_timer = Instant::now();
    state.widgets_states.set_current_layer(None);

    state.shortcuts_manager.init_cycle(&state.user_input);

    std::mem::swap(&mut state.current_event_queue, &mut state.next_event_queue);
    state.next_event_queue.clear();

    // Collect async events
    while let Ok(event) = state.async_rx.try_recv() {
        state.current_event_queue.push(event.into());
    }
}

pub fn finalize_cycle(state: &mut UiState) {
    state.shortcuts_manager.finalize_cycle(&state.user_input);
    state.performance_metrics.cycle = state.cycle_timer.elapsed();

    let alive_layers = state.layers.sweep_layers();
    let is_alive = |layer_id: WidgetId| -> bool { alive_layers.contains(&layer_id) };

    state.layers.current_frame += 1;

    state.widgets_states.sweep(&is_alive);
    state.widgets_states.next_frame();
    state.phase_allocator.reset();

    // profiler::end_cycle();
}
