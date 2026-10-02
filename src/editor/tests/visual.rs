use super::super::*;

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
