use super::*;
use std::fs;

#[test]
fn toggle_case_supports_unicode_expansion_counts_line_changes_and_repeat() {
    let mut e = Editor::new("aBßz\n  xY\n  Qr", None);
    for key in "3~".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "AbSSz\n  xY\n  Qr");
    assert_eq!(e.cursor.col, 4);
    e.undo(false);
    assert_eq!(e.text(), "aBßz\n  xY\n  Qr");
    e.cursor = Pos { row: 1, col: 2 };
    for key in "2g~~".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "aBßz\n  Xy\n  qR");
    e.normal_key('.');
    assert_eq!(e.text(), "aBßz\n  xY\n  Qr");
    e.undo(false);
    assert_eq!(e.text(), "aBßz\n  Xy\n  qR");
}

#[test]
fn visual_toggle_case_respects_rectangles_and_is_repeatable() {
    let mut e = Editor::new("abc\ndef\nghi\njkl", None);
    e.cursor.col = 1;
    e.control_key('v');
    e.normal_key('j');
    e.normal_key('l');
    e.normal_key('~');
    assert_eq!(e.text(), "aBC\ndEF\nghi\njkl");
    e.cursor = Pos { row: 2, col: 1 };
    e.normal_key('.');
    assert_eq!(e.text(), "aBC\ndEF\ngHI\njKL");
    assert_eq!(e.mode, Mode::Normal);
}

#[test]
fn block_replace_and_counted_paste_preserve_rows_and_pad_short_destinations() {
    let mut e = Editor::new("abcd\nx\nefgh", None);
    e.cursor.col = 1;
    e.control_key('v');
    e.normal_key('j');
    e.normal_key('j');
    e.normal_key('l');
    for key in "rλ".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "aλλd\nx\neλλh");
    e.undo(false);
    e.cursor = Pos { row: 0, col: 1 };
    e.control_key('v');
    e.cursor = Pos { row: 2, col: 2 };
    e.normal_key('y');
    e.cursor = Pos { row: 2, col: 2 };
    for key in "2p".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "abcd\nx\nefgbcbch\n       \n   fgfg");
    assert_eq!(e.mode, Mode::Normal);
    e.control_key('v');
    e.control_key('v');
    assert_eq!(e.mode, Mode::Normal);
}

#[test]
fn block_selection_handles_reversed_corners_ragged_rows_and_column_paste() {
    let mut e = Editor::new("abcdef\nx\nuvwxyz", None);
    e.cursor = Pos { row: 2, col: 3 };
    e.control_key('v');
    e.move_by(-2, -2, 1);
    e.move_by(-2, 0, 1);
    assert_eq!(
        e.selection(),
        (Pos { row: 0, col: 1 }, Pos { row: 2, col: 3 })
    );
    assert!(!e.selected_cell(Pos { row: 1, col: 0 }));
    assert!(e.selected_cell(Pos { row: 1, col: 2 }));
    e.normal_key('y');
    assert_eq!(
        e.register
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<_>>(),
        vec!["bcd", "   ", "vwx"]
    );
    e.cursor = Pos { row: 0, col: 0 };
    e.normal_key('P');
    assert_eq!(e.text(), "bcdabcdef\n   x\nvwxuvwxyz");
    e.undo(false);
    assert_eq!(e.text(), "abcdef\nx\nuvwxyz");
    e.cursor = Pos { row: 0, col: 1 };
    e.control_key('v');
    e.move_by(2, 2, 1);
    e.move_by(2, 0, 1);
    e.normal_key('d');
    assert_eq!(e.text(), "aef\nx\nuyz");
    e.undo(false);
    assert_eq!(e.text(), "abcdef\nx\nuvwxyz");
}

#[test]
fn block_selection_keeps_virtual_columns_and_repeat_deletes_a_rectangle() {
    let mut e = Editor::new("abcd\nx\nefgh\nijkl\nm\nnopq", None);
    e.cursor.col = 2;
    e.control_key('v');
    e.normal_key('j');
    assert_eq!(e.cursor.col, 2);
    assert_eq!(e.display_window(0, 100, 2, true), e.display_rows(2, true));
    e.normal_key('j');
    e.normal_key('l');
    e.normal_key('d');
    assert_eq!(e.text(), "ab\nx\nef\nijkl\nm\nnopq");
    e.cursor = Pos { row: 3, col: 2 };
    e.normal_key('.');
    assert_eq!(e.text(), "ab\nx\nef\nij\nm\nno");
    e.undo(false);
    assert_eq!(e.text(), "ab\nx\nef\nijkl\nm\nnopq");
}

