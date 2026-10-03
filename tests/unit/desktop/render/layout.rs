use super::*;
#[test]
fn wrapped_cursor_position_includes_parent_indent_and_one_extra_level() {
    let (_, cols, cell_width, line_height) = text_grid(BASE_FONT_SIZE);
    let mut editor = Editor::new(&format!("    {}", "x".repeat(cols * 2)), None);
    editor.cursor.col = cols + 2;
    let point = cursor_board_position(&editor, BASE_FONT_SIZE, true);
    let x = (point.x / BOARD_W + 0.5) * TEX_W as f32;
    let y = (0.5 - point.y / BOARD_H) * TEX_H as f32;
    assert!((x - (TEXT_X + 10.5 * cell_width)).abs() < 0.001);
    assert!((y - (TEXT_Y + 1.5 * line_height)).abs() < 0.001);
}

#[test]
fn cursor_row_is_relative_to_the_scrolled_viewport() {
    assert_eq!(visible_cursor_row(12, 10, 5), Some(2));
    assert_eq!(visible_cursor_row(9, 10, 5), None);
    assert_eq!(visible_cursor_row(15, 10, 5), None);
}
