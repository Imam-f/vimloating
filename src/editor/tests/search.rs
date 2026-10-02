use super::super::*;

#[test]
fn star_hash_search_whole_unicode_words_with_counts_wraparound_and_repeat() {
    let mut e = Editor::new("λ foo foo_bar foobar\nfoo λ food foo", None);
    e.cursor.col = 4;
    e.normal_key('*');
    e.advance_search();
    assert_eq!(e.search, "foo");
    assert_eq!(e.cursor, Pos { row: 1, col: 0 });
    e.normal_key('n');
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 1, col: 11 });
    e.normal_key('N');
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 1, col: 0 });
    e.cursor.col = 1;
    e.normal_key('#');
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 0, col: 2 });
    e.normal_key('2');
    e.normal_key('*');
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 1, col: 11 });
    e.cursor = Pos::default();
    e.normal_key('#');
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 1, col: 4 });
    e.normal_key('/');
    e.prompt = "foo".into();
    e.submit_prompt();
    assert!(!e.search_whole_word);
}

#[test]
fn search_wraps_and_uses_character_columns() {
    let mut e = Editor::new("é猫 x 猫\n猫", None);
    e.search = "猫".into();
    e.find(false);
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 0, col: 1 });
    e.find(true);
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 1, col: 0 });
    e.find(false);
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 0, col: 1 });
}

#[test]
fn backward_search_and_n_n_follow_the_search_direction() {
    let mut e = Editor::new("cat dog cat dog", None);
    e.cursor.col = 14;
    e.normal_key('?');
    assert_eq!(e.mode, Mode::Search);
    assert!(e.search_prompt_backwards);
    e.prompt = "dog".into();
    e.submit_prompt();
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 0, col: 12 });

    e.normal_key('n');
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 0, col: 4 });

    e.normal_key('N');
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 0, col: 12 });

    e.normal_key('n');
    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 0, col: 4 });
}

#[test]
fn long_search_work_is_spread_across_frames() {
    let mut e = Editor::new(&format!("{}needle", "x".repeat(50_000)), None);
    e.search = "needle".into();
    e.find(false);

    assert_eq!(e.cursor, Pos { row: 0, col: 0 });
    assert!(e.search_task.is_some());

    e.advance_search();
    assert_eq!(e.cursor, Pos { row: 0, col: 0 });
    assert!(e.search_task.is_some());

    while e.search_task.is_some() {
        e.advance_search();
    }
    assert_eq!(
        e.cursor,
        Pos {
            row: 0,
            col: 50_000
        }
    );
}
