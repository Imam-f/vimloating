use super::*;

#[test]
fn terminal_alt_h_l_indent_lines_and_visual_selections_with_undo_and_repeat() {
    let mut editor = Editor::new("one\ntwo\nthree", None);
    let mut ui = Ui::default();
    let alt = |editor: &mut Editor, ui: &mut Ui, ch| {
        handle_key(
            editor,
            ui,
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::ALT),
            8,
            20,
        );
    };
    alt(&mut editor, &mut ui, 'l');
    assert_eq!(editor.text(), "    one\ntwo\nthree");
    assert_eq!(editor.cursor.col, 4);
    press(&mut editor, &mut ui, KeyCode::Char('u'));
    assert_eq!(editor.text(), "one\ntwo\nthree");
    assert_eq!(editor.cursor.col, 0);

    type_keys(&mut editor, &mut ui, "Vj");
    let endpoints = (editor.anchor, editor.cursor);
    alt(&mut editor, &mut ui, 'L');
    assert_eq!(editor.text(), "    one\n    two\nthree");
    assert_eq!(editor.mode, Mode::Visual);
    assert_eq!((editor.anchor, editor.cursor), endpoints);
    alt(&mut editor, &mut ui, 'h');
    assert_eq!(editor.text(), "one\ntwo\nthree");
    alt(&mut editor, &mut ui, 'l');
    alt(&mut editor, &mut ui, 'H');
    assert_eq!(editor.text(), "one\ntwo\nthree");

    press(&mut editor, &mut ui, KeyCode::Esc);
    alt(&mut editor, &mut ui, 'l');
    type_keys(&mut editor, &mut ui, "j.");
    assert_eq!(editor.text(), "one\n    two\n    three");
    press(&mut editor, &mut ui, KeyCode::Char('u'));
    assert_eq!(editor.text(), "one\n    two\nthree");
}

#[test]
fn terminal_control_a_x_adjust_numbers_with_counts_undo_and_repeat() {
    let mut editor = Editor::new("value -009", None);
    let mut ui = Ui::default();
    type_keys(&mut editor, &mut ui, "3");
    control(&mut editor, &mut ui, 'a');
    assert_eq!(editor.text(), "value -006");
    assert!(editor.count.is_empty());
    type_keys(&mut editor, &mut ui, "2");
    control(&mut editor, &mut ui, 'x');
    assert_eq!(editor.text(), "value -008");
    assert!(editor.count.is_empty());
    press(&mut editor, &mut ui, KeyCode::Char('.'));
    assert_eq!(editor.text(), "value -010");
    press(&mut editor, &mut ui, KeyCode::Char('u'));
    assert_eq!(editor.text(), "value -008");
    control(&mut editor, &mut ui, 'r');
    assert_eq!(editor.text(), "value -010");

    press(&mut editor, &mut ui, KeyCode::Char('v'));
    control(&mut editor, &mut ui, 'a');
    control(&mut editor, &mut ui, 'x');
    assert_eq!(editor.text(), "value -010");
    assert_eq!(editor.mode, Mode::Visual);
}

