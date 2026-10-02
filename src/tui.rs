use crate::{
    config::{Color as ThemeColor, INSERT_IDLE_TIMEOUT},
    editor::{BufferAction, Editor, Mode, Pos, buffers::BufferList},
};
use crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::{
    io::{self, IsTerminal},
    time::{Duration, Instant},
};
use unicode_width::UnicodeWidthChar;

pub const WELCOME: &str = "// vimloating — terminal editor\n\n// i: insert · Esc: normal · h j k l: move · w b e: words\n// v / V: select · y / d: yank / delete · p: paste\n// u / Ctrl+R: undo / redo · / or ?: search · n / N: repeat\n// :w notes.rs: save · :e path: open · :Explore: browse\n// :ls: buffers · :bn / :bp: switch · :q: quit · :q!: discard\n// :theme everforest / solarized-blue: colors\n// Enter Enter: toggle wrap · F1: terminal controls\n\n";
const HELP: &str = "vimloating terminal controls\n\ni / a / I / A / o / O   Insert text\nEsc / Ctrl+C           Return to Normal mode\nh j k l / arrows       Move; counts supported\nw b e · gg G · f F t T  Vim motions and character-find hints\nv / V · y d x · p P    Select, yank, delete, paste\nu / Ctrl+R             Undo / redo\n/ or ? · n / N         Search forward/backward and repeat\n:w [path] · Ctrl+S     Save\n:e path · :Explore     Open file / directory browser\n:ls · :b id · :bn :bp  List / switch buffers\nCtrl+6 / Ctrl+^        Last active buffer (terminal dependent)\n:!command · :.!command Shell output / filter current line\n:q / :q! / :wq         Quit / discard / save and quit\nEnter Enter            Toggle word wrap in Normal mode\nCtrl+E / Y             Scroll one display line\nCtrl+D / U · F / B     Half-page / full-page movement\nCtrl+H / L             Horizontal scroll (disables wrap)\nCtrl+J / K             Move five lines (terminal dependent)\nAlt+J / K              Move current or selected lines\nTerminal paste         Paste in Insert mode\nOutput: j/k, PgUp/Down Scroll captured output / buffer list\nF1 / Esc               Close this help\n\nFont size and clipboard shortcuts are controlled by your terminal.";

/// Restores terminal state on normal return, I/O errors, and unwinding panics.
struct TerminalSession;

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let session = Self;
        execute!(io::stdout(), EnterAlternateScreen)?;
        // These enhancements are optional on older Windows console hosts.
        let _ = execute!(io::stdout(), EnableBracketedPaste);
        let _ = execute!(io::stdout(), SetCursorStyle::BlinkingBar);
        Ok(session)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableBracketedPaste);
        let _ = execute!(io::stdout(), SetCursorStyle::DefaultUserShape);
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

#[derive(Default)]
struct Ui {
    wrap: bool,
    help: bool,
    output_top: usize,
    last_enter: Option<Instant>,
}

fn grid(area: Rect, editor: &Editor) -> (usize, usize, usize) {
    let gutter =
        (editor.lines.len().to_string().len() + 2).min(area.width.saturating_sub(1) as usize);
    (
        area.height.saturating_sub(2).max(1) as usize,
        (area.width as usize).saturating_sub(gutter).max(1),
        gutter,
    )
}

pub fn run(editor: Editor) -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "an interactive terminal is required",
        ));
    }
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut buffers = BufferList::new(editor);
    let mut ui = Ui {
        wrap: true,
        ..Ui::default()
    };
    let mut last_activity = Instant::now();
    let mut redraw = true;
    loop {
        buffers.process_pending();
        buffers.protect_quit();
        if buffers.active().quit {
            break;
        }
        let size = terminal.size()?;
        let (rows, cols, _) = grid(Rect::new(0, 0, size.width, size.height), buffers.active());
        let editor = buffers.active_mut();
        let previous = (editor.cursor, editor.mode, editor.message.clone());
        editor.advance_search();
        if editor.mode == Mode::Insert
            && last_activity.elapsed().as_secs_f64() >= INSERT_IDLE_TIMEOUT
        {
            editor.escape();
            editor.message = "Idle timeout · returned to Normal mode".into();
        }
        editor.reveal_cursor(rows, cols, ui.wrap);
        if redraw || previous != (editor.cursor, editor.mode, editor.message.clone()) {
            terminal.draw(|frame| draw(frame, editor, &ui))?;
            redraw = false;
        }
        if event::poll(Duration::from_millis(30))? {
            last_activity = Instant::now();
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    handle_key(editor, &mut ui, key, rows, cols);
                    redraw = true;
                }
                Event::Paste(text) if !ui.help => {
                    match editor.mode {
                        Mode::Insert => {
                            editor.insert_text(&text.replace("\r\n", "\n").replace('\r', "\n"))
                        }
                        Mode::Command | Mode::Search => editor
                            .prompt
                            .extend(text.chars().filter(|ch| !ch.is_control())),
                        _ => editor.message = "Enter Insert mode to paste terminal text".into(),
                    };
                    redraw = true;
                }
                Event::Resize(_, _) => redraw = true,
                _ => {}
            }
        }
    }
    Ok(())
}

