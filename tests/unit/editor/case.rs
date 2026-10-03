use super::super::*;

#[test]
fn lowercase_uppercase_lines_and_visual_selections_support_counts_undo_and_repeat() {
    let mut e = Editor::new("  HéLLo\nßabc\nMiXeD", None);
    for key in "2guu".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "  héllo\nßabc\nMiXeD");
    e.cursor.row = 2;
    e.normal_key('.');
    assert_eq!(e.text(), "  héllo\nßabc\nmixed");
    e.undo(false);
    assert_eq!(e.text(), "  héllo\nßabc\nMiXeD");
    e.cursor.row = 1;
    for key in "gUU".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "  héllo\nSSABC\nMiXeD");
    e.undo(false);
    assert_eq!(e.text(), "  héllo\nßabc\nMiXeD");
    e.cursor = Pos { row: 2, col: 1 };
    for key in "vllgu".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "  héllo\nßabc\nMixeD");
    e.cursor = Pos { row: 0, col: 2 };
    for key in "viwgU".chars() {
        e.normal_key(key);
    }
    assert_eq!(e.text(), "  HÉLLO\nßabc\nMixeD");
}

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
