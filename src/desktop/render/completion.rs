use super::{
    layout::{text_grid, visible_cursor_row},
    text::{ui_label, ui_rectangle},
};
use macroquad::prelude::*;
use vimloating::{
    config::{TEXT_X, TEXT_Y},
    editor::{Editor, Mode},
};

pub(super) fn draw_completion_popup(
    editor: &Editor,
    font: Option<&Font>,
    font_size: u16,
    word_wrap: bool,
    scale: f32,
) {
    let Some(popup) = editor.completion_popup() else {
        return;
    };
    let (rows, cols, cell_width, line_height) = text_grid(font_size);
    let (anchor_col, cursor_row) = if editor.mode == Mode::Command {
        (0, rows)
    } else {
        let index = editor.display_index(editor.cursor, cols, word_wrap);
        let Some(row) = visible_cursor_row(index, editor.top, rows) else {
            return;
        };
        let segment = editor.display_segment_start(editor.cursor, cols, word_wrap);
        let start = if word_wrap { segment } else { editor.left };
        let indent = editor
            .display_row_layout(editor.cursor.row, segment, cols, word_wrap)
            .0;
        (indent + popup.anchor.col.saturating_sub(start), row)
    };
    let Some(layout) = popup.layout(anchor_col, cursor_row, cols, rows) else {
        return;
    };
    let palette = editor.theme.palette();
    let x = TEXT_X + layout.x as f32 * cell_width;
    let y = TEXT_Y + layout.y as f32 * line_height;
    let width = layout.width as f32 * cell_width;
    let height = layout.height as f32 * line_height;
    ui_rectangle(x, y, width, height, palette.accent, scale);
    ui_rectangle(
        x + 1.0,
        y + 1.0,
        width - 2.0,
        height - 2.0,
        palette.status,
        scale,
    );
    let baseline = line_height * 0.78;
    ui_label(
        &popup
            .title()
            .chars()
            .take(layout.width - 2)
            .collect::<String>(),
        x + cell_width,
        y + baseline,
        font_size,
        palette.accent,
        font,
        scale,
    );
    for offset in 0..layout.visible {
        let index = layout.first + offset;
        let row_y = y + (offset + 1) as f32 * line_height;
        let selected = index == popup.selected;
        if selected {
            ui_rectangle(
                x + 1.0,
                row_y,
                width - 2.0,
                line_height,
                palette.accent,
                scale,
            );
        }
        // Draw each character in its cell, including visible tabs and control chars.
        for (col, ch) in popup
            .candidate_text(index, layout.width - 2)
            .chars()
            .enumerate()
        {
            let ch = if ch == '\t' {
                '→'
            } else if ch.is_control() {
                '�'
            } else {
                ch
            };
            ui_label(
                &ch.to_string(),
                x + (col + 1) as f32 * cell_width,
                row_y + baseline,
                font_size,
                if selected {
                    palette.cursor_text
                } else {
                    palette.text
                },
                font,
                scale,
            );
        }
    }
}
