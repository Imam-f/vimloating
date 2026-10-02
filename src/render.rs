use crate::config::*;
use crate::editor::{Editor, Mode, Pos};
use crate::view::View;
use macroquad::prelude::*;

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

pub fn system_font() -> Option<Font> {
    for path in [
        "C:/Windows/Fonts/consola.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
    ] {
        if let Ok(bytes) = std::fs::read(path)
            && let Ok(font) = load_ttf_font_from_bytes(&bytes)
        {
            return Some(font);
        }
    }
    None
}

fn label(text: &str, x: f32, y: f32, size: u16, color: Color, font: Option<&Font>) {
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font,
            font_size: size,
            color,
            ..Default::default()
        },
    );
}

fn ui_label(text: &str, x: f32, y: f32, size: u16, color: Color, font: Option<&Font>, scale: f32) {
    label(
        text,
        x * scale,
        y * scale,
        (size as f32 * scale).round().clamp(1.0, u16::MAX as f32) as u16,
        color,
        font,
    );
}

fn ui_line(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color, scale: f32) {
    draw_line(
        x1 * scale,
        y1 * scale,
        x2 * scale,
        y2 * scale,
        thickness * scale,
        color,
    );
}

fn ui_rectangle(x: f32, y: f32, width: f32, height: f32, color: Color, scale: f32) {
    draw_rectangle(x * scale, y * scale, width * scale, height * scale, color);
}

fn mode_color(mode: Mode, palette: ThemePalette) -> Color {
    match mode {
        Mode::Insert => palette.insert,
        Mode::Visual => palette.visual,
        Mode::Command
        | Mode::CommandWindow
        | Mode::Search
        | Mode::ShellOutput
        | Mode::BufferList => palette.command,
        Mode::Normal => palette.accent,
    }
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "INSERT",
        Mode::Visual => "VISUAL",
        Mode::Command => "COMMAND",
        Mode::CommandWindow => "HISTORY",
        Mode::Search => "SEARCH",
        Mode::ShellOutput => "SHELL",
        Mode::BufferList => "BUFFERS",
    }
}

pub fn text_grid(font_size: u16) -> (usize, usize, f32, f32) {
    let scale = font_size as f32 / BASE_FONT_SIZE as f32;
    let cell_width = CELL * scale;
    let line_height = LINE * scale;
    let rows = ((890.0 - TEXT_Y - 8.0) / line_height).floor().max(1.0) as usize;
    let cols = ((TEX_W as f32 - TEXT_X - 30.0) / cell_width)
        .floor()
        .max(1.0) as usize;
    (rows, cols, cell_width, line_height)
}

fn visible_cursor_row(cursor_index: usize, top: usize, rows: usize) -> Option<usize> {
    let row = cursor_index.checked_sub(top)?;
    (row < rows).then_some(row)
}

fn search_prefix(needle: &[char]) -> Vec<usize> {
    let mut prefix = vec![0; needle.len()];
    let mut matched = 0;
    for index in 1..needle.len() {
        while matched > 0 && needle[index] != needle[matched] {
            matched = prefix[matched - 1];
        }
        if needle[index] == needle[matched] {
            matched += 1;
        }
        prefix[index] = matched;
    }
    prefix
}

fn search_highlights(
    line: &[char],
    needle: &[char],
    prefix: &[usize],
    visible_start: usize,
    visible_end: usize,
) -> Vec<bool> {
    let width = visible_end.saturating_sub(visible_start);
    let mut changes = vec![0isize; width + 1];
    if needle.is_empty() {
        return vec![false; width];
    }

    let mut matched = 0;
    let scan_start = visible_start.saturating_sub(needle.len());
    let scan_end = (visible_end + needle.len()).min(line.len());
    for (col, ch) in line
        .iter()
        .enumerate()
        .skip(scan_start)
        .take(scan_end.saturating_sub(scan_start))
    {
        while matched > 0 && *ch != needle[matched] {
            matched = prefix[matched - 1];
        }
        if *ch == needle[matched] {
            matched += 1;
        }
        if matched == needle.len() {
            let start = col + 1 - needle.len();
            let overlap_start = start.max(visible_start);
            let overlap_end = (start + needle.len()).min(visible_end);
            if overlap_start < overlap_end {
                changes[overlap_start - visible_start] += 1;
                changes[overlap_end - visible_start] -= 1;
            }
            matched = prefix[matched - 1];
        }
    }

    let mut active = 0;
    changes
        .into_iter()
        .take(width)
        .map(|change| {
            active += change;
            active > 0
        })
        .collect()
}

