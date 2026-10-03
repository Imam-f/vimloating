use super::super::*;

#[test]
fn normal_yiw_and_yaw_copy_unicode_words_without_editing_or_moving_the_cursor() {
    let mut e = Editor::new("one café tail", None);
    e.cursor.col = 6;
    e.anchor = Pos { row: 0, col: 12 };
    for key in "yiw".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.register[0].iter().collect::<String>(), "café");
    assert_eq!(e.cursor, Pos { row: 0, col: 6 });
    assert_eq!(e.mode, Mode::Normal);
    assert!(!e.linewise);
    assert!(!e.dirty());
    assert!(e.undo.is_empty());
    assert!(e.yank_highlight.is_some());
    for key in "yaw".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.register[0].iter().collect::<String>(), "café ");
    assert_eq!(e.text(), "one café tail");
}

#[test]
fn normal_diw_and_daw_delete_inner_or_around_words_and_support_undo_and_paste() {
    for (command, remaining, copied) in [("diw", "one  tail", "café"), ("daw", "one tail", "café ")]
    {
        let mut e = Editor::new("one café tail", None);
        e.cursor.col = 6;
        for key in command.chars() {
            e.normal_key(key);
        }
        assert_eq!(e.text(), remaining);
        assert_eq!(e.register[0].iter().collect::<String>(), copied);
        assert_eq!(e.undo.len(), 1);
        assert_eq!(e.mode, Mode::Normal);
        e.normal_key('P');
        assert_eq!(e.text(), "one café tail");
        e.undo(false);
        e.undo(false);
        assert_eq!(e.text(), "one café tail");
    }
    let mut e = Editor::new("one   tail", None);
    e.cursor.col = 4;
    for key in "daw".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "one");
    assert_eq!(e.register[0].iter().collect::<String>(), "   tail");
    let mut e = Editor::new("one tail", None);
    e.cursor.col = 5;
    for key in "daw".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "one");
    assert_eq!(e.register[0].iter().collect::<String>(), " tail");
}

#[test]
fn text_object_operator_counts_multiply_and_linewise_operations_still_work() {
    for command in ["2diw", "d2iw", "di2w"] {
        let mut e = Editor::new("one two three four tail", None);
        for key in command.chars() {
            e.normal_key(key);
        }
        assert_eq!(e.text(), " three four tail", "{command}");
        assert_eq!(e.register[0].iter().collect::<String>(), "one two");
    }
    let mut e = Editor::new("one two three four tail", None);
    for key in "2d2iw".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), " tail");
    let mut e = Editor::new("one\ntwo\nthree\nfour\nfive", None);
    for key in "2y2y".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.register.len(), 4);
    assert!(e.linewise);
    for key in "2d2d".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "five");
}

#[test]
fn dot_repeats_the_delete_object_at_the_new_word_instead_of_its_old_width() {
    let mut e = Editor::new("a longer enormous", None);
    for key in "daw".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "longer enormous");
    e.normal_key('.');
    assert_eq!(e.text(), "enormous");
    e.normal_key('.');
    assert_eq!(e.text(), "");
    e.undo(false);
    assert_eq!(e.text(), "enormous");
    let mut e = Editor::new("a longer enormous", None);
    for key in "diw".chars() {
        e.normal_key(key);
    }
    e.normal_key('w');
    e.normal_key('.');
    assert_eq!(e.text(), "  enormous");
}

#[test]
fn text_object_operators_share_blocks_quotes_and_tags_and_ignore_stale_visual_anchors() {
    let mut e = Editor::new("left (one\ntwo) tail", None);
    e.cursor = Pos { row: 1, col: 1 };
    for key in "yib".chars() {
        e.normal_key(key);
    }
    assert_eq!(
        e.register
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<_>>(),
        vec!["one", "two"]
    );
    assert_eq!(e.cursor, Pos { row: 1, col: 1 });
    for key in "dab".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "left  tail");
    e.undo(false);
    assert_eq!(e.text(), "left (one\ntwo) tail");
    let mut e = Editor::new("<div><b>value</b></div>", None);
    e.cursor.col = 10;
    e.anchor = Pos::default();
    for key in "dit".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "<div><b></b></div>");
    let mut e = Editor::new("say \"hello\"", None);
    e.cursor.col = 6;
    for key in "ya\"".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.register[0].iter().collect::<String>(), "\"hello\"");
}

#[test]
fn failed_or_cancelled_text_object_operators_leave_contents_and_register_unchanged() {
    let mut e = Editor::new("word ()", None);
    for key in "yiw".chars() {
        e.normal_key(key);
    }
    e.cursor.col = 5;
    for key in "dib".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "word ()");
    assert_eq!(e.register[0].iter().collect::<String>(), "word");
    assert_eq!(e.mode, Mode::Normal);
    assert!(e.undo.is_empty());
    for key in "dix".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "word ()");
    for key in "2di".chars() {
        e.normal_key(key);
    }
    e.escape();
    assert!(e.pending.is_none());
    assert!(e.pending_operator.is_none());
    e.cursor.col = 0;
    for key in "yiw".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.register[0].iter().collect::<String>(), "word");
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
