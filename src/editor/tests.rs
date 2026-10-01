use super::*;
use std::fs;

#[test]
fn unicode_editing_and_insert_session_undo() {
    let mut e = Editor::new("héllo\n世界", None);
    e.begin_insert('A');
    e.insert_char('🦀');
    e.newline();
    e.insert_char('λ');
    e.escape();
    assert_eq!(e.text(), "héllo🦀\nλ\n世界");
    e.undo(false);
    assert_eq!(e.text(), "héllo\n世界");
    assert!(!e.dirty());
    e.undo(true);
    assert_eq!(e.text(), "héllo🦀\nλ\n世界");
}

#[test]
fn counted_delete_and_linewise_paste() {
    let mut e = Editor::new("one\ntwo\nthree\nfour", None);
    for c in "2dd".chars() {
        e.normal_key(c);
    }
    assert_eq!(e.text(), "three\nfour");
    e.normal_key('p');
    assert_eq!(e.text(), "three\none\ntwo\nfour");
    e.undo(false);
    e.undo(false);
    assert_eq!(e.text(), "one\ntwo\nthree\nfour");
}

#[test]
fn capital_x_deletes_before_cursor_and_supports_count_undo_and_paste() {
    let mut e = Editor::new("abcdef", None);
    e.cursor.col = 4;
    for c in "2X".chars() {
        e.normal_key(c);
    }
    assert_eq!(e.text(), "abef");
    assert_eq!(e.cursor.col, 2);
    e.normal_key('P');
    assert_eq!(e.text(), "abcdef");
    e.undo(false);
    assert_eq!(e.text(), "abef");
    e.undo(false);
    assert_eq!(e.text(), "abcdef");
}

#[test]
fn visual_multiline_delete_preserves_boundary_text() {
    let mut e = Editor::new("abcd\nefgh\nijkl", None);
    e.cursor.col = 2;
    e.normal_key('v');
    e.move_by(0, 1, 1);
    e.move_by(-1, 0, 1);
    e.normal_key('d');
    assert_eq!(e.text(), "abgh\nijkl");
    e.normal_key('P');
    assert_eq!(e.text(), "abcd\nefgh\nijkl");
}

#[test]
fn visual_line_selection_yanks_and_pastes_complete_lines() {
    let mut e = Editor::new("one\ntwo\nthree\nfour", None);
    e.normal_key('V');
    e.move_by(0, 1, 2);
    e.normal_key('y');
    assert_eq!(e.mode, Mode::Normal);
    assert!(e.linewise);
    assert_eq!(
        e.register,
        vec![
            "one".chars().collect::<Vec<_>>(),
            "two".chars().collect::<Vec<_>>(),
            "three".chars().collect::<Vec<_>>()
        ]
    );

    e.normal_key('p');
    assert_eq!(e.text(), "one\ntwo\nthree\none\ntwo\nthree\nfour");
}

#[test]
fn visual_line_selection_delete_keeps_an_editable_line() {
    let mut e = Editor::new("one\ntwo\nthree", None);
    e.cursor.row = 1;
    e.normal_key('V');
    e.move_by(0, -1, 1);
    e.normal_key('d');
    assert_eq!(e.text(), "three");
    assert_eq!(e.cursor, Pos { row: 0, col: 0 });

    e.normal_key('V');
    e.normal_key('d');
    assert_eq!(e.text(), "");
    assert_eq!(e.lines.len(), 1);
}

#[test]
fn search_wraps_and_uses_character_columns() {
    let mut e = Editor::new("é猫 x 猫\n猫", None);
    e.search = "猫".into();
    e.find(false);
    assert_eq!(e.cursor, Pos { row: 0, col: 1 });
    e.find(true);
    assert_eq!(e.cursor, Pos { row: 1, col: 0 });
    e.find(false);
    assert_eq!(e.cursor, Pos { row: 0, col: 1 });
}

