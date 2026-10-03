use super::*;

#[test]
fn desktop_frames_repeat_ctrl_n_p_and_stop_when_ctrl_is_released() {
    let mut editor = Editor::new("hello help helmet\nhe", None);
    editor.cursor.row = 1;
    editor.begin_insert('A');
    let mut keyboard = KeyboardHarness::new();
    for (time, pressed, selected) in [(0.0, true, 0), (0.35, false, 1), (0.42, false, 2)] {
        keyboard.frame(
            &mut editor,
            time,
            &[KeyCode::LeftControl, KeyCode::N],
            if pressed { &[KeyCode::N] } else { &[] },
            "",
        );
        assert_eq!(editor.completion_popup().unwrap().selected, selected);
    }
    keyboard.frame(&mut editor, 0.5, &[KeyCode::N], &[], "");
    assert_eq!(editor.completion_popup().unwrap().selected, 2);
    for (time, pressed, selected) in [(0.6, true, 1), (0.96, false, 0), (1.03, false, 2)] {
        keyboard.frame(
            &mut editor,
            time,
            &[KeyCode::RightControl, KeyCode::P],
            if pressed { &[KeyCode::P] } else { &[] },
            "",
        );
        assert_eq!(editor.completion_popup().unwrap().selected, selected);
    }
    keyboard.frame(&mut editor, 1.1, &[], &[], "");
    assert_eq!(editor.completion_popup().unwrap().selected, 2);
}

#[test]
fn timed_ctrl_n_p_cycle_completion_and_move_in_normal_mode() {
    let mut editor = Editor::new("hello help helmet\nhe", None);
    editor.cursor.row = 1;
    editor.begin_insert('A');
    let mut state = None;
    for (time, pressed, selected) in [(0.0, true, 0), (0.35, false, 1), (0.42, false, 2)] {
        let key = repeat_due(&mut state, Some(KeyCode::N), pressed, editor.mode, time).unwrap();
        assert!(handle_repeated_key(&mut editor, key));
        assert_eq!(editor.completion_popup().unwrap().selected, selected);
    }
    for (time, pressed, selected) in [(0.5, true, 1), (0.86, false, 0)] {
        let key = repeat_due(&mut state, Some(KeyCode::P), pressed, editor.mode, time).unwrap();
        handle_repeated_key(&mut editor, key);
        assert_eq!(editor.completion_popup().unwrap().selected, selected);
    }
    editor.escape();
    handle_repeated_key(&mut editor, KeyCode::P);
    assert_eq!(editor.cursor.row, 0);
    handle_repeated_key(&mut editor, KeyCode::N);
    assert_eq!(editor.cursor.row, 1);
}