#[test]
fn terminal_repeat_events_cycle_and_delete_while_release_events_do_nothing() {
    let mut e = Editor::new("hello help helmet\nhe", None);
    let mut ui = Ui::default();
    e.cursor.row = 1;
    e.begin_insert('A');
    for (kind, selected) in [
        (KeyEventKind::Press, 0),
        (KeyEventKind::Repeat, 1),
        (KeyEventKind::Repeat, 2),
        (KeyEventKind::Release, 2),
    ] {
        handle_key(
            &mut e,
            &mut ui,
            KeyEvent::new_with_kind(KeyCode::Char('n'), KeyModifiers::CONTROL, kind),
            8,
            20,
        );
        assert_eq!(e.completion_popup().unwrap().selected, selected);
    }
    handle_key(
        &mut e,
        &mut ui,
        KeyEvent::new_with_kind(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL,
            KeyEventKind::Repeat,
        ),
        8,
        20,
    );
    assert_eq!(e.completion_popup().unwrap().selected, 1);
    for key in [KeyCode::Backspace, KeyCode::Delete] {
        let mut e = Editor::new("abcdef", None);
        if key == KeyCode::Backspace {
            e.cursor.col = 5;
        }
        for kind in [KeyEventKind::Press, KeyEventKind::Repeat] {
            handle_key(
                &mut e,
                &mut ui,
                KeyEvent::new_with_kind(key, KeyModifiers::NONE, kind),
                8,
                20,
            );
        }
        assert_eq!(e.text(), "abcdef");
        assert_eq!(e.mode, Mode::Normal);
        assert_eq!(e.cursor.col, if key == KeyCode::Backspace { 3 } else { 0 });
        e.begin_insert('i');
        for kind in [
            KeyEventKind::Press,
            KeyEventKind::Repeat,
            KeyEventKind::Repeat,
            KeyEventKind::Release,
        ] {
            handle_key(
                &mut e,
                &mut ui,
                KeyEvent::new_with_kind(key, KeyModifiers::NONE, kind),
                8,
                20,
            );
        }
        assert_eq!(
            e.text(),
            if key == KeyCode::Backspace {
                "abcdef"
            } else {
                "def"
            }
        );
        assert_eq!(e.mode, Mode::Insert);
        assert_eq!(e.cursor.col, 0);
        e.escape();
        e.undo(false);
        assert_eq!(e.text(), "abcdef");
    }
}

#[test]
fn terminal_edit_search_undo_and_quit_protection_work_together() {
    let mut editor = Editor::new("hello world", None);
    let mut ui = Ui {
        wrap: true,
        ..Ui::default()
    };
    press(&mut editor, &mut ui, KeyCode::Char('A'));
    for ch in " café".chars() {
        press(&mut editor, &mut ui, KeyCode::Char(ch));
    }
    press(&mut editor, &mut ui, KeyCode::Esc);
    assert_eq!(editor.text(), "hello world café");
    press(&mut editor, &mut ui, KeyCode::Char('/'));
    for ch in "world".chars() {
        press(&mut editor, &mut ui, KeyCode::Char(ch));
    }
    press(&mut editor, &mut ui, KeyCode::Enter);
    editor.advance_search();
    assert_eq!(editor.cursor.col, 6);
    press(&mut editor, &mut ui, KeyCode::Char(':'));
    press(&mut editor, &mut ui, KeyCode::Char('q'));
    press(&mut editor, &mut ui, KeyCode::Enter);
    assert!(!editor.quit);
    press(&mut editor, &mut ui, KeyCode::Char('u'));
    assert_eq!(editor.text(), "hello world");
    handle_key(
        &mut editor,
        &mut ui,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        8,
        20,
    );
    assert_eq!(editor.text(), "hello world café");
}

#[test]
fn legacy_control_encodings_edit_insert_text_and_submit_prompts() {
    let mut editor = Editor::new("", None);
    let mut ui = Ui::default();
    press(&mut editor, &mut ui, KeyCode::Char('i'));
    press(&mut editor, &mut ui, KeyCode::Char('a'));
    for ch in ['j', 'i', 'h'] {
        handle_key(
            &mut editor,
            &mut ui,
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL),
            8,
            20,
        );
    }
    assert_eq!(editor.text(), "a\n    ");
    assert_eq!(editor.cursor, Pos { row: 1, col: 3 });
    handle_key(
        &mut editor,
        &mut ui,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL),
        8,
        20,
    );
    assert_eq!(editor.text(), "a\n   \n    ");
    press(&mut editor, &mut ui, KeyCode::Esc);
    press(&mut editor, &mut ui, KeyCode::Char(':'));
    for ch in "q!".chars() {
        press(&mut editor, &mut ui, KeyCode::Char(ch));
    }
    handle_key(
        &mut editor,
        &mut ui,
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::CONTROL),
        8,
        20,
    );
    assert!(editor.quit && editor.force_quit);
}
