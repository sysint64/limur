use std::time::{Duration, Instant};

use smallvec::SmallVec;

use crate::keyboard::{KeyCode, KeyModifiers};

#[derive(Default, Debug, Clone)]
pub struct UserInput {
    pub cursor: Cursor,

    // Mouse state
    pub mouse_left_pressed: bool,
    pub mouse_right_pressed: bool,
    pub mouse_middle_pressed: bool,
    pub mouse_left_released: bool,
    pub mouse_right_released: bool,
    pub mouse_middle_released: bool,
    pub mouse_pressed: bool,
    pub mouse_released: bool,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_wheel_delta_x: f32,
    pub mouse_wheel_delta_y: f32,
    pub mouse_left_click_count: u32,

    // Keyboard state
    pub key_pressed: SmallVec<[(Option<KeyModifiers>, Option<KeyCode>); 4]>,
    pub key_pressed_repeat: SmallVec<[(Option<KeyModifiers>, Option<KeyCode>); 4]>,

    pub is_key_pressed: bool,
    pub is_key_released: bool,

    // // Text input and IME
    pub text_input_actions: Vec<TextInputAction>,
    pub text_input: String,
    pub ime_preedit: String,
    pub ime_last_preedit: String,
    pub ime_cursor_range: Option<(usize, usize)>,

    pub(crate) mouse_left_click_tracker: ClickTracker,
    pub(crate) last_click_time: Option<Instant>,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Cursor {
    #[default]
    Default,
    Pointer,
    Text,
    EwResize,   // East-West (horizontal double-headed arrow)
    NsResize,   // North-South (vertical double-headed arrow)
    NeswResize, // Northeast-Southwest diagonal
    NwseResize, // Northwest-Southeast diagonal
}

#[derive(Default, Copy, Clone, Debug)]
pub enum TextInputAction {
    #[default]
    None,
    ImeCommit,
    ImePreedit,
    ImeDisable,
    ImeEnable,
    Insert,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct ClickTracker {
    click_count: u32,
    last_click_time: Option<Instant>,
    last_click_position: Option<(f32, f32)>,
}

impl ClickTracker {
    pub(crate) fn on_click(&mut self, mouse_x: f32, mouse_y: f32, scale_factor: f32) -> u32 {
        let now = Instant::now();
        let click_time = Duration::from_millis(500);

        if let Some(last_time) = self.last_click_time
            && let Some((last_mouse_x, last_mouse_y)) = self.last_click_position
        {
            let distance_threshold = 5. * scale_factor;
            let distance_x = (last_mouse_x - mouse_x).abs();
            let distance_y = (last_mouse_y - mouse_y).abs();

            let time_diff = now.duration_since(last_time);

            if time_diff < click_time
                && distance_x < distance_threshold
                && distance_y < distance_threshold
            {
                self.click_count += 1;
            } else {
                self.click_count = 1;
            }
        } else {
            self.click_count = 1;
        }

        self.last_click_time = Some(now);
        self.last_click_position = Some((mouse_x, mouse_y));
        self.click_count
    }

    pub(crate) fn reset(&mut self) {
        self.click_count = 0;
        self.last_click_time = None;
        self.last_click_position = None;
    }
}

impl UserInput {
    pub fn reset(&mut self) {
        self.mouse_pressed = false;
        self.mouse_released = false;
        self.mouse_left_pressed = false;
        self.mouse_right_pressed = false;
        self.mouse_middle_pressed = false;
        self.mouse_left_released = false;
        self.mouse_right_released = false;
        self.mouse_middle_released = false;
    }

    pub fn clear_frame_events(&mut self) {
        self.mouse_wheel_delta_x = 0.0;
        self.mouse_wheel_delta_y = 0.0;

        self.text_input.clear();
    }

    pub fn get_text_input(&self) -> &str {
        &self.text_input
    }

    pub fn get_ime_preedit(&self) -> &str {
        &self.ime_preedit
    }
}
