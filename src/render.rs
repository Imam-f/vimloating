use crate::config::*;
use crate::editor::{Editor, Mode, Pos};
use crate::view::View;
use macroquad::prelude::*;

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
        Mode::Command | Mode::Search | Mode::ShellOutput | Mode::BufferList => palette.command,
        Mode::Normal => palette.accent,
    }
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "INSERT",
        Mode::Visual => "VISUAL",
        Mode::Command => "COMMAND",
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

pub fn draw_buffer(
    editor: &Editor,
    target: &RenderTarget,
    font: Option<&Font>,
    font_size: u16,
    word_wrap: bool,
    scale: f32,
) {
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
    ui_line(90.0, 55.0, 90.0, 871.0, 1.0, palette.divider, scale);

    let (rows, cols, cell_width, line_height) = text_grid(font_size);
    let baseline = line_height * 0.78;
    let display_rows = editor.display_rows(cols, word_wrap);
    let search: Vec<char> = editor.search.chars().collect();
    for visible in 0..rows {
        let display_index = editor.top + visible;
        let y = TEXT_Y + visible as f32 * line_height;
        let Some(&(row, segment_start)) = display_rows.get(display_index) else {
            ui_label("~", 48.0, y + baseline, 23, palette.gutter, font, scale);
            continue;
        };
        let line = &editor.lines[row];
        let visible_start = segment_start + if word_wrap { 0 } else { editor.left };
        let visible_end = (visible_start + cols).min(line.len());
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
        let comment = line
            .iter()
            .skip_while(|c| c.is_whitespace())
            .take(2)
            .collect::<String>()
            == "//";
        let mut quoted = false;
        for (col, &ch) in line.iter().enumerate() {
            if ch == '"' {
                quoted = !quoted;
            }
            if col < visible_start || col >= visible_end {
                continue;
            }
            let x = TEXT_X + (col - visible_start) as f32 * cell_width;
            if !search.is_empty() {
                let starts = col.saturating_sub(search.len() - 1)..=col;
                if starts.clone().any(|start| {
                    start + search.len() <= line.len()
                        && line[start..start + search.len()] == search
                }) {
                    ui_rectangle(x, y, cell_width, line_height - 1.0, palette.search, scale);
                }
            }
            if editor.mode == Mode::Visual && !editor.visual_linewise {
                let (a, b) = editor.selection();
                let pos = Pos { row, col };
                if a <= pos && pos <= b {
                    ui_rectangle(
                        x,
                        y,
                        cell_width,
                        line_height - 1.0,
                        palette.selection,
                        scale,
                    );
                }
            }
            let color = if comment {
                palette.comment
            } else if quoted || ch == '"' {
                palette.string
            } else if ch.is_ascii_digit() {
                palette.number
            } else if "{}()[];:,.!".contains(ch) {
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
        }
    }

    let cursor_segment = if word_wrap {
        editor.cursor.col / cols * cols
    } else {
        0
    };
    if let Some(cursor_row) = display_rows
        .iter()
        .position(|&(row, segment)| row == editor.cursor.row && segment == cursor_segment)
        .filter(|&index| {
            index >= editor.top
                && index < editor.top + rows
                && (word_wrap
                    || (editor.cursor.col >= editor.left && editor.cursor.col < editor.left + cols))
        })
    {
        let visible_start = cursor_segment + if word_wrap { 0 } else { editor.left };
        let x = TEXT_X + editor.cursor.col.saturating_sub(visible_start) as f32 * cell_width;
        let y = TEXT_Y + (cursor_row - editor.top) as f32 * line_height;
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
        ui_rectangle(
            94.0,
            TEXT_Y - 12.0,
            1480.0,
            800.0,
            palette.background,
            scale,
        );
        if let Some(output) = &editor.output_view {
            let output_rows = output.lines().count();
            let visible_rows = if output_rows > rows {
                rows.saturating_sub(1)
            } else {
                rows
            };
            for (visible, line) in output.lines().take(visible_rows).enumerate() {
                let line: String = line.chars().take(cols).collect();
                ui_label(
                    &line,
                    TEXT_X,
                    TEXT_Y + visible as f32 * line_height + baseline,
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
            if output_rows > rows {
                ui_label(
                    "… output truncated · Esc to close",
                    TEXT_X,
                    TEXT_Y + visible_rows as f32 * line_height + baseline,
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
    let mode_label = if editor.mode == Mode::Visual && editor.visual_linewise {
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
            "F2             straight home view",
            "F4             flat-only / orbit",
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
        "F1 controls  ·  F4 flat-only/orbit  ·  Ctrl +/- font",
        29.0,
        height - 24.0,
        14,
        palette.muted,
        font,
    );
}
