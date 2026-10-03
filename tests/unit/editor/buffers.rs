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
