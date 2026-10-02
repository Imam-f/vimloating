use super::super::*;

fn open_history(editor: &mut Editor, shortcut: &str) {
    editor.normal_key(':');
    for key in shortcut.chars() {
        editor.edit_prompt(Some(key));
    }
}

#[test]
fn command_windows_edit_history_without_changing_source_and_can_execute_or_cancel() {
    let mut e = Editor::new("source", None);
    e.normal_key(':');
    e.prompt = "theme default".into();
    e.submit_prompt();
    open_history(&mut e, "q:");
    assert_eq!(e.mode, Mode::CommandWindow);
    let window = e.command_window.as_mut().unwrap();
    window.normal_key('k');
    assert_eq!(window.text(), "theme default\n");
    e.control_key('c');
    assert_eq!(e.mode, Mode::Command);
    assert_eq!(e.prompt, "theme default");
    e.control_key('c');
    assert_eq!(e.mode, Mode::Normal);
    open_history(&mut e, "q/");
    let window = e.command_window.as_mut().unwrap();
    window.begin_insert('i');
    window.insert_text("source");
    e.finish_command_window(true);
    assert_eq!(e.search, "source");
    assert_eq!(e.text(), "source");
    assert!(!e.dirty());
    assert!(e.undo.is_empty());
    open_history(&mut e, "q:");
    e.escape();
    assert!(e.command_window.is_none());
}

#[test]
fn history_shortcuts_open_only_from_command_mode_and_select_the_requested_history() {
    for (shortcut, searching, history) in [
        ("q:", false, "theme default"),
        ("q/", true, "needle"),
        ("q\\", true, "needle"),
    ] {
        let mut e = Editor::new("source", None);
        e.command_history.push("theme default".into());
        e.search_history.push("needle".into());
        e.normal_key('q');
        assert!(e.pending.is_none());
        assert_eq!(e.mode, Mode::Normal);
        assert!(e.command_window.is_none());
        open_history(&mut e, shortcut);
        assert_eq!(e.mode, Mode::CommandWindow, "{shortcut}");
        assert_eq!(e.command_window_search, searching, "{shortcut}");
        assert_eq!(
            e.command_window.as_ref().unwrap().text(),
            format!("{history}\n")
        );
        assert!(e.prompt.is_empty());
        assert_eq!(e.command_history, vec!["theme default"]);
        assert_eq!(e.search_history, vec!["needle"]);
        e.command_window.as_mut().unwrap().normal_key('k');
        e.control_key('c');
        assert_eq!(
            e.mode,
            if searching {
                Mode::Search
            } else {
                Mode::Command
            }
        );
        assert_eq!(e.prompt, history);
        assert_eq!(e.text(), "source");
        assert!(e.undo.is_empty());
        e.escape();
        for key in shortcut.chars() {
            e.normal_key(key);
        }
        assert!(e.command_window.is_none(), "Normal-mode {shortcut}");
    }
}

#[test]
fn history_shortcuts_preserve_regular_commands_searches_and_backspace() {
    let mut e = Editor::new("source", None);
    e.normal_key(':');
    for key in "e q/file".chars() {
        e.edit_prompt(Some(key));
    }
    assert_eq!(e.mode, Mode::Command);
    assert_eq!(e.prompt, "e q/file");
    e.escape();
    e.normal_key('/');
    for key in "q:".chars() {
        e.edit_prompt(Some(key));
    }
    assert_eq!(e.mode, Mode::Search);
    assert_eq!(e.prompt, "q:");
    e.escape();
    e.normal_key(':');
    e.edit_prompt(Some('q'));
    e.edit_prompt(None);
    assert_eq!(e.mode, Mode::Command);
    assert!(e.prompt.is_empty());
    e.edit_prompt(Some('q'));
    assert!(e.command_window.is_none());
    e.submit_prompt();
    assert!(e.quit);
}

#[test]
fn control_n_p_move_lines_and_browse_separate_prompt_histories() {
    let mut e = Editor::new("a\nb\nc", None);
    e.normal_key('2');
    e.control_key('n');
    assert_eq!(e.cursor.row, 2);
    e.control_key('p');
    assert_eq!(e.cursor.row, 1);
    for command in ["theme default", "noh", "theme everforest"] {
        e.normal_key(':');
        e.prompt = command.into();
        e.submit_prompt();
    }
    e.normal_key('/');
    e.prompt = "needle".into();
    e.submit_prompt();
    e.normal_key(':');
    e.prompt = "theme".into();
    e.control_key('p');
    assert_eq!(e.prompt, "theme everforest");
    e.control_key('p');
    assert_eq!(e.prompt, "theme default");
    e.control_key('n');
    assert_eq!(e.prompt, "theme everforest");
    e.control_key('n');
    assert_eq!(e.prompt, "theme");
    e.escape();
    e.normal_key('/');
    e.control_key('p');
    assert_eq!(e.prompt, "needle");
    e.edit_prompt(Some('x'));
    e.control_key('p');
    assert_eq!(e.prompt, "needlex");
}
