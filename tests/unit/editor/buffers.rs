use super::super::*;
use std::fs;

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
    let mut buffers = crate::editor::buffers::BufferList::new(editor);
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

    let mut buffers = crate::editor::buffers::BufferList::new(Editor::new("welcome", None));
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

    buffers.active_mut().normal_key(' ');
    assert_eq!(buffers.active().pending, Some(' '));
    buffers.active_mut().normal_key(' ');
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "first");

    buffers.active_mut().normal_key(' ');
    buffers.active_mut().normal_key(' ');
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "second");

    fs::remove_dir_all(root).unwrap();
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

    let mut buffers = crate::editor::buffers::BufferList::new(Editor::new("welcome", None));
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
    buffers.active_mut().command("b2");
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "first buffer");
    buffers.active_mut().command("b3");
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
fn help_opens_readme_and_searches_the_requested_topic() {
    let mut buffers = crate::editor::buffers::BufferList::new(Editor::new("welcome", None));

    buffers.active_mut().command("help");
    buffers.process_pending();
    assert!(
        buffers
            .active()
            .path
            .as_deref()
            .unwrap()
            .ends_with("README.md")
    );
    assert!(buffers.active().text().contains("## Terminal editor"));

    buffers.active_mut().command("help Terminal editor");
    buffers.process_pending();
    assert_eq!(buffers.active().search, "Terminal editor");
    buffers.active_mut().advance_search();
    assert_eq!(buffers.active().cursor.row, 41);
    assert_eq!(buffers.active().cursor.col, 3);
}

#[test]
fn buffer_and_line_pickers_filter_and_jump_to_the_selected_result() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vimfloating-picker-{unique}"));
    fs::create_dir_all(&root).unwrap();
    let target = root.join("picker-target.txt");
    fs::write(&target, "target head\nother target line").unwrap();

    let mut buffers =
        crate::editor::buffers::BufferList::new(Editor::new("alpha\nneedle local\nomega", None));
    buffers
        .active_mut()
        .command(&format!("e {}", target.display()));
    buffers.process_pending();

    buffers.active_mut().command("Buffer");
    buffers.process_pending();
    assert!(buffers.active().picker_active());
    buffers.active_mut().picker_type('p');
    buffers.active_mut().picker_type('i');
    buffers.active_mut().picker_type('c');
    buffers.active_mut().picker_type('k');
    buffers.active_mut().picker_type('e');
    buffers.active_mut().picker_type('r');
    buffers.active_mut().picker_type('-');
    buffers.active_mut().picker_type('t');
    buffers.active_mut().picker_type('a');
    buffers.active_mut().picker_type('r');
    buffers.active_mut().picker_type('g');
    buffers.active_mut().picker_type('e');
    buffers.active_mut().picker_type('t');
    buffers.active_mut().accept_picker();
    buffers.process_pending();
    assert_eq!(buffers.active().path.as_deref(), Some(target.as_path()));

    buffers.active_mut().command("Blines");
    buffers.process_pending();
    for ch in "needle local".chars() {
        buffers.active_mut().picker_type(ch);
    }
    buffers.active_mut().accept_picker();
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "alpha\nneedle local\nomega");
    assert_eq!(buffers.active().cursor.row, 1);

    buffers.active_mut().command("Lines");
    buffers.process_pending();
    for ch in "alpha".chars() {
        buffers.active_mut().picker_type(ch);
    }
    buffers.active_mut().accept_picker();
    buffers.process_pending();
    assert_eq!(buffers.active().cursor.row, 0);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rg_and_git_picker_commands_use_the_external_tools() {
    let mut buffers = crate::editor::buffers::BufferList::new(Editor::new("", None));

    if std::process::Command::new("rg")
        .arg("--version")
        .output()
        .is_ok()
    {
        buffers.active_mut().command("Files");
        buffers.process_pending();
        assert!(buffers.active().picker_active());
        for ch in "README.md".chars() {
            buffers.active_mut().picker_type(ch);
        }
        assert!(
            buffers
                .active()
                .output_view
                .as_deref()
                .unwrap()
                .contains("README.md")
        );
    }

    if std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_ok()
    {
        buffers.active_mut().escape();
        buffers.active_mut().command("Gfiles");
        buffers.process_pending();
        assert!(buffers.active().picker_active());
        for ch in "Cargo.toml".chars() {
            buffers.active_mut().picker_type(ch);
        }
        assert!(
            buffers
                .active()
                .output_view
                .as_deref()
                .unwrap()
                .contains("Cargo.toml")
        );

        buffers.active_mut().escape();
        buffers.active_mut().command("Gfiles?");
        buffers.process_pending();
        assert!(buffers.active().picker_active());
    }
}

