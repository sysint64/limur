use crate::{io::Cursor, state::UiState};

pub fn init_cycle(state: &mut UiState) {
    state.layout_commands.clear();
    state.render_state.commands.clear();
    state.widget_placements.clear();
    state.layout_items.clear();
    state.non_interactable.clear();
    state.user_input.cursor = Cursor::Default;

    state.shortcuts_manager.init_cycle(&state.user_input);

    std::mem::swap(&mut state.current_event_queue, &mut state.next_event_queue);
    state.next_event_queue.clear();

    // Collect async events
    while let Ok(event) = state.async_rx.try_recv() {
        state.current_event_queue.push(event.into());
    }
}

pub fn finalize_cycle(state: &mut UiState) {
    state.shortcuts_manager.finalize_cycle();
    state.user_input.reset();
}
