use macroquad::prelude::*;
use vimloating::{
    config::*,
    editor::{Editor, Mode},
};

pub(super) fn mode_color(mode: Mode, palette: ThemePalette) -> Color {
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

pub(super) fn mode_name(mode: Mode) -> &'static str {
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

pub(super) fn visible_cursor_row(cursor_index: usize, top: usize, rows: usize) -> Option<usize> {
    let row = cursor_index.checked_sub(top)?;
    (row < rows).then_some(row)
}

pub fn cursor_board_position(editor: &Editor, font_size: u16, word_wrap: bool) -> Vec2 {
    let (_, cols, cell_width, line_height) = text_grid(font_size);
    let segment = editor.display_segment_start(editor.cursor, cols, word_wrap);
    let display_index = editor.display_index(editor.cursor, cols, word_wrap);
    let row_in_view = display_index.saturating_sub(editor.top);
    let visible_start = segment + if word_wrap { 0 } else { editor.left };
    let indent = editor
        .display_row_layout(editor.cursor.row, segment, cols, word_wrap)
        .0;
    let x = TEXT_X
        + (indent + editor.cursor.col.saturating_sub(visible_start)) as f32 * cell_width
        + cell_width * 0.5;
    let y = TEXT_Y + row_in_view as f32 * line_height + line_height * 0.5;
    vec2(
        (x / TEX_W as f32 - 0.5) * BOARD_W,
        (0.5 - y / TEX_H as f32) * BOARD_H,
    )
}

#[cfg(test)]
#[path = "../../../tests/unit/desktop/render/layout.rs"]
mod tests;
