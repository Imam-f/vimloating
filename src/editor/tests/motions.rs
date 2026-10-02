use super::super::*;

#[test]
fn percent_matches_nested_multiline_delimiters_and_ignores_strings_and_comments() {
    let mut e = Editor::new("λ call({\n  \"}\"; /* } */ [value]\n})", None);
    e.normal_key('%');
    assert_eq!(e.cursor, Pos { row: 2, col: 1 });
    e.normal_key('%');
    assert_eq!(e.cursor, Pos { row: 0, col: 6 });
    e.cursor.col = 7;
    e.normal_key('%');
    assert_eq!(e.cursor, Pos { row: 2, col: 0 });
    e.cursor = Pos { row: 1, col: 15 };
    e.normal_key('%');
    assert_eq!(e.cursor, Pos { row: 1, col: 21 });
    let mut invalid = Editor::new("(unclosed", None);
    invalid.normal_key('%');
    assert_eq!(invalid.cursor, Pos::default());
    assert!(invalid.message.contains("No matching"));
}

#[test]
fn marks_jump_exactly_or_to_first_nonblank_and_stay_buffer_local() {
    let mut e = Editor::new("first\n  second", None);
    e.cursor = Pos { row: 1, col: 5 };
    for key in "ma".chars() {
        e.normal_key(key);
    }
    e.cursor = Pos::default();
    for key in "`a".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.cursor, Pos { row: 1, col: 5 });
    for key in "'a".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.cursor, Pos { row: 1, col: 2 });
    let mut other = Editor::new("other", None);
    for key in "`a".chars() {
        other.normal_key(key);
    }
    assert!(other.message.contains("not set"));
    e.lines.truncate(1);
    for key in "`a".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.cursor.row, 0);
}

#[test]
fn character_find_motions_repeat_and_highlight_the_matched_target() {
    let mut e = Editor::new("a x a y a", None);
    e.normal_key('f');
    e.normal_key('a');
    assert_eq!(e.cursor, Pos { row: 0, col: 4 });
    assert_eq!(e.char_find_highlight, Some(Pos { row: 0, col: 4 }));

    e.normal_key(';');
    assert_eq!(e.cursor, Pos { row: 0, col: 8 });
    assert_eq!(e.char_find_highlight, Some(Pos { row: 0, col: 8 }));

    e.normal_key('F');
    e.normal_key('a');
    assert_eq!(e.cursor, Pos { row: 0, col: 4 });

    e.normal_key('t');
    e.normal_key('a');
    assert_eq!(e.cursor, Pos { row: 0, col: 7 });
    assert_eq!(e.char_find_highlight, Some(Pos { row: 0, col: 8 }));

    e.normal_key('T');
    e.normal_key('a');
    assert_eq!(e.cursor, Pos { row: 0, col: 5 });
    assert_eq!(e.char_find_highlight, Some(Pos { row: 0, col: 4 }));
}

#[test]
fn comma_repeats_a_character_find_in_the_opposite_direction() {
    let mut e = Editor::new("a x a y a", None);
    e.normal_key('f');
    e.normal_key('a');
    assert_eq!(e.cursor.col, 4);

    e.normal_key(',');
    assert_eq!(e.cursor.col, 0);
    e.normal_key(';');
    assert_eq!(e.cursor.col, 4);
}

#[test]
fn character_find_suggests_letters_in_every_following_or_previous_word() {
    let mut forward = Editor::new("aaa abc deff", None);
    forward.normal_key('f');
    assert_eq!(forward.pending, Some('f'));
    assert_eq!(
        forward.char_find_hints,
        vec![Pos { row: 0, col: 5 }, Pos { row: 0, col: 8 }]
    );
    forward.normal_key('b');
    assert_eq!(forward.cursor, Pos { row: 0, col: 5 });
    assert!(forward.char_find_hints.is_empty());
    assert_eq!(forward.char_find_highlight, Some(Pos { row: 0, col: 5 }));

    let mut backward = Editor::new("cat axa dog", None);
    backward.cursor.col = 10;
    backward.normal_key('F');
    assert_eq!(
        backward.char_find_hints,
        vec![Pos { row: 0, col: 2 }, Pos { row: 0, col: 5 }]
    );
    backward.normal_key('x');
    assert_eq!(backward.cursor, Pos { row: 0, col: 5 });
}

#[test]
fn counted_character_find_uses_the_requested_occurrence() {
    let mut e = Editor::new("a x a y a", None);
    for key in "2fa".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.cursor, Pos { row: 0, col: 8 });
    assert_eq!(e.char_find_highlight, Some(Pos { row: 0, col: 8 }));

    let mut e = Editor::new("a x a y a", None);
    for key in "2ta".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.cursor, Pos { row: 0, col: 7 });
    assert_eq!(e.char_find_highlight, Some(Pos { row: 0, col: 8 }));
}

#[test]
fn brace_motions_move_to_empty_separator_lines_and_support_counts() {
    let mut e = Editor::new("alpha\ncontinued\n \nbeta\ncontinued\n\nomega", None);
    e.cursor.row = 1;
    for key in "2}".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.cursor, Pos { row: 5, col: 0 });

    e.normal_key('{');
    assert_eq!(e.cursor, Pos { row: 2, col: 0 });
    e.normal_key('{');
    assert_eq!(e.cursor, Pos { row: 0, col: 0 });
}

#[test]
fn parenthesis_motions_move_between_sentences_and_support_counts() {
    let mut e = Editor::new("First sentence. Second sentence!\nThird one? End.", None);
    e.normal_key(')');
    assert_eq!(e.cursor, Pos { row: 0, col: 16 });

    for key in "2)".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.cursor, Pos { row: 1, col: 11 });

    e.normal_key('(');
    assert_eq!(e.cursor, Pos { row: 1, col: 0 });
    e.normal_key('(');
    assert_eq!(e.cursor, Pos { row: 0, col: 16 });
}

#[test]
fn backward_character_find_hints_remain_searchable_with_leading_separators() {
    for prefix in ["  ", "(", "\t"] {
        for motion in ['F', 'T'] {
            let mut e = Editor::new(&format!("{prefix}cat axa dog"), None);
            let offset = prefix.chars().count();
            e.cursor.col = offset + 10;
            e.normal_key(motion);

            let targets = [
                Pos {
                    row: 0,
                    col: offset + 2,
                },
                Pos {
                    row: 0,
                    col: offset + 5,
                },
            ];
            for target in targets {
                assert!(
                    e.char_find_hints.binary_search(&target).is_ok(),
                    "{motion} hint at {target:?} is not searchable with prefix {prefix:?}: {:?}",
                    e.char_find_hints
                );
            }

            e.normal_key('x');
            assert_eq!(e.char_find_highlight, Some(targets[1]));
            assert_eq!(e.cursor.col, offset + 5 + usize::from(motion == 'T'));
        }
    }
}
