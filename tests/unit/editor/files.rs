use super::super::*;
use std::fs;

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
    let mut buffers = crate::editor::buffers::BufferList::new(editor);
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
fn leader_e_opens_the_current_file_directory_browser() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vimfloating-leader-explore-{unique}"));
    fs::create_dir_all(&root).unwrap();
    let file = root.join("notes.txt");
    fs::write(&file, "browse from here").unwrap();

    let mut buffers = crate::editor::buffers::BufferList::new(Editor::open_path(file).unwrap());
    buffers.active_mut().normal_key(' ');
    buffers.active_mut().normal_key('e');
    buffers.process_pending();

    assert!(buffers.active().is_directory_browser());
    let expected_directory = root.canonicalize().unwrap();
    assert_eq!(
        buffers.active().path.as_deref(),
        Some(expected_directory.as_path())
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn explore_does_not_discard_unsaved_buffer_changes() {
    let mut editor = Editor::new("keep this buffer", None);
    editor.begin_insert('A');
    editor.insert_char('!');
    editor.escape();

    let mut buffers = crate::editor::buffers::BufferList::new(editor);
    buffers.active_mut().command("Explore");
    buffers.process_pending();
    assert!(buffers.active().is_directory_browser());

    buffers.active_mut().command("bp");
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "keep this buffer!");
    assert!(buffers.active().dirty());
}
