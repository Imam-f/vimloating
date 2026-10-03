use super::*;

#[test]
fn desktop_frames_repeat_backspace_as_one_character_movement_without_deletion() {
    let mut editor = Editor::new("aé日z", None);
    editor.cursor.col = 3;
    editor.begin_insert('i');
    let mut keyboard = KeyboardHarness::new();
    keyboard.frame(
        &mut editor,
        0.0,
        &[KeyCode::Backspace],
        &[KeyCode::Backspace],
        "",
    );
    assert_eq!(editor.cursor.col, 2);
    assert_eq!(editor.text(), "aé日z");
    keyboard.frame(&mut editor, 0.1, &[KeyCode::Backspace], &[], "");
    assert_eq!(editor.cursor.col, 2);
    keyboard.frame(&mut editor, 0.35, &[KeyCode::Backspace], &[], "");
    assert_eq!(editor.cursor.col, 1);
    keyboard.frame(&mut editor, 0.42, &[KeyCode::Backspace], &[], "");
    assert_eq!(editor.cursor.col, 0);
    keyboard.frame(&mut editor, 0.5, &[], &[], "");
    keyboard.frame(&mut editor, 1.0, &[], &[], "");
    assert_eq!(editor.cursor.col, 0);
    assert_eq!(editor.text(), "aé日z");
    assert!(!editor.dirty());
}

#[test]
fn desktop_frames_repeat_delete_once_per_tick_and_ignore_duplicate_os_events() {
    let mut editor = Editor::new("abcdef", None);
    editor.begin_insert('i');
    let mut keyboard = KeyboardHarness::new();
    keyboard.frame(&mut editor, 0.0, &[KeyCode::Delete], &[KeyCode::Delete], "");
    assert_eq!(editor.text(), "bcdef");
    keyboard.frame(&mut editor, 0.35, &[KeyCode::Delete], &[], "");
    assert_eq!(editor.text(), "cdef");
    keyboard.frame(
        &mut editor,
        0.36,
        &[KeyCode::Delete],
        &[KeyCode::Delete],
        "",
    );
    assert_eq!(editor.text(), "cdef");
    keyboard.frame(&mut editor, 0.42, &[KeyCode::Delete], &[], "");
    assert_eq!(editor.text(), "def");
    keyboard.frame(&mut editor, 0.5, &[], &[], "");
    keyboard.frame(&mut editor, 1.0, &[], &[], "");
    assert_eq!(editor.text(), "def");
    editor.escape();
    editor.undo(false);
    assert_eq!(editor.text(), "abcdef");
}

#[test]
fn desktop_frames_repeat_modified_editing_and_plain_character_events() {
    for key in [KeyCode::W, KeyCode::Backspace] {
        let mut editor = Editor::new("one two three four", None);
        editor.begin_insert('A');
        KeyboardHarness::new().hold(&mut editor, key, &[KeyCode::LeftControl]);
        assert_eq!(editor.text(), "one ");
        assert_eq!(editor.cursor.col, 4);
        editor.escape();
        editor.undo(false);
        assert_eq!(editor.text(), "one two three four");
    }

    let mut editor = Editor::new("text", None);
    let mut keyboard = KeyboardHarness::new();
    keyboard.hold(&mut editor, KeyCode::L, &[KeyCode::LeftAlt]);
    assert_eq!(editor.text(), "            text");
    keyboard.hold(&mut editor, KeyCode::H, &[KeyCode::LeftAlt]);
    assert_eq!(editor.text(), "text");

    let mut editor = Editor::new("abcdef", None);
    for (now, pressed) in [(0.0, true), (0.35, false), (0.42, false)] {
        keyboard.frame(
            &mut editor,
            now,
            &[KeyCode::L],
            if pressed { &[KeyCode::L] } else { &[] },
            "l",
        );
    }
    assert_eq!(editor.cursor.col, 3);
    editor.begin_insert('i');
    for now in [0.5, 0.85, 0.92] {
        keyboard.frame(&mut editor, now, &[KeyCode::A], &[], "a");
    }
    assert_eq!(editor.text(), "abcaaadef");
}