#[test]
fn deleting_entire_document_keeps_editable_line() {
    let mut e = Editor::new("a\nb", None);
    for c in "9dd".chars() {
        e.normal_key(c);
    }
    assert_eq!(e.lines, vec![Vec::<char>::new()]);
    e.begin_insert('i');
    e.insert_char('x');
    assert_eq!(e.text(), "x");
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
fn multiline_paste_preserves_indentation_and_suffix() {
    let mut e = Editor::new("    tail", None);
    e.cursor.col = 4;
    e.begin_insert('i');
    e.insert_text("first\r\n\tsecond\n");
    assert_eq!(e.text(), "    first\n\tsecond\ntail");
    e.escape();
    e.undo(false);
    assert_eq!(e.text(), "    tail");
}

#[test]
fn ctrl_backspace_removes_previous_unicode_word_and_spacing() {
    let mut e = Editor::new("one café  two", None);
    e.cursor.col = e.lines[0].len();
    e.mode = Mode::Insert;
    e.delete_prev_word();
    assert_eq!(e.text(), "one café  ");
    e.delete_prev_word();
    assert_eq!(e.text(), "one ");
    assert_eq!(e.cursor.col, 4);
}

#[test]
fn wrapped_visual_rows_track_long_lines_and_insert_end() {
    let mut e = Editor::new("abcdef\nx\n", None);
    assert_eq!(
        e.display_rows(3, true),
        vec![(0, 0), (0, 3), (1, 0), (2, 0)]
    );
    e.cursor = Pos { row: 0, col: 6 };
    e.mode = Mode::Insert;
    assert_eq!(
        e.display_rows(3, true),
        vec![(0, 0), (0, 3), (0, 6), (1, 0), (2, 0)]
    );
    e.reveal_cursor(2, 3, true);
    assert_eq!(e.top, 1);
    assert_eq!(e.left, 0);
}

#[test]
fn horizontal_scroll_moves_the_buffer_view_without_moving_the_cursor() {
    let mut e = Editor::new("abcdefghijklmnopqrstuvwxyz", None);
    e.scroll_horizontal(1, 10);
    e.reveal_cursor(5, 10, false);
    assert_eq!(e.left, 5);
    assert_eq!(e.cursor.col, 0);

    e.move_by(1, 0, 1);
    e.reveal_cursor(5, 10, false);
    assert_eq!(e.left, 1);
}

#[test]
fn vertical_scroll_can_overscroll_until_only_the_last_line_is_visible() {
    let mut e = Editor::new("0\n1\n2\n3\n4\n5\n6\n7\n8\n9", None);
    e.scroll_vertical(1, 100, 5, 20, false);

    assert_eq!(e.top, 9);
    assert_eq!(e.cursor.row, 9);
    assert_eq!(e.display_rows(20, false).len() - e.top, 1);

    e.scroll_vertical(-1, 1, 5, 20, false);
    assert_eq!(e.top, 8);
    assert_eq!(e.cursor.row, 9);
}

#[test]
fn zoom_does_not_scroll_until_the_cursor_moves() {
    let mut e = Editor::new("0\n1\n2\n3\n4\n5\n6\n7\n8\n9", None);
    e.cursor.row = 8;
    e.reveal_cursor_after_motion(e.cursor, 5, 20, true);
    assert_eq!(e.top, 0);

    let previous_cursor = e.cursor;
    e.cursor.row = 9;
    e.reveal_cursor_after_motion(previous_cursor, 5, 20, true);
    assert_eq!(e.top, 5);
}

#[test]
fn save_round_trip_and_failed_save_keeps_dirty_state() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("vimloating-{}-{unique}.txt", std::process::id()));
    let mut e = Editor::new("", Some(path.clone()));
    e.begin_insert('i');
    e.insert_text("café\n世界\n");
    e.escape();
    assert!(e.dirty());
    assert!(e.save(None));
    assert!(!e.dirty());
    let text = fs::read_to_string(&path).unwrap();
    assert_eq!(text, "café\n世界\n");
    let reopened = Editor::new(&text, Some(path.clone()));
    assert_eq!(reopened.lines, e.lines);
    e.begin_insert('A');
    e.insert_char('x');
    assert!(!e.save(Some(&path.join("missing/file").to_string_lossy())));
    assert!(e.dirty());
    assert_eq!(e.path.as_ref(), Some(&path));
    fs::remove_file(path).unwrap();
}

#[test]
fn directory_browser_opens_subdirectories_and_files() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vimloating-netrw-{unique}"));
    let subdir = root.join("folder");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("notes.txt"), "browse me").unwrap();

    let editor = Editor::open_path(root.clone()).unwrap();
    let mut buffers = super::buffers::BufferList::new(editor);
    assert!(buffers.active().is_directory_browser());
    let folder_row = buffers
        .active()
        .directory_entries
        .as_ref()
        .unwrap()
        .iter()
        .position(|entry| entry.ends_with("folder"))
        .unwrap();
    buffers.active_mut().cursor.row = folder_row;
    buffers.active_mut().open_directory_entry();
    buffers.process_pending();
    assert!(buffers.active().is_directory_browser());

    let file_row = buffers
        .active()
        .directory_entries
        .as_ref()
        .unwrap()
        .iter()
        .position(|entry| entry.ends_with("notes.txt"))
        .unwrap();
    buffers.active_mut().cursor.row = file_row;
    buffers.active_mut().open_directory_entry();
    buffers.process_pending();
    assert!(!buffers.active().is_directory_browser());
    assert_eq!(buffers.active().text(), "browse me");

    fs::remove_dir_all(root).unwrap();
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
fn explore_does_not_discard_unsaved_buffer_changes() {
    let mut editor = Editor::new("keep this buffer", None);
    editor.begin_insert('A');
    editor.insert_char('!');
    editor.escape();

    let mut buffers = super::buffers::BufferList::new(editor);
    buffers.active_mut().command("Explore");
    buffers.process_pending();
    assert!(buffers.active().is_directory_browser());

    buffers.active_mut().command("bp");
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "keep this buffer!");
    assert!(buffers.active().dirty());
}