#[test]
fn yank_blinks_only_copied_ranges_and_expires_without_editing_the_buffer() {
    let mut e = Editor::new("abcd\nefgh", None);
    e.cursor.col = 1;
    e.normal_key('v');
    e.cursor = Pos { row: 1, col: 2 };
    e.normal_key('y');
    let highlight = e.yank_highlight.as_ref().unwrap();
    assert_eq!(highlight.ranges, vec![(0, 1..4), (1, 0..3)]);
    assert!(highlight.visible_at(0.01));
    assert!(!highlight.visible_at(0.15));
    assert!(highlight.visible_at(0.25));
    assert!(!highlight.visible_at(0.65));
    assert!(!e.dirty());
    assert!(e.undo.is_empty());
    e.normal_key('x');
    assert!(e.yank_highlight.is_none());
    let mut e = Editor::new("a\n\nb", None);
    for key in "2yy".chars() {
        e.normal_key(key);
    }
    let highlight = e.yank_highlight.as_ref().unwrap();
    assert!(highlight.linewise);
    assert_eq!(highlight.ranges, vec![(0, 0..1), (1, 0..1)]);
}

#[test]
fn visual_word_and_block_objects_yank_and_expand_nested_selections() {
    let mut e = Editor::new("λ word tail", None);
    e.cursor.col = 4;
    for key in "viwy".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.register, vec!["word".chars().collect::<Vec<_>>()]);
    for key in "vawy".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.register, vec!["word ".chars().collect::<Vec<_>>()]);
    let mut e = Editor::new("outer(inner(value))", None);
    e.cursor.col = 13;
    for key in "vib".chars() {
        e.normal_key(key);
    }
    assert_eq!(
        e.selection(),
        (Pos { row: 0, col: 12 }, Pos { row: 0, col: 16 })
    );
    for key in "ib".chars() {
        e.normal_key(key);
    }
    assert_eq!(
        e.selection(),
        (Pos { row: 0, col: 6 }, Pos { row: 0, col: 17 })
    );
    e.normal_key('y');
    assert_eq!(e.register[0].iter().collect::<String>(), "inner(value)");
}

#[test]
fn visual_objects_support_tags_attributes_quotes_counts_and_empty_blocks() {
    let mut e = Editor::new("<div title=\"a>b\"><div>λ</div><br/></div>", None);
    e.cursor.col = 22;
    for key in "vity".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.register[0], vec!['λ']);
    e.cursor.col = 22;
    for key in "v2aty".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.register[0].iter().collect::<String>(), e.text());
    let mut e = Editor::new("say \"a\\\"b\" now", None);
    e.cursor.col = 6;
    for key in "vi\"y".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.register[0].iter().collect::<String>(), "a\\\"b");
    let mut e = Editor::new("()", None);
    for key in "vib".chars() {
        e.normal_key(key);
    }
    assert!(e.message.contains("No text object"));
    assert_eq!(e.text(), "()");
}

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
fn replace_supports_unicode_counts_undo_dot_and_insufficient_characters() {
    let mut e = Editor::new("abcdef", None);
    for key in "2rλ".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "λλcdef");
    assert_eq!(e.cursor.col, 1);
    e.cursor.col = 3;
    e.normal_key('.');
    assert_eq!(e.text(), "λλcλλf");
    e.undo(false);
    assert_eq!(e.text(), "λλcdef");
    e.cursor.col = 5;
    for key in "2rz".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "λλcdef");
    e.normal_key('r');
    e.escape();
    assert_eq!(e.pending, None);
    e.cursor.col = 2;
    e.normal_key('r');
    e.normal_key('\n');
    assert_eq!(e.text(), "λλ\ndef");
}

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

