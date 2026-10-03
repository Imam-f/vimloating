use super::super::*;
use std::fs;

struct Files(PathBuf);

impl Files {
    fn new() -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("vimloating-paths-{}-{unique}", std::process::id()));
        fs::create_dir_all(root.join("folder")).unwrap();
        fs::write(root.join("alpha.txt"), "first\nsecond\nthird").unwrap();
        fs::write(root.join("alpine.txt"), "alpine").unwrap();
        fs::write(root.join("folder/日本 notes.txt"), "unicode file").unwrap();
        Self(root)
    }

    fn editor(&self, text: &str) -> Editor {
        Editor::new(text, Some(self.0.join("source.txt")))
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn insert_completion_cycles_in_both_directions_and_undoes_as_one_session() {
    let files = Files::new();
    let mut e = files.editor("see ");
    e.begin_insert('A');
    e.insert_text("al");
    e.control_key('x');
    e.control_key('f');
    assert_eq!(e.text(), "see alpha.txt");
    assert_eq!(e.completion_popup().unwrap().kind, "Path");
    e.control_key('n');
    assert_eq!(e.text(), "see alpine.txt");
    e.control_key('p');
    assert_eq!(e.text(), "see alpha.txt");
    e.control_key('p');
    assert_eq!(e.text(), "see alpine.txt");
    e.escape();
    e.undo(false);
    assert_eq!(e.text(), "see ");
    e.undo(true);
    assert_eq!(e.text(), "see alpine.txt");
}

#[test]
fn insert_completion_preserves_unicode_prefix_quotes_and_line_suffix() {
    let files = Files::new();
    let mut e = files.editor("é = \"folder/日\"; tail");
    e.cursor.col = "é = \"folder/日".chars().count();
    e.begin_insert('i');
    e.control_key('x');
    e.control_key('f');
    assert_eq!(e.text(), "é = \"folder/日本 notes.txt\"; tail");
    assert_eq!(e.cursor.col, "é = \"folder/日本 notes.txt".chars().count());
}

#[test]
fn completion_can_descend_into_a_directory_and_dot_repeats_the_inserted_path() {
    let files = Files::new();
    let mut e = files.editor("\n");
    e.begin_insert('i');
    e.insert_text("fol");
    e.control_key('x');
    e.control_key('f');
    assert_eq!(e.text(), "folder/\n");
    e.control_key('x');
    e.control_key('f');
    assert_eq!(e.text(), "folder/日本 notes.txt\n");
    e.escape();
    e.normal_key('j');
    e.normal_key('.');
    assert_eq!(e.text(), "folder/日本 notes.txt\nfolder/日本 notes.txt");
}

#[test]
fn editing_and_movement_reset_the_completion_cycle() {
    let files = Files::new();
    let mut e = files.editor("al");
    e.begin_insert('A');
    e.complete_path(false);
    e.backspace();
    e.complete_path(false);
    assert_eq!(e.text(), "alpha.txt");
    e.move_by(-1, 0, 1);
    e.complete_path(false);
    assert_eq!(e.text(), "alpha.txtt");
    e.escape();
    e.normal_key('A');
    e.complete_path(false);
    assert!(e.message.contains("No file or path completions"));
}

#[test]
fn missing_completion_and_missing_navigation_leave_text_unchanged() {
    let files = Files::new();
    let mut e = files.editor("missing.txt");
    e.begin_insert('A');
    e.complete_path(false);
    assert_eq!(e.text(), "missing.txt");
    assert!(!e.dirty());
    e.escape();
    e.normal_key('g');
    e.normal_key('f');
    assert!(e.buffer_action.is_none());
    assert!(e.message.contains("File not found"));
    e.cursor.col = 0;
    e.lines = vec![vec![]];
    e.open_cursor_path();
    assert_eq!(e.message, "No file path under cursor");
}

#[test]
fn gf_opens_relative_address_and_keeps_unsaved_changes() {
    let files = Files::new();
    let mut e = files.editor("see (alpha.txt:2:3)");
    e.begin_insert('A');
    e.insert_char('!');
    e.escape();
    e.cursor.col = 10;
    let mut buffers = buffers::BufferList::new(e);
    buffers.active_mut().normal_key('g');
    buffers.active_mut().normal_key('f');
    buffers.process_pending();
    assert_eq!(buffers.active().path, Some(files.0.join("alpha.txt")));
    assert_eq!(buffers.active().cursor, Pos { row: 1, col: 2 });
    buffers.active_mut().command("bp");
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "see (alpha.txt:2:3)!");
    assert!(buffers.active().dirty());
}

