use super::*;
#[test]
fn cursor_row_is_relative_to_the_scrolled_viewport() {
    assert_eq!(visible_cursor_row(12, 10, 5), Some(2));
    assert_eq!(visible_cursor_row(9, 10, 5), None);
    assert_eq!(visible_cursor_row(15, 10, 5), None);
}
