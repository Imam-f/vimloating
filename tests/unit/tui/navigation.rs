use super::*;

#[test]
fn terminal_ge_moves_backward_by_word_with_counts() {
    let mut editor = Editor::new("one two three", None);
    editor.cursor.col = 12;
    let mut ui = Ui {
        wrap: true,
        ..Ui::default()
    };
    type_keys(&mut editor, &mut ui, "2ge");
    assert_eq!(editor.cursor.col, 2);
    assert!(!editor.dirty());
}

#[test]
fn terminal_j_k_follow_wrapping_and_gj_gk_skip_wrapped_rows() {
    let mut editor = Editor::new("  abcdefghijklmnopqrstuvwxyz\nsecond line", None);
    editor.cursor.col = 9;
    let mut ui = Ui {
        wrap: true,
        ..Ui::default()
    };
    type_keys(&mut editor, &mut ui, "j");
    assert_eq!(editor.cursor, Pos { row: 0, col: 23 });
    type_keys(&mut editor, &mut ui, "k");
    assert_eq!(editor.cursor, Pos { row: 0, col: 9 });
    type_keys(&mut editor, &mut ui, "gj");
    assert_eq!(editor.cursor, Pos { row: 1, col: 9 });
    type_keys(&mut editor, &mut ui, "gk");
    assert_eq!(editor.cursor, Pos { row: 0, col: 9 });
    for _ in 0..2 {
        handle_key(
            &mut editor,
            &mut ui,
            KeyEvent::new_with_kind(KeyCode::Char('j'), KeyModifiers::NONE, KeyEventKind::Repeat),
            8,
            20,
        );
    }
    assert_eq!(editor.cursor, Pos { row: 1, col: 9 });
    assert!(!editor.dirty());
}

#[test]
fn terminal_control_j_k_scroll_viewport_and_keep_cursor_visible() {
    let text = (0..30)
        .map(|row| format!("line {row}\n"))
        .collect::<String>();
    let mut editor = Editor::new(&text, None);
    let mut ui = Ui::default();
    editor.cursor = Pos { row: 6, col: 2 };
    control(&mut editor, &mut ui, 'j');
    assert_eq!(editor.top, 5);
    assert_eq!(editor.cursor, Pos { row: 6, col: 2 });
    editor.reveal_cursor(8, 20, ui.wrap);
    assert_eq!(editor.top, 5);
    control(&mut editor, &mut ui, 'k');
    assert_eq!(editor.top, 0);
    assert_eq!(editor.cursor, Pos { row: 6, col: 2 });
    control(&mut editor, &mut ui, 'k');
    assert_eq!(editor.top, 0);

    control(&mut editor, &mut ui, 'j');
    control(&mut editor, &mut ui, 'j');
    assert_eq!(editor.top, 10);
    assert_eq!(editor.cursor, Pos { row: 10, col: 2 });
    for _ in 0..10 {
        control(&mut editor, &mut ui, 'j');
    }
    assert_eq!(editor.top, 30);
    assert_eq!(editor.cursor.row, 30);
    assert_eq!(editor.text(), text);
    assert!(!editor.dirty());
}

#[test]
fn terminal_control_j_k_scroll_wrapped_display_lines() {
    let mut editor = Editor::new(&format!("{}\ntail", "a".repeat(200)), None);
    let mut ui = Ui {
        wrap: true,
        ..Ui::default()
    };
    editor.cursor.col = 121;
    control(&mut editor, &mut ui, 'j');
    assert_eq!(editor.top, 5);
    assert_eq!(editor.cursor, Pos { row: 0, col: 121 });
    control(&mut editor, &mut ui, 'k');
    assert_eq!(editor.top, 0);
    assert_eq!(editor.cursor, Pos { row: 0, col: 121 });
    control(&mut editor, &mut ui, 'j');
    control(&mut editor, &mut ui, 'j');
    assert_eq!(editor.top, 10);
    assert_eq!(editor.cursor, Pos { row: 0, col: 169 });
    assert!(!editor.dirty());
}

#[test]
fn uppercase_find_targets_are_not_intercepted_as_screen_motions() {
    let mut editor = Editor::new("a H M L\nsecond line", None);
    let mut ui = Ui::default();
    press(&mut editor, &mut ui, KeyCode::Char('f'));
    press(&mut editor, &mut ui, KeyCode::Char('L'));
    assert_eq!(editor.cursor, Pos { row: 0, col: 6 });
    assert_eq!(editor.char_find_highlight, Some(editor.cursor));
}

#[test]
fn output_scroll_does_not_move_the_buffer_cursor_and_escape_restores_editor() {
    let mut editor = Editor::new("keep this", None);
    editor.mode = Mode::ShellOutput;
    editor.output_view = Some((0..30).map(|n| format!("output {n}\n")).collect());
    let mut ui = Ui::default();
    press(&mut editor, &mut ui, KeyCode::End);
    assert_eq!(ui.output_top, 22);
    assert_eq!(editor.cursor, Pos::default());
    let mut terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0..30).map(|x| buffer[(x, 0)].symbol()).collect();
    assert!(text.contains("output 22"));
    press(&mut editor, &mut ui, KeyCode::Esc);
    assert_eq!(editor.mode, Mode::Normal);
    assert_eq!(editor.text(), "keep this");
    assert!(editor.output_view.is_none());
}