#[test]
fn gf_handles_quoted_spaces_absolute_paths_and_clamps_addresses() {
    let files = Files::new();
    for address in [
        "\"folder/日本 notes.txt\":99:99".into(),
        format!("{}:99:99", files.0.join("alpha.txt").display()),
    ] {
        let mut e = files.editor(&address);
        e.cursor.col = 4;
        let mut buffers = buffers::BufferList::new(e);
        buffers.active_mut().normal_key('g');
        buffers.active_mut().normal_key('F');
        buffers.process_pending();
        let editor = buffers.active();
        assert_eq!(editor.cursor.row, editor.lines.len() - 1);
        assert_eq!(editor.cursor.col, editor.lines.last().unwrap().len() - 1);
        assert!(!editor.name().ends_with("source.txt"));
    }
}

#[test]
fn gf_reuses_modified_target_buffers_and_reveals_folded_addresses() {
    let files = Files::new();
    let mut target = Editor::new("root\n  child\nend", Some(files.0.join("alpha.txt")));
    target.begin_insert('A');
    target.insert_char('!');
    target.escape();
    target.normal_key('z');
    target.normal_key('c');
    let mut buffers = buffers::BufferList::new(target);
    buffers
        .active_mut()
        .command(&format!("e {}", files.0.join("alpine.txt").display()));
    buffers.process_pending();
    buffers.active_mut().begin_insert('A');
    buffers.active_mut().insert_text(" alpha.txt:2");
    buffers.active_mut().escape();
    buffers.active_mut().normal_key('g');
    buffers.active_mut().normal_key('f');
    buffers.process_pending();
    assert_eq!(buffers.active().text(), "root!\n  child\nend");
    assert_eq!(buffers.active().cursor, Pos { row: 1, col: 0 });
    assert!(buffers.active().dirty());
    assert_eq!(buffers.active().display_total(80, false), 3);
}

#[test]
fn command_paths_complete_spaces_directories_and_all_save_commands() {
    let files = Files::new();
    let mut e = files.editor("");
    e.mode = Mode::Command;
    for command in ["e", "e!", "w", "wq", "x", "Ex", "Explore"] {
        e.prompt = format!("{command} {}/folder/日", files.0.display());
        e.complete_command();
        assert_eq!(
            e.prompt,
            format!("{command} {}/folder/日本 notes.txt", files.0.display())
        );
    }
    e.prompt = format!("e {}/fol", files.0.display());
    e.complete_command();
    assert!(e.prompt.ends_with("folder\\") || e.prompt.ends_with("folder/"));
}

#[test]
fn home_paths_expand_for_completion_and_file_commands() {
    #[cfg(windows)]
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    #[cfg(not(windows))]
    let home = std::env::var_os("HOME");
    let home = PathBuf::from(home.unwrap());
    assert_eq!(paths::expand_home("~/file.txt"), home.join("file.txt"));
    let mut e = Editor::new("", None);
    e.command("e ~/file.txt");
    assert!(
        matches!(e.buffer_action, Some(BufferAction::Open { path, .. }) if path == home.join("file.txt"))
    );
    let candidates = paths::path_completions("~/", std::path::Path::new("."));
    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|path| path.starts_with("~/")));
}

#[test]
fn a_filename_without_a_parent_completes_from_the_working_directory() {
    let mut e = Editor::new("Cargo.t", Some("notes.txt".into()));
    e.begin_insert('A');
    e.control_key('x');
    e.control_key('f');
    assert_eq!(e.text(), "Cargo.toml");
}

#[test]
fn navigation_handles_the_address_after_a_quote_and_rejects_empty_filenames() {
    let files = Files::new();
    let mut e = files.editor("\"alpha.txt\":2:3");
    e.cursor.col = 14;
    e.open_cursor_path();
    assert!(matches!(
        e.buffer_action,
        Some(BufferAction::OpenAddress {
            position: Some(Pos { row: 1, col: 2 }),
            ..
        })
    ));
    let mut e = files.editor(":12");
    e.open_cursor_path();
    assert!(e.buffer_action.is_none());
    assert_eq!(e.message, "No file path under cursor");
}
