use super::*;

#[test]
fn wrapped_indent_aligns_text_search_highlights_and_insert_cursor() {
    let mut editor = Editor::new("  abcdefghijklmnopqrstuvwxyz", None);
    editor.cursor.col = 17;
    editor.begin_insert('i');
    editor.search = "op".into();
    let ui = Ui {
        wrap: true,
        ..Ui::default()
    };
    let mut terminal = Terminal::new(TestBackend::new(19, 7)).unwrap();
    terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();
    let buffer = terminal.backend().buffer();
    let text = |y| (3..19).map(|x| buffer[(x, y)].symbol()).collect::<String>();
    assert_eq!(text(0), "  abcdefghijklmn");
    assert_eq!(text(1), "      opqrstuvwx");
    assert_eq!(text(2), "      yz        ");
    assert_eq!(buffer[(1, 1)].symbol(), ">");
    assert_eq!(buffer[(9, 1)].bg, rgb(editor.theme.palette().search));
    assert_eq!(buffer[(10, 1)].bg, rgb(editor.theme.palette().search));
    let cursor: (u16, u16) = terminal.get_cursor_position().unwrap().into();
    assert_eq!(cursor, (10, 1));
    assert_eq!(editor.text(), "  abcdefghijklmnopqrstuvwxyz");
}

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

#[test]
fn buffer_list_is_a_compact_bottom_anchored_overlay() {
    let mut editor = Editor::new("source line", None);
    editor.show_buffer_list("Buffers\n%   1  current\n:b switch".into());
    editor.mode = Mode::Command;
    editor.prompt = "w".into();
    let ui = Ui::default();
    let mut terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    terminal.draw(|frame| draw(frame, &editor, &ui)).unwrap();

    let buffer = terminal.backend().buffer();
    let row = |y| (0..30).map(|x| buffer[(x, y)].symbol()).collect::<String>();
    assert!(row(0).contains("source line"));
    assert!(row(5).contains("Buffers"));
    assert!(row(6).contains("%   1  current"));
    assert!(row(7).contains(":b switch"));
}
