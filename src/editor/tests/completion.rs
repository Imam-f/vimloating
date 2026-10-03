use super::super::*;

#[test]
fn word_completion_cycles_unique_buffer_words_in_both_directions() {
    let mut e = Editor::new("hello help hello helmet\nhe", None);
    e.cursor.row = 1;
    e.begin_insert('A');
    e.control_key('n');
    assert_eq!(e.text(), "hello help hello helmet\nhello");
    assert!(e.message.contains("1/3"));
    e.control_key('n');
    assert_eq!(e.text(), "hello help hello helmet\nhelmet");
    e.control_key('p');
    assert_eq!(e.text(), "hello help hello helmet\nhello");
    e.control_key('p');
    assert_eq!(e.text(), "hello help hello helmet\nhelp");
    e.control_key('n');
    assert_eq!(e.text(), "hello help hello helmet\nhello");
    e.escape();
    e.undo(false);
    assert_eq!(e.text(), "hello help hello helmet\nhe");
    e.undo(true);
    assert_eq!(e.text(), "hello help hello helmet\nhello");
}

#[test]
fn word_completion_handles_unicode_identifiers_case_and_punctuation() {
    let mut e = Editor::new("日本語 日本人 get_value Get_other\n(日); tail", None);
    e.cursor = Pos { row: 1, col: 2 };
    e.begin_insert('i');
    e.control_key('p');
    assert_eq!(
        e.text(),
        "日本語 日本人 get_value Get_other\n(日本語); tail"
    );
    assert_eq!(e.cursor, Pos { row: 1, col: 4 });
    e.escape();
    e.normal_key('o');
    e.insert_text("get_");
    e.control_key('x');
    e.control_key('n');
    assert_eq!(
        e.lines.last().unwrap().iter().collect::<String>(),
        "get_value"
    );
}

#[test]
fn empty_word_prefix_can_complete_and_current_word_does_not_complete_itself() {
    let mut e = Editor::new("alpha beta\n", None);
    e.cursor.row = 1;
    e.begin_insert('i');
    e.control_key('p');
    assert_eq!(e.text(), "alpha beta\nbeta");
    let mut e = Editor::new("hello", None);
    e.cursor.col = 2;
    e.begin_insert('i');
    e.control_key('n');
    assert_eq!(e.text(), "hello");
    assert_eq!(e.cursor.col, 2);
    assert_eq!(e.message, "No word completions");
    assert!(!e.dirty());
}

#[test]
fn line_completion_preserves_indentation_deduplicates_and_cycles() {
    let mut e = Editor::new(
        "  let alpha = 1;\n\tlet beta = 2;\nlet alpha = 1;\n    let ",
        None,
    );
    e.cursor.row = 3;
    e.begin_insert('A');
    e.control_key('x');
    e.control_key('l');
    assert_eq!(e.lines[3].iter().collect::<String>(), "    let alpha = 1;");
    assert!(e.message.contains("1/2"));
    e.control_key('l');
    assert_eq!(e.lines[3].iter().collect::<String>(), "    let beta = 2;");
    e.control_key('n');
    assert_eq!(e.lines[3].iter().collect::<String>(), "    let alpha = 1;");
    e.control_key('p');
    assert_eq!(e.lines[3].iter().collect::<String>(), "    let beta = 2;");
    e.escape();
    e.undo(false);
    assert_eq!(e.lines[3].iter().collect::<String>(), "    let ");
    e.undo(true);
    assert_eq!(e.lines[3].iter().collect::<String>(), "    let beta = 2;");
}

#[test]
fn line_completion_supports_blank_prefix_tabs_and_preserves_the_suffix() {
    let mut e = Editor::new("  日本\ttext\n\t; tail\n", None);
    e.cursor = Pos { row: 1, col: 1 };
    e.begin_insert('i');
    e.control_key('x');
    e.control_key('l');
    assert_eq!(e.text(), "  日本\ttext\n\t日本\ttext; tail\n");
    assert_eq!(e.cursor, Pos { row: 1, col: 8 });
    let mut e = Editor::new("only this line", None);
    e.begin_insert('A');
    e.complete_line(false);
    assert_eq!(e.text(), "only this line");
    assert_eq!(e.message, "No line completions");
    assert!(!e.dirty());
}

#[test]
fn typing_movement_escape_and_explicit_prefix_reset_completion_kind() {
    let mut e = Editor::new("hello there\nhelp here\nhe", None);
    e.cursor.row = 2;
    e.begin_insert('A');
    e.control_key('x');
    e.control_key('l');
    assert_eq!(e.lines[2].iter().collect::<String>(), "hello there");
    e.control_key('x');
    e.control_key('p');
    assert_eq!(e.lines[2].iter().collect::<String>(), "hello there");
    assert_eq!(e.message, "No word completions");
    e.backspace();
    e.control_key('n');
    assert_eq!(e.lines[2].iter().collect::<String>(), "hello there");
    assert!(e.message.starts_with("Word completion"));
    e.move_by(-1, 0, 2);
    e.control_key('n');
    assert!(e.message.starts_with("Word completion"));
    e.escape();
    assert!(!e.insert_control_key('n'));
    e.normal_key('A');
    e.control_key('n');
    assert_eq!(e.message, "No word completions");
}