fn handle_key(editor: &mut Editor, ui: &mut Ui, key: KeyEvent, rows: usize, cols: usize) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if key.code == KeyCode::F(1) {
        ui.help = !ui.help;
        ui.output_top = 0;
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
    if ctrl {
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
                editor.prompt.pop();
            }
            KeyCode::Char('s') => {
                editor.save(None);
            }
            KeyCode::Char('r') if editor.mode == Mode::Normal => editor.undo(true),
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
            KeyCode::Char(ch @ ('j' | 'k')) => editor.move_by(0, if ch == 'j' { 1 } else { -1 }, 5),
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
                editor.prompt.pop();
            }
            KeyCode::Tab if editor.mode == Mode::Command => editor.complete_command(),
            KeyCode::Char(ch) => editor.prompt.push(ch),
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
                editor.normal_key(ch);
            }
        }
        _ => {}
    }
}

fn rgb(color: ThemeColor) -> Color {
    Color::Rgb(
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
    )
}

// The shared viewport counts Unicode scalar values, one per cell. Keep the
// terminal grid aligned without changing the actual characters in the buffer.
fn cell(ch: char) -> char {
    if ch == '\t' {
        '→'
    } else if ch.width() == Some(1) && !ch.is_control() {
        ch
    } else {
        '�'
    }
}

fn safe_text(text: &str) -> String {
    text.chars().map(cell).collect()
}

