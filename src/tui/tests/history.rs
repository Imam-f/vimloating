use super::*;

#[test]
fn terminal_history_windows_render_edit_execute_and_transfer_to_the_prompt() {
    let mut editor = Editor::new("source", None);
    let mut ui = Ui::default();
    type_keys(&mut editor, &mut ui, ":theme default");
    press(&mut editor, &mut ui, KeyCode::Enter);
    type_keys(&mut editor, &mut ui, ":q:");
    assert_eq!(editor.mode, Mode::CommandWindow);
    press(&mut editor, &mut ui, KeyCode::Char('k'));
    let mut terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();
    let text: String = (0..30)
        .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
        .collect();
    assert!(text.contains("theme default"));
    control(&mut editor, &mut ui, 'c');
    assert_eq!(editor.mode, Mode::Command);
    assert_eq!(editor.prompt, "theme default");
    press(&mut editor, &mut ui, KeyCode::Esc);
    for shortcut in [":q/", ":q\\"] {
        type_keys(&mut editor, &mut ui, shortcut);
        press(&mut editor, &mut ui, KeyCode::Char('i'));
        handle_paste(&mut editor, "source");
        press(&mut editor, &mut ui, KeyCode::Enter);
        assert_eq!(editor.mode, Mode::Normal);
        assert_eq!(editor.search, "source");
        assert_eq!(editor.text(), "source");
    }
    type_keys(&mut editor, &mut ui, ":q:");
    press(&mut editor, &mut ui, KeyCode::Esc);
    assert!(editor.command_window.is_none());
    assert!(!editor.dirty());
}

#[test]
fn terminal_control_n_p_browse_history_and_move_lines() {
    let mut editor = Editor::new("one\ntwo\nthree", None);
    let mut ui = Ui::default();
    control(&mut editor, &mut ui, 'n');
    assert_eq!(editor.cursor.row, 1);
    control(&mut editor, &mut ui, 'p');
    assert_eq!(editor.cursor.row, 0);
    type_keys(&mut editor, &mut ui, ":theme default");
    press(&mut editor, &mut ui, KeyCode::Enter);
    type_keys(&mut editor, &mut ui, ":theme");
    control(&mut editor, &mut ui, 'p');
    assert_eq!(editor.prompt, "theme default");
    control(&mut editor, &mut ui, 'n');
    assert_eq!(editor.prompt, "theme");
}