#[test]
fn indentation_folds_hide_nested_lines_and_motion_skips_them() {
    use super::folds::{FoldProvider, FoldRange, IndentFoldProvider};
    let mut e = Editor::new("root\n    child\n        leaf\n    sibling\n\nnext", None);
    assert_eq!(
        IndentFoldProvider.ranges(&e.lines),
        vec![
            FoldRange { start: 0, end: 4 },
            FoldRange { start: 1, end: 2 }
        ]
    );
    for key in "zc".chars() {
        e.normal_key(key);
    }
    assert_eq!(
        e.display_window(0, 10, 3, true),
        vec![(0, 0), (5, 0), (5, 3)]
    );
    e.normal_key('j');
    assert_eq!(e.cursor.row, 5);
    e.normal_key('k');
    assert_eq!(e.cursor.row, 0);
    for key in "zo".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.display_total(80, false), 6);
    for key in "zM".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.display_total(80, false), 2);
    e.cursor = Pos { row: 2, col: 0 };
    e.reveal_cursor(5, 80, false);
    assert_eq!(e.display_total(80, false), 6);
    for key in "zM".chars() {
        e.normal_key(key);
    }
    for key in "zR".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.display_total(80, false), 6);
}

#[test]
fn custom_fold_provider_and_edits_use_the_same_fold_interface() {
    use super::folds::{FoldProvider, FoldRange};
    struct SyntaxFolds;
    impl FoldProvider for SyntaxFolds {
        fn ranges(&self, _: &[Vec<char>]) -> Vec<FoldRange> {
            vec![FoldRange { start: 0, end: 1 }]
        }
    }
    let mut e = Editor::new("a\nb\nc", None);
    e.set_fold_provider(Box::new(SyntaxFolds));
    for key in "zc".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.display_rows(80, false), vec![(0, 0), (2, 0)]);
    e.begin_insert('i');
    e.insert_char('x');
    assert_eq!(e.display_total(80, false), 3);
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
fn dot_repeats_counted_deletes_and_insert_sessions() {
    let mut e = Editor::new("abcdef", None);
    for key in "2x".chars() {
        e.normal_key(key);
    }
    e.normal_key('.');
    assert_eq!(e.text(), "ef");

    let mut e = Editor::new("one\none", None);
    e.begin_insert('i');
    e.insert_text("X");
    e.escape();
    e.cursor = Pos { row: 1, col: 0 };
    e.normal_key('.');
    assert_eq!(e.text(), "Xone\nXone");

    let mut e = Editor::new("one\ntwo\nthree", None);
    e.normal_key('d');
    e.normal_key('d');
    e.normal_key('.');
    assert_eq!(e.text(), "three");

    let mut e = Editor::new("value", None);
    e.normal_key('>');
    e.normal_key('.');
    assert_eq!(e.text(), "        value");
}

#[test]
fn dot_repeats_a_visual_delete_with_the_same_selection_shape() {
    let mut e = Editor::new("abcd\nefgh\nijkl\nmnop", None);
    e.cursor = Pos { row: 0, col: 1 };
    e.normal_key('v');
    e.cursor = Pos { row: 1, col: 2 };
    e.normal_key('d');
    assert_eq!(e.text(), "ah\nijkl\nmnop");

    e.cursor = Pos { row: 1, col: 1 };
    e.normal_key('.');
    assert_eq!(e.text(), "ah\nip");
}

#[test]
fn single_angle_brackets_indent_and_unindent_the_current_line() {
    let mut e = Editor::new("  value\nnext", None);
    e.cursor = Pos { row: 0, col: 3 };

    e.normal_key('>');

    assert_eq!(e.text(), "      value\nnext");
    assert_eq!(e.cursor, Pos { row: 0, col: 7 });
    e.undo(false);
    assert_eq!(e.text(), "  value\nnext");
    assert_eq!(e.cursor, Pos { row: 0, col: 3 });

    e.normal_key('<');

    assert_eq!(e.text(), "value\nnext");
    assert_eq!(e.cursor, Pos { row: 0, col: 1 });
}

#[test]
fn number_adjustment_preserves_zero_padding_and_repeats_with_dot() {
    let mut e = Editor::new("value 009 and 20", None);
    e.cursor.col = 6;
    assert!(e.adjust_number(1));
    assert_eq!(e.text(), "value 010 and 20");
    assert!(e.adjust_number(-2));
    assert_eq!(e.text(), "value 008 and 20");

    e.cursor.col = 14;
    e.normal_key('.');
    assert_eq!(e.text(), "value 008 and 18");
}

