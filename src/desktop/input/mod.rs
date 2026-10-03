mod motion;
mod repeat;

use super::render::text_grid;
use macroquad::prelude::*;
pub use motion::{VerticalMotion, advance_vertical_motion};
pub use repeat::KeyRepeat;
use repeat::{poll_key_repeat, repeating_key};
use std::collections::HashSet;
use vimloating::editor::{BufferAction, Editor, Mode};
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

#[cfg(test)]
mod tests;
