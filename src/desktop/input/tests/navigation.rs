use super::*;

#[test]
fn desktop_frames_repeat_arrows_and_page_keys() {
    let text = std::iter::repeat_n("abcdefghijk", 300)
        .collect::<Vec<_>>()
        .join("\n");
    for (key, expected) in [
        (KeyCode::Left, vimloating::editor::Pos { row: 5, col: 0 }),
        (KeyCode::Right, vimloating::editor::Pos { row: 5, col: 6 }),
        (KeyCode::Up, vimloating::editor::Pos { row: 2, col: 3 }),
        (KeyCode::Down, vimloating::editor::Pos { row: 8, col: 3 }),
    ] {
        let mut editor = Editor::new(&text, None);
        editor.cursor = vimloating::editor::Pos { row: 5, col: 3 };
        KeyboardHarness::new().hold(&mut editor, key, &[]);
        assert_eq!(editor.cursor, expected);
        assert!(!editor.dirty());
    }
    for key in [KeyCode::PageUp, KeyCode::PageDown] {
        let mut editor = Editor::new(&text, None);
        editor.cursor.row = 150;
        let mut keyboard = KeyboardHarness::new();
        keyboard.frame(&mut editor, 0.0, &[key], &[key], "");
        let first = editor.cursor.row;
        keyboard.frame(&mut editor, 0.35, &[key], &[], "");
        assert!(if key == KeyCode::PageUp {
            editor.cursor.row < first
        } else {
            editor.cursor.row > first
        });
    }
}

#[test]
fn desktop_frames_repeat_ctrl_scrolling_and_font_shortcuts() {
    let text = std::iter::repeat_n("line", 300)
        .collect::<Vec<_>>()
        .join("\n");
    for (key, backwards) in [
        (KeyCode::D, false),
        (KeyCode::U, true),
        (KeyCode::F, false),
        (KeyCode::B, true),
    ] {
        let mut editor = Editor::new(&text, None);
        editor.cursor.row = 150;
        let mut keyboard = KeyboardHarness::new();
        keyboard.frame(&mut editor, 0.0, &[KeyCode::LeftControl, key], &[key], "");
        let first = editor.cursor.row;
        keyboard.frame(&mut editor, 0.35, &[KeyCode::LeftControl, key], &[], "");
        assert!(if backwards {
            editor.cursor.row < first
        } else {
            editor.cursor.row > first
        });
    }
    for (key, expected) in [(KeyCode::E, 53), (KeyCode::Y, 47)] {
        let mut editor = Editor::new(&text, None);
        editor.cursor.row = 50;
        editor.top = 50;
        KeyboardHarness::new().hold(&mut editor, key, &[KeyCode::LeftControl]);
        assert_eq!(editor.top, expected);
    }
    let mut editor = Editor::new(&"x".repeat(1000), None);
    let mut keyboard = KeyboardHarness::new();
    keyboard.hold(&mut editor, KeyCode::L, &[KeyCode::LeftControl]);
    assert!(!keyboard.wrap);
    let right = editor.left;
    assert!(right > text_grid(keyboard.font_size).1 / 2);
    keyboard.hold(&mut editor, KeyCode::H, &[KeyCode::LeftControl]);
    assert!(editor.left < right);
    keyboard.hold(&mut editor, KeyCode::Minus, &[KeyCode::LeftControl]);
    assert_eq!(
        keyboard.font_size,
        vimloating::config::BASE_FONT_SIZE.saturating_sub(6).max(12)
    );
}

#[test]
fn desktop_frames_keep_ctrl_j_k_animation_repeating_while_held() {
    let text = std::iter::repeat_n("line", 100)
        .collect::<Vec<_>>()
        .join("\n");
    for (key, expected) in [(KeyCode::J, 60), (KeyCode::K, 40)] {
        let mut editor = Editor::new(&text, None);
        editor.cursor.row = 50;
        let mut keyboard = KeyboardHarness::new();
        keyboard.frame(&mut editor, 0.0, &[KeyCode::LeftControl, key], &[key], "");
        for now in [0.1, 0.125, 0.2, 0.23] {
            keyboard.frame(&mut editor, now, &[KeyCode::LeftControl, key], &[], "");
        }
        assert_eq!(editor.cursor.row, expected);
        // Releasing Ctrl finishes the current five-line animation.
        keyboard.frame(&mut editor, 0.5, &[], &[], "");
        let after_release = editor.cursor;
        assert!(keyboard.motion.is_none());
        keyboard.frame(&mut editor, 1.0, &[], &[], "");
        assert_eq!(editor.cursor, after_release);
    }
}

#[test]
fn shifted_find_keys_are_preserved_even_when_character_events_are_missing() {
    let mut chars = vec!['f', 'x'];
    promote_shifted_find_key(&mut chars, 'f', 'F', true);
    assert_eq!(chars, vec!['F', 'x']);

    let mut chars = vec!['x'];
    promote_shifted_find_key(&mut chars, 't', 'T', true);
    assert_eq!(chars, vec!['T', 'x']);

    promote_shifted_find_key(&mut chars, 'f', 'F', false);
    assert_eq!(chars, vec!['T', 'x']);
}