#[test]
fn completed_word_and_line_are_replayed_by_dot() {
    for (source, prefix, kind) in [("hello", "he", 'n'), ("let value = 1;", "let ", 'l')] {
        let mut e = Editor::new(&format!("{source}\n\n"), None);
        e.cursor.row = 1;
        e.begin_insert('i');
        e.insert_text(prefix);
        e.control_key('x');
        e.control_key(kind);
        e.escape();
        e.normal_key('j');
        e.normal_key('.');
        assert_eq!(e.text(), format!("{source}\n{source}\n{source}"));
    }
}

#[test]
fn unsupported_key_and_regular_typing_cancel_the_ctrl_x_prefix() {
    let mut e = Editor::new("hello\nhe", None);
    e.cursor.row = 1;
    e.begin_insert('A');
    e.control_key('x');
    assert!(!e.insert_control_key('s'));
    assert!(!e.insert_control_key('l'));
    e.control_key('x');
    e.insert_char('l');
    assert!(!e.insert_control_key('l'));
    e.control_key('n');
    assert_eq!(e.text(), "hello\nhello");
}

#[test]
fn popup_tracks_the_current_completion_and_disappears_on_edits_movement_and_escape() {
    let mut e = Editor::new("hello help helmet\nhe", None);
    e.cursor.row = 1;
    e.begin_insert('A');
    assert!(e.completion_popup().is_none());
    e.control_key('n');
    let popup = e.completion_popup().unwrap();
    assert_eq!(popup.kind, "Word");
    assert_eq!(popup.anchor, Pos { row: 1, col: 0 });
    assert_eq!(popup.candidates, &["hello", "helmet", "help"]);
    assert_eq!(popup.selected, 0);
    e.control_key('n');
    assert_eq!(e.completion_popup().unwrap().selected, 1);
    e.insert_char('!');
    assert!(e.completion_popup().is_none());
    e.backspace();
    for _ in 0..4 {
        e.backspace();
    }
    e.complete_line(false);
    assert_eq!(e.completion_popup().unwrap().kind, "Line");
    e.move_by(-1, 0, 1);
    assert!(e.completion_popup().is_none());
    e.complete_word(false);
    assert!(e.completion_popup().is_some());
    e.escape();
    assert!(e.completion_popup().is_none());
    e.normal_key(':');
    e.prompt = "buf".into();
    e.complete_command();
    assert_eq!(
        e.completion_popup().unwrap().candidates,
        &["buffer", "buffers"]
    );
    e.complete_command();
    assert_eq!(e.completion_popup().unwrap().selected, 1);
    e.edit_prompt(Some('!'));
    assert!(e.completion_popup().is_none());
}

#[test]
fn popup_layout_scrolls_to_selection_flips_above_and_clamps_to_viewport() {
    let candidates: Vec<_> = (0..20).map(|i| format!("candidate {i:02}")).collect();
    let popup = CompletionPopup {
        kind: "Word",
        candidates: &candidates,
        selected: 17,
        anchor: Pos::default(),
    };
    let layout = popup.layout(78, 18, 80, 20).unwrap();
    assert_eq!(layout.visible, 8);
    assert!(layout.first <= popup.selected && popup.selected < layout.first + layout.visible);
    assert!(layout.y + layout.height <= 18);
    assert!(layout.x + layout.width <= 80);
    let layout = popup.layout(1, 0, 80, 20).unwrap();
    assert_eq!(layout.y, 1);
    // Tiny and resized views must never underflow, draw offscreen, or omit selection.
    for cols in 1..25 {
        for rows in 0..14 {
            for cursor_row in 0..=rows {
                if let Some(layout) = popup.layout(cols + 5, cursor_row, cols, rows) {
                    assert!(layout.x + layout.width <= cols);
                    assert!(layout.y + layout.height <= rows);
                    assert!(
                        layout.first <= popup.selected
                            && popup.selected < layout.first + layout.visible
                    );
                    assert!(layout.y > cursor_row || layout.y + layout.height <= cursor_row);
                }
            }
        }
    }
}

#[test]
fn popup_truncation_keeps_unicode_cells_and_long_path_filenames_visible() {
    let candidates = vec!["folder/very-long-directory/日本語.txt".into()];
    let popup = CompletionPopup {
        kind: "Path",
        candidates: &candidates,
        selected: 0,
        anchor: Pos::default(),
    };
    let text = popup.candidate_text(0, 20);
    assert_eq!(text.chars().count(), 20);
    assert!(text.contains('…'));
    assert!(text.ends_with("日本語.txt"));
    assert_eq!(popup.candidate_text(0, 1), "…");
    assert_eq!(popup.candidate_text(0, 0), "");
    let popup = CompletionPopup {
        kind: "Line",
        ..popup
    };
    assert_eq!(popup.candidate_text(0, 7), "folder…");
}
