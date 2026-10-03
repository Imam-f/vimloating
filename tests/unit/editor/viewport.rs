use super::super::*;

#[test]
fn j_k_follow_wrapped_rows_with_counts_and_restore_columns_after_short_rows() {
    let mut editor = Editor::new(
        "  abcdefghijklmnopqrstuvwxyz\nx\n  abcdefghijklmnopqrstuvwxyz",
        None,
    );
    editor.cursor.col = 9;
    editor.normal_key_with_viewport('k', 16, true);
    assert_eq!(editor.cursor, Pos { row: 0, col: 9 });
    for (key, expected) in [
        ('j', Pos { row: 0, col: 19 }),
        ('j', Pos { row: 0, col: 27 }),
        ('j', Pos { row: 1, col: 0 }),
        ('j', Pos { row: 2, col: 9 }),
        ('k', Pos { row: 1, col: 0 }),
        ('k', Pos { row: 0, col: 27 }),
        ('k', Pos { row: 0, col: 19 }),
        ('k', Pos { row: 0, col: 9 }),
    ] {
        editor.normal_key_with_viewport(key, 16, true);
        assert_eq!(editor.cursor, expected);
    }
    for key in "2j".chars() {
        editor.normal_key_with_viewport(key, 16, true);
    }
    assert_eq!(editor.cursor, Pos { row: 0, col: 27 });
    for key in "2k".chars() {
        editor.normal_key_with_viewport(key, 16, true);
    }
    assert_eq!(editor.cursor, Pos { row: 0, col: 9 });
    assert_eq!(editor.top, 0);
    assert!(!editor.dirty());
}

#[test]
fn gj_gk_move_source_lines_with_counts_and_preserve_source_columns() {
    let mut editor = Editor::new(
        "  abcdefghijklmnopqrstuvwxyz\nx\n  abcdefghijklmnopqrstuvwxyz",
        None,
    );
    editor.cursor.col = 9;
    for key in "gj".chars() {
        editor.normal_key_with_viewport(key, 16, true);
    }
    assert_eq!(editor.cursor, Pos { row: 1, col: 0 });
    for key in "gj".chars() {
        editor.normal_key_with_viewport(key, 16, true);
    }
    assert_eq!(editor.cursor, Pos { row: 2, col: 9 });
    for key in "2gk".chars() {
        editor.normal_key_with_viewport(key, 16, true);
    }
    assert_eq!(editor.cursor, Pos { row: 0, col: 9 });
    for key in "g2j".chars() {
        editor.normal_key_with_viewport(key, 16, true);
    }
    assert_eq!(editor.cursor, Pos { row: 2, col: 9 });
    assert_eq!(editor.pending, None);
    assert!(editor.count.is_empty());
}

#[test]
fn switching_between_display_and_source_motions_uses_the_correct_column() {
    let mut editor = Editor::new(
        "  abcdefghijklmnopqrstuvwxyz\n  abcdefghijklmnopqrstuvwxyz",
        None,
    );
    editor.cursor.col = 9;
    for key in "jgjgkj".chars() {
        editor.normal_key_with_viewport(key, 16, true);
    }
    assert_eq!(editor.cursor, Pos { row: 0, col: 27 });
    // Horizontal movement resets the desired display column.
    editor.normal_key_with_viewport('h', 16, true);
    editor.normal_key_with_viewport('k', 16, true);
    assert_eq!(editor.cursor, Pos { row: 0, col: 16 });
}

#[test]
fn vertical_motions_work_without_wrap_and_extend_visual_selections() {
    let text = "  abcdefghijklmnopqrstuvwxyz\nsecond line\nthird line";
    for keys in ["2j2k", "2gj2gk"] {
        let mut editor = Editor::new(text, None);
        editor.cursor.col = 9;
        for key in keys.chars() {
            editor.normal_key_with_viewport(key, 16, false);
        }
        assert_eq!(editor.cursor, Pos { row: 0, col: 9 });
    }
    for visual in ['v', 'V'] {
        let mut editor = Editor::new(text, None);
        editor.cursor.col = 9;
        editor.normal_key_with_viewport(visual, 16, true);
        editor.normal_key_with_viewport('j', 16, true);
        assert_eq!(editor.cursor, Pos { row: 0, col: 19 });
        assert_eq!(editor.anchor, Pos { row: 0, col: 9 });
        for key in "gj".chars() {
            editor.normal_key_with_viewport(key, 16, true);
        }
        assert_eq!(editor.cursor, Pos { row: 1, col: 10 });
        assert_eq!(editor.mode, Mode::Visual);
        assert_eq!(editor.anchor, Pos { row: 0, col: 9 });
        assert!(!editor.dirty());
    }
}