#[test]
fn rg_picker_searches_live_and_jumps_to_the_match_position() {
    if std::process::Command::new("rg")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }

    let mut buffers = crate::editor::buffers::BufferList::new(Editor::new("search source", None));
    buffers.active_mut().command("Rg");
    buffers.process_pending();
    assert!(
        buffers
            .active()
            .output_view
            .as_deref()
            .unwrap()
            .contains("Type a pattern")
    );

    for ch in "## Terminal editor".chars() {
        buffers.active_mut().picker_type(ch);
    }
    assert!(
        buffers
            .active()
            .output_view
            .as_deref()
            .unwrap()
            .contains("README.md:42:1")
    );
    buffers.active_mut().accept_picker();
    buffers.process_pending();
    assert!(
        buffers
            .active()
            .path
            .as_deref()
            .unwrap()
            .ends_with("README.md")
    );
    assert_eq!(buffers.active().cursor.row, 41);
    assert_eq!(buffers.active().cursor.col, 0);
}

#[test]
fn picker_cursor_moves_within_the_visible_window_before_it_scrolls() {
    let text = (0..20)
        .map(|row| format!("line {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut buffers = crate::editor::buffers::BufferList::new(Editor::new(&text, None));
    buffers.active_mut().command("Lines");
    buffers.process_pending();

    for _ in 0..13 {
        buffers.active_mut().picker_move(1);
    }
    let before_scroll = buffers.active().output_view.clone().unwrap();
    assert!(before_scroll.contains(">    14  line 13"));
    buffers.active_mut().picker_move(1);
    let after_scroll = buffers.active().output_view.clone().unwrap();
    assert!(after_scroll.starts_with(":Lines  20 matches"));
    buffers.active_mut().picker_move(-1);
    let after_move_up = buffers.active().output_view.as_deref().unwrap();
    assert!(after_move_up.contains(">    14  line 13"));
    assert_eq!(
        after_scroll.lines().nth(3),
        after_move_up.lines().nth(3),
        "moving up from the lower edge should move the selection before scrolling the list"
    );
}

#[test]
fn marks_histories_commands_and_help_topics_are_pickable() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let file = std::env::temp_dir().join(format!("vimfloating-picker-history-{unique}.txt"));
    fs::write(&file, "first line\nmarked line\nlast line").unwrap();

    let mut buffers =
        crate::editor::buffers::BufferList::new(Editor::open_path(file.clone()).unwrap());
    buffers.active_mut().cursor.row = 1;
    buffers.active_mut().normal_key('m');
    buffers.active_mut().normal_key('a');

    buffers.active_mut().mode = Mode::Command;
    buffers.active_mut().prompt = "theme default".into();
    buffers.active_mut().submit_prompt();
    buffers.active_mut().mode = Mode::Search;
    buffers.active_mut().prompt = "marked line".into();
    buffers.active_mut().submit_prompt();

    buffers.active_mut().command("Marks");
    buffers.process_pending();
    for ch in "marked line".chars() {
        buffers.active_mut().picker_type(ch);
    }
    buffers.active_mut().accept_picker();
    buffers.process_pending();
    assert_eq!(buffers.active().cursor.row, 1);

    buffers.active_mut().command("History");
    buffers.process_pending();
    for ch in file.file_name().unwrap().to_string_lossy().chars() {
        buffers.active_mut().picker_type(ch);
    }
    buffers.active_mut().accept_picker();
    buffers.process_pending();
    assert!(
        buffers
            .active()
            .path
            .as_deref()
            .unwrap()
            .ends_with(file.file_name().unwrap())
    );

    buffers.active_mut().command("History:");
    buffers.process_pending();
    for ch in "theme default".chars() {
        buffers.active_mut().picker_type(ch);
    }
    assert!(
        buffers
            .active()
            .output_view
            .as_deref()
            .unwrap()
            .contains("1 matches")
    );
    buffers.active_mut().accept_picker();
    buffers.process_pending();
    assert_eq!(buffers.active().mode, Mode::Command);
    assert_eq!(buffers.active().prompt, "theme default");

    buffers.active_mut().escape();
    buffers.active_mut().command("History/");
    buffers.process_pending();
    for ch in "marked line".chars() {
        buffers.active_mut().picker_type(ch);
    }
    buffers.active_mut().accept_picker();
    buffers.process_pending();
    assert_eq!(buffers.active().mode, Mode::Search);
    assert_eq!(buffers.active().prompt, "marked line");

    buffers.active_mut().escape();
    buffers.active_mut().command("Command");
    buffers.process_pending();
    for ch in "theme".chars() {
        buffers.active_mut().picker_type(ch);
    }
    buffers.active_mut().accept_picker();
    buffers.process_pending();
    assert_eq!(buffers.active().mode, Mode::Command);
    assert_eq!(buffers.active().prompt, "theme");

    buffers.active_mut().escape();
    buffers.active_mut().command("Help");
    buffers.process_pending();
    for ch in "Vim controls".chars() {
        buffers.active_mut().picker_type(ch);
    }
    buffers.active_mut().accept_picker();
    buffers.process_pending();
    let heading: String = buffers.active().lines[buffers.active().cursor.row]
        .iter()
        .collect();
    assert_eq!(heading, "## Vim controls");

    fs::remove_file(file).unwrap();
}

#[test]
fn quitting_checks_modified_hidden_buffers() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("vimloating-hidden-buffer-{unique}.txt"));
    fs::write(&path, "file buffer").unwrap();

    let mut buffers = crate::editor::buffers::BufferList::new(Editor::new("welcome", None));
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
