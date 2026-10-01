use crate::config::VERTICAL_MOTION_STEP;
use crate::editor::{Editor, Mode};
use crate::render::text_grid;
use macroquad::prelude::*;

pub struct VerticalMotion {
    direction: isize,
    remaining: usize,
    next_step: f64,
}

pub fn handle_keyboard(
    editor: &mut Editor,
    font_size: &mut u16,
    word_wrap: &mut bool,
    vertical_motion: &mut Option<VerticalMotion>,
) {
    let mut chars = Vec::new();
    while let Some(ch) = get_char_pressed() {
        chars.push(ch);
    }
    let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let scroll_left = ctrl && is_key_pressed(KeyCode::H);
    let scroll_right = ctrl && is_key_pressed(KeyCode::L);
    let scroll_up = ctrl
        && (is_key_pressed(KeyCode::Y) || is_key_pressed(KeyCode::U) || is_key_pressed(KeyCode::B));
    let scroll_down = ctrl
        && (is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::D) || is_key_pressed(KeyCode::F));
    if ctrl && !scroll_left && !scroll_right {
        editor.follow_cursor_horizontally();
    }
    if is_key_pressed(KeyCode::Escape) {
        *vertical_motion = None;
        editor.escape();
        return;
    }
    if matches!(editor.mode, Mode::ShellOutput | Mode::BufferList) {
        return;
    }
    if editor.mode == Mode::Command && is_key_pressed(KeyCode::Tab) {
        editor.complete_command();
        return;
    }
    if ctrl {
        if is_key_pressed(KeyCode::J) || is_key_pressed(KeyCode::K) {
            *vertical_motion = Some(VerticalMotion {
                direction: if is_key_pressed(KeyCode::J) { 1 } else { -1 },
                remaining: 5,
                next_step: get_time(),
            });
            return;
        }
        if [
            KeyCode::Minus,
            KeyCode::Equal,
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
            && let Some(text) = macroquad::miniquad::window::clipboard_get()
        {
            editor.insert_text(&text);
        }
        if is_key_pressed(KeyCode::R) && editor.mode == Mode::Normal {
            editor.undo(true);
        }
        let (rows, cols, _, _) = text_grid(*font_size);
        if scroll_up || scroll_down {
            let (direction, amount) = if is_key_pressed(KeyCode::E) {
                (1, 1)
            } else if is_key_pressed(KeyCode::Y) {
                (-1, 1)
            } else if is_key_pressed(KeyCode::D) {
                (1, rows / 2)
            } else if is_key_pressed(KeyCode::U) {
                (-1, rows / 2)
            } else if is_key_pressed(KeyCode::F) {
                (1, rows)
            } else {
                (-1, rows)
            };
            editor.scroll_vertical(direction, amount.max(1), rows, cols, *word_wrap);
        }
        if scroll_left || scroll_right {
            if *word_wrap {
                *word_wrap = false;
                editor.left = 0;
                editor.message = "Word wrap off · horizontal scrolling".into();
            }
            editor.scroll_horizontal(if scroll_left { -1 } else { 1 }, text_grid(*font_size).1);
        }
        if is_key_pressed(KeyCode::C) {
            editor.escape();
        }
        return;
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
            if is_key_pressed(KeyCode::Backspace) {
                editor.backspace();
            }
            if is_key_pressed(KeyCode::Delete) {
                editor.delete_forward();
            }
            if is_key_pressed(KeyCode::Tab) {
                for _ in 0..4 {
                    editor.insert_char(' ');
                }
            }
        }
        Mode::Command | Mode::Search if is_key_pressed(KeyCode::Backspace) => {
            editor.prompt.pop();
        }
        _ => {}
    }
    for ch in chars {
        match editor.mode {
            Mode::Insert => editor.insert_char(ch),
            Mode::Command | Mode::Search => {
                if !ch.is_control() {
                    editor.prompt.push(ch);
                }
            }
            Mode::Normal | Mode::Visual => editor.normal_key(ch),
            Mode::ShellOutput | Mode::BufferList => {}
        }
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
        editor.follow_cursor_horizontally();
        editor.cursor.col = 0;
    }
    if is_key_pressed(KeyCode::End) {
        editor.follow_cursor_horizontally();
        editor.cursor.col = editor.lines[editor.cursor.row].len();
        editor.clamp();
    }
}

pub fn advance_vertical_motion(editor: &mut Editor, vertical_motion: &mut Option<VerticalMotion>) {
    let now = get_time();
    let Some(motion) = vertical_motion.as_mut() else {
        return;
    };
    while motion.remaining > 0 && now >= motion.next_step {
        editor.move_by(0, motion.direction, 1);
        motion.remaining -= 1;
        motion.next_step += VERTICAL_MOTION_STEP;
    }
    if motion.remaining == 0 {
        *vertical_motion = None;
    }
}
