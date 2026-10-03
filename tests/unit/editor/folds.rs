use super::super::*;

#[test]
fn separately_closed_nested_folds_keep_the_outer_header_visible() {
    let mut e = Editor::new("root\n    child\n        leaf\nnext", None);
    e.cursor.row = 1;
    for key in "zc".chars() {
        e.normal_key(key);
    }
    e.cursor.row = 0;
    for key in "zcjk".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.cursor, Pos::default());
    assert_eq!(e.display_rows(80, false), vec![(0, 0), (3, 0)]);
    for key in "zo".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.display_rows(80, false), vec![(0, 0), (1, 0), (3, 0)]);
    e.normal_key('j');
    for key in "zo".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.display_total(80, false), 4);
}

#[test]
fn indentation_folds_hide_nested_lines_and_motion_skips_them() {
    use crate::editor::folds::{FoldProvider, FoldRange, IndentFoldProvider};
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
    use crate::editor::folds::{FoldProvider, FoldRange};
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
