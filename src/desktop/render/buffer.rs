use super::{
    completion::draw_completion_popup,
    layout::{mode_color, mode_name, text_grid, visible_cursor_row},
    search::{search_highlights, search_prefix},
    text::{ui_label, ui_line, ui_rectangle},
};
use macroquad::prelude::*;
use vimloating::{
    config::*,
    editor::{Editor, Mode, Pos},
};
/// Per-line work is bounded to this many screens' worth of columns, so a single
/// pathologically long line cannot stall a frame.
const HORIZONTAL_SCAN_FACTOR: usize = 10;

fn is_comment_line(line: &[char], scan_cap: usize) -> bool {
    let mut seen = 0;
    for &ch in line.iter().take(scan_cap) {
        if ch.is_whitespace() {
            continue;
        }
        if ch != '/' {
            return false;
        }
        seen += 1;
        if seen == 2 {
            return true;
        }
    }
    false
}

pub fn draw_buffer(
    editor: &Editor,
    target: &RenderTarget,
    font: Option<&Font>,
    font_size: u16,
    word_wrap: bool,
    scale: f32,
) {
    if let Some(window) = &editor.command_window {
        draw_buffer(window, target, font, font_size, word_wrap, scale);
        return;
    }
    let palette = editor.theme.palette();
    set_camera(&Camera2D {
        render_target: Some(target.clone()),
        ..Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            target.texture.width(),
            target.texture.height(),
        ))
    });
    clear_background(palette.background);
    ui_line(90.0, TEXT_Y, 90.0, 871.0, 1.0, palette.divider, scale);

    let (rows, cols, cell_width, line_height) = text_grid(font_size);
    let baseline = line_height * 0.78;
    let display_rows = editor.display_window(editor.top, rows, cols, word_wrap);
    let horizontal_scan_cap = cols.saturating_mul(HORIZONTAL_SCAN_FACTOR);
    let search_text = if editor.mode == Mode::Search {
        &editor.prompt
    } else {
        &editor.search
    };
    let search: Vec<char> = search_text.chars().collect();
    let has_search = !search.is_empty();
    let search_prefix = search_prefix(&search);
    for visible in 0..rows {
        let y = TEXT_Y + visible as f32 * line_height;
        let Some(&(row, segment_start)) = display_rows.get(visible) else {
            ui_label("~", 48.0, y + baseline, 23, palette.gutter, font, scale);
            continue;
        };
        let folded_line;
        let line = if let Some(range) = editor.folded_range(row) {
            folded_line = format!(
                "+-- {} lines: {}",
                range.end - range.start + 1,
                editor.lines[row].iter().collect::<String>().trim()
            )
            .chars()
            .collect::<Vec<_>>();
            &folded_line
        } else {
            &editor.lines[row]
        };
        let visible_start = segment_start + if word_wrap { 0 } else { editor.left };
        let (indent, row_cols) = editor.display_row_layout(row, segment_start, cols, word_wrap);
        let text_x = TEXT_X + indent as f32 * cell_width;
        let visible_end = (visible_start + row_cols).min(line.len());
        let highlighted_cells = has_search.then(|| {
            search_highlights(
                line,
                &search,
                &search_prefix,
                visible_start,
                visible_end,
                editor.mode != Mode::Search && editor.search_whole_word,
            )
        });
        let cursor_segment = editor.display_segment_start(editor.cursor, cols, word_wrap);
        if row == editor.cursor.row && segment_start == cursor_segment {
            ui_rectangle(
                94.0,
                y - 1.0,
                1480.0,
                line_height,
                palette.current_line,
                scale,
            );
        }
        if editor.mode == Mode::Visual
            && editor.visual_linewise
            && editor.selection().0.row <= row
            && row <= editor.selection().1.row
        {
            ui_rectangle(
                94.0,
                y - 1.0,
                1480.0,
                line_height,
                palette.line_selection,
                scale,
            );
        }
        let num = if segment_start == 0 {
            editor
                .line_number(row)
                .map_or_else(|| "    ".into(), |number| format!("{number:>4}"))
        } else {
            if editor.number || editor.relative_number {
                "   >".into()
            } else {
                "    ".into()
            }
        };
        if editor.yank_blink_line(row) {
            ui_rectangle(
                text_x,
                y,
                row_cols as f32 * cell_width,
                line_height - 1.0,
                palette.search,
                scale,
            );
        } else if line.is_empty() && editor.yank_blink_cell(Pos { row, col: 0 }) {
            ui_rectangle(
                text_x,
                y,
                cell_width,
                line_height - 1.0,
                palette.search,
                scale,
            );
        }
        ui_label(
            &num,
            18.0,
            y + baseline,
            (20.0 * font_size as f32 / BASE_FONT_SIZE as f32)
                .round()
                .max(10.0) as u16,
            if row == editor.cursor.row && segment_start == cursor_segment {
                palette.accent
            } else {
                palette.muted
            },
            font,
            scale,
        );
        let comment = is_comment_line(line, horizontal_scan_cap);
        let mut quoted = false;
        for &ch in line.iter().take(visible_start.min(horizontal_scan_cap)) {
            if ch == '"' {
                quoted = !quoted;
            }
        }
        if editor.mode == Mode::Visual && !editor.visual_linewise {
            // Empty selected cells use the same softer shade as line padding.
            for col in visible_start.max(line.len())..visible_start + row_cols {
                if editor.selected_cell(Pos { row, col }) {
                    ui_rectangle(
                        text_x + (col - visible_start) as f32 * cell_width,
                        y,
                        cell_width,
                        line_height - 1.0,
                        palette.line_selection,
                        scale,
                    );
                }
            }
        }
        for col in visible_start..visible_end {
            let ch = line[col];
            if ch == '"' {
                quoted = !quoted;
            }
            let x = text_x + (col - visible_start) as f32 * cell_width;
            if highlighted_cells
                .as_ref()
                .is_some_and(|cells| cells[col - visible_start])
            {
                ui_rectangle(x, y, cell_width, line_height - 1.0, palette.search, scale);
            }
            let position = Pos { row, col };
            if editor.yank_blink_cell(position) {
                ui_rectangle(x, y, cell_width, line_height - 1.0, palette.search, scale);
            }
            let is_find_hint = editor.char_find_hints.binary_search(&position).is_ok();
            let is_find_target = editor.char_find_highlight == Some(position);
            if is_find_hint || is_find_target {
                ui_rectangle(x, y, cell_width, line_height - 1.0, palette.search, scale);
            }
            if editor.selected_cell(position) {
                ui_rectangle(
                    x,
                    y,
                    cell_width,
                    line_height - 1.0,
                    palette.selection,
                    scale,
                );
            }
            let color = if comment {
                palette.comment
            } else if quoted || ch == '"' {
                palette.string
            } else if ch.is_ascii_digit() {
                palette.number
            } else if matches!(
                ch,
                '{' | '}' | '(' | ')' | '[' | ']' | ';' | ':' | ',' | '.' | '!'
            ) {
                palette.punctuation
            } else {
                palette.text
            };
            if ch != '\t' {
                ui_label(
                    &ch.to_string(),
                    x,
                    y + baseline,
                    font_size,
                    color,
                    font,
                    scale,
                );
            } else {
                ui_label(
                    "→",
                    x,
                    y + baseline,
                    (font_size as f32 * 0.74) as u16,
                    palette.muted,
                    font,
                    scale,
                );
            }
            if is_find_hint || is_find_target {
                ui_line(
                    x,
                    y + line_height - 1.0,
                    x + cell_width,
                    y + line_height - 1.0,
                    1.5,
                    palette.accent,
                    scale,
                );
            }
        }
    }

    let cursor_segment = editor.display_segment_start(editor.cursor, cols, word_wrap);
    let cursor_index = editor.display_index(editor.cursor, cols, word_wrap);
    if let Some(cursor_row) = visible_cursor_row(cursor_index, editor.top, rows) {
        let visible_start = cursor_segment + if word_wrap { 0 } else { editor.left };
        let indent = editor
            .display_row_layout(editor.cursor.row, cursor_segment, cols, word_wrap)
            .0;
        let x =
            TEXT_X + (indent + editor.cursor.col.saturating_sub(visible_start)) as f32 * cell_width;
        let y = TEXT_Y + cursor_row as f32 * line_height;
        if editor.mode == Mode::Insert {
            let alpha = 0.65 + 0.35 * (get_time() as f32 * 4.0).sin().abs();
            ui_rectangle(
                x - 1.0,
                y,
                2.5,
                line_height - 1.0,
                Color {
                    a: alpha,
                    ..mode_color(editor.mode, palette)
                },
                scale,
            );
        } else if matches!(editor.mode, Mode::Normal | Mode::Visual) {
            ui_rectangle(
                x,
                y,
                cell_width,
                line_height - 1.0,
                mode_color(editor.mode, palette),
                scale,
            );
            if let Some(ch) = editor.lines[editor.cursor.row].get(editor.cursor.col) {
                ui_label(
                    &ch.to_string(),
                    x,
                    y + baseline,
                    font_size,
                    palette.cursor_text,
                    font,
                    scale,
                );
            }
        }
    }

    if matches!(editor.mode, Mode::ShellOutput | Mode::BufferList) {
        let panel_height = 480.0;
        let panel_y = 890.0 - panel_height - 12.0;
        let output_top = panel_y + 12.0;
        let panel_rows = ((panel_height - 12.0) / line_height).floor().max(1.0) as usize;
        ui_rectangle(
            94.0,
            panel_y,
            1480.0,
            panel_height,
            palette.background,
            scale,
        );
        if let Some(output) = &editor.output_view {
            let output_rows = output.lines().count();
            let visible_rows = if output_rows > panel_rows {
                panel_rows.saturating_sub(1)
            } else {
                panel_rows.min(rows)
            };
            for (visible, line) in output.lines().take(visible_rows).enumerate() {
                let line: String = line.chars().take(cols).collect();
                ui_label(
                    &line,
                    TEXT_X,
                    output_top + visible as f32 * line_height + baseline,
                    font_size,
                    if visible == 0 {
                        palette.accent
                    } else {
                        palette.text
                    },
                    font,
                    scale,
                );
            }
            if output_rows > panel_rows {
                ui_label(
                    "… output truncated · Esc to close",
                    TEXT_X,
                    output_top + visible_rows as f32 * line_height + baseline,
                    font_size,
                    palette.muted,
                    font,
                    scale,
                );
            }
        }
    }

    draw_completion_popup(editor, font, font_size, word_wrap, scale);
    ui_rectangle(0.0, 890.0, TEX_W as f32, 49.0, palette.status, scale);
    ui_rectangle(
        0.0,
        890.0,
        170.0,
        49.0,
        mode_color(editor.mode, palette),
        scale,
    );
    let mode_label = if editor.mode == Mode::Visual && editor.visual_blockwise {
        "VISUAL BLOCK"
    } else if editor.mode == Mode::Visual && editor.visual_linewise {
        "VISUAL LINE"
    } else {
        mode_name(editor.mode)
    };
    ui_label(
        mode_label,
        25.0,
        922.0,
        23,
        palette.cursor_text,
        font,
        scale,
    );
    let pending = format!("{}{}", editor.count, editor.pending.unwrap_or(' '));
    ui_label(&pending, 196.0, 922.0, 23, palette.accent, font, scale);
    let wrap_label = if word_wrap { "WRAP ON" } else { "WRAP OFF" };
    let status = format!("{font_size}px  ·  {wrap_label}");
    ui_label(&status, 295.0, 921.0, 19, palette.muted, font, scale);
    let position = format!(
        "Ln {}, Col {}    {} lines",
        editor.cursor.row + 1,
        editor.cursor.col + 1,
        editor.lines.len()
    );
    ui_label(&position, 1040.0, 922.0, 22, palette.text, font, scale);
    let prompt = if matches!(editor.mode, Mode::Command | Mode::Search) {
        format!(
            "{}{}|",
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
    let prompt: String = prompt
        .chars()
        .rev()
        .take(112)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    ui_label(
        &prompt,
        30.0,
        977.0,
        22,
        if matches!(editor.mode, Mode::Command | Mode::Search) {
            palette.accent
        } else {
            palette.muted
        },
        font,
        scale,
    );
    set_default_camera();
}
