use super::*;

#[test]
fn terminal_block_selection_highlights_only_the_rectangle_and_case_operators_work() {
    let mut editor = Editor::new("aBc\ndEf", None);
    let mut ui = Ui::default();
    editor.cursor.col = 1;
    control(&mut editor, &mut ui, 'v');
    type_keys(&mut editor, &mut ui, "jl");
    let mut terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
    terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();
    let buffer = terminal.backend().buffer();
    let selection = rgb(editor.theme.palette().selection);
    assert_eq!(buffer[(4, 0)].bg, selection);
    assert_eq!(buffer[(5, 0)].bg, selection);
    assert_eq!(buffer[(4, 1)].bg, selection);
    assert_ne!(buffer[(3, 1)].bg, selection);
    type_keys(&mut editor, &mut ui, "gU");
    assert_eq!(editor.text(), "aBC\ndEF");
    type_keys(&mut editor, &mut ui, "guiw");
    assert_eq!(editor.text(), "abc\ndEF");
}

#[test]
fn terminal_folds_and_yanked_text_have_visible_feedback() {
    let mut editor = Editor::new("root\n  Child\n  Other\ntail", None);
    let mut ui = Ui::default();
    type_keys(&mut editor, &mut ui, "zc");
    let mut terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
    terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();
    let text: String = (0..30)
        .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
        .collect();
    assert!(text.contains("root … 2 lines"));
    assert_eq!(terminal.backend().buffer()[(3, 1)].symbol(), "t");
    type_keys(&mut editor, &mut ui, "zoyiw");
    assert!(editor.yank_blink_active());
    terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();
    assert_eq!(
        terminal.backend().buffer()[(4, 0)].bg,
        rgb(editor.theme.palette().search)
    );
    press(&mut editor, &mut ui, KeyCode::Char('r'));
    press(&mut editor, &mut ui, KeyCode::Enter);
    assert_eq!(editor.text(), "\noot\n  Child\n  Other\ntail");
}

#[test]
fn terminal_cells_highlights_and_prompt_remain_aligned_at_small_sizes() {
    let mut editor = Editor::new("a\t界\u{301}\u{1b}z", None);
    editor.cursor.col = 5;
    editor.search = "z".into();
    let ui = Ui {
        wrap: true,
        ..Ui::default()
    };
    let mut terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
    terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(4, 0)].symbol(), "→");
    assert_eq!(buffer[(5, 0)].symbol(), "�");
    assert_eq!(buffer[(8, 0)].symbol(), "z");
    assert_eq!(buffer[(8, 0)].bg, rgb(editor.theme.palette().accent));
    assert_eq!(editor.text(), "a\t界\u{301}\u{1b}z");
    editor.mode = Mode::Command;
    editor.prompt = "w a very long path with spaces.rs".into();
    for (width, height) in [(1, 1), (2, 2), (8, 4), (80, 24)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();
        let (x, y) = terminal.get_cursor_position().unwrap().into();
        assert!(x < width && y < height);
    }
}