#[test]
fn wrapped_continuations_indent_beyond_the_parent_without_changing_text() {
    let text = "  abcdefghijklmnopqrstuvwxyz\n\tαβγδεζηθικλμνξοπρστυφχψω";
    let editor = Editor::new(text, None);
    assert_eq!(editor.display_row_layout(0, 0, 16, true), (0, 16));
    assert_eq!(editor.display_row_layout(0, 16, 16, true), (6, 10));
    assert_eq!(editor.display_row_layout(1, 16, 16, true), (5, 11));
    assert_eq!(
        editor.display_window(0, 10, 16, true),
        vec![(0, 0), (0, 16), (0, 26), (1, 0), (1, 16)]
    );
    assert_eq!(editor.display_row_layout(0, 16, 16, false), (0, 16));
    assert_eq!(editor.display_total(16, false), 2);
    assert_eq!(editor.text(), text);
    assert!(!editor.dirty());
}

#[test]
fn wrapped_rows_preserve_every_character_even_in_narrow_viewports() {
    let editor = Editor::new("          αβγδεζηθικλμνξοπρστυφχψω\nabc", None);
    for cols in [1, 2, 4, 12, 16, 80] {
        let rows = editor.display_window(0, 100, cols, true);
        for row in 0..editor.lines.len() {
            let mut recovered = Vec::new();
            for &(source_row, start) in &rows {
                if source_row != row {
                    continue;
                }
                let (indent, width) = editor.display_row_layout(row, start, cols, true);
                assert!(width > 0);
                assert_eq!(indent + width, cols);
                let end = (start + width).min(editor.lines[row].len());
                for col in start..end {
                    let pos = Pos { row, col };
                    assert_eq!(editor.display_segment_start(pos, cols, true), start);
                    assert_eq!(
                        editor.position_at_display_column(
                            row,
                            start,
                            indent + col - start,
                            cols,
                            true
                        ),
                        pos
                    );
                }
                recovered.extend_from_slice(&editor.lines[row][start..end]);
            }
            assert_eq!(recovered, editor.lines[row]);
        }
    }
}

#[test]
fn insert_cursor_at_wrap_boundaries_uses_an_indented_empty_row() {
    for (len, expected_index, expected_start) in [(12, 1, 12), (20, 2, 20), (21, 2, 20)] {
        let mut editor = Editor::new(&"a".repeat(len), None);
        editor.begin_insert('A');
        assert_eq!(editor.display_total(12, true), expected_index + 1);
        assert_eq!(
            editor.display_index(editor.cursor, 12, true),
            expected_index
        );
        assert_eq!(
            editor.display_segment_start(editor.cursor, 12, true),
            expected_start
        );
        assert_eq!(
            editor.display_window(expected_index, 1, 12, true),
            vec![(0, expected_start)]
        );
    }
}

#[test]
fn display_motion_and_clicks_account_for_virtual_indent() {
    let mut editor = Editor::new("  abcdefghijklmnopqrstuvwxyz", None);
    editor.cursor.col = 9;
    editor.move_by_display_rows(1, 1, 5, 16, true);
    assert_eq!(editor.cursor.col, 19);
    editor.move_by_display_rows(-1, 1, 5, 16, true);
    assert_eq!(editor.cursor.col, 9);
    for x in 0..=6 {
        assert_eq!(
            editor.position_at_display_column(0, 16, x, 16, true).col,
            16
        );
    }
    assert_eq!(
        editor.position_at_display_column(0, 16, 15, 16, true).col,
        25
    );
    assert_eq!(
        editor.position_at_display_column(0, 0, 100, 16, false).col,
        100
    );
}

#[test]
fn editing_parent_indent_rebuilds_wrapped_rows_and_undo_restores_them() {
    let mut editor = Editor::new(&"a".repeat(20), None);
    assert_eq!(editor.display_total(12, true), 2);
    editor.begin_insert('i');
    editor.insert_char(' ');
    assert_eq!(editor.display_row_layout(0, 12, 12, true), (5, 7));
    assert_eq!(
        editor.display_window(0, 10, 12, true),
        vec![(0, 0), (0, 12), (0, 19)]
    );
    editor.escape();
    editor.undo(false);
    assert_eq!(editor.display_total(12, true), 2);
    assert_eq!(editor.display_row_layout(0, 12, 12, true), (4, 8));
}

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
        vec![(0, 0), (0, 3), (0, 4), (0, 5), (1, 0), (2, 0)]
    );
    e.cursor = Pos { row: 0, col: 6 };
    e.mode = Mode::Insert;
    assert_eq!(
        e.display_rows(3, true),
        vec![(0, 0), (0, 3), (0, 4), (0, 5), (0, 6), (1, 0), (2, 0)]
    );
    e.reveal_cursor(2, 3, true);
    assert_eq!(e.top, 3);
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

    assert_eq!(e.cursor, Pos { row: 0, col: 4 });
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
