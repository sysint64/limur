pub(crate) fn from_winit_modifiers(
    value: winit::keyboard::ModifiersState,
) -> Option<limur::keyboard::KeyModifiers> {
    limur::keyboard::KeyModifiers::from_bits(value.bits())
}

pub(crate) fn from_winit_key_code(
    value: winit::keyboard::KeyCode,
) -> Option<limur::keyboard::KeyCode> {
    match value {
        winit::keyboard::KeyCode::Backquote => Some(limur::keyboard::KeyCode::Backquote),
        winit::keyboard::KeyCode::Backslash => Some(limur::keyboard::KeyCode::Backslash),
        winit::keyboard::KeyCode::BracketLeft => Some(limur::keyboard::KeyCode::BracketLeft),
        winit::keyboard::KeyCode::BracketRight => Some(limur::keyboard::KeyCode::BracketRight),
        winit::keyboard::KeyCode::Comma => Some(limur::keyboard::KeyCode::Comma),
        winit::keyboard::KeyCode::Digit0 => Some(limur::keyboard::KeyCode::Digit0),
        winit::keyboard::KeyCode::Digit1 => Some(limur::keyboard::KeyCode::Digit1),
        winit::keyboard::KeyCode::Digit2 => Some(limur::keyboard::KeyCode::Digit2),
        winit::keyboard::KeyCode::Digit3 => Some(limur::keyboard::KeyCode::Digit3),
        winit::keyboard::KeyCode::Digit4 => Some(limur::keyboard::KeyCode::Digit4),
        winit::keyboard::KeyCode::Digit5 => Some(limur::keyboard::KeyCode::Digit5),
        winit::keyboard::KeyCode::Digit6 => Some(limur::keyboard::KeyCode::Digit6),
        winit::keyboard::KeyCode::Digit7 => Some(limur::keyboard::KeyCode::Digit7),
        winit::keyboard::KeyCode::Digit8 => Some(limur::keyboard::KeyCode::Digit8),
        winit::keyboard::KeyCode::Digit9 => Some(limur::keyboard::KeyCode::Digit9),
        winit::keyboard::KeyCode::Equal => Some(limur::keyboard::KeyCode::Equal),
        winit::keyboard::KeyCode::IntlBackslash => Some(limur::keyboard::KeyCode::IntlBackslash),
        winit::keyboard::KeyCode::IntlRo => Some(limur::keyboard::KeyCode::IntlRo),
        winit::keyboard::KeyCode::IntlYen => Some(limur::keyboard::KeyCode::IntlYen),
        winit::keyboard::KeyCode::KeyA => Some(limur::keyboard::KeyCode::KeyA),
        winit::keyboard::KeyCode::KeyB => Some(limur::keyboard::KeyCode::KeyB),
        winit::keyboard::KeyCode::KeyC => Some(limur::keyboard::KeyCode::KeyC),
        winit::keyboard::KeyCode::KeyD => Some(limur::keyboard::KeyCode::KeyD),
        winit::keyboard::KeyCode::KeyE => Some(limur::keyboard::KeyCode::KeyE),
        winit::keyboard::KeyCode::KeyF => Some(limur::keyboard::KeyCode::KeyF),
        winit::keyboard::KeyCode::KeyG => Some(limur::keyboard::KeyCode::KeyG),
        winit::keyboard::KeyCode::KeyH => Some(limur::keyboard::KeyCode::KeyH),
        winit::keyboard::KeyCode::KeyI => Some(limur::keyboard::KeyCode::KeyI),
        winit::keyboard::KeyCode::KeyJ => Some(limur::keyboard::KeyCode::KeyJ),
        winit::keyboard::KeyCode::KeyK => Some(limur::keyboard::KeyCode::KeyK),
        winit::keyboard::KeyCode::KeyL => Some(limur::keyboard::KeyCode::KeyL),
        winit::keyboard::KeyCode::KeyM => Some(limur::keyboard::KeyCode::KeyM),
        winit::keyboard::KeyCode::KeyN => Some(limur::keyboard::KeyCode::KeyN),
        winit::keyboard::KeyCode::KeyO => Some(limur::keyboard::KeyCode::KeyO),
        winit::keyboard::KeyCode::KeyP => Some(limur::keyboard::KeyCode::KeyP),
        winit::keyboard::KeyCode::KeyQ => Some(limur::keyboard::KeyCode::KeyQ),
        winit::keyboard::KeyCode::KeyR => Some(limur::keyboard::KeyCode::KeyR),
        winit::keyboard::KeyCode::KeyS => Some(limur::keyboard::KeyCode::KeyS),
        winit::keyboard::KeyCode::KeyT => Some(limur::keyboard::KeyCode::KeyT),
        winit::keyboard::KeyCode::KeyU => Some(limur::keyboard::KeyCode::KeyU),
        winit::keyboard::KeyCode::KeyV => Some(limur::keyboard::KeyCode::KeyV),
        winit::keyboard::KeyCode::KeyW => Some(limur::keyboard::KeyCode::KeyW),
        winit::keyboard::KeyCode::KeyX => Some(limur::keyboard::KeyCode::KeyX),
        winit::keyboard::KeyCode::KeyY => Some(limur::keyboard::KeyCode::KeyY),
        winit::keyboard::KeyCode::KeyZ => Some(limur::keyboard::KeyCode::KeyZ),
        winit::keyboard::KeyCode::Minus => Some(limur::keyboard::KeyCode::Minus),
        winit::keyboard::KeyCode::Period => Some(limur::keyboard::KeyCode::Period),
        winit::keyboard::KeyCode::Quote => Some(limur::keyboard::KeyCode::Quote),
        winit::keyboard::KeyCode::Semicolon => Some(limur::keyboard::KeyCode::Semicolon),
        winit::keyboard::KeyCode::Slash => Some(limur::keyboard::KeyCode::Slash),
        winit::keyboard::KeyCode::AltLeft => Some(limur::keyboard::KeyCode::AltLeft),
        winit::keyboard::KeyCode::AltRight => Some(limur::keyboard::KeyCode::AltRight),
        winit::keyboard::KeyCode::Backspace => Some(limur::keyboard::KeyCode::Backspace),
        winit::keyboard::KeyCode::CapsLock => Some(limur::keyboard::KeyCode::CapsLock),
        winit::keyboard::KeyCode::ContextMenu => Some(limur::keyboard::KeyCode::ContextMenu),
        winit::keyboard::KeyCode::ControlLeft => Some(limur::keyboard::KeyCode::ControlLeft),
        winit::keyboard::KeyCode::ControlRight => Some(limur::keyboard::KeyCode::ControlRight),
        winit::keyboard::KeyCode::Enter => Some(limur::keyboard::KeyCode::Enter),
        winit::keyboard::KeyCode::SuperLeft => Some(limur::keyboard::KeyCode::SuperLeft),
        winit::keyboard::KeyCode::SuperRight => Some(limur::keyboard::KeyCode::SuperRight),
        winit::keyboard::KeyCode::ShiftLeft => Some(limur::keyboard::KeyCode::ShiftLeft),
        winit::keyboard::KeyCode::ShiftRight => Some(limur::keyboard::KeyCode::ShiftRight),
        winit::keyboard::KeyCode::Space => Some(limur::keyboard::KeyCode::Space),
        winit::keyboard::KeyCode::Tab => Some(limur::keyboard::KeyCode::Tab),
        winit::keyboard::KeyCode::Convert => Some(limur::keyboard::KeyCode::Convert),
        winit::keyboard::KeyCode::KanaMode => Some(limur::keyboard::KeyCode::KanaMode),
        winit::keyboard::KeyCode::Lang1 => Some(limur::keyboard::KeyCode::Lang1),
        winit::keyboard::KeyCode::Lang2 => Some(limur::keyboard::KeyCode::Lang2),
        winit::keyboard::KeyCode::Lang3 => Some(limur::keyboard::KeyCode::Lang3),
        winit::keyboard::KeyCode::Lang4 => Some(limur::keyboard::KeyCode::Lang4),
        winit::keyboard::KeyCode::Lang5 => Some(limur::keyboard::KeyCode::Lang5),
        winit::keyboard::KeyCode::NonConvert => Some(limur::keyboard::KeyCode::NonConvert),
        winit::keyboard::KeyCode::Delete => Some(limur::keyboard::KeyCode::Delete),
        winit::keyboard::KeyCode::End => Some(limur::keyboard::KeyCode::End),
        winit::keyboard::KeyCode::Help => Some(limur::keyboard::KeyCode::Help),
        winit::keyboard::KeyCode::Home => Some(limur::keyboard::KeyCode::Home),
        winit::keyboard::KeyCode::Insert => Some(limur::keyboard::KeyCode::Insert),
        winit::keyboard::KeyCode::PageDown => Some(limur::keyboard::KeyCode::PageDown),
        winit::keyboard::KeyCode::PageUp => Some(limur::keyboard::KeyCode::PageUp),
        winit::keyboard::KeyCode::ArrowDown => Some(limur::keyboard::KeyCode::ArrowDown),
        winit::keyboard::KeyCode::ArrowLeft => Some(limur::keyboard::KeyCode::ArrowLeft),
        winit::keyboard::KeyCode::ArrowRight => Some(limur::keyboard::KeyCode::ArrowRight),
        winit::keyboard::KeyCode::ArrowUp => Some(limur::keyboard::KeyCode::ArrowUp),
        winit::keyboard::KeyCode::NumLock => Some(limur::keyboard::KeyCode::NumLock),
        winit::keyboard::KeyCode::Numpad0 => Some(limur::keyboard::KeyCode::Numpad0),
        winit::keyboard::KeyCode::Numpad1 => Some(limur::keyboard::KeyCode::Numpad1),
        winit::keyboard::KeyCode::Numpad2 => Some(limur::keyboard::KeyCode::Numpad2),
        winit::keyboard::KeyCode::Numpad3 => Some(limur::keyboard::KeyCode::Numpad3),
        winit::keyboard::KeyCode::Numpad4 => Some(limur::keyboard::KeyCode::Numpad4),
        winit::keyboard::KeyCode::Numpad5 => Some(limur::keyboard::KeyCode::Numpad5),
        winit::keyboard::KeyCode::Numpad6 => Some(limur::keyboard::KeyCode::Numpad6),
        winit::keyboard::KeyCode::Numpad7 => Some(limur::keyboard::KeyCode::Numpad7),
        winit::keyboard::KeyCode::Numpad8 => Some(limur::keyboard::KeyCode::Numpad8),
        winit::keyboard::KeyCode::Numpad9 => Some(limur::keyboard::KeyCode::Numpad9),
        winit::keyboard::KeyCode::NumpadAdd => Some(limur::keyboard::KeyCode::NumpadAdd),
        winit::keyboard::KeyCode::NumpadBackspace => {
            Some(limur::keyboard::KeyCode::NumpadBackspace)
        }
        winit::keyboard::KeyCode::NumpadClear => Some(limur::keyboard::KeyCode::NumpadClear),
        winit::keyboard::KeyCode::NumpadClearEntry => {
            Some(limur::keyboard::KeyCode::NumpadClearEntry)
        }
        winit::keyboard::KeyCode::NumpadComma => Some(limur::keyboard::KeyCode::NumpadComma),
        winit::keyboard::KeyCode::NumpadDecimal => Some(limur::keyboard::KeyCode::NumpadDecimal),
        winit::keyboard::KeyCode::NumpadDivide => Some(limur::keyboard::KeyCode::NumpadDivide),
        winit::keyboard::KeyCode::NumpadEnter => Some(limur::keyboard::KeyCode::NumpadEnter),
        winit::keyboard::KeyCode::NumpadEqual => Some(limur::keyboard::KeyCode::NumpadEqual),
        winit::keyboard::KeyCode::NumpadHash => Some(limur::keyboard::KeyCode::NumpadHash),
        winit::keyboard::KeyCode::NumpadMemoryAdd => {
            Some(limur::keyboard::KeyCode::NumpadMemoryAdd)
        }
        winit::keyboard::KeyCode::NumpadMemoryClear => {
            Some(limur::keyboard::KeyCode::NumpadMemoryClear)
        }
        winit::keyboard::KeyCode::NumpadMemoryRecall => {
            Some(limur::keyboard::KeyCode::NumpadMemoryRecall)
        }
        winit::keyboard::KeyCode::NumpadMemoryStore => {
            Some(limur::keyboard::KeyCode::NumpadMemoryStore)
        }
        winit::keyboard::KeyCode::NumpadMemorySubtract => {
            Some(limur::keyboard::KeyCode::NumpadMemorySubtract)
        }
        winit::keyboard::KeyCode::NumpadMultiply => Some(limur::keyboard::KeyCode::NumpadMultiply),
        winit::keyboard::KeyCode::NumpadParenLeft => {
            Some(limur::keyboard::KeyCode::NumpadParenLeft)
        }
        winit::keyboard::KeyCode::NumpadParenRight => {
            Some(limur::keyboard::KeyCode::NumpadParenRight)
        }
        winit::keyboard::KeyCode::NumpadStar => Some(limur::keyboard::KeyCode::NumpadStar),
        winit::keyboard::KeyCode::NumpadSubtract => Some(limur::keyboard::KeyCode::NumpadSubtract),
        winit::keyboard::KeyCode::Escape => Some(limur::keyboard::KeyCode::Escape),
        winit::keyboard::KeyCode::Fn => Some(limur::keyboard::KeyCode::Fn),
        winit::keyboard::KeyCode::FnLock => Some(limur::keyboard::KeyCode::FnLock),
        winit::keyboard::KeyCode::PrintScreen => Some(limur::keyboard::KeyCode::PrintScreen),
        winit::keyboard::KeyCode::ScrollLock => Some(limur::keyboard::KeyCode::ScrollLock),
        winit::keyboard::KeyCode::Pause => Some(limur::keyboard::KeyCode::Pause),
        winit::keyboard::KeyCode::BrowserBack => Some(limur::keyboard::KeyCode::BrowserBack),
        winit::keyboard::KeyCode::BrowserFavorites => {
            Some(limur::keyboard::KeyCode::BrowserFavorites)
        }
        winit::keyboard::KeyCode::BrowserForward => Some(limur::keyboard::KeyCode::BrowserForward),
        winit::keyboard::KeyCode::BrowserHome => Some(limur::keyboard::KeyCode::BrowserHome),
        winit::keyboard::KeyCode::BrowserRefresh => Some(limur::keyboard::KeyCode::BrowserRefresh),
        winit::keyboard::KeyCode::BrowserSearch => Some(limur::keyboard::KeyCode::BrowserSearch),
        winit::keyboard::KeyCode::BrowserStop => Some(limur::keyboard::KeyCode::BrowserStop),
        winit::keyboard::KeyCode::Eject => Some(limur::keyboard::KeyCode::Eject),
        winit::keyboard::KeyCode::LaunchApp1 => Some(limur::keyboard::KeyCode::LaunchApp1),
        winit::keyboard::KeyCode::LaunchApp2 => Some(limur::keyboard::KeyCode::LaunchApp2),
        winit::keyboard::KeyCode::LaunchMail => Some(limur::keyboard::KeyCode::LaunchMail),
        winit::keyboard::KeyCode::MediaPlayPause => Some(limur::keyboard::KeyCode::MediaPlayPause),
        winit::keyboard::KeyCode::MediaSelect => Some(limur::keyboard::KeyCode::MediaSelect),
        winit::keyboard::KeyCode::MediaStop => Some(limur::keyboard::KeyCode::MediaStop),
        winit::keyboard::KeyCode::MediaTrackNext => Some(limur::keyboard::KeyCode::MediaTrackNext),
        winit::keyboard::KeyCode::MediaTrackPrevious => {
            Some(limur::keyboard::KeyCode::MediaTrackPrevious)
        }
        winit::keyboard::KeyCode::Power => Some(limur::keyboard::KeyCode::Power),
        winit::keyboard::KeyCode::Sleep => Some(limur::keyboard::KeyCode::Sleep),
        winit::keyboard::KeyCode::AudioVolumeDown => {
            Some(limur::keyboard::KeyCode::AudioVolumeDown)
        }
        winit::keyboard::KeyCode::AudioVolumeMute => {
            Some(limur::keyboard::KeyCode::AudioVolumeMute)
        }
        winit::keyboard::KeyCode::AudioVolumeUp => Some(limur::keyboard::KeyCode::AudioVolumeUp),
        winit::keyboard::KeyCode::WakeUp => Some(limur::keyboard::KeyCode::WakeUp),
        winit::keyboard::KeyCode::Meta => Some(limur::keyboard::KeyCode::Meta),
        winit::keyboard::KeyCode::Hyper => Some(limur::keyboard::KeyCode::Hyper),
        winit::keyboard::KeyCode::Turbo => Some(limur::keyboard::KeyCode::Turbo),
        winit::keyboard::KeyCode::Abort => Some(limur::keyboard::KeyCode::Abort),
        winit::keyboard::KeyCode::Resume => Some(limur::keyboard::KeyCode::Resume),
        winit::keyboard::KeyCode::Suspend => Some(limur::keyboard::KeyCode::Suspend),
        winit::keyboard::KeyCode::Again => Some(limur::keyboard::KeyCode::Again),
        winit::keyboard::KeyCode::Copy => Some(limur::keyboard::KeyCode::Copy),
        winit::keyboard::KeyCode::Cut => Some(limur::keyboard::KeyCode::Cut),
        winit::keyboard::KeyCode::Find => Some(limur::keyboard::KeyCode::Find),
        winit::keyboard::KeyCode::Open => Some(limur::keyboard::KeyCode::Open),
        winit::keyboard::KeyCode::Paste => Some(limur::keyboard::KeyCode::Paste),
        winit::keyboard::KeyCode::Props => Some(limur::keyboard::KeyCode::Props),
        winit::keyboard::KeyCode::Select => Some(limur::keyboard::KeyCode::Select),
        winit::keyboard::KeyCode::Undo => Some(limur::keyboard::KeyCode::Undo),
        winit::keyboard::KeyCode::Hiragana => Some(limur::keyboard::KeyCode::Hiragana),
        winit::keyboard::KeyCode::Katakana => Some(limur::keyboard::KeyCode::Katakana),
        winit::keyboard::KeyCode::F1 => Some(limur::keyboard::KeyCode::F1),
        winit::keyboard::KeyCode::F2 => Some(limur::keyboard::KeyCode::F2),
        winit::keyboard::KeyCode::F3 => Some(limur::keyboard::KeyCode::F3),
        winit::keyboard::KeyCode::F4 => Some(limur::keyboard::KeyCode::F4),
        winit::keyboard::KeyCode::F5 => Some(limur::keyboard::KeyCode::F5),
        winit::keyboard::KeyCode::F6 => Some(limur::keyboard::KeyCode::F6),
        winit::keyboard::KeyCode::F7 => Some(limur::keyboard::KeyCode::F7),
        winit::keyboard::KeyCode::F8 => Some(limur::keyboard::KeyCode::F8),
        winit::keyboard::KeyCode::F9 => Some(limur::keyboard::KeyCode::F9),
        winit::keyboard::KeyCode::F10 => Some(limur::keyboard::KeyCode::F10),
        winit::keyboard::KeyCode::F11 => Some(limur::keyboard::KeyCode::F11),
        winit::keyboard::KeyCode::F12 => Some(limur::keyboard::KeyCode::F12),
        winit::keyboard::KeyCode::F13 => Some(limur::keyboard::KeyCode::F13),
        winit::keyboard::KeyCode::F14 => Some(limur::keyboard::KeyCode::F14),
        winit::keyboard::KeyCode::F15 => Some(limur::keyboard::KeyCode::F15),
        winit::keyboard::KeyCode::F16 => Some(limur::keyboard::KeyCode::F16),
        winit::keyboard::KeyCode::F17 => Some(limur::keyboard::KeyCode::F17),
        winit::keyboard::KeyCode::F18 => Some(limur::keyboard::KeyCode::F18),
        winit::keyboard::KeyCode::F19 => Some(limur::keyboard::KeyCode::F19),
        winit::keyboard::KeyCode::F20 => Some(limur::keyboard::KeyCode::F20),
        winit::keyboard::KeyCode::F21 => Some(limur::keyboard::KeyCode::F21),
        winit::keyboard::KeyCode::F22 => Some(limur::keyboard::KeyCode::F22),
        winit::keyboard::KeyCode::F23 => Some(limur::keyboard::KeyCode::F23),
        winit::keyboard::KeyCode::F24 => Some(limur::keyboard::KeyCode::F24),
        winit::keyboard::KeyCode::F25 => Some(limur::keyboard::KeyCode::F25),
        winit::keyboard::KeyCode::F26 => Some(limur::keyboard::KeyCode::F26),
        winit::keyboard::KeyCode::F27 => Some(limur::keyboard::KeyCode::F27),
        winit::keyboard::KeyCode::F28 => Some(limur::keyboard::KeyCode::F28),
        winit::keyboard::KeyCode::F29 => Some(limur::keyboard::KeyCode::F29),
        winit::keyboard::KeyCode::F30 => Some(limur::keyboard::KeyCode::F30),
        winit::keyboard::KeyCode::F31 => Some(limur::keyboard::KeyCode::F31),
        winit::keyboard::KeyCode::F32 => Some(limur::keyboard::KeyCode::F32),
        winit::keyboard::KeyCode::F33 => Some(limur::keyboard::KeyCode::F33),
        winit::keyboard::KeyCode::F34 => Some(limur::keyboard::KeyCode::F34),
        winit::keyboard::KeyCode::F35 => Some(limur::keyboard::KeyCode::F35),
        _ => None,
    }
}
