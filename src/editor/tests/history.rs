use super::super::*;

#[test]
fn command_windows_edit_history_without_changing_source_and_can_execute_or_cancel() {
    let mut e = Editor::new("source", None);
    e.normal_key(':');
    e.prompt = "theme default".into();
    e.submit_prompt();
    for key in "q:".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.mode, Mode::CommandWindow);
    let window = e.command_window.as_mut().unwrap();
    window.normal_key('k');
    assert_eq!(window.text(), "theme default\n");
    e.control_key('c');
    assert_eq!(e.mode, Mode::Command);
    assert_eq!(e.prompt, "theme default");
    e.control_key('c');
    assert_eq!(e.mode, Mode::Normal);
    for key in "q/".chars() {
        e.normal_key(key);
    }
    let window = e.command_window.as_mut().unwrap();
    window.begin_insert('i');
    window.insert_text("source");
    e.finish_command_window(true);
    assert_eq!(e.search, "source");
    assert_eq!(e.text(), "source");
    assert!(!e.dirty());
    assert!(e.undo.is_empty());
    for key in "q:".chars() {
        e.normal_key(key);
    }
    e.escape();
    assert!(e.command_window.is_none());
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
