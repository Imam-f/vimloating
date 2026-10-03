use super::*;

#[test]
fn terminal_path_completion_and_gf_use_the_shared_editor() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vimloating-tui-paths-{unique}"));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("alpha.txt"), "alpha\nsecond").unwrap();
    std::fs::write(root.join("alpine.txt"), "alpine").unwrap();
    let mut editor = Editor::new("", Some(root.join("source.txt")));
    let mut ui = Ui::default();
    type_keys(&mut editor, &mut ui, "ial");
    control(&mut editor, &mut ui, 'x');
    control(&mut editor, &mut ui, 'f');
    assert_eq!(editor.text(), "alpha.txt");
    control(&mut editor, &mut ui, 'n');
    assert_eq!(editor.text(), "alpine.txt");
    control(&mut editor, &mut ui, 'p');
    assert_eq!(editor.text(), "alpha.txt");
    type_keys(&mut editor, &mut ui, ":2:2");
    press(&mut editor, &mut ui, KeyCode::Esc);
    let mut buffers = BufferList::new(editor);
    type_keys(buffers.active_mut(), &mut ui, "gf");
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "alpha\nsecond");
    assert_eq!(buffers.active().cursor, Pos { row: 1, col: 1 });
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn terminal_word_and_line_completion_use_ctrl_n_p_and_ctrl_x_l() {
    let mut editor = Editor::new("hello there\nhelp here\n    he", None);
    let mut ui = Ui {
        wrap: true,
        ..Ui::default()
    };
    editor.cursor.row = 2;
    type_keys(&mut editor, &mut ui, "A");
    control(&mut editor, &mut ui, 'n');
    assert_eq!(editor.lines[2].iter().collect::<String>(), "    hello");
    control(&mut editor, &mut ui, 'p');
    assert_eq!(editor.lines[2].iter().collect::<String>(), "    here");
    // Replace the word prefix before starting whole-line completion.
    for _ in 0..2 {
        press(&mut editor, &mut ui, KeyCode::Backspace);
        press(&mut editor, &mut ui, KeyCode::Delete);
    }
    control(&mut editor, &mut ui, 'x');
    control(&mut editor, &mut ui, 'l');
    assert_eq!(
        editor.lines[2].iter().collect::<String>(),
        "    hello there"
    );
    control(&mut editor, &mut ui, 'l');
    assert_eq!(editor.lines[2].iter().collect::<String>(), "    help here");
    control(&mut editor, &mut ui, 'p');
    assert_eq!(
        editor.lines[2].iter().collect::<String>(),
        "    hello there"
    );
    assert!(ui.wrap);
    press(&mut editor, &mut ui, KeyCode::Esc);
    control(&mut editor, &mut ui, 'p');
    assert_eq!(editor.cursor.row, 1);
    control(&mut editor, &mut ui, 'l');
    assert!(!ui.wrap);
}

#[test]
fn terminal_popup_renders_candidates_and_selection_then_disappears_after_typing() {
    let mut e = Editor::new("hello help helmet\nhe", None);
    e.cursor.row = 1;
    e.begin_insert('A');
    e.control_key('n');
    let ui = Ui::default();
    let mut terminal = Terminal::new(TestBackend::new(24, 12)).unwrap();
    terminal.draw(|frame| draw(frame, &e, &ui)).unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("Word 1/3"));
    assert!(text.contains("helmet"));
    assert_eq!(buffer[(4, 3)].bg, rgb(e.theme.palette().accent));
    assert_eq!(buffer[(4, 3)].symbol(), "h");
    assert_ne!(buffer[(4, 4)].bg, rgb(e.theme.palette().accent));
    e.control_key('n');
    terminal.draw(|frame| draw(frame, &e, &ui)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_ne!(buffer[(4, 3)].bg, rgb(e.theme.palette().accent));
    assert_eq!(buffer[(4, 4)].bg, rgb(e.theme.palette().accent));
    e.insert_char('!');
    terminal.draw(|frame| draw(frame, &e, &ui)).unwrap();
    assert_ne!(
        terminal.backend().buffer()[(4, 4)].bg,
        rgb(e.theme.palette().accent)
    );
    for (width, height) in [(1, 1), (5, 3), (12, 6), (30, 12)] {
        terminal.backend_mut().resize(width, height);
        terminal.resize(Rect::new(0, 0, width, height)).unwrap();
        e.backspace();
        e.complete_word(false);
        terminal.draw(|frame| draw(frame, &e, &ui)).unwrap();
    }
}

#[test]
fn terminal_command_popup_stays_above_the_status_and_prompt() {
    let mut e = Editor::new("one\ntwo", None);
    e.normal_key(':');
    e.prompt = "buf".into();
    e.complete_command();
    let mut terminal = Terminal::new(TestBackend::new(24, 10)).unwrap();
    terminal
        .draw(|frame| draw(frame, &e, &Ui::default()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let row = |y| (0..24).map(|x| buffer[(x, y)].symbol()).collect::<String>();
    assert!(row(4).contains("Command 1/2"));
    assert!(row(5).contains("buffer"));
    assert!(row(6).contains("buffers"));
    assert!(row(8).contains("COMMAND"));
    assert!(row(9).starts_with(":buffer"));
}
