use super::*;
use std::fs;

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
fn visual_multiline_delete_preserves_boundary_text() {
    let mut e = Editor::new("abcd\nefgh\nijkl", None);
    e.cursor.col = 2;
    e.normal_key('v');
    e.move_by(0, 1, 1);
    e.move_by(-1, 0, 1);
    e.normal_key('d');
    assert_eq!(e.text(), "abgh\nijkl");
    e.normal_key('P');
    assert_eq!(e.text(), "abcd\nefgh\nijkl");
}

#[test]
fn search_wraps_and_uses_character_columns() {
    let mut e = Editor::new("é猫 x 猫\n猫", None);
    e.search = "猫".into();
    e.find(false);
    assert_eq!(e.cursor, Pos { row: 0, col: 1 });
    e.find(true);
    assert_eq!(e.cursor, Pos { row: 1, col: 0 });
    e.find(false);
    assert_eq!(e.cursor, Pos { row: 0, col: 1 });
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
fn quit_protects_unsaved_changes() {
    let mut e = Editor::new("", None);
    e.begin_insert('i');
    e.insert_char('x');
    e.escape();
    e.command("q");
    assert!(!e.quit);
    e.command("q!");
    assert!(e.quit);
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
fn horizontal_scroll_moves_the_buffer_view_without_moving_the_cursor() {
    let mut e = Editor::new("abcdefghijklmnopqrstuvwxyz", None);
    e.scroll_horizontal(1, 10);
    e.reveal_cursor(5, 10, false);
    assert_eq!(e.left, 5);
    assert_eq!(e.cursor.col, 0);

    e.move_by(1, 0, 1);
    e.reveal_cursor(5, 10, false);
    assert_eq!(e.left, 1);
}

#[test]
fn save_round_trip_and_failed_save_keeps_dirty_state() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("vimloating-{}-{unique}.txt", std::process::id()));
    let mut e = Editor::new("", Some(path.clone()));
    e.begin_insert('i');
    e.insert_text("café\n世界\n");
    e.escape();
    assert!(e.dirty());
    assert!(e.save(None));
    assert!(!e.dirty());
    let text = fs::read_to_string(&path).unwrap();
    assert_eq!(text, "café\n世界\n");
    let reopened = Editor::new(&text, Some(path.clone()));
    assert_eq!(reopened.lines, e.lines);
    e.begin_insert('A');
    e.insert_char('x');
    assert!(!e.save(Some(&path.join("missing/file").to_string_lossy())));
    assert!(e.dirty());
    assert_eq!(e.path.as_ref(), Some(&path));
    fs::remove_file(path).unwrap();
}