#[test]
fn number_adjustment_handles_negative_values_and_missing_numbers() {
    let mut e = Editor::new("-09 and text", None);
    assert!(e.adjust_number(1));
    assert_eq!(e.text(), "-08 and text");
    assert!(e.adjust_number(-1));
    assert_eq!(e.text(), "-09 and text");

    e.cursor.col = 5;
    assert!(!e.adjust_number(1));
    assert_eq!(e.text(), "-09 and text");

    let mut e = Editor::new("199", None);
    e.cursor.col = 1;
    assert!(e.adjust_number(1));
    assert_eq!(e.text(), "200");
}

#[test]
fn moving_a_line_reindents_it_to_the_previous_nonblank_line() {
    let mut e = Editor::new("root\n    moved\n   \nsibling", None);
    e.cursor = Pos { row: 1, col: 6 };

    e.move_line(1);

    assert_eq!(e.text(), "root\n   \nmoved\nsibling");
    assert_eq!(e.cursor, Pos { row: 2, col: 2 });
    e.undo(false);
    assert_eq!(e.text(), "root\n    moved\n   \nsibling");
    assert_eq!(e.cursor, Pos { row: 1, col: 6 });
}

#[test]
fn moving_a_line_at_the_document_edge_is_a_noop() {
    let mut e = Editor::new("first\nsecond", None);

    e.move_line(-1);

    assert_eq!(e.text(), "first\nsecond");
    assert!(e.undo.is_empty());
}

#[test]
fn visual_indent_applies_to_every_selected_line_and_undo_restores_endpoints() {
    let mut e = Editor::new("alpha\n  beta\ngamma", None);
    e.normal_key('V');
    e.move_by(0, 1, 1);

    e.normal_key('>');

    assert_eq!(e.text(), "    alpha\n      beta\ngamma");
    assert_eq!(e.mode, Mode::Visual);
    assert_eq!(e.anchor, Pos { row: 0, col: 0 });
    assert_eq!(e.cursor, Pos { row: 1, col: 0 });

    e.undo(false);

    assert_eq!(e.text(), "alpha\n  beta\ngamma");
    assert_eq!(e.anchor, Pos { row: 0, col: 0 });
    assert_eq!(e.cursor, Pos { row: 1, col: 0 });
}

#[test]
fn characterwise_visual_indent_keeps_selection_endpoints_attached_to_text() {
    let mut e = Editor::new("alpha\n  beta\ngamma", None);
    e.cursor.col = 1;
    e.normal_key('v');
    e.move_by(0, 1, 1);

    e.normal_key('>');

    assert_eq!(e.text(), "    alpha\n      beta\ngamma");
    assert_eq!(e.anchor, Pos { row: 0, col: 5 });
    assert_eq!(e.cursor, Pos { row: 1, col: 5 });

    e.undo(false);

    assert_eq!(e.anchor, Pos { row: 0, col: 1 });
    assert_eq!(e.cursor, Pos { row: 1, col: 1 });
}

#[test]
fn visual_alt_j_moves_the_selected_block_and_preserves_its_selection() {
    let mut e = Editor::new("root\n    alpha\n      child\nsibling\ntail", None);
    e.cursor = Pos { row: 1, col: 0 };
    e.normal_key('V');
    e.move_by(0, 1, 1);

    e.move_line(1);

    assert_eq!(e.text(), "root\nsibling\nalpha\n  child\ntail");
    assert_eq!(e.mode, Mode::Visual);
    assert_eq!(e.anchor, Pos { row: 2, col: 0 });
    assert_eq!(e.cursor, Pos { row: 3, col: 0 });

    e.undo(false);

    assert_eq!(e.text(), "root\n    alpha\n      child\nsibling\ntail");
    assert_eq!(e.anchor, Pos { row: 1, col: 0 });
    assert_eq!(e.cursor, Pos { row: 2, col: 0 });
}