#[test]
fn opening_another_file_keeps_unsaved_changes_in_the_previous_buffer() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("vimloating-open-unsaved-{unique}.txt"));
    fs::write(&path, "new file").unwrap();

    let mut editor = Editor::new("unsaved", None);
    editor.begin_insert('A');
    editor.insert_char('!');
    editor.escape();
    let mut buffers = super::buffers::BufferList::new(editor);
    buffers
        .active_mut()
        .command(&format!("e {}", path.display()));
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "new file");

    buffers.active_mut().begin_insert('A');
    buffers.active_mut().insert_char('?');
    buffers.active_mut().escape();
    buffers
        .active_mut()
        .command(&format!("e {}", path.display()));
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "new file?");
    assert!(buffers.active().dirty());

    buffers.active_mut().command("bp");
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "unsaved!");
    assert!(buffers.active().dirty());

    fs::remove_file(path).unwrap();
}

#[test]
fn ctrl6_toggles_between_the_active_and_last_opened_buffer() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vimloating-alternate-buffer-{unique}"));
    fs::create_dir_all(&root).unwrap();
    let first_path = root.join("first.txt");
    let second_path = root.join("second.txt");
    fs::write(&first_path, "first").unwrap();
    fs::write(&second_path, "second").unwrap();

    let mut buffers = super::buffers::BufferList::new(Editor::new("welcome", None));
    buffers
        .active_mut()
        .command(&format!("e {}", first_path.display()));
    buffers.process_pending();
    buffers
        .active_mut()
        .command(&format!("e {}", second_path.display()));
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "second");

    buffers.active_mut().buffer_action = Some(BufferAction::Last);
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "first");
    buffers.active_mut().buffer_action = Some(BufferAction::Last);
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "second");

    fs::remove_dir_all(root).unwrap();
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
fn buffers_can_be_listed_selected_cycled_and_deleted() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vimloating-buffers-{unique}"));
    fs::create_dir_all(&root).unwrap();
    let first_path = root.join("first.txt");
    let second_path = root.join("second.txt");
    fs::write(&first_path, "first buffer").unwrap();
    fs::write(&second_path, "second buffer").unwrap();

    let mut buffers = super::buffers::BufferList::new(Editor::new("welcome", None));
    buffers
        .active_mut()
        .command(&format!("e {}", first_path.display()));
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "first buffer");

    buffers
        .active_mut()
        .command(&format!("e {}", second_path.display()));
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "second buffer");

    buffers.active_mut().command("bp");
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "first buffer");
    buffers
        .active_mut()
        .command(&format!("b {}", second_path.display()));
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "second buffer");
    buffers.active_mut().command("bp");
    buffers.process_pending();
    buffers.active_mut().command("bn");
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "second buffer");

    buffers.active_mut().command("ls");
    buffers.process_pending();
    assert_eq!(buffers.active().mode, Mode::BufferList);
    assert!(
        buffers
            .active()
            .output_view
            .as_deref()
            .unwrap()
            .contains("first.txt")
    );
    buffers.active_mut().escape();

    buffers.active_mut().command("b delete");
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "first buffer");

    buffers.active_mut().begin_insert('A');
    buffers.active_mut().insert_char('!');
    buffers.active_mut().escape();
    buffers.active_mut().command("bd");
    buffers.process_pending();
    assert!(buffers.active().dirty());
    buffers.active_mut().command("bd!");
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "welcome");

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn quitting_checks_modified_hidden_buffers() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("vimloating-hidden-buffer-{unique}.txt"));
    fs::write(&path, "file buffer").unwrap();

    let mut buffers = super::buffers::BufferList::new(Editor::new("welcome", None));
    buffers
        .active_mut()
        .command(&format!("e {}", path.display()));
    buffers.process_pending();
    buffers.active_mut().command("bp");
    buffers.process_pending();
    buffers.active_mut().begin_insert('A');
    buffers.active_mut().insert_char('!');
    buffers.active_mut().escape();
    buffers.active_mut().command("bn");
    buffers.process_pending();

    buffers.active_mut().command("q");
    buffers.protect_quit();
    assert!(!buffers.active().quit);
    assert!(buffers.active().message.contains("Modified buffers"));

    buffers.active_mut().command("q!");
    buffers.protect_quit();
    assert!(buffers.active().quit);
    fs::remove_file(path).unwrap();
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
