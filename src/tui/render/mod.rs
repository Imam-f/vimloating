mod completion;

use super::{HELP, Ui, grid};
use crate::{
    config::Color as ThemeColor,
    editor::{Editor, Mode, Pos},
};
use completion::draw_completion_popup;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthChar;
pub(super) fn rgb(color: ThemeColor) -> Color {
    Color::Rgb(
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
    )
}

// The shared viewport counts Unicode scalar values, one per cell. Keep the
// terminal grid aligned without changing the actual characters in the buffer.
pub(super) fn cell(ch: char) -> char {
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

pub(super) fn draw(frame: &mut Frame, editor: &Editor, ui: &Ui) {
    if editor.mode == Mode::CommandWindow
        && !ui.help
        && let Some(window) = editor.command_window.as_ref()
    {
        draw(frame, window, ui);
        return;
    }
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
            let (indent, row_cols) = editor.display_row_layout(row, segment, cols, ui.wrap);
            let folded;
            let line = if let Some(range) = editor.folded_range(row) {
                folded = format!(
                    "{} … {} lines",
                    editor.lines[row].iter().collect::<String>(),
                    range.end - range.start
                )
                .chars()
                .collect::<Vec<_>>();
                &folded
            } else {
                &editor.lines[row]
            };
            let current = row == editor.cursor.row;
            let number = if segment == 0 {
                format!("{:>width$} ", row + 1, width = gutter.saturating_sub(1))
            } else {
                format!("{:>width$} ", ">", width = gutter.saturating_sub(1))
            };
            let mut spans = vec![Span::styled(
                number.chars().take(gutter).collect::<String>(),
                base.fg(rgb(if current {
                    palette.accent
                } else {
                    palette.muted
                })),
            )];
            let padding_style = if current {
                base.bg(rgb(palette.current_line))
            } else {
                base
            };
            spans.push(Span::styled(" ".repeat(indent), padding_style));
            let comment = line
                .iter()
                .position(|ch| !ch.is_whitespace())
                .is_some_and(|i| {
                    line[i] == '#' || (line[i] == '/' && line.get(i + 1) == Some(&'/'))
                });
            let mut quoted = line.iter().take(start).filter(|&&ch| ch == '"').count() % 2 == 1;
            let end = (start + row_cols).min(line.len());
            let mut highlights = vec![false; row_cols];
            if !search.is_empty() {
                let scan_start = start.saturating_sub(search.len() - 1);
                for index in scan_start..end {
                    if line.get(index..index.saturating_add(search.len()))
                        == Some(search.as_slice())
                        && (editor.mode == Mode::Search
                            || !editor.search_whole_word
                            || (!index
                                .checked_sub(1)
                                .and_then(|i| line.get(i))
                                .is_some_and(|ch| ch.is_alphanumeric() || *ch == '_')
                                && !line
                                    .get(index + search.len())
                                    .is_some_and(|ch| ch.is_alphanumeric() || *ch == '_')))
                    {
                        for col in index.max(start)..(index + search.len()).min(start + row_cols) {
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
                if editor.yank_blink_line(row) || editor.yank_blink_cell(pos) {
                    style = style.bg(rgb(palette.search));
                }
                if editor.selected_cell(pos) {
                    style = style.bg(rgb(if col < line.len() {
                        palette.selection
                    } else {
                        palette.line_selection
                    }));
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
            let segment = editor.display_segment_start(editor.cursor, cols, ui.wrap);
            let start = if ui.wrap { segment } else { editor.left };
            let indent = editor
                .display_row_layout(editor.cursor.row, segment, cols, ui.wrap)
                .0;
            let x = indent + editor.cursor.col.saturating_sub(start);
            if index >= editor.top && index - editor.top < content_height as usize && x < cols {
                frame.set_cursor_position((
                    area.x + gutter as u16 + x as u16,
                    area.y + (index - editor.top) as u16,
                ));
            }
        }
    }
    if !ui.help {
        draw_completion_popup(frame, editor, content, gutter, cols, ui.wrap);
    }
    if area.height >= 2 {
        let mode = match editor.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Visual if editor.visual_linewise => "VISUAL LINE",
            Mode::Visual if editor.visual_blockwise => "VISUAL BLOCK",
            Mode::Visual => "VISUAL",
            Mode::Command => "COMMAND",
            Mode::CommandWindow => "HISTORY",
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
