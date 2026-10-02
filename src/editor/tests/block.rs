use super::super::*;

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
