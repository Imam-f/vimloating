use super::*;

#[test]
fn desktop_ge_moves_backward_by_word_with_counts() {
    let mut editor = Editor::new("one two three", None);
    editor.cursor.col = 12;
    KeyboardHarness::new().frame(&mut editor, 0.0, &[], &[], "2ge");
    assert_eq!(editor.cursor.col, 2);
    assert!(!editor.dirty());
}

#[test]
fn desktop_j_k_use_current_wrap_width_and_gj_gk_use_source_lines() {
    let mut keyboard = KeyboardHarness::new();
    let cols = text_grid(keyboard.font_size).1;
    let text = format!("  {}\n  {}", "x".repeat(cols * 2), "y".repeat(cols * 2));
    let mut editor = Editor::new(&text, None);
    editor.cursor.col = 9;
    for (now, chars, expected) in [
        (
            0.0,
            "j",
            vimloating::editor::Pos {
                row: 0,
                col: cols + 3,
            },
        ),
        (0.1, "k", vimloating::editor::Pos { row: 0, col: 9 }),
        (0.2, "gj", vimloating::editor::Pos { row: 1, col: 9 }),
        (0.3, "gk", vimloating::editor::Pos { row: 0, col: 9 }),
    ] {
        keyboard.frame(&mut editor, now, &[], &[], chars);
        assert_eq!(editor.cursor, expected);
    }
    keyboard.wrap = false;
    keyboard.frame(&mut editor, 0.4, &[], &[], "j");
    assert_eq!(editor.cursor, vimloating::editor::Pos { row: 1, col: 9 });
}

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
fn desktop_buffer_list_scrolls_with_ctrl_u_and_ctrl_d() {
    let mut editor = Editor::new("keep this", None);
    editor.mode = Mode::BufferList;
    editor.output_view = Some((0..100).map(|n| format!("buffer {n}\n")).collect());
    let mut keyboard = KeyboardHarness::new();
    let rows = text_grid(keyboard.font_size).0;
    let page = (rows / 2).max(1);

    keyboard.frame(
        &mut editor,
        0.0,
        &[KeyCode::LeftControl, KeyCode::D],
        &[KeyCode::D],
        "",
    );
    assert_eq!(editor.output_top, page);
    keyboard.frame(
        &mut editor,
        0.1,
        &[KeyCode::LeftControl, KeyCode::U],
        &[KeyCode::U],
        "",
    );
    assert_eq!(editor.output_top, 0);
    keyboard.frame(
        &mut editor,
        0.2,
        &[KeyCode::LeftControl, KeyCode::N],
        &[KeyCode::N],
        "",
    );
    assert_eq!(editor.output_top, page);
    keyboard.frame(
        &mut editor,
        0.3,
        &[KeyCode::LeftControl, KeyCode::P],
        &[KeyCode::P],
        "",
    );
    assert_eq!(editor.output_top, 0);
    assert_eq!(editor.cursor, vimloating::editor::Pos::default());
}

#[test]
fn desktop_buffer_list_can_open_command_mode_without_hiding_the_panel() {
    let mut editor = Editor::new("keep this", None);
    editor.show_buffer_list("Buffers\n%   1  current".into());
    let mut keyboard = KeyboardHarness::new();

    keyboard.frame(&mut editor, 0.0, &[], &[], ":");
    assert_eq!(editor.mode, Mode::Command);
    assert!(editor.buffer_list_visible);
    assert!(editor.output_view.is_some());
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
fn desktop_picker_repeats_up_down_and_accepts_ctrl_n_p_j_m() {
    let mut buffers =
        vimloating::editor::buffers::BufferList::new(Editor::new("alpha\nbeta\ngamma", None));
    buffers.active_mut().command("Lines");
    buffers.process_pending();
    let mut keyboard = KeyboardHarness::new();

    keyboard.frame(
        buffers.active_mut(),
        0.0,
        &[KeyCode::Down],
        &[KeyCode::Down],
        "",
    );
    keyboard.frame(buffers.active_mut(), 0.35, &[KeyCode::Down], &[], "");
    keyboard.frame(
        buffers.active_mut(),
        0.5,
        &[KeyCode::LeftControl, KeyCode::P],
        &[KeyCode::LeftControl, KeyCode::P],
        "",
    );
    keyboard.frame(
        buffers.active_mut(),
        0.6,
        &[KeyCode::LeftControl, KeyCode::N],
        &[KeyCode::LeftControl, KeyCode::N],
        "",
    );
    assert!(
        buffers
            .active()
            .output_view
            .as_deref()
            .unwrap()
            .contains(">     3  gamma")
    );
    keyboard.frame(
        buffers.active_mut(),
        0.7,
        &[KeyCode::LeftControl, KeyCode::J],
        &[KeyCode::LeftControl, KeyCode::J],
        "",
    );
    buffers.process_pending();
    assert_eq!(buffers.active().cursor.row, 2);

    buffers.active_mut().command("Lines");
    buffers.process_pending();
    keyboard.frame(
        buffers.active_mut(),
        0.8,
        &[KeyCode::LeftControl, KeyCode::M],
        &[KeyCode::LeftControl, KeyCode::M],
        "",
    );
    buffers.process_pending();
    assert_eq!(buffers.active().cursor.row, 0);
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