#[test]
fn desktop_frames_stop_held_backspace_on_escape_until_release() {
    let mut editor = Editor::new("abcdef", None);
    editor.begin_insert('A');
    let mut keyboard = KeyboardHarness::new();
    keyboard.frame(
        &mut editor,
        0.0,
        &[KeyCode::Backspace],
        &[KeyCode::Backspace],
        "",
    );
    keyboard.frame(
        &mut editor,
        0.1,
        &[KeyCode::Backspace, KeyCode::Escape],
        &[KeyCode::Escape],
        "",
    );
    let cursor_after_escape = editor.cursor;
    keyboard.frame(&mut editor, 0.5, &[KeyCode::Backspace], &[], "");
    assert_eq!(editor.cursor, cursor_after_escape);
    keyboard.frame(&mut editor, 0.6, &[], &[], "");
    keyboard.frame(
        &mut editor,
        0.7,
        &[KeyCode::Backspace],
        &[KeyCode::Backspace],
        "",
    );
    assert_eq!(editor.cursor.col, cursor_after_escape.col - 1);
    assert_eq!(editor.text(), "abcdef");
}

#[test]
fn desktop_frames_repeat_line_completion_prompt_backspace_tab_and_enter() {
    let mut editor = Editor::new("hello there\nhelp here\nhe", None);
    editor.cursor.row = 2;
    editor.begin_insert('A');
    let mut keyboard = KeyboardHarness::new();
    keyboard.frame(
        &mut editor,
        0.0,
        &[KeyCode::LeftControl, KeyCode::X],
        &[KeyCode::X],
        "",
    );
    keyboard.frame(
        &mut editor,
        0.1,
        &[KeyCode::LeftControl, KeyCode::L],
        &[KeyCode::L],
        "",
    );
    assert_eq!(editor.completion_popup().unwrap().selected, 0);
    keyboard.frame(
        &mut editor,
        0.46,
        &[KeyCode::LeftControl, KeyCode::L],
        &[],
        "",
    );
    assert_eq!(editor.completion_popup().unwrap().selected, 1);
    assert!(keyboard.wrap);
    let mut editor = Editor::new("", None);
    editor.begin_insert('i');
    let mut keyboard = KeyboardHarness::new();
    keyboard.hold(&mut editor, KeyCode::Tab, &[]);
    assert_eq!(editor.text(), " ".repeat(12));
    keyboard.hold(&mut editor, KeyCode::Enter, &[]);
    assert_eq!(editor.cursor.row, 3);
    editor.escape();
    editor.normal_key(':');
    editor.prompt = "theme".into();
    keyboard.hold(&mut editor, KeyCode::Backspace, &[]);
    assert_eq!(editor.prompt, "th");
}

#[test]
fn timed_insert_backspace_and_delete_repeat_and_undo_as_one_session() {
    for key in [KeyCode::Backspace, KeyCode::Delete] {
        let mut editor = Editor::new("abcdef", None);
        if key == KeyCode::Backspace {
            editor.cursor.col = 5;
        }
        editor.begin_insert('i');
        let mut state = None;
        for (time, pressed) in [(0.0, true), (0.35, false), (0.42, false)] {
            let fired = repeat_due(&mut state, Some(key), pressed, editor.mode, time).unwrap();
            handle_repeated_key(&mut editor, fired);
        }
        assert_eq!(
            editor.text(),
            if key == KeyCode::Backspace {
                "abcdef"
            } else {
                "def"
            }
        );
        assert_eq!(editor.mode, Mode::Insert);
        repeat_due(&mut state, None, false, editor.mode, 0.43);
        assert_eq!(repeat_due(&mut state, None, false, editor.mode, 1.0), None);
        editor.escape();
        editor.undo(false);
        assert_eq!(editor.text(), "abcdef");
    }
}

#[test]
fn held_insert_backspace_moves_and_delete_joins_lines_then_stops_after_escape() {
    for key in [KeyCode::Backspace, KeyCode::Delete] {
        let mut editor = Editor::new("ab\ncd", None);
        editor.cursor = vimloating::editor::Pos {
            row: usize::from(key == KeyCode::Backspace),
            col: 1,
        };
        editor.begin_insert('i');
        let mut state = None;
        for (time, pressed) in [(0.0, true), (0.35, false), (0.42, false)] {
            let fired = repeat_due(&mut state, Some(key), pressed, editor.mode, time).unwrap();
            handle_repeated_key(&mut editor, fired);
        }
        assert_eq!(
            editor.text(),
            if key == KeyCode::Backspace {
                "ab\ncd"
            } else {
                "ad"
            }
        );
        editor.escape();
        assert_eq!(
            repeat_due(&mut state, Some(key), false, editor.mode, 0.5),
            None
        );
        assert_eq!(
            editor.text(),
            if key == KeyCode::Backspace {
                "ab\ncd"
            } else {
                "ad"
            }
        );
        editor.undo(false);
        assert_eq!(editor.text(), "ab\ncd");
    }
}