#[test]
fn visual_alt_k_moves_a_reversed_selection_up_as_a_block() {
    let mut e = Editor::new("root\nsibling\n    alpha\n      child\ntail", None);
    e.cursor = Pos { row: 3, col: 0 };
    e.normal_key('V');
    e.move_by(0, -1, 1);

    e.move_line(-1);

    assert_eq!(e.text(), "root\nalpha\n  child\nsibling\ntail");
    assert_eq!(e.mode, Mode::Visual);
    assert_eq!(e.anchor, Pos { row: 2, col: 0 });
    assert_eq!(e.cursor, Pos { row: 1, col: 0 });
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
fn visual_o_swaps_the_active_selection_endpoint() {
    let mut e = Editor::new("one\ntwo\nthree", None);
    e.normal_key('v');
    e.move_by(0, 2, 1);
    assert_eq!(e.cursor, Pos { row: 2, col: 0 });

    e.normal_key('o');

    assert_eq!(e.mode, Mode::Visual);
    assert_eq!(e.cursor, Pos { row: 0, col: 0 });
    assert_eq!(e.anchor, Pos { row: 2, col: 0 });
}

#[test]
fn center_cursor_centers_the_current_display_line_and_clamps_at_the_end() {
    let mut e = Editor::new(
        &(0..20)
            .map(|row| row.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
        None,
    );
    e.cursor.row = 10;
    e.center_cursor(5, 20, false);
    assert_eq!(e.top, 8);

    e.cursor.row = 19;
    e.center_cursor(5, 20, false);
    assert_eq!(e.top, 15);
}

#[test]
fn screen_line_motions_select_first_nonblank_visible_lines() {
    let mut e = Editor::new("  top\n  middle\n  bottom\n  last", None);
    e.top = 1;

    e.move_to_screen_line(0, 20, false);
    assert_eq!(e.cursor, Pos { row: 1, col: 2 });

    e.move_to_screen_line(1, 20, false);
    assert_eq!(e.cursor, Pos { row: 2, col: 2 });

    e.move_to_screen_line(2, 20, false);
    assert_eq!(e.cursor, Pos { row: 3, col: 2 });
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
fn horizontal_scroll_moves_the_cursor_to_the_left_edge_when_it_would_be_hidden() {
    let mut e = Editor::new("abcdefghijklmnopqrstuvwxyz", None);
    e.scroll_horizontal(1, 10);
    e.reveal_cursor(5, 10, false);
    assert_eq!(e.left, 5);
    assert_eq!(e.cursor.col, 5);

    e.scroll_horizontal(1, 10);
    e.reveal_cursor(5, 10, false);
    assert_eq!(e.left, 10);
    assert_eq!(e.cursor.col, 10);
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
fn display_page_motion_moves_the_cursor_by_the_requested_rows() {
    let text = (0..30)
        .map(|row| row.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let mut e = Editor::new(&text, None);
    e.cursor.row = 10;

    e.move_by_display_rows(1, 10, 20, 80, false);
    assert_eq!(e.cursor.row, 20);
    assert_eq!(e.top, 10);

    e.move_by_display_rows(-1, 10, 20, 80, false);
    assert_eq!(e.cursor.row, 10);
    assert_eq!(e.top, 0);
}

#[test]
fn display_page_motion_tracks_wrapped_screen_rows() {
    let mut e = Editor::new("abcdefghij\nnext", None);
    e.cursor = Pos { row: 0, col: 1 };

    e.move_by_display_rows(1, 1, 5, 4, true);

    assert_eq!(e.cursor, Pos { row: 0, col: 5 });
}

#[test]
fn full_page_motion_clamps_at_both_file_edges() {
    let text = (0..25)
        .map(|row| row.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let mut e = Editor::new(&text, None);

    e.move_by_display_rows(1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 20);
    e.move_by_display_rows(1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 24);
    e.move_by_display_rows(1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 24);

    e.move_by_display_rows(-1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 4);
    e.move_by_display_rows(-1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 0);
    e.move_by_display_rows(-1, 20, 20, 80, false);
    assert_eq!(e.cursor.row, 0);
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

#[test]
fn windowed_display_index_matches_the_full_row_map() {
    let mut editor = Editor::new(
        "short\nthis line is definitely longer than four columns\n\nx",
        None,
    );
    for cols in [1usize, 4, 7, 80] {
        for wrap in [true, false] {
            for mode in [Mode::Normal, Mode::Insert] {
                editor.mode = mode;
                let full = editor.display_rows(cols, wrap);
                assert_eq!(editor.display_total(cols, wrap), full.len());
                assert_eq!(editor.display_window(0, full.len(), cols, wrap), full);

                for start in 0..full.len() {
                    let slice = editor.display_window(start, full.len() - start, cols, wrap);
                    assert_eq!(slice, full[start..].to_vec());
                }

                for (index, &(row, col)) in full.iter().enumerate() {
                    let pos = Pos { row, col };
                    assert_eq!(editor.display_index(pos, cols, wrap), index);
                }
            }
        }
    }
}
