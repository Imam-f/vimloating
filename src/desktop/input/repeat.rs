use super::KeyboardInput;
use macroquad::prelude::*;
use vimloating::editor::Mode;

pub struct KeyRepeat {
    pub(super) key: KeyCode,
    pub(super) mode: Mode,
    pub(super) next_repeat: f64,
}

const KEY_REPEAT_DELAY: f64 = 0.35;
const KEY_REPEAT_INTERVAL: f64 = 0.06;

pub(super) fn repeating_key(key: KeyCode, mode: Mode, input: &KeyboardInput) -> bool {
    if matches!(
        mode,
        Mode::ShellOutput | Mode::BufferList | Mode::CommandWindow
    ) {
        return false;
    }
    if input.ctrl() {
        match key {
            KeyCode::N
            | KeyCode::P
            | KeyCode::E
            | KeyCode::Y
            | KeyCode::H
            | KeyCode::L
            | KeyCode::D
            | KeyCode::U
            | KeyCode::F
            | KeyCode::B
            | KeyCode::Minus
            | KeyCode::Equal => true,
            KeyCode::W | KeyCode::Backspace => mode == Mode::Insert,
            KeyCode::A | KeyCode::X | KeyCode::R => mode == Mode::Normal,
            _ => false,
        }
    } else if input.alt() {
        matches!(mode, Mode::Normal | Mode::Visual)
            && matches!(key, KeyCode::H | KeyCode::J | KeyCode::K | KeyCode::L)
    } else {
        match key {
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown => matches!(mode, Mode::Normal | Mode::Visual | Mode::Insert),
            KeyCode::Backspace => matches!(
                mode,
                Mode::Normal | Mode::Visual | Mode::Insert | Mode::Command | Mode::Search
            ),
            KeyCode::Delete | KeyCode::Enter => mode == Mode::Insert,
            KeyCode::Tab => matches!(mode, Mode::Insert | Mode::Command),
            _ => false,
        }
    }
}

pub(super) fn repeat_due(
    state: &mut Option<KeyRepeat>,
    key: Option<KeyCode>,
    pressed: bool,
    mode: Mode,
    now: f64,
) -> Option<KeyCode> {
    let Some(key) = key else {
        *state = None;
        return None;
    };
    let same = state
        .as_ref()
        .is_some_and(|repeat| repeat.key == key && repeat.mode == mode);
    if !same {
        // A held key must not leak into another mode until it is released.
        if state
            .as_ref()
            .is_some_and(|repeat| repeat.key == key && repeat.mode != mode)
            && !pressed
        {
            return None;
        }
        // The press edge may have been consumed by a mode change or another key.
        *state = Some(KeyRepeat {
            key,
            mode,
            next_repeat: now + KEY_REPEAT_DELAY,
        });
        return Some(key);
    } else if let Some(repeat) = state.as_mut()
        && now >= repeat.next_repeat
    {
        repeat.next_repeat = now + KEY_REPEAT_INTERVAL;
        return Some(key);
    }
    None
}

pub(super) fn poll_key_repeat(
    state: &mut Option<KeyRepeat>,
    mode: Mode,
    input: &KeyboardInput,
) -> Option<KeyCode> {
    // A newly pressed key wins when the previous repeat key is still down.
    let enabled = |key: KeyCode| repeating_key(key, mode, input);
    let key = input
        .pressed
        .iter()
        .copied()
        .filter(|&key| enabled(key))
        .min_by_key(|&key| key as u32)
        .or_else(|| {
            state
                .as_ref()
                .map(|repeat| repeat.key)
                .filter(|&key| enabled(key) && input.down.contains(&key))
        })
        .or_else(|| {
            input
                .down
                .iter()
                .copied()
                .filter(|&key| enabled(key))
                .min_by_key(|&key| key as u32)
        });
    repeat_due(
        state,
        key,
        key.is_some_and(|key| input.pressed.contains(&key)),
        mode,
        input.now,
    )
}
