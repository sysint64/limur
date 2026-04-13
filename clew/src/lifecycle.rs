use std::time::Instant;

use crate::{io::Cursor, profiler, state::UiState};

pub fn init_cycle(state: &mut UiState) {
    profiler::start_cycle();

    state.animations_stepped_this_frame.clear();
    state.render_state.commands.clear();
    state.widget_placements.clear();
    state.non_interactable.clear();
    state.user_input.cursor = Cursor::Default;
    state.cycle_timer = Instant::now();

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
    state
        .layers
        .sweep(&state.widgets_states.accessed_this_frame);
    state.widgets_states.sweep();
    state.phase_allocator.reset();

    profiler::end_cycle();
}