pub fn cursor_board_position(editor: &Editor, font_size: u16, word_wrap: bool) -> Vec2 {
    let (_, cols, cell_width, line_height) = text_grid(font_size);
    let segment = if word_wrap {
        editor.cursor.col / cols.max(1) * cols.max(1)
    } else {
        0
    };
    let display_index = editor.display_index(editor.cursor, cols, word_wrap);
    let row_in_view = display_index.saturating_sub(editor.top);
    let visible_start = segment + if word_wrap { 0 } else { editor.left };
    let x = TEXT_X
        + editor.cursor.col.saturating_sub(visible_start) as f32 * cell_width
        + cell_width * 0.5;
    let y = TEXT_Y + row_in_view as f32 * line_height + line_height * 0.5;
    vec2(
        (x / TEX_W as f32 - 0.5) * BOARD_W,
        (0.5 - y / TEX_H as f32) * BOARD_H,
    )
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
        let visible_end = (visible_start + cols).min(line.len());
        let highlighted_cells = has_search
            .then(|| search_highlights(line, &search, &search_prefix, visible_start, visible_end));
        let cursor_segment = if word_wrap {
            editor.cursor.col / cols * cols
        } else {
            0
        };
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
            format!("{:>4}", row + 1)
        } else {
            "   ↪".into()
        };
        if editor.yank_blink_line(row) {
            ui_rectangle(
                TEXT_X,
                y,
                cols as f32 * cell_width,
                line_height - 1.0,
                palette.search,
                scale,
            );
        } else if line.is_empty() && editor.yank_blink_cell(Pos { row, col: 0 }) {
            ui_rectangle(
                TEXT_X,
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
        if editor.mode == Mode::Visual && editor.visual_blockwise {
            let (a, b) = editor.selection();
            if a.row <= row && row <= b.row {
                let start = a.col.max(visible_start);
                let end = (b.col + 1).min(visible_start + cols);
                if start < end {
                    ui_rectangle(
                        TEXT_X + (start - visible_start) as f32 * cell_width,
                        y,
                        (end - start) as f32 * cell_width,
                        line_height - 1.0,
                        palette.selection,
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
            let x = TEXT_X + (col - visible_start) as f32 * cell_width;
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
            if editor.selected_cell(position) && !editor.visual_linewise {
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

    let cursor_segment = if word_wrap {
        editor.cursor.col / cols * cols
    } else {
        0
    };
    let cursor_index = editor.display_index(editor.cursor, cols, word_wrap);
    if let Some(cursor_row) = visible_cursor_row(cursor_index, editor.top, rows) {
        let visible_start = cursor_segment + if word_wrap { 0 } else { editor.left };
        let x = TEXT_X + editor.cursor.col.saturating_sub(visible_start) as f32 * cell_width;
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

pub fn board_mesh(texture: Texture2D) -> Mesh {
    let w = BOARD_W / 2.0;
    let h = BOARD_H / 2.0;
    Mesh {
        vertices: vec![
            Vertex::new(-w, h, 0.0, 0.0, 1.0, WHITE),
            Vertex::new(w, h, 0.0, 1.0, 1.0, WHITE),
            Vertex::new(w, -h, 0.0, 1.0, 0.0, WHITE),
            Vertex::new(-w, -h, 0.0, 0.0, 0.0, WHITE),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: Some(texture),
    }
}

pub fn editor_target_dimensions() -> (u32, u32) {
    let viewport_width = screen_width() * screen_dpi_scale();
    let width = (viewport_width * 2.0)
        .round()
        .clamp(MIN_RENDER_WIDTH as f32, MAX_RENDER_WIDTH as f32) as u32;
    let height = (width as f32 * TEX_H as f32 / TEX_W as f32)
        .round()
        .max(1.0) as u32;
    (width, height)
}

pub fn create_editor_target(width: u32, height: u32) -> RenderTarget {
    let target = render_target(width, height);
    target.texture.set_filter(FilterMode::Linear);
    target
}

pub fn draw_world(view: &View, board: &Mesh, show_floor: bool, theme: Theme) {
    let palette = theme.palette();
    clear_background(palette.world_background);
    set_camera(&view.camera());
    if show_floor {
        for i in -20..=20 {
            let p = i as f32 * 1.5;
            let color = if i == 0 {
                palette.floor_axis
            } else {
                palette.floor
            };
            draw_line_3d(vec3(p, -4.55, -30.0), vec3(p, -4.55, 12.0), color);
            draw_line_3d(vec3(-30.0, -4.55, p), vec3(30.0, -4.55, p), color);
        }
    }
    if !view.flat_only {
        draw_cube(
            vec3(0.0, 0.0, -0.13),
            vec3(BOARD_W + 0.13, BOARD_H + 0.13, 0.20),
            None,
            palette.board_frame,
        );
        draw_cube_wires(
            vec3(0.0, 0.0, -0.13),
            vec3(BOARD_W + 0.14, BOARD_H + 0.14, 0.22),
            palette.board_border,
        );
        draw_line_3d(
            vec3(-6.0, 3.81, 0.01),
            vec3(-3.4, 3.81, 0.01),
            palette.accent,
        );
    }
    draw_mesh(board);
    set_default_camera();
}

pub fn draw_2d_only(texture: &Texture2D, theme: Theme) {
    clear_background(theme.palette().background);
    draw_texture_ex(
        texture,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(screen_width(), screen_height())),
            flip_y: true,
            ..Default::default()
        },
    );
}

pub fn draw_overlay(help: bool, flat_only: bool, font: Option<&Font>, theme: Theme) {
    let palette = theme.palette();
    let height = screen_height();
    if flat_only {
        return;
    }
    if help {
        let x = (screen_width() - 357.0).max(20.0);
        let y = (height - 275.0).max(95.0);
        draw_rectangle(x, y, 329.0, 220.0, palette.help_background);
        draw_rectangle_lines(x, y, 329.0, 220.0, 1.0, palette.help_border);
        label(
            "MOVE THROUGH SPACE",
            x + 17.0,
            y + 28.0,
            15,
            palette.accent,
            font,
        );
        for (i, text) in [
            "wheel          zoom",
            "RMB outside pan; F4 unlocks orbit",
            "middle drag    pan",
            "F2 home · F5 2D-only on/off",
            "F4 flat/orbit · F5 2D-only on/off",
            "F3             toggle floor grid",
            "double Enter   toggle word wrap",
            "Ctrl+H / L     scroll horizontally; disables wrap",
            "F1             hide controls",
        ]
        .iter()
        .enumerate()
        {
            label(
                text,
                x + 17.0,
                y + 53.0 + i as f32 * 20.0,
                14,
                palette.text,
                font,
            );
        }
    }
    label(
        "F1 controls  ·  F4 flat/orbit · F5 2D-only · Ctrl +/- font",
        29.0,
        height - 24.0,
        14,
        palette.muted,
        font,
    );
}

#[cfg(test)]
mod tests {
    use super::{search_highlights, search_prefix, visible_cursor_row};

    #[test]
    fn cursor_row_is_relative_to_the_scrolled_viewport() {
        assert_eq!(visible_cursor_row(12, 10, 5), Some(2));
        assert_eq!(visible_cursor_row(9, 10, 5), None);
        assert_eq!(visible_cursor_row(15, 10, 5), None);
    }

    #[test]
    fn search_highlighting_covers_matches_overlapping_the_visible_columns() {
        let line: Vec<_> = "banana".chars().collect();
        let needle: Vec<_> = "ana".chars().collect();
        let prefix = search_prefix(&needle);

        assert_eq!(
            search_highlights(&line, &needle, &prefix, 2, 5),
            vec![true, true, true]
        );
    }
}
