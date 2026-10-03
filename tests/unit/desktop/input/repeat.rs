use super::*;

#[test]
fn desktop_frames_recover_a_hold_whose_press_edge_was_consumed() {
    let mut editor = Editor::new("abcdef", None);
    editor.begin_insert('A');
    let mut keyboard = KeyboardHarness::new();
    keyboard.frame(&mut editor, 0.0, &[KeyCode::Backspace], &[], "");
    assert_eq!(editor.cursor.col, 5);
    keyboard.frame(&mut editor, 0.35, &[KeyCode::Backspace], &[], "");
    assert_eq!(editor.cursor.col, 4);
    assert_eq!(editor.text(), "abcdef");
}

#[test]
fn held_keys_wait_then_repeat_without_double_counting_os_repeat_events() {
    let mut state = None;
    for key in [
        KeyCode::N,
        KeyCode::P,
        KeyCode::Backspace,
        KeyCode::Delete,
        KeyCode::E,
        KeyCode::Y,
    ] {
        assert_eq!(
            repeat_due(&mut state, Some(key), true, Mode::Normal, 0.0),
            Some(key)
        );
        assert_eq!(
            repeat_due(&mut state, Some(key), true, Mode::Normal, 0.1),
            None
        );
        assert_eq!(
            repeat_due(&mut state, Some(key), false, Mode::Normal, 0.34),
            None
        );
        assert_eq!(
            repeat_due(&mut state, Some(key), false, Mode::Normal, 0.35),
            Some(key)
        );
        assert_eq!(
            repeat_due(&mut state, Some(key), true, Mode::Normal, 0.36),
            None
        );
        assert_eq!(
            repeat_due(&mut state, Some(key), false, Mode::Normal, 0.42),
            Some(key)
        );
        assert_eq!(
            repeat_due(&mut state, None, false, Mode::Normal, 0.43),
            None
        );
        assert!(state.is_none());
    }
}

#[test]
fn switching_keys_release_and_mode_changes_reset_the_repeat_delay() {
    let mut state = None;
    repeat_due(&mut state, Some(KeyCode::N), true, Mode::Insert, 0.0);
    assert_eq!(
        repeat_due(&mut state, Some(KeyCode::P), true, Mode::Insert, 0.1),
        Some(KeyCode::P)
    );
    assert_eq!(
        repeat_due(&mut state, Some(KeyCode::P), false, Mode::Insert, 0.35),
        None
    );
    assert_eq!(
        repeat_due(&mut state, Some(KeyCode::P), false, Mode::Insert, 0.46),
        Some(KeyCode::P)
    );
    assert_eq!(
        repeat_due(&mut state, Some(KeyCode::P), false, Mode::Normal, 0.8),
        None
    );
    assert_eq!(
        repeat_due(&mut state, Some(KeyCode::P), false, Mode::Normal, 0.85),
        None
    );
    assert_eq!(
        repeat_due(&mut state, Some(KeyCode::P), true, Mode::Normal, 0.9),
        Some(KeyCode::P)
    );
    repeat_due(&mut state, None, false, Mode::Normal, 1.0);
    assert_eq!(
        repeat_due(&mut state, Some(KeyCode::P), true, Mode::Normal, 1.01),
        Some(KeyCode::P)
    );
    assert_eq!(
        repeat_due(&mut state, Some(KeyCode::P), false, Mode::Normal, 1.1),
        None
    );
}
