use super::super::*;

fn keys(editor: &mut Editor, command: &str) {
    for key in command.chars() {
        editor.normal_key(key);
    }
}

fn register_text(editor: &Editor) -> String {
    editor
        .register
        .iter()
        .map(|line| line.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn selection_text(editor: &Editor) -> String {
    let (start, end) = editor.selection();
    if editor.visual_linewise {
        editor.lines[start.row..=end.row]
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        editor
            .source_chars()
            .into_iter()
            .filter(|(pos, _)| start <= *pos && *pos <= end)
            .map(|(_, ch)| ch)
            .collect()
    }
}

#[test]
fn all_text_objects_support_visual_delete_yank_and_case_operators() {
    // Each fixture describes the actual selected text and deletion, independently of the parser.
    let fixtures = [
        (
            'w',
            "pre aB tail",
            Pos { row: 0, col: 5 },
            "aB",
            "aB ",
            "pre  tail",
            "pre tail",
        ),
        (
            'W',
            "pre aB-cD tail",
            Pos { row: 0, col: 5 },
            "aB-cD",
            "aB-cD ",
            "pre  tail",
            "pre tail",
        ),
        (
            's',
            "One. aB! Tail.",
            Pos { row: 0, col: 6 },
            "aB!",
            "aB! ",
            "One.  Tail.",
            "One. Tail.",
        ),
        (
            'p',
            "pre\n\naB\n\npost",
            Pos { row: 2, col: 1 },
            "aB",
            "aB\n",
            "pre\n\n\npost",
            "pre\n\npost",
        ),
        (
            'b',
            "pre (aB) tail",
            Pos { row: 0, col: 6 },
            "aB",
            "(aB)",
            "pre () tail",
            "pre  tail",
        ),
        (
            'B',
            "pre {aB} tail",
            Pos { row: 0, col: 6 },
            "aB",
            "{aB}",
            "pre {} tail",
            "pre  tail",
        ),
        (
            '[',
            "pre [aB] tail",
            Pos { row: 0, col: 6 },
            "aB",
            "[aB]",
            "pre [] tail",
            "pre  tail",
        ),
        (
            '<',
            "pre <aB> tail",
            Pos { row: 0, col: 6 },
            "aB",
            "<aB>",
            "pre <> tail",
            "pre  tail",
        ),
        (
            't',
            "<b>aB</b>",
            Pos { row: 0, col: 4 },
            "aB",
            "<b>aB</b>",
            "<b></b>",
            "",
        ),
        (
            '"',
            "pre \"aB\" tail",
            Pos { row: 0, col: 6 },
            "aB",
            "\"aB\"",
            "pre \"\" tail",
            "pre  tail",
        ),
        (
            '\'',
            "pre 'aB' tail",
            Pos { row: 0, col: 6 },
            "aB",
            "'aB'",
            "pre '' tail",
            "pre  tail",
        ),
        (
            '`',
            "pre `aB` tail",
            Pos { row: 0, col: 6 },
            "aB",
            "`aB`",
            "pre `` tail",
            "pre  tail",
        ),
    ];
    for (object, text, origin, inner, around, delete_inner, delete_around) in fixtures {
        for (scope, selected, deleted) in [('i', inner, delete_inner), ('a', around, delete_around)]
        {
            for operator in ["v", "d", "y", "gu", "gU", "g~"] {
                let command = format!("{operator}{scope}{object}");
                let mut e = Editor::new(text, None);
                e.cursor = origin;
                keys(&mut e, &command);
                match operator {
                    "v" => {
                        assert_eq!(e.mode, Mode::Visual, "{command}");
                        assert_eq!(selection_text(&e), selected, "{command}");
                        assert_eq!(e.text(), text, "{command}");
                    }
                    "y" => {
                        assert_eq!(register_text(&e), selected, "{command}");
                        assert_eq!(e.text(), text, "{command}");
                        assert_eq!(e.cursor, origin, "{command}");
                        assert_eq!(e.mode, Mode::Normal, "{command}");
                        assert!(e.undo.is_empty(), "{command}");
                        assert!(e.yank_highlight.is_some(), "{command}");
                    }
                    _ => {
                        let expected = match operator {
                            "d" => deleted.to_owned(),
                            "gu" => text.replacen(selected, &selected.to_lowercase(), 1),
                            "gU" => text.replacen(selected, &selected.to_uppercase(), 1),
                            "g~" => {
                                let toggled: String = selected
                                    .chars()
                                    .flat_map(|ch| {
                                        if ch.is_uppercase() {
                                            ch.to_lowercase().collect::<Vec<_>>()
                                        } else {
                                            ch.to_uppercase().collect::<Vec<_>>()
                                        }
                                    })
                                    .collect();
                                text.replacen(selected, &toggled, 1)
                            }
                            _ => unreachable!(),
                        };
                        assert_eq!(e.text(), expected, "{command}");
                        assert_eq!(e.mode, Mode::Normal, "{command}");
                        assert_eq!(e.undo.len(), 1, "{command}");
                        if operator == "d" {
                            assert_eq!(register_text(&e), selected, "{command}");
                        }
                        e.undo(false);
                        assert_eq!(e.text(), text, "undo {command}");
                        e.undo(true);
                        assert_eq!(e.text(), expected, "redo {command}");
                    }
                }
                assert!(e.pending.is_none(), "{command}");
                assert!(e.pending_operator.is_none(), "{command}");
            }
        }
    }
}

#[test]
fn case_text_object_counts_multiply_at_every_prefix_and_survive_repeat() {
    for (operator, expected_two, expected_four, repeated) in [
        (
            "gu",
            "one two ThReE FoUr tail",
            "one two three four tail",
            "one two three four tail",
        ),
        (
            "gU",
            "ONE TWO ThReE FoUr tail",
            "ONE TWO THREE FOUR tail",
            "ONE TWO THREE FOUR tail",
        ),
        (
            "g~",
            "oNe tWo ThReE FoUr tail",
            "oNe tWo tHrEe fOuR tail",
            "oNe tWo tHrEe fOuR tail",
        ),
    ] {
        for command in [
            format!("2{operator}iw"),
            format!("{operator}2iw"),
            format!("{operator}i2w"),
        ] {
            let mut e = Editor::new("OnE TwO ThReE FoUr tail", None);
            keys(&mut e, &command);
            assert_eq!(e.text(), expected_two, "{command}");
            e.cursor.col = 9;
            e.normal_key('.');
            assert_eq!(e.text(), repeated, "repeat {command}");
            assert_eq!(e.undo.len(), 2);
        }
        let mut e = Editor::new("OnE TwO ThReE FoUr tail", None);
        keys(&mut e, &format!("2{operator}2iw"));
        assert_eq!(e.text(), expected_four, "{operator}");
    }
}

#[test]
fn repeating_case_text_objects_resolves_the_new_word_width_and_keeps_the_register() {
    for (operator, first, repeated) in [
        ("gu", "aB LONGER enormous", "ab longer enormous"),
        ("gU", "Ab longer enormous", "AB LONGER enormous"),
        ("g~", "aB lOnGeR enormous", "Ab LoNgEr enormous"),
    ] {
        let mut e = Editor::new(first, None);
        keys(&mut e, "yy");
        let register = e.register.clone();
        keys(&mut e, &format!("{operator}iw"));
        e.cursor.col = 5;
        e.normal_key('.');
        assert_eq!(e.text(), repeated, "{operator}");
        assert_eq!(e.register, register, "{operator}");
        assert_eq!(e.undo.len(), 2, "{operator}");
    }
}

#[test]
fn case_objects_support_multiline_nesting_aliases_and_unicode_expansion() {
    let mut e = Editor::new("pre (OuTeR (InNeR\nßaB) EnD) tail", None);
    e.cursor = Pos { row: 1, col: 1 };
    keys(&mut e, "2gUib");
    assert_eq!(e.text(), "pre (OUTER (INNER\nSSAB) END) tail");
    e.undo(false);
    assert_eq!(e.text(), "pre (OuTeR (InNeR\nßaB) EnD) tail");
    for (text, col, aliases) in [
        ("(aB)", 2, "b()"),
        ("{aB}", 2, "B{}"),
        ("[aB]", 2, "[]"),
        ("<aB>", 2, "<>"),
    ] {
        for object in aliases.chars() {
            let mut e = Editor::new(text, None);
            e.cursor.col = col;
            keys(&mut e, &format!("g~i{object}"));
            assert_eq!(e.text(), text.replace("aB", "Ab"), "{object}");
        }
    }
}

#[test]
fn invalid_cancelled_and_unchanged_case_objects_do_not_replace_the_last_change() {
    for operator in ["gu", "gU", "g~"] {
        let mut e = Editor::new("aB () tail", None);
        keys(&mut e, "~");
        for command in [format!("{operator}ib"), format!("{operator}ix")] {
            e.cursor.col = 4;
            keys(&mut e, &command);
            assert_eq!(e.text(), "AB () tail", "{command}");
            assert_eq!(e.mode, Mode::Normal);
            assert_eq!(e.undo.len(), 1);
        }
        for cancel_with_ctrl_c in [false, true] {
            keys(&mut e, &format!("2{operator}i"));
            if cancel_with_ctrl_c {
                e.control_key('c');
            } else {
                e.escape();
            }
            assert!(e.pending.is_none());
            assert!(e.pending_operator.is_none());
            assert!(e.count.is_empty());
        }
        e.cursor.col = 1;
        e.normal_key('.');
        assert_eq!(e.text(), "Ab () tail");
    }
    let mut e = Editor::new("aB lower", None);
    keys(&mut e, "~");
    e.cursor.col = 5;
    keys(&mut e, "guiw");
    assert_eq!(e.undo.len(), 1);
    assert_eq!(e.mode, Mode::Normal);
    e.cursor.col = 1;
    e.normal_key('.');
    assert_eq!(e.text(), "Ab lower");
}

#[test]
fn counted_line_case_operators_still_work_with_counts_after_the_operator() {
    for (command, expected) in [
        ("2gu2u", "ab\ncd\nef\ngh\nIj"),
        ("2gU2U", "AB\nCD\nEF\nGH\nIj"),
        ("2g~2~", "aB\ncD\neF\ngH\nIj"),
    ] {
        let mut e = Editor::new("Ab\nCd\nEf\nGh\nIj", None);
        keys(&mut e, command);
        assert_eq!(e.text(), expected, "{command}");
        assert_eq!(e.undo.len(), 1);
    }
}
