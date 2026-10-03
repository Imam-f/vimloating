use super::{HELP, Ui};
use crate::editor::{BufferAction, Editor, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};
pub(super) fn handle_paste(editor: &mut Editor, text: &str) {
    match editor.mode {
        Mode::CommandWindow => {
            if let Some(window) = editor.command_window.as_mut() {
                handle_paste(window, text);
            }
        }
        Mode::Insert => editor.insert_text(&text.replace("\r\n", "\n").replace('\r', "\n")),
        Mode::Command | Mode::Search => {
            for ch in text.chars().filter(|ch| !ch.is_control()) {
                if !matches!(editor.mode, Mode::Command | Mode::Search) {
                    break;
                }
                editor.edit_prompt(Some(ch));
            }
        }
        _ => editor.message = "Enter Insert mode to paste terminal text".into(),
    }
}

pub(super) fn handle_key(
    editor: &mut Editor,
    ui: &mut Ui,
    key: KeyEvent,
    rows: usize,
    cols: usize,
) {
    if key.kind == KeyEventKind::Release {
        return;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if key.code == KeyCode::F(1) {
        ui.help = !ui.help;
        ui.output_top = 0;
        return;
    }
    if editor.mode == Mode::CommandWindow && !ui.help {
        if key.code == KeyCode::Esc {
            editor.escape();
        } else if ctrl && key.code == KeyCode::Char('c') {
            editor.control_key('c');
        } else if key.code == KeyCode::Enter
            || (ctrl && matches!(key.code, KeyCode::Char('j' | 'm')))
        {
            editor.finish_command_window(true);
        } else if let Some(window) = editor.command_window.as_mut() {
            handle_key(window, ui, key, rows, cols);
            if window.quit {
                editor.escape();
            }
        }
        ui.last_enter = None;
        return;
    }
    if key.code == KeyCode::Esc || (ctrl && key.code == KeyCode::Char('c')) {
        if ui.help {
            ui.help = false;
            ui.output_top = 0;
        } else {
            editor.escape();
            ui.output_top = 0;
        }
        ui.last_enter = None;
        return;
    }
    if ui.help || matches!(editor.mode, Mode::ShellOutput | Mode::BufferList) {
        let text = if ui.help {
            HELP
        } else {
            editor.output_view.as_deref().unwrap_or("")
        };
        let max = text.lines().count().saturating_sub(rows);
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => ui.output_top = (ui.output_top + 1).min(max),
            KeyCode::Char('k') | KeyCode::Up => ui.output_top = ui.output_top.saturating_sub(1),
            KeyCode::PageDown => ui.output_top = (ui.output_top + rows).min(max),
            KeyCode::PageUp => ui.output_top = ui.output_top.saturating_sub(rows),
            KeyCode::Home => ui.output_top = 0,
            KeyCode::End => ui.output_top = max,
            _ => {}
        }
        return;
    }
    if matches!(editor.mode, Mode::Normal | Mode::Visual)
        && editor.pending == Some('r')
        && let Some(ch) = match key.code {
            KeyCode::Enter => Some('\n'),
            KeyCode::Tab => Some('\t'),
            _ => None,
        }
    {
        editor.normal_key(ch);
        return;
    }
    if ctrl {
        if let KeyCode::Char(ch) = key.code
            && editor.insert_control_key(ch)
        {
            return;
        }
        match key.code {
            // Legacy terminal encodings and Windows console paste can report
            // newline, tab, and backspace as their control-key equivalents.
            KeyCode::Enter | KeyCode::Char('j' | 'm') if editor.mode == Mode::Insert => {
                editor.newline()
            }
            KeyCode::Char('i') if editor.mode == Mode::Insert => editor.insert_text("    "),
            KeyCode::Char('h') if editor.mode == Mode::Insert => editor.backspace(),
            KeyCode::Enter | KeyCode::Char('j' | 'm')
                if matches!(editor.mode, Mode::Command | Mode::Search) =>
            {
                editor.submit_prompt();
                ui.output_top = 0;
            }
            KeyCode::Char('h') if matches!(editor.mode, Mode::Command | Mode::Search) => {
                editor.edit_prompt(None);
            }
            KeyCode::Char(ch @ ('n' | 'p' | 'v')) => editor.control_key(ch),
            KeyCode::Char('s') => {
                editor.save(None);
            }
            KeyCode::Char('r') if editor.mode == Mode::Normal => editor.undo(true),
            KeyCode::Char(ch @ ('a' | 'x')) if editor.mode == Mode::Normal => {
                let amount = editor.count.parse::<i128>().unwrap_or(1).clamp(1, 10_000);
                editor.count.clear();
                editor.adjust_number(if ch == 'a' { amount } else { -amount });
            }
            KeyCode::Char('6' | '^') if editor.mode == Mode::Normal => {
                editor.buffer_action = Some(BufferAction::Last)
            }
            KeyCode::Char('w') | KeyCode::Backspace if editor.mode == Mode::Insert => {
                editor.delete_prev_word()
            }
            KeyCode::Char('e' | 'y') => editor.scroll_vertical(
                if key.code == KeyCode::Char('e') {
                    1
                } else {
                    -1
                },
                1,
                rows,
                cols,
                ui.wrap,
            ),
            KeyCode::Char(ch @ ('d' | 'u' | 'f' | 'b')) => editor.move_by_display_rows(
                if matches!(ch, 'd' | 'f') { 1 } else { -1 },
                if matches!(ch, 'f' | 'b') {
                    rows
                } else {
                    (rows / 2).max(1)
                },
                rows,
                cols,
                ui.wrap,
            ),
            KeyCode::Char(ch @ ('j' | 'k')) => {
                editor.scroll_vertical(if ch == 'j' { 1 } else { -1 }, 5, rows, cols, ui.wrap)
            }
            KeyCode::Char(ch @ ('h' | 'l')) => {
                ui.wrap = false;
                editor.scroll_horizontal(if ch == 'h' { -1 } else { 1 }, cols);
                editor.message = "Word wrap off · horizontal scrolling".into();
            }
            _ => {}
        }
        return;
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        if matches!(editor.mode, Mode::Normal | Mode::Visual) {
            match key.code {
                KeyCode::Char('h' | 'H') => editor.change_indent(false),
                KeyCode::Char('l' | 'L') => editor.change_indent(true),
                KeyCode::Char('j' | 'J') => editor.move_line(1),
                KeyCode::Char('k' | 'K') => editor.move_line(-1),
                _ => {}
            }
        }
        return;
    }
    if key.code != KeyCode::Enter {
        ui.last_enter = None;
    }
    if matches!(editor.mode, Mode::Command | Mode::Search) {
        match key.code {
            KeyCode::Enter => {
                editor.submit_prompt();
                ui.output_top = 0;
            }
            KeyCode::Backspace => {
                editor.edit_prompt(None);
            }
            KeyCode::Tab if editor.mode == Mode::Command => editor.complete_command(),
            KeyCode::Char(ch) => editor.edit_prompt(Some(ch)),
            _ => {}
        }
        return;
    }
    match key.code {
        KeyCode::Left => editor.move_by(-1, 0, 1),
        KeyCode::Right => editor.move_by(1, 0, 1),
        KeyCode::Up => editor.move_by(0, -1, 1),
        KeyCode::Down => editor.move_by(0, 1, 1),
        KeyCode::PageDown => {
            editor.move_by_display_rows(1, rows.saturating_sub(2).max(1), rows, cols, ui.wrap)
        }
        KeyCode::PageUp => {
            editor.move_by_display_rows(-1, rows.saturating_sub(2).max(1), rows, cols, ui.wrap)
        }
        KeyCode::Home | KeyCode::End => {
            editor.cancel_search();
            editor.char_find_hints.clear();
            editor.follow_cursor_horizontally();
            editor.cursor.col = if key.code == KeyCode::Home {
                0
            } else {
                editor.lines[editor.cursor.row].len()
            };
            editor.clamp();
        }
        KeyCode::Enter if editor.mode == Mode::Insert => editor.newline(),
        KeyCode::Backspace if editor.mode == Mode::Insert => editor.backspace(),
        KeyCode::Backspace if matches!(editor.mode, Mode::Normal | Mode::Visual) => {
            editor.move_back_character()
        }
        KeyCode::Delete if editor.mode == Mode::Insert => editor.delete_forward(),
        KeyCode::Tab if editor.mode == Mode::Insert => editor.insert_text("    "),
        KeyCode::Enter if editor.mode == Mode::Normal && editor.is_directory_browser() => {
            editor.open_directory_entry()
        }
        KeyCode::Enter if editor.mode == Mode::Normal => {
            let now = Instant::now();
            if ui
                .last_enter
                .is_some_and(|last| now.duration_since(last) <= Duration::from_millis(450))
            {
                ui.wrap = !ui.wrap;
                editor.left = 0;
                editor.message = format!("Word wrap {}", if ui.wrap { "on" } else { "off" });
                ui.last_enter = None;
            } else {
                ui.last_enter = Some(now);
            }
        }
        KeyCode::Char(ch) if editor.mode == Mode::Insert => editor.insert_char(ch),
        KeyCode::Char(ch) => {
            if editor.pending == Some('z') && ch == 'z' {
                editor.pending = None;
                editor.count.clear();
                editor.center_cursor(rows, cols, ui.wrap);
            } else if editor.mode == Mode::Normal
                && matches!(ch, 'H' | 'M' | 'L')
                && editor.pending.is_none()
            {
                editor.move_to_screen_line(
                    match ch {
                        'H' => 0,
                        'M' => rows / 2,
                        _ => rows - 1,
                    },
                    cols,
                    ui.wrap,
                );
            } else {
                editor.normal_key_with_viewport(ch, cols, ui.wrap);
            }
        }
        _ => {}
    }
}
