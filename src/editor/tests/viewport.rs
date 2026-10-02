use super::super::*;

#[test]
fn center_cursor_centers_the_current_display_line_and_clamps_at_the_end() {
    let mut e = Editor::new(
        &(0..20)
            .map(|row| row.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
        None,
    );
    e.cursor.row = 10;
    e.center_cursor(5, 20, false);
    assert_eq!(e.top, 8);

    e.cursor.row = 19;
    e.center_cursor(5, 20, false);
    assert_eq!(e.top, 15);
}

#[test]
fn screen_line_motions_select_first_nonblank_visible_lines() {
    let mut e = Editor::new("  top\n  middle\n  bottom\n  last", None);
    e.top = 1;

    e.move_to_screen_line(0, 20, false);
    assert_eq!(e.cursor, Pos { row: 1, col: 2 });

    e.move_to_screen_line(1, 20, false);
    assert_eq!(e.cursor, Pos { row: 2, col: 2 });

    e.move_to_screen_line(2, 20, false);
    assert_eq!(e.cursor, Pos { row: 3, col: 2 });
}

#[test]
fn wrapped_visual_rows_track_long_lines_and_insert_end() {
    let mut e = Editor::new("abcdef\nx\n", None);
    assert_eq!(
        e.display_rows(3, true),
        vec![(0, 0), (0, 3), (1, 0), (2, 0)]
    );
    e.cursor = Pos { row: 0, col: 6 };
    e.mode = Mode::Insert;
    assert_eq!(
        e.display_rows(3, true),
        vec![(0, 0), (0, 3), (0, 6), (1, 0), (2, 0)]
    );
    e.reveal_cursor(2, 3, true);
    assert_eq!(e.top, 1);
    assert_eq!(e.left, 0);
}

#[test]
fn horizontal_scroll_moves_the_cursor_to_the_left_edge_when_it_would_be_hidden() {
    let mut e = Editor::new("abcdefghijklmnopqrstuvwxyz", None);
    e.scroll_horizontal(1, 10);
    e.reveal_cursor(5, 10, false);
    assert_eq!(e.left, 5);
    assert_eq!(e.cursor.col, 5);

    e.scroll_horizontal(1, 10);
    e.reveal_cursor(5, 10, false);
    assert_eq!(e.left, 10);
    assert_eq!(e.cursor.col, 10);
}

#[test]
fn vertical_scroll_can_overscroll_until_only_the_last_line_is_visible() {
    let mut e = Editor::new("0\n1\n2\n3\n4\n5\n6\n7\n8\n9", None);
    e.scroll_vertical(1, 100, 5, 20, false);

    assert_eq!(e.top, 9);
    assert_eq!(e.cursor.row, 9);
    assert_eq!(e.display_rows(20, false).len() - e.top, 1);

    e.scroll_vertical(-1, 1, 5, 20, false);
    assert_eq!(e.top, 8);
    assert_eq!(e.cursor.row, 9);
}

#[test]
fn display_page_motion_moves_the_cursor_by_the_requested_rows() {
    let text = (0..30)
        .map(|row| row.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let mut e = Editor::new(&text, None);
    e.cursor.row = 10;

    e.move_by_display_rows(1, 10, 20, 80, false);
    assert_eq!(e.cursor.row, 20);
    assert_eq!(e.top, 10);

    e.move_by_display_rows(-1, 10, 20, 80, false);
    assert_eq!(e.cursor.row, 10);
    assert_eq!(e.top, 0);
}

#[test]
fn display_page_motion_tracks_wrapped_screen_rows() {
    let mut e = Editor::new("abcdefghij\nnext", None);
    e.cursor = Pos { row: 0, col: 1 };

    e.move_by_display_rows(1, 1, 5, 4, true);

    assert_eq!(e.cursor, Pos { row: 0, col: 5 });
}

#[test]
fn full_page_motion_clamps_at_both_file_edges() {
    let text = (0..25)
        .map(|row| row.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let mut e = Editor::new(&text, None);

    e.move_by_display_rows(1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 20);
    e.move_by_display_rows(1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 24);
    e.move_by_display_rows(1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 24);

    e.move_by_display_rows(-1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 4);
    e.move_by_display_rows(-1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 0);
    e.move_by_display_rows(-1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 0);
}

#[test]
fn windowed_display_index_matches_the_full_row_map() {
    let mut editor = Editor::new(
        "short\nthis line is definitely longer than four columns\n\nx",
        None,
    );
    for cols in [1usize, 4, 7, 80] {
        for wrap in [true, false] {
            for mode in [Mode::Normal, Mode::Insert] {
                editor.mode = mode;
                let full = editor.display_rows(cols, wrap);
                assert_eq!(editor.display_total(cols, wrap), full.len());
                assert_eq!(editor.display_window(0, full.len(), cols, wrap), full);

                for start in 0..full.len() {
                    let slice = editor.display_window(start, full.len() - start, cols, wrap);
                    assert_eq!(slice, full[start..].to_vec());
                }

                for (index, &(row, col)) in full.iter().enumerate() {
                    let pos = Pos { row, col };
                    assert_eq!(editor.display_index(pos, cols, wrap), index);
                }
            }
        }
    }
}
