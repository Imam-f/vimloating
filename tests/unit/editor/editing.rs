use super::super::*;

#[test]
fn move_back_character_preserves_unicode_text_and_crosses_line_boundaries() {
    let mut e = Editor::new("aé日\nz", None);
    e.cursor = Pos { row: 1, col: 0 };
    e.begin_insert('i');
    for expected in [
        Pos { row: 0, col: 3 },
        Pos { row: 0, col: 2 },
        Pos { row: 0, col: 1 },
        Pos { row: 0, col: 0 },
        Pos { row: 0, col: 0 },
    ] {
        e.move_back_character();
        assert_eq!(e.cursor, expected);
        assert_eq!(e.text(), "aé日\nz");
        assert!(!e.dirty());
    }
}

#[test]
fn insert_dot_replay_preserves_backspace_movement_before_typing() {
    let mut e = Editor::new("abc\nabc", None);
    e.begin_insert('A');
    e.move_back_character();
    e.insert_char('X');
    e.escape();
    assert_eq!(e.text(), "abXc\nabc");
    e.cursor = Pos { row: 1, col: 0 };
    e.normal_key('.');
    assert_eq!(e.text(), "abXc\nabXc");
    e.undo(false);
    assert_eq!(e.text(), "abXc\nabc");
}

#[test]
fn replace_supports_unicode_counts_undo_dot_and_insufficient_characters() {
    let mut e = Editor::new("abcdef", None);
    for key in "2rλ".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "λλcdef");
    assert_eq!(e.cursor.col, 1);
    e.cursor.col = 3;
    e.normal_key('.');
    assert_eq!(e.text(), "λλcλλf");
    e.undo(false);
    assert_eq!(e.text(), "λλcdef");
    e.cursor.col = 5;
    for key in "2rz".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "λλcdef");
    e.normal_key('r');
    e.escape();
    assert_eq!(e.pending, None);
    e.cursor.col = 2;
    e.normal_key('r');
    e.normal_key('\n');
    assert_eq!(e.text(), "λλ\ndef");
}

#[test]
fn unicode_editing_and_insert_session_undo() {
    let mut e = Editor::new("héllo\n世界", None);
    e.begin_insert('A');
    e.insert_char('🦀');
    e.newline();
    e.insert_char('λ');
    e.escape();
    assert_eq!(e.text(), "héllo🦀\nλ\n世界");
    e.undo(false);
    assert_eq!(e.text(), "héllo\n世界");
    assert!(!e.dirty());
    e.undo(true);
    assert_eq!(e.text(), "héllo🦀\nλ\n世界");
}

#[test]
fn counted_delete_and_linewise_paste() {
    let mut e = Editor::new("one\ntwo\nthree\nfour", None);
    for c in "2dd".chars() {
        e.normal_key(c);
    }
    assert_eq!(e.text(), "three\nfour");
    e.normal_key('p');
    assert_eq!(e.text(), "three\none\ntwo\nfour");
    e.undo(false);
    e.undo(false);
    assert_eq!(e.text(), "one\ntwo\nthree\nfour");
}

#[test]
fn dot_repeats_counted_deletes_and_insert_sessions() {
    let mut e = Editor::new("abcdef", None);
    for key in "2x".chars() {
        e.normal_key(key);
    }
    e.normal_key('.');
    assert_eq!(e.text(), "ef");

    let mut e = Editor::new("one\none", None);
    e.begin_insert('i');
    e.insert_text("X");
    e.escape();
    e.cursor = Pos { row: 1, col: 0 };
    e.normal_key('.');
    assert_eq!(e.text(), "Xone\nXone");

    let mut e = Editor::new("one\ntwo\nthree", None);
    e.normal_key('d');
    e.normal_key('d');
    e.normal_key('.');
    assert_eq!(e.text(), "three");

    let mut e = Editor::new("value", None);
    e.normal_key('>');
    e.normal_key('.');
    assert_eq!(e.text(), "        value");
}

#[test]
fn single_angle_brackets_indent_and_unindent_the_current_line() {
    let mut e = Editor::new("  value\nnext", None);
    e.cursor = Pos { row: 0, col: 3 };

    e.normal_key('>');

    assert_eq!(e.text(), "      value\nnext");
    assert_eq!(e.cursor, Pos { row: 0, col: 7 });
    e.undo(false);
    assert_eq!(e.text(), "  value\nnext");
    assert_eq!(e.cursor, Pos { row: 0, col: 3 });

    e.normal_key('<');

    assert_eq!(e.text(), "value\nnext");
    assert_eq!(e.cursor, Pos { row: 0, col: 1 });
}

#[test]
fn number_adjustment_preserves_zero_padding_and_repeats_with_dot() {
    let mut e = Editor::new("value 009 and 20", None);
    e.cursor.col = 6;
    assert!(e.adjust_number(1));
    assert_eq!(e.text(), "value 010 and 20");
    assert!(e.adjust_number(-2));
    assert_eq!(e.text(), "value 008 and 20");

    e.cursor.col = 14;
    e.normal_key('.');
    assert_eq!(e.text(), "value 008 and 18");
}

