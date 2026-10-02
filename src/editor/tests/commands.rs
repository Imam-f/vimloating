use super::super::*;

#[test]
fn numeric_command_jumps_to_first_nonblank_and_clamps_to_document_bounds() {
    let text = (1..=130)
        .map(|row| format!("  line {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut e = Editor::new(&text, None);
    e.move_by(0, 1, 1);
    e.normal_key(':');
    e.prompt = "120".into();
    e.submit_prompt();
    assert_eq!(e.cursor, Pos { row: 119, col: 2 });
    assert_eq!(e.mode, Mode::Normal);
    e.normal_key('j');
    assert_eq!(e.cursor, Pos { row: 120, col: 2 });
    e.command("999");
    assert_eq!(e.cursor, Pos { row: 129, col: 2 });
    e.command("0");
    assert_eq!(e.cursor, Pos { row: 0, col: 2 });
    e.command("-1");
    assert_eq!(e.cursor, Pos { row: 0, col: 2 });
    let mut folded = Editor::new("root\n    child\nnext", None);
    for key in "zc".chars() {
        folded.normal_key(key);
    }
    folded.command("2");
    assert_eq!(folded.cursor, Pos { row: 1, col: 4 });
    assert_eq!(folded.display_total(80, false), 3);
}

#[test]
fn quit_protects_unsaved_changes() {
    let mut e = Editor::new("", None);
    e.begin_insert('i');
    e.insert_char('x');
    e.escape();
    e.command("q");
    assert!(!e.quit);
    e.command("q!");
    assert!(e.quit);
}

#[test]
fn shell_command_captures_output_and_escape_returns_to_the_buffer() {
    let mut editor = Editor::new("keep this buffer", None);
    #[cfg(windows)]
    editor.command("!echo vimloating-shell-test");
    #[cfg(not(windows))]
    editor.command("!printf vimloating-shell-test");

    assert_eq!(editor.mode, Mode::ShellOutput);
    assert!(
        editor
            .output_view
            .as_deref()
            .unwrap()
            .contains("vimloating-shell-test")
    );
    editor.escape();
    assert_eq!(editor.mode, Mode::Normal);
    assert!(editor.output_view.is_none());
    assert_eq!(editor.text(), "keep this buffer");
}

#[test]
fn theme_command_switches_palettes_and_accepts_aliases() {
    let mut editor = Editor::new("", None);
    assert_eq!(editor.theme, crate::config::Theme::Vimfloating);

    editor.command("theme everforest");
    assert_eq!(editor.theme, crate::config::Theme::Everforest);
    assert_eq!(editor.message, "Theme: Everforest");

    editor.command("theme solarized-blue");
    assert_eq!(editor.theme, crate::config::Theme::SolarizedBlue);
    assert_eq!(editor.theme.name(), "Solarized Dark Blue");

    editor.command("theme default");
    assert_eq!(editor.theme, crate::config::Theme::Vimfloating);
}

#[test]
fn command_tab_completion_cycles_matching_commands() {
    let mut editor = Editor::new("", None);
    editor.mode = Mode::Command;
    editor.prompt = "buf".into();

    editor.complete_command();
    assert_eq!(editor.prompt, "buffer");
    editor.complete_command();
    assert_eq!(editor.prompt, "buffers");
    editor.complete_command();
    assert_eq!(editor.prompt, "buffer");
}

#[test]
fn shell_filter_replaces_the_current_line_and_can_be_undone() {
    let original = "replace this\nkeep this";
    let mut editor = Editor::new(original, None);
    #[cfg(windows)]
    editor.command(".!echo filtered-line");
    #[cfg(not(windows))]
    editor.command(".!printf filtered-line");

    assert_eq!(editor.text(), "filtered-line\nkeep this");
    assert_eq!(editor.cursor.row, 0);
    editor.undo(false);
    assert_eq!(editor.text(), original);
}

#[test]
fn shell_pwd_filter_replaces_the_line_with_the_current_directory() {
    let mut editor = Editor::new("replace this", None);
    editor.command(".!pwd");
    assert_eq!(
        editor.text(),
        std::env::current_dir().unwrap().to_string_lossy()
    );
}