fn draw(frame: &mut Frame, editor: &Editor, ui: &Ui) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }
    let palette = editor.theme.palette();
    let base = Style::default()
        .fg(rgb(palette.text))
        .bg(rgb(palette.background));
    frame.render_widget(Paragraph::new("").style(base), area);
    let (rows, cols, gutter) = grid(area, editor);
    let content_height = area.height.saturating_sub(2);
    let content = Rect::new(area.x, area.y, area.width, content_height);
    if ui.help || matches!(editor.mode, Mode::ShellOutput | Mode::BufferList) {
        let text = if ui.help {
            HELP
        } else {
            editor.output_view.as_deref().unwrap_or("")
        };
        let lines: Vec<Line> = text
            .lines()
            .skip(ui.output_top)
            .take(content_height as usize)
            .map(|s| Line::raw(safe_text(s)))
            .collect();
        frame.render_widget(Paragraph::new(lines).style(base), content);
    } else {
        let search: Vec<char> = if editor.mode == Mode::Search {
            &editor.prompt
        } else {
            &editor.search
        }
        .chars()
        .collect();
        let display = editor.display_window(editor.top, rows, cols, ui.wrap);
        for visible in 0..content_height as usize {
            let row_area = Rect::new(area.x, area.y + visible as u16, area.width, 1);
            let Some(&(row, segment)) = display.get(visible) else {
                frame.render_widget(
                    Paragraph::new("~").style(base.fg(rgb(palette.muted))),
                    row_area,
                );
                continue;
            };
            let start = segment + if ui.wrap { 0 } else { editor.left };
            let line = &editor.lines[row];
            let current = row == editor.cursor.row;
            let number = if segment == 0 {
                format!("{:>width$} ", row + 1, width = gutter.saturating_sub(1))
            } else {
                format!("{:>width$} ", "↪", width = gutter.saturating_sub(1))
            };
            let mut spans = vec![Span::styled(
                number.chars().take(gutter).collect::<String>(),
                base.fg(rgb(if current {
                    palette.accent
                } else {
                    palette.muted
                })),
            )];
            let (a, b) = editor.selection();
            let comment = line
                .iter()
                .position(|ch| !ch.is_whitespace())
                .is_some_and(|i| {
                    line[i] == '#' || (line[i] == '/' && line.get(i + 1) == Some(&'/'))
                });
            let mut quoted = line.iter().take(start).filter(|&&ch| ch == '"').count() % 2 == 1;
            let end = (start + cols).min(line.len());
            let mut highlights = vec![false; cols];
            if !search.is_empty() {
                let scan_start = start.saturating_sub(search.len() - 1);
                for index in scan_start..end {
                    if line.get(index..index.saturating_add(search.len()))
                        == Some(search.as_slice())
                    {
                        for col in index.max(start)..(index + search.len()).min(start + cols) {
                            highlights[col - start] = true;
                        }
                    }
                }
            }
            for (offset, &highlighted) in highlights.iter().enumerate() {
                let col = start + offset;
                let pos = Pos { row, col };
                let ch = line.get(col).copied().unwrap_or(' ');
                if ch == '"' {
                    quoted = !quoted;
                }
                let fg = if comment {
                    palette.comment
                } else if quoted || ch == '"' {
                    palette.string
                } else if ch.is_ascii_digit() {
                    palette.number
                } else {
                    palette.text
                };
                let mut style = base.fg(rgb(fg));
                if current {
                    style = style.bg(rgb(palette.current_line));
                }
                if highlighted {
                    style = style.bg(rgb(palette.search));
                }
                if editor.char_find_highlight == Some(pos)
                    || editor.char_find_hints.binary_search(&pos).is_ok()
                {
                    style = style
                        .bg(rgb(palette.search))
                        .add_modifier(Modifier::UNDERLINED);
                }
                if editor.mode == Mode::Visual
                    && (if editor.visual_linewise {
                        a.row <= row && row <= b.row
                    } else {
                        a <= pos && pos <= b
                    })
                {
                    style = style.bg(rgb(palette.selection));
                }
                if pos == editor.cursor && matches!(editor.mode, Mode::Normal | Mode::Visual) {
                    style = style
                        .bg(rgb(if editor.mode == Mode::Visual {
                            palette.visual
                        } else {
                            palette.accent
                        }))
                        .fg(rgb(palette.cursor_text));
                }
                spans.push(Span::styled(cell(ch).to_string(), style));
            }
            frame.render_widget(Paragraph::new(Line::from(spans)), row_area);
        }
        if editor.mode == Mode::Insert {
            let index = editor.display_index(editor.cursor, cols, ui.wrap);
            let segment = if ui.wrap {
                editor.cursor.col / cols * cols
            } else {
                editor.left
            };
            let x = editor.cursor.col.saturating_sub(segment);
            if index >= editor.top && index - editor.top < content_height as usize && x < cols {
                frame.set_cursor_position((
                    area.x + gutter as u16 + x as u16,
                    area.y + (index - editor.top) as u16,
                ));
            }
        }
    }
    if area.height >= 2 {
        let mode = match editor.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Visual if editor.visual_linewise => "VISUAL LINE",
            Mode::Visual => "VISUAL",
            Mode::Command => "COMMAND",
            Mode::Search => "SEARCH",
            Mode::ShellOutput => "OUTPUT",
            Mode::BufferList => "BUFFERS",
        };
        let status = format!(
            " {mode}  {}{}  {}  {},{}  {}{}",
            editor.name(),
            if editor.dirty() { " [+]" } else { "" },
            if ui.wrap { "WRAP" } else { "NOWRAP" },
            editor.cursor.row + 1,
            editor.cursor.col + 1,
            editor.count,
            editor.pending.unwrap_or(' ')
        );
        frame.render_widget(
            Paragraph::new(safe_text(&status)).style(base.bg(rgb(palette.status))),
            Rect::new(area.x, area.y + area.height - 2, area.width, 1),
        );
    }
    let prompt_mode = matches!(editor.mode, Mode::Command | Mode::Search);
    let prompt = if ui.help {
        "F1 / Esc to close help".into()
    } else if prompt_mode {
        format!(
            "{}{}",
            if editor.mode == Mode::Command {
                ':'
            } else if editor.search_prompt_backwards {
                '?'
            } else {
                '/'
            },
            editor.prompt
        )
    } else {
        editor.message.clone()
    };
    // Keep the end of long commands visible, including the prompt cursor.
    let chars: Vec<char> = prompt.chars().collect();
    let start = if prompt_mode {
        chars
            .len()
            .saturating_sub(area.width.saturating_sub(1) as usize)
    } else {
        0
    };
    let text: String = chars
        .iter()
        .skip(start)
        .take(area.width as usize)
        .copied()
        .map(cell)
        .collect();
    frame.render_widget(
        Paragraph::new(text).style(base),
        Rect::new(area.x, area.y + area.height - 1, area.width, 1),
    );
    if prompt_mode && !ui.help {
        frame.set_cursor_position((
            area.x + (chars.len() - start).min(area.width.saturating_sub(1) as usize) as u16,
            area.y + area.height - 1,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn press(editor: &mut Editor, ui: &mut Ui, code: KeyCode) {
        handle_key(editor, ui, KeyEvent::new(code, KeyModifiers::NONE), 8, 20);
    }

    #[test]
    fn terminal_edit_search_undo_and_quit_protection_work_together() {
        let mut editor = Editor::new("hello world", None);
        let mut ui = Ui {
            wrap: true,
            ..Ui::default()
        };
        press(&mut editor, &mut ui, KeyCode::Char('A'));
        for ch in " café".chars() {
            press(&mut editor, &mut ui, KeyCode::Char(ch));
        }
        press(&mut editor, &mut ui, KeyCode::Esc);
        assert_eq!(editor.text(), "hello world café");
        press(&mut editor, &mut ui, KeyCode::Char('/'));
        for ch in "world".chars() {
            press(&mut editor, &mut ui, KeyCode::Char(ch));
        }
        press(&mut editor, &mut ui, KeyCode::Enter);
        editor.advance_search();
        assert_eq!(editor.cursor.col, 6);
        press(&mut editor, &mut ui, KeyCode::Char(':'));
        press(&mut editor, &mut ui, KeyCode::Char('q'));
        press(&mut editor, &mut ui, KeyCode::Enter);
        assert!(!editor.quit);
        press(&mut editor, &mut ui, KeyCode::Char('u'));
        assert_eq!(editor.text(), "hello world");
        handle_key(
            &mut editor,
            &mut ui,
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            8,
            20,
        );
        assert_eq!(editor.text(), "hello world café");
    }

    #[test]
    fn uppercase_find_targets_are_not_intercepted_as_screen_motions() {
        let mut editor = Editor::new("a H M L\nsecond line", None);
        let mut ui = Ui::default();
        press(&mut editor, &mut ui, KeyCode::Char('f'));
        press(&mut editor, &mut ui, KeyCode::Char('L'));
        assert_eq!(editor.cursor, Pos { row: 0, col: 6 });
        assert_eq!(editor.char_find_highlight, Some(editor.cursor));
    }

    #[test]
    fn legacy_control_encodings_edit_insert_text_and_submit_prompts() {
        let mut editor = Editor::new("", None);
        let mut ui = Ui::default();
        press(&mut editor, &mut ui, KeyCode::Char('i'));
        press(&mut editor, &mut ui, KeyCode::Char('a'));
        for ch in ['j', 'i', 'h'] {
            handle_key(
                &mut editor,
                &mut ui,
                KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL),
                8,
                20,
            );
        }
        assert_eq!(editor.text(), "a\n   ");
        handle_key(
            &mut editor,
            &mut ui,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL),
            8,
            20,
        );
        assert_eq!(editor.text(), "a\n   \n   ");
        press(&mut editor, &mut ui, KeyCode::Esc);
        press(&mut editor, &mut ui, KeyCode::Char(':'));
        for ch in "q!".chars() {
            press(&mut editor, &mut ui, KeyCode::Char(ch));
        }
        handle_key(
            &mut editor,
            &mut ui,
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::CONTROL),
            8,
            20,
        );
        assert!(editor.quit && editor.force_quit);
    }

    #[test]
    fn output_scroll_does_not_move_the_buffer_cursor_and_escape_restores_editor() {
        let mut editor = Editor::new("keep this", None);
        editor.mode = Mode::ShellOutput;
        editor.output_view = Some((0..30).map(|n| format!("output {n}\n")).collect());
        let mut ui = Ui::default();
        press(&mut editor, &mut ui, KeyCode::End);
        assert_eq!(ui.output_top, 22);
        assert_eq!(editor.cursor, Pos::default());
        let mut terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
        terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();
        let buffer = terminal.backend().buffer();
        let text: String = (0..30).map(|x| buffer[(x, 0)].symbol()).collect();
        assert!(text.contains("output 22"));
        press(&mut editor, &mut ui, KeyCode::Esc);
        assert_eq!(editor.mode, Mode::Normal);
        assert_eq!(editor.text(), "keep this");
        assert!(editor.output_view.is_none());
    }

    #[test]
    fn terminal_cells_highlights_and_prompt_remain_aligned_at_small_sizes() {
        let mut editor = Editor::new("a\t界\u{301}\u{1b}z", None);
        editor.cursor.col = 5;
        editor.search = "z".into();
        let ui = Ui {
            wrap: true,
            ..Ui::default()
        };
        let mut terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
        terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(4, 0)].symbol(), "→");
        assert_eq!(buffer[(5, 0)].symbol(), "�");
        assert_eq!(buffer[(8, 0)].symbol(), "z");
        assert_eq!(buffer[(8, 0)].bg, rgb(editor.theme.palette().accent));
        assert_eq!(editor.text(), "a\t界\u{301}\u{1b}z");
        editor.mode = Mode::Command;
        editor.prompt = "w a very long path with spaces.rs".into();
        for (width, height) in [(1, 1), (2, 2), (8, 4), (80, 24)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();
            let (x, y) = terminal.get_cursor_position().unwrap().into();
            assert!(x < width && y < height);
        }
    }
}
