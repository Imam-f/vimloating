use super::super::*;

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