#[test]
fn number_adjustment_handles_negative_values_and_missing_numbers() {
    let mut e = Editor::new("-09 and text", None);
    assert!(e.adjust_number(1));
    assert_eq!(e.text(), "-08 and text");
    assert!(e.adjust_number(-1));
    assert_eq!(e.text(), "-09 and text");

    e.cursor.col = 5;
    assert!(!e.adjust_number(1));
    assert_eq!(e.text(), "-09 and text");

    let mut e = Editor::new("199", None);
    e.cursor.col = 1;
    assert!(e.adjust_number(1));
    assert_eq!(e.text(), "200");
}

#[test]
fn moving_a_line_reindents_it_to_the_previous_nonblank_line() {
    let mut e = Editor::new("root\n    moved\n   \nsibling", None);
    e.cursor = Pos { row: 1, col: 6 };

    e.move_line(1);

    assert_eq!(e.text(), "root\n   \nmoved\nsibling");
    assert_eq!(e.cursor, Pos { row: 2, col: 2 });
    e.undo(false);
    assert_eq!(e.text(), "root\n    moved\n   \nsibling");
    assert_eq!(e.cursor, Pos { row: 1, col: 6 });
}

#[test]
fn moving_a_line_at_the_document_edge_is_a_noop() {
    let mut e = Editor::new("first\nsecond", None);

    e.move_line(-1);

    assert_eq!(e.text(), "first\nsecond");
    assert!(e.undo.is_empty());
}

#[test]
fn capital_x_deletes_before_cursor_and_supports_count_undo_and_paste() {
    let mut e = Editor::new("abcdef", None);
    e.cursor.col = 4;
    for c in "2X".chars() {
        e.normal_key(c);
    }
    assert_eq!(e.text(), "abef");
    assert_eq!(e.cursor.col, 2);
    e.normal_key('P');
    assert_eq!(e.text(), "abcdef");
    e.undo(false);
    assert_eq!(e.text(), "abef");
    e.undo(false);
    assert_eq!(e.text(), "abcdef");
}

#[test]
fn deleting_entire_document_keeps_editable_line() {
    let mut e = Editor::new("a\nb", None);
    for c in "9dd".chars() {
        e.normal_key(c);
    }
    assert_eq!(e.lines, vec![Vec::<char>::new()]);
    e.begin_insert('i');
    e.insert_char('x');
    assert_eq!(e.text(), "x");
}

#[test]
fn multiline_paste_preserves_indentation_and_suffix() {
    let mut e = Editor::new("    tail", None);
    e.cursor.col = 4;
    e.begin_insert('i');
    e.insert_text("first\r\n\tsecond\n");
    assert_eq!(e.text(), "    first\n\tsecond\ntail");
    e.escape();
    e.undo(false);
    assert_eq!(e.text(), "    tail");
}

#[test]
fn ctrl_backspace_removes_previous_unicode_word_and_spacing() {
    let mut e = Editor::new("one café  two", None);
    e.cursor.col = e.lines[0].len();
    e.mode = Mode::Insert;
    e.delete_prev_word();
    assert_eq!(e.text(), "one café  ");
    e.delete_prev_word();
    assert_eq!(e.text(), "one ");
    assert_eq!(e.cursor.col, 4);
}

#[test]
fn insert_mode_deletions_can_be_pasted_from_the_yank_register() {
    let mut backspace = Editor::new("one two", None);
    backspace.cursor.col = 2;
    backspace.begin_insert('i');
    backspace.backspace();
    backspace.escape();
    assert_eq!(backspace.register, vec![vec!['n']]);
    backspace.normal_key('p');
    assert_eq!(backspace.text(), "one two");

    let mut delete = Editor::new("one two", None);
    delete.cursor.col = 1;
    delete.begin_insert('i');
    delete.delete_forward();
    delete.escape();
    assert_eq!(delete.register, vec![vec!['n']]);
    delete.normal_key('p');
    assert_eq!(delete.text(), "one two");

    let mut delete_word = Editor::new("one two", None);
    delete_word.cursor.col = 7;
    delete_word.begin_insert('i');
    delete_word.delete_prev_word();
    delete_word.escape();
    assert_eq!(delete_word.register, vec![vec!['t', 'w', 'o']]);
    delete_word.normal_key('p');
    assert_eq!(delete_word.text(), "one two");

    let mut join = Editor::new("one\ntwo", None);
    join.cursor = Pos { row: 1, col: 0 };
    join.begin_insert('i');
    join.backspace();
    join.escape();
    assert_eq!(join.register, vec![vec![], vec![]]);
    join.normal_key('p');
    assert_eq!(join.text(), "one\ntwo");
}
