use crate::config::VERTICAL_MOTION_STEP;
use crate::editor::{BufferAction, Editor, Mode};
use crate::render::text_grid;
use macroquad::prelude::*;
use std::collections::HashSet;

pub struct VerticalMotion {
    direction: isize,
    remaining: usize,
    next_step: f64,
    repeat_key: KeyCode,
}

pub struct KeyRepeat {
    key: KeyCode,
    mode: Mode,
    next_repeat: f64,
}

const KEY_REPEAT_DELAY: f64 = 0.35;
const KEY_REPEAT_INTERVAL: f64 = 0.06;

struct KeyboardInput {
    pressed: HashSet<KeyCode>,
    down: HashSet<KeyCode>,
    chars: Vec<char>,
    now: f64,
    clipboard: Option<String>,
}

impl KeyboardInput {
    fn ctrl(&self) -> bool {
        self.down.contains(&KeyCode::LeftControl) || self.down.contains(&KeyCode::RightControl)
    }

    fn alt(&self) -> bool {
        self.down.contains(&KeyCode::LeftAlt) || self.down.contains(&KeyCode::RightAlt)
    }
}

fn repeating_key(key: KeyCode, mode: Mode, input: &KeyboardInput) -> bool {
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

fn repeat_due(
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

fn poll_key_repeat(
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

fn handle_repeated_key(editor: &mut Editor, key: KeyCode) -> bool {
    match key {
        KeyCode::N | KeyCode::P => editor.control_key(if key == KeyCode::N { 'n' } else { 'p' }),
        KeyCode::Backspace if matches!(editor.mode, Mode::Normal | Mode::Visual | Mode::Insert) => {
            editor.move_back_character()
        }
        KeyCode::Delete if editor.mode == Mode::Insert => editor.delete_forward(),
        _ => return false,
    }
    true
}

fn promote_shifted_find_key(chars: &mut Vec<char>, lower: char, upper: char, pressed: bool) {
    if !pressed {
        return;
    }
    if let Some(index) = chars.iter().position(|&ch| ch == lower || ch == upper) {
        chars[index] = upper;
    } else {
        chars.insert(0, upper);
    }
}

pub fn handle_keyboard(
    editor: &mut Editor,
    font_size: &mut u16,
    word_wrap: &mut bool,
    vertical_motion: &mut Option<VerticalMotion>,
    key_repeat: &mut Option<KeyRepeat>,
) {
    let mut chars = Vec::new();
    while let Some(ch) = get_char_pressed() {
        chars.push(ch);
    }
    let mut input = KeyboardInput {
        pressed: get_keys_pressed(),
        down: get_keys_down(),
        chars,
        now: get_time(),
        clipboard: None,
    };
    if input.ctrl() && input.pressed.contains(&KeyCode::V) {
        input.clipboard = macroquad::miniquad::window::clipboard_get();
    }
    handle_keyboard_input(
        editor,
        font_size,
        word_wrap,
        vertical_motion,
        key_repeat,
        &input,
    );
}

fn handle_keyboard_input(
    editor: &mut Editor,
    font_size: &mut u16,
    word_wrap: &mut bool,
    vertical_motion: &mut Option<VerticalMotion>,
    key_repeat: &mut Option<KeyRepeat>,
    input: &KeyboardInput,
) {
    let is_key_pressed = |key| input.pressed.contains(&key);
    let is_key_down = |key| input.down.contains(&key);
    let get_time = || input.now;
    if editor.mode == Mode::CommandWindow {
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if is_key_pressed(KeyCode::Escape)
            || (ctrl && is_key_pressed(KeyCode::C))
            || is_key_pressed(KeyCode::Enter)
        {
            *vertical_motion = None;
            *key_repeat = None;
            if is_key_pressed(KeyCode::Escape) {
                editor.escape();
            } else if ctrl && is_key_pressed(KeyCode::C) {
                editor.control_key('c');
            } else {
                editor.finish_command_window(true);
            }
        } else if let Some(window) = editor.command_window.as_mut() {
            handle_keyboard_input(
                window,
                font_size,
                word_wrap,
                vertical_motion,
                key_repeat,
                input,
            );
            if window.quit {
                editor.escape();
            }
        }
        return;
    }
    let mut chars = input.chars.clone();
    let shift_down = is_key_down(KeyCode::LeftShift)
        || is_key_down(KeyCode::RightShift)
        || is_key_pressed(KeyCode::LeftShift)
        || is_key_pressed(KeyCode::RightShift);
    let shifted_f = shift_down && is_key_pressed(KeyCode::F);
    let shifted_t = shift_down && is_key_pressed(KeyCode::T);
    promote_shifted_find_key(&mut chars, 'f', 'F', shifted_f);
    promote_shifted_find_key(&mut chars, 't', 'T', shifted_t);
    let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let scroll_left = ctrl && is_key_down(KeyCode::H);
    let scroll_right = ctrl && is_key_down(KeyCode::L);
    if ctrl && !scroll_left && !scroll_right {
        editor.follow_cursor_horizontally();
    }
    if is_key_pressed(KeyCode::Escape) {
        *vertical_motion = None;
        editor.escape();
        return;
    }
    if ctrl && is_key_pressed(KeyCode::C) {
        *vertical_motion = None;
        editor.control_key('c');
        return;
    }
    if matches!(editor.mode, Mode::Normal | Mode::Visual) && editor.pending == Some('r') {
        if is_key_pressed(KeyCode::Enter) {
            editor.normal_key('\n');
            return;
        }
        if is_key_pressed(KeyCode::Tab) {
            editor.normal_key('\t');
            return;
        }
    }
    if matches!(editor.mode, Mode::ShellOutput | Mode::BufferList) {
        *key_repeat = None;
        return;
    }
    let repeated_key = poll_key_repeat(key_repeat, editor.mode, input);
    // Every repeatable key uses the same pulse. Physical OS repeat events must
    // not dispatch an additional action between our repeat ticks.
    let repeat_mode = editor.mode;
    let is_key_pressed = |key| {
        if repeating_key(key, repeat_mode, input) {
            repeated_key == Some(key)
        } else {
            input.pressed.contains(&key)
        }
    };
    let scroll_left = ctrl && is_key_pressed(KeyCode::H);
    let scroll_right = ctrl && is_key_pressed(KeyCode::L);
    if editor.mode == Mode::Command && is_key_pressed(KeyCode::Tab) {
        editor.complete_command();
        return;
    }
    if ctrl {
        for (key, ch) in [(KeyCode::X, 'x'), (KeyCode::F, 'f'), (KeyCode::L, 'l')] {
            if is_key_pressed(key) && editor.insert_control_key(ch) {
                *vertical_motion = None;
                return;
            }
        }
    }
    if repeated_key
        .is_some_and(|key| !(ctrl && key == KeyCode::Backspace) && handle_repeated_key(editor, key))
    {
        *vertical_motion = None;
        return;
    }
    if ctrl {
        if is_key_pressed(KeyCode::V) && matches!(editor.mode, Mode::Normal | Mode::Visual) {
            *vertical_motion = None;
            editor.control_key('v');
            return;
        }
        if is_key_pressed(KeyCode::Key6) && editor.mode == Mode::Normal {
            *vertical_motion = None;
            editor.buffer_action = Some(BufferAction::Last);
            return;
        }
        if (is_key_pressed(KeyCode::A) || is_key_pressed(KeyCode::X)) && editor.mode == Mode::Normal
        {
            *vertical_motion = None;
            let amount = editor.count.parse::<i128>().unwrap_or(1).clamp(1, 10_000);
            editor.count.clear();
            editor.adjust_number(if is_key_pressed(KeyCode::A) {
                amount
            } else {
                -amount
            });
            return;
        }
        if is_key_pressed(KeyCode::J) || is_key_pressed(KeyCode::K) {
            let repeat_key = if is_key_pressed(KeyCode::J) {
                KeyCode::J
            } else {
                KeyCode::K
            };
            *vertical_motion = Some(VerticalMotion {
                direction: if repeat_key == KeyCode::J { 1 } else { -1 },
                remaining: 5,
                next_step: get_time(),
                repeat_key,
            });
            return;
        }
        if [
            KeyCode::Minus,
            KeyCode::Equal,
            KeyCode::Key6,
            KeyCode::A,
            KeyCode::X,
            KeyCode::S,
            KeyCode::Backspace,
            KeyCode::W,
            KeyCode::V,
            KeyCode::R,
            KeyCode::D,
            KeyCode::U,
            KeyCode::E,
            KeyCode::Y,
            KeyCode::F,
            KeyCode::B,
            KeyCode::H,
            KeyCode::L,
            KeyCode::C,
        ]
        .into_iter()
        .any(is_key_pressed)
        {
            *vertical_motion = None;
        }
        if is_key_pressed(KeyCode::Minus) {
            *font_size = font_size.saturating_sub(2).max(12);
            editor.message = format!("Font size: {font_size}px");
        }
        if is_key_pressed(KeyCode::Equal) {
            *font_size = (*font_size + 2).min(42);
            editor.message = format!("Font size: {font_size}px");
        }
        if is_key_pressed(KeyCode::S) {
            editor.save(None);
        }
        if is_key_pressed(KeyCode::Backspace) && editor.mode == Mode::Insert {
            editor.delete_prev_word();
        }
        if is_key_pressed(KeyCode::W) && editor.mode == Mode::Insert {
            editor.delete_prev_word();
        }
        if is_key_pressed(KeyCode::V)
            && editor.mode == Mode::Insert
            && let Some(text) = input.clipboard.as_ref()
        {
            editor.insert_text(text);
        }
        if is_key_pressed(KeyCode::R) && editor.mode == Mode::Normal {
            editor.undo(true);
        }
        let scroll_up = repeated_key == Some(KeyCode::Y);
        let scroll_down = repeated_key == Some(KeyCode::E);
        let (rows, cols, _, _) = text_grid(*font_size);
        if [KeyCode::D, KeyCode::U, KeyCode::F, KeyCode::B]
            .into_iter()
            .any(is_key_pressed)
        {
            let full_page = is_key_pressed(KeyCode::F) || is_key_pressed(KeyCode::B);
            editor.move_by_display_rows(
                if is_key_pressed(KeyCode::D) || is_key_pressed(KeyCode::F) {
                    1
                } else {
                    -1
                },
                if full_page { rows } else { (rows / 2).max(1) },
                rows,
                cols,
                *word_wrap,
            );
        } else if scroll_up || scroll_down {
            let direction = if scroll_down { 1 } else { -1 };
            editor.scroll_vertical(direction, 1, rows, cols, *word_wrap);
        }
        if scroll_left || scroll_right {
            if *word_wrap {
                *word_wrap = false;
                editor.left = 0;
                editor.message = "Word wrap off · horizontal scrolling".into();
            }
            editor.scroll_horizontal(if scroll_left { -1 } else { 1 }, text_grid(*font_size).1);
        }
        return;
    }
    let alt = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
    if alt && matches!(editor.mode, Mode::Normal | Mode::Visual) {
        if is_key_pressed(KeyCode::H) {
            *vertical_motion = None;
            editor.change_indent(false);
            return;
        }
        if is_key_pressed(KeyCode::L) {
            *vertical_motion = None;
            editor.change_indent(true);
            return;
        }
        if is_key_pressed(KeyCode::J) {
            *vertical_motion = None;
            editor.move_line(1);
            return;
        }
        if is_key_pressed(KeyCode::K) {
            *vertical_motion = None;
            editor.move_line(-1);
            return;
        }
    }
    if !chars.is_empty()
        || is_key_pressed(KeyCode::Left)
        || is_key_pressed(KeyCode::Right)
        || is_key_pressed(KeyCode::Up)
        || is_key_pressed(KeyCode::Down)
        || is_key_pressed(KeyCode::Enter)
        || is_key_pressed(KeyCode::Backspace)
        || is_key_pressed(KeyCode::Delete)
        || is_key_pressed(KeyCode::PageUp)
        || is_key_pressed(KeyCode::PageDown)
        || is_key_pressed(KeyCode::Home)
        || is_key_pressed(KeyCode::End)
    {
        *vertical_motion = None;
    }
    match editor.mode {
        Mode::Insert => {
            if is_key_pressed(KeyCode::Enter) {
                editor.newline();
            }
            if is_key_pressed(KeyCode::Tab) {
                for _ in 0..4 {
                    editor.insert_char(' ');
                }
            }
        }
        Mode::Command | Mode::Search if is_key_pressed(KeyCode::Backspace) => {
            editor.edit_prompt(None);
        }
        _ => {}
    }
    for ch in chars {
        match editor.mode {
            Mode::Insert => editor.insert_char(ch),
            Mode::Command | Mode::Search => {
                if !ch.is_control() {
                    editor.edit_prompt(Some(ch));
                }
            }
            Mode::Normal | Mode::Visual => {
                if editor.mode == Mode::Normal
                    && editor.pending.is_none()
                    && matches!(ch, 'H' | 'M' | 'L')
                {
                    let (rows, cols, _, _) = text_grid(*font_size);
                    let offset = match ch {
                        'H' => 0,
                        'M' => rows / 2,
                        'L' => rows - 1,
                        _ => unreachable!(),
                    };
                    editor.move_to_screen_line(offset, cols, *word_wrap);
                } else if editor.pending == Some('z') && ch == 'z' {
                    let (rows, cols, _, _) = text_grid(*font_size);
                    editor.pending = None;
                    editor.count.clear();
                    editor.center_cursor(rows, cols, *word_wrap);
                } else {
                    editor.normal_key(ch);
                }
            }
            Mode::ShellOutput | Mode::BufferList | Mode::CommandWindow => {}
        }
    }
    if editor.mode == Mode::CommandWindow {
        *vertical_motion = None;
        return;
    }
    if is_key_pressed(KeyCode::Enter)
        && editor.mode == Mode::Normal
        && editor.is_directory_browser()
    {
        editor.open_directory_entry();
        return;
    }
    if matches!(editor.mode, Mode::Command | Mode::Search) {
        if is_key_pressed(KeyCode::Enter) {
            editor.submit_prompt();
        }
        return;
    }
    for (key, dx, dy) in [
        (KeyCode::Left, -1, 0),
        (KeyCode::Right, 1, 0),
        (KeyCode::Up, 0, -1),
        (KeyCode::Down, 0, 1),
    ] {
        if is_key_pressed(key) {
            editor.move_by(dx, dy, 1);
        }
    }
    if is_key_pressed(KeyCode::PageDown) {
        editor.move_by(0, 1, text_grid(*font_size).0 - 2);
    }
    if is_key_pressed(KeyCode::PageUp) {
        editor.move_by(0, -1, text_grid(*font_size).0 - 2);
    }
    if is_key_pressed(KeyCode::Home) {
        editor.cancel_search();
        editor.char_find_hints.clear();
        editor.follow_cursor_horizontally();
        editor.cursor.col = 0;
    }
    if is_key_pressed(KeyCode::End) {
        editor.cancel_search();
        editor.char_find_hints.clear();
        editor.follow_cursor_horizontally();
        editor.cursor.col = editor.lines[editor.cursor.row].len();
        editor.clamp();
    }
}

pub fn advance_vertical_motion(editor: &mut Editor, vertical_motion: &mut Option<VerticalMotion>) {
    let now = get_time();
    let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let repeat_held = ctrl
        && vertical_motion
            .as_ref()
            .is_some_and(|motion| is_key_down(motion.repeat_key));
    advance_vertical_motion_input(editor, vertical_motion, now, repeat_held);
}

fn advance_vertical_motion_input(
    editor: &mut Editor,
    vertical_motion: &mut Option<VerticalMotion>,
    now: f64,
    repeat_held: bool,
) {
    let Some(motion) = vertical_motion.as_mut() else {
        return;
    };
    while motion.remaining > 0 && now >= motion.next_step {
        editor.move_by(0, motion.direction, 1);
        motion.remaining -= 1;
        motion.next_step += VERTICAL_MOTION_STEP;
    }
    if motion.remaining == 0 {
        if repeat_held {
            motion.remaining = 5;
        } else {
            *vertical_motion = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct KeyboardHarness {
        font_size: u16,
        wrap: bool,
        motion: Option<VerticalMotion>,
        repeat: Option<KeyRepeat>,
    }

    impl KeyboardHarness {
        fn new() -> Self {
            Self {
                font_size: crate::config::BASE_FONT_SIZE,
                wrap: true,
                motion: None,
                repeat: None,
            }
        }

        fn frame(
            &mut self,
            editor: &mut Editor,
            now: f64,
            down: &[KeyCode],
            pressed: &[KeyCode],
            chars: &str,
        ) {
            let input = KeyboardInput {
                pressed: pressed.iter().copied().collect(),
                down: down.iter().copied().collect(),
                chars: chars.chars().collect(),
                now,
                clipboard: None,
            };
            handle_keyboard_input(
                editor,
                &mut self.font_size,
                &mut self.wrap,
                &mut self.motion,
                &mut self.repeat,
                &input,
            );
            let repeat_held = input.ctrl()
                && self
                    .motion
                    .as_ref()
                    .is_some_and(|motion| input.down.contains(&motion.repeat_key));
            advance_vertical_motion_input(editor, &mut self.motion, now, repeat_held);
        }

        fn hold(&mut self, editor: &mut Editor, key: KeyCode, modifiers: &[KeyCode]) {
            let mut down = modifiers.to_vec();
            down.push(key);
            let initial_press = [key];
            for (time, pressed) in [(0.0, true), (0.1, false), (0.35, false), (0.42, false)] {
                self.frame(
                    editor,
                    time,
                    &down,
                    if pressed { &initial_press } else { &[] },
                    "",
                );
            }
            self.frame(editor, 0.5, &[], &[], "");
        }
    }

    #[test]
    fn desktop_frames_repeat_backspace_as_one_character_movement_without_deletion() {
        let mut editor = Editor::new("aé日z", None);
        editor.cursor.col = 3;
        editor.begin_insert('i');
        let mut keyboard = KeyboardHarness::new();
        keyboard.frame(
            &mut editor,
            0.0,
            &[KeyCode::Backspace],
            &[KeyCode::Backspace],
            "",
        );
        assert_eq!(editor.cursor.col, 2);
        assert_eq!(editor.text(), "aé日z");
        keyboard.frame(&mut editor, 0.1, &[KeyCode::Backspace], &[], "");
        assert_eq!(editor.cursor.col, 2);
        keyboard.frame(&mut editor, 0.35, &[KeyCode::Backspace], &[], "");
        assert_eq!(editor.cursor.col, 1);
        keyboard.frame(&mut editor, 0.42, &[KeyCode::Backspace], &[], "");
        assert_eq!(editor.cursor.col, 0);
        keyboard.frame(&mut editor, 0.5, &[], &[], "");
        keyboard.frame(&mut editor, 1.0, &[], &[], "");
        assert_eq!(editor.cursor.col, 0);
        assert_eq!(editor.text(), "aé日z");
        assert!(!editor.dirty());
    }

    #[test]
    fn desktop_frames_repeat_delete_once_per_tick_and_ignore_duplicate_os_events() {
        let mut editor = Editor::new("abcdef", None);
        editor.begin_insert('i');
        let mut keyboard = KeyboardHarness::new();
        keyboard.frame(&mut editor, 0.0, &[KeyCode::Delete], &[KeyCode::Delete], "");
        assert_eq!(editor.text(), "bcdef");
        keyboard.frame(&mut editor, 0.35, &[KeyCode::Delete], &[], "");
        assert_eq!(editor.text(), "cdef");
        keyboard.frame(
            &mut editor,
            0.36,
            &[KeyCode::Delete],
            &[KeyCode::Delete],
            "",
        );
        assert_eq!(editor.text(), "cdef");
        keyboard.frame(&mut editor, 0.42, &[KeyCode::Delete], &[], "");
        assert_eq!(editor.text(), "def");
        keyboard.frame(&mut editor, 0.5, &[], &[], "");
        keyboard.frame(&mut editor, 1.0, &[], &[], "");
        assert_eq!(editor.text(), "def");
        editor.escape();
        editor.undo(false);
        assert_eq!(editor.text(), "abcdef");
    }

    #[test]
    fn desktop_frames_repeat_ctrl_n_p_and_stop_when_ctrl_is_released() {
        let mut editor = Editor::new("hello help helmet\nhe", None);
        editor.cursor.row = 1;
        editor.begin_insert('A');
        let mut keyboard = KeyboardHarness::new();
        for (time, pressed, selected) in [(0.0, true, 0), (0.35, false, 1), (0.42, false, 2)] {
            keyboard.frame(
                &mut editor,
                time,
                &[KeyCode::LeftControl, KeyCode::N],
                if pressed { &[KeyCode::N] } else { &[] },
                "",
            );
            assert_eq!(editor.completion_popup().unwrap().selected, selected);
        }
        keyboard.frame(&mut editor, 0.5, &[KeyCode::N], &[], "");
        assert_eq!(editor.completion_popup().unwrap().selected, 2);
        for (time, pressed, selected) in [(0.6, true, 1), (0.96, false, 0), (1.03, false, 2)] {
            keyboard.frame(
                &mut editor,
                time,
                &[KeyCode::RightControl, KeyCode::P],
                if pressed { &[KeyCode::P] } else { &[] },
                "",
            );
            assert_eq!(editor.completion_popup().unwrap().selected, selected);
        }
        keyboard.frame(&mut editor, 1.1, &[], &[], "");
        assert_eq!(editor.completion_popup().unwrap().selected, 2);
    }

    #[test]
    fn desktop_frames_repeat_arrows_and_page_keys() {
        let text = std::iter::repeat_n("abcdefghijk", 300)
            .collect::<Vec<_>>()
            .join("\n");
        for (key, expected) in [
            (KeyCode::Left, crate::editor::Pos { row: 5, col: 0 }),
            (KeyCode::Right, crate::editor::Pos { row: 5, col: 6 }),
            (KeyCode::Up, crate::editor::Pos { row: 2, col: 3 }),
            (KeyCode::Down, crate::editor::Pos { row: 8, col: 3 }),
        ] {
            let mut editor = Editor::new(&text, None);
            editor.cursor = crate::editor::Pos { row: 5, col: 3 };
            KeyboardHarness::new().hold(&mut editor, key, &[]);
            assert_eq!(editor.cursor, expected);
            assert!(!editor.dirty());
        }
        for key in [KeyCode::PageUp, KeyCode::PageDown] {
            let mut editor = Editor::new(&text, None);
            editor.cursor.row = 150;
            let mut keyboard = KeyboardHarness::new();
            keyboard.frame(&mut editor, 0.0, &[key], &[key], "");
            let first = editor.cursor.row;
            keyboard.frame(&mut editor, 0.35, &[key], &[], "");
            assert!(if key == KeyCode::PageUp {
                editor.cursor.row < first
            } else {
                editor.cursor.row > first
            });
        }
    }

    #[test]
    fn desktop_frames_repeat_ctrl_scrolling_and_font_shortcuts() {
        let text = std::iter::repeat_n("line", 300)
            .collect::<Vec<_>>()
            .join("\n");
        for (key, backwards) in [
            (KeyCode::D, false),
            (KeyCode::U, true),
            (KeyCode::F, false),
            (KeyCode::B, true),
        ] {
            let mut editor = Editor::new(&text, None);
            editor.cursor.row = 150;
            let mut keyboard = KeyboardHarness::new();
            keyboard.frame(&mut editor, 0.0, &[KeyCode::LeftControl, key], &[key], "");
            let first = editor.cursor.row;
            keyboard.frame(&mut editor, 0.35, &[KeyCode::LeftControl, key], &[], "");
            assert!(if backwards {
                editor.cursor.row < first
            } else {
                editor.cursor.row > first
            });
        }
        for (key, expected) in [(KeyCode::E, 53), (KeyCode::Y, 47)] {
            let mut editor = Editor::new(&text, None);
            editor.cursor.row = 50;
            editor.top = 50;
            KeyboardHarness::new().hold(&mut editor, key, &[KeyCode::LeftControl]);
            assert_eq!(editor.top, expected);
        }
        let mut editor = Editor::new(&"x".repeat(1000), None);
        let mut keyboard = KeyboardHarness::new();
        keyboard.hold(&mut editor, KeyCode::L, &[KeyCode::LeftControl]);
        assert!(!keyboard.wrap);
        let right = editor.left;
        assert!(right > text_grid(keyboard.font_size).1 / 2);
        keyboard.hold(&mut editor, KeyCode::H, &[KeyCode::LeftControl]);
        assert!(editor.left < right);
        keyboard.hold(&mut editor, KeyCode::Minus, &[KeyCode::LeftControl]);
        assert_eq!(
            keyboard.font_size,
            crate::config::BASE_FONT_SIZE.saturating_sub(6).max(12)
        );
    }

    #[test]
    fn desktop_frames_recover_a_hold_whose_press_edge_was_consumed() {
        let mut editor = Editor::new("abcdef", None);
        editor.begin_insert('A');
        let mut keyboard = KeyboardHarness::new();
        keyboard.frame(&mut editor, 0.0, &[KeyCode::Backspace], &[], "");
        assert_eq!(editor.cursor.col, 5);
        keyboard.frame(&mut editor, 0.35, &[KeyCode::Backspace], &[], "");
        assert_eq!(editor.cursor.col, 4);
        assert_eq!(editor.text(), "abcdef");
    }

    #[test]
    fn desktop_frames_repeat_modified_editing_and_plain_character_events() {
        for key in [KeyCode::W, KeyCode::Backspace] {
            let mut editor = Editor::new("one two three four", None);
            editor.begin_insert('A');
            KeyboardHarness::new().hold(&mut editor, key, &[KeyCode::LeftControl]);
            assert_eq!(editor.text(), "one ");
            assert_eq!(editor.cursor.col, 4);
            editor.escape();
            editor.undo(false);
            assert_eq!(editor.text(), "one two three four");
        }

        let mut editor = Editor::new("text", None);
        let mut keyboard = KeyboardHarness::new();
        keyboard.hold(&mut editor, KeyCode::L, &[KeyCode::LeftAlt]);
        assert_eq!(editor.text(), "            text");
        keyboard.hold(&mut editor, KeyCode::H, &[KeyCode::LeftAlt]);
        assert_eq!(editor.text(), "text");

        let mut editor = Editor::new("abcdef", None);
        for (now, pressed) in [(0.0, true), (0.35, false), (0.42, false)] {
            keyboard.frame(
                &mut editor,
                now,
                &[KeyCode::L],
                if pressed { &[KeyCode::L] } else { &[] },
                "l",
            );
        }
        assert_eq!(editor.cursor.col, 3);
        editor.begin_insert('i');
        for now in [0.5, 0.85, 0.92] {
            keyboard.frame(&mut editor, now, &[KeyCode::A], &[], "a");
        }
        assert_eq!(editor.text(), "abcaaadef");
    }

    #[test]
    fn desktop_frames_stop_held_backspace_on_escape_until_release() {
        let mut editor = Editor::new("abcdef", None);
        editor.begin_insert('A');
        let mut keyboard = KeyboardHarness::new();
        keyboard.frame(
            &mut editor,
            0.0,
            &[KeyCode::Backspace],
            &[KeyCode::Backspace],
            "",
        );
        keyboard.frame(
            &mut editor,
            0.1,
            &[KeyCode::Backspace, KeyCode::Escape],
            &[KeyCode::Escape],
            "",
        );
        let cursor_after_escape = editor.cursor;
        keyboard.frame(&mut editor, 0.5, &[KeyCode::Backspace], &[], "");
        assert_eq!(editor.cursor, cursor_after_escape);
        keyboard.frame(&mut editor, 0.6, &[], &[], "");
        keyboard.frame(
            &mut editor,
            0.7,
            &[KeyCode::Backspace],
            &[KeyCode::Backspace],
            "",
        );
        assert_eq!(editor.cursor.col, cursor_after_escape.col - 1);
        assert_eq!(editor.text(), "abcdef");
    }

    #[test]
    fn desktop_frames_keep_ctrl_j_k_animation_repeating_while_held() {
        let text = std::iter::repeat_n("line", 100)
            .collect::<Vec<_>>()
            .join("\n");
        for (key, expected) in [(KeyCode::J, 60), (KeyCode::K, 40)] {
            let mut editor = Editor::new(&text, None);
            editor.cursor.row = 50;
            let mut keyboard = KeyboardHarness::new();
            keyboard.frame(&mut editor, 0.0, &[KeyCode::LeftControl, key], &[key], "");
            for now in [0.1, 0.125, 0.2, 0.23] {
                keyboard.frame(&mut editor, now, &[KeyCode::LeftControl, key], &[], "");
            }
            assert_eq!(editor.cursor.row, expected);
            // Releasing Ctrl finishes the current five-line animation.
            keyboard.frame(&mut editor, 0.5, &[], &[], "");
            let after_release = editor.cursor;
            assert!(keyboard.motion.is_none());
            keyboard.frame(&mut editor, 1.0, &[], &[], "");
            assert_eq!(editor.cursor, after_release);
        }
    }

    #[test]
    fn desktop_frames_repeat_line_completion_prompt_backspace_tab_and_enter() {
        let mut editor = Editor::new("hello there\nhelp here\nhe", None);
        editor.cursor.row = 2;
        editor.begin_insert('A');
        let mut keyboard = KeyboardHarness::new();
        keyboard.frame(
            &mut editor,
            0.0,
            &[KeyCode::LeftControl, KeyCode::X],
            &[KeyCode::X],
            "",
        );
        keyboard.frame(
            &mut editor,
            0.1,
            &[KeyCode::LeftControl, KeyCode::L],
            &[KeyCode::L],
            "",
        );
        assert_eq!(editor.completion_popup().unwrap().selected, 0);
        keyboard.frame(
            &mut editor,
            0.46,
            &[KeyCode::LeftControl, KeyCode::L],
            &[],
            "",
        );
        assert_eq!(editor.completion_popup().unwrap().selected, 1);
        assert!(keyboard.wrap);
        let mut editor = Editor::new("", None);
        editor.begin_insert('i');
        let mut keyboard = KeyboardHarness::new();
        keyboard.hold(&mut editor, KeyCode::Tab, &[]);
        assert_eq!(editor.text(), " ".repeat(12));
        keyboard.hold(&mut editor, KeyCode::Enter, &[]);
        assert_eq!(editor.cursor.row, 3);
        editor.escape();
        editor.normal_key(':');
        editor.prompt = "theme".into();
        keyboard.hold(&mut editor, KeyCode::Backspace, &[]);
        assert_eq!(editor.prompt, "th");
    }

    #[test]
    fn held_keys_wait_then_repeat_without_double_counting_os_repeat_events() {
        let mut state = None;
        for key in [
            KeyCode::N,
            KeyCode::P,
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::E,
            KeyCode::Y,
        ] {
            assert_eq!(
                repeat_due(&mut state, Some(key), true, Mode::Normal, 0.0),
                Some(key)
            );
            assert_eq!(
                repeat_due(&mut state, Some(key), true, Mode::Normal, 0.1),
                None
            );
            assert_eq!(
                repeat_due(&mut state, Some(key), false, Mode::Normal, 0.34),
                None
            );
            assert_eq!(
                repeat_due(&mut state, Some(key), false, Mode::Normal, 0.35),
                Some(key)
            );
            assert_eq!(
                repeat_due(&mut state, Some(key), true, Mode::Normal, 0.36),
                None
            );
            assert_eq!(
                repeat_due(&mut state, Some(key), false, Mode::Normal, 0.42),
                Some(key)
            );
            assert_eq!(
                repeat_due(&mut state, None, false, Mode::Normal, 0.43),
                None
            );
            assert!(state.is_none());
        }
    }

    #[test]
    fn switching_keys_release_and_mode_changes_reset_the_repeat_delay() {
        let mut state = None;
        repeat_due(&mut state, Some(KeyCode::N), true, Mode::Insert, 0.0);
        assert_eq!(
            repeat_due(&mut state, Some(KeyCode::P), true, Mode::Insert, 0.1),
            Some(KeyCode::P)
        );
        assert_eq!(
            repeat_due(&mut state, Some(KeyCode::P), false, Mode::Insert, 0.35),
            None
        );
        assert_eq!(
            repeat_due(&mut state, Some(KeyCode::P), false, Mode::Insert, 0.46),
            Some(KeyCode::P)
        );
        assert_eq!(
            repeat_due(&mut state, Some(KeyCode::P), false, Mode::Normal, 0.8),
            None
        );
        assert_eq!(
            repeat_due(&mut state, Some(KeyCode::P), false, Mode::Normal, 0.85),
            None
        );
        assert_eq!(
            repeat_due(&mut state, Some(KeyCode::P), true, Mode::Normal, 0.9),
            Some(KeyCode::P)
        );
        repeat_due(&mut state, None, false, Mode::Normal, 1.0);
        assert_eq!(
            repeat_due(&mut state, Some(KeyCode::P), true, Mode::Normal, 1.01),
            Some(KeyCode::P)
        );
        assert_eq!(
            repeat_due(&mut state, Some(KeyCode::P), false, Mode::Normal, 1.1),
            None
        );
    }

    #[test]
    fn timed_ctrl_n_p_cycle_completion_and_move_in_normal_mode() {
        let mut editor = Editor::new("hello help helmet\nhe", None);
        editor.cursor.row = 1;
        editor.begin_insert('A');
        let mut state = None;
        for (time, pressed, selected) in [(0.0, true, 0), (0.35, false, 1), (0.42, false, 2)] {
            let key = repeat_due(&mut state, Some(KeyCode::N), pressed, editor.mode, time).unwrap();
            assert!(handle_repeated_key(&mut editor, key));
            assert_eq!(editor.completion_popup().unwrap().selected, selected);
        }
        for (time, pressed, selected) in [(0.5, true, 1), (0.86, false, 0)] {
            let key = repeat_due(&mut state, Some(KeyCode::P), pressed, editor.mode, time).unwrap();
            handle_repeated_key(&mut editor, key);
            assert_eq!(editor.completion_popup().unwrap().selected, selected);
        }
        editor.escape();
        handle_repeated_key(&mut editor, KeyCode::P);
        assert_eq!(editor.cursor.row, 0);
        handle_repeated_key(&mut editor, KeyCode::N);
        assert_eq!(editor.cursor.row, 1);
    }

    #[test]
    fn timed_insert_backspace_and_delete_repeat_and_undo_as_one_session() {
        for key in [KeyCode::Backspace, KeyCode::Delete] {
            let mut editor = Editor::new("abcdef", None);
            if key == KeyCode::Backspace {
                editor.cursor.col = 5;
            }
            editor.begin_insert('i');
            let mut state = None;
            for (time, pressed) in [(0.0, true), (0.35, false), (0.42, false)] {
                let fired = repeat_due(&mut state, Some(key), pressed, editor.mode, time).unwrap();
                handle_repeated_key(&mut editor, fired);
            }
            assert_eq!(
                editor.text(),
                if key == KeyCode::Backspace {
                    "abcdef"
                } else {
                    "def"
                }
            );
            assert_eq!(editor.mode, Mode::Insert);
            repeat_due(&mut state, None, false, editor.mode, 0.43);
            assert_eq!(repeat_due(&mut state, None, false, editor.mode, 1.0), None);
            editor.escape();
            editor.undo(false);
            assert_eq!(editor.text(), "abcdef");
        }
    }

    #[test]
    fn held_insert_backspace_moves_and_delete_joins_lines_then_stops_after_escape() {
        for key in [KeyCode::Backspace, KeyCode::Delete] {
            let mut editor = Editor::new("ab\ncd", None);
            editor.cursor = crate::editor::Pos {
                row: usize::from(key == KeyCode::Backspace),
                col: 1,
            };
            editor.begin_insert('i');
            let mut state = None;
            for (time, pressed) in [(0.0, true), (0.35, false), (0.42, false)] {
                let fired = repeat_due(&mut state, Some(key), pressed, editor.mode, time).unwrap();
                handle_repeated_key(&mut editor, fired);
            }
            assert_eq!(
                editor.text(),
                if key == KeyCode::Backspace {
                    "ab\ncd"
                } else {
                    "ad"
                }
            );
            editor.escape();
            assert_eq!(
                repeat_due(&mut state, Some(key), false, editor.mode, 0.5),
                None
            );
            assert_eq!(
                editor.text(),
                if key == KeyCode::Backspace {
                    "ab\ncd"
                } else {
                    "ad"
                }
            );
            editor.undo(false);
            assert_eq!(editor.text(), "ab\ncd");
        }
    }

    #[test]
    fn shifted_find_keys_are_preserved_even_when_character_events_are_missing() {
        let mut chars = vec!['f', 'x'];
        promote_shifted_find_key(&mut chars, 'f', 'F', true);
        assert_eq!(chars, vec!['F', 'x']);

        let mut chars = vec!['x'];
        promote_shifted_find_key(&mut chars, 't', 'T', true);
        assert_eq!(chars, vec!['T', 'x']);

        promote_shifted_find_key(&mut chars, 'f', 'F', false);
        assert_eq!(chars, vec!['T', 'x']);
    }
}
