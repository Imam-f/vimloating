use super::{CharFind, Editor, Pos};
use std::collections::HashMap;

impl Editor {
    fn best_word_hint(
        line: &[char],
        row: usize,
        cursor: usize,
        start: usize,
        end: usize,
        backwards: bool,
        seen: &mut HashMap<char, usize>,
    ) -> Pos {
        let mut frequencies = HashMap::new();
        for &ch in &line[start..end] {
            *frequencies.entry(ch).or_insert(0usize) += 1;
        }

        let mut best = None;
        let mut consider = |col: usize, seen_count: usize| {
            let ch = line[col];
            let repeat_count = seen_count.saturating_sub(1);
            let score = (repeat_count, frequencies[&ch], cursor.abs_diff(col), col);
            if best.is_none_or(|current| score < current) {
                best = Some(score);
            }
        };

        if backwards {
            for col in (start..end).rev() {
                let seen_count = seen.entry(line[col]).or_insert(0);
                *seen_count += 1;
                consider(col, *seen_count);
            }
        } else {
            for (col, ch) in line.iter().enumerate().take(end).skip(start) {
                let seen_count = seen.entry(*ch).or_insert(0);
                *seen_count += 1;
                consider(col, *seen_count);
            }
        }

        Pos {
            row,
            col: best.expect("a word has at least one character").3,
        }
    }

    pub(super) fn word_find_hints(&self, backwards: bool) -> Vec<Pos> {
        let row = self.cursor.row;
        let line = &self.lines[row];
        let is_word = |ch: char| ch.is_alphanumeric() || ch == '_';
        let cursor = self.cursor.col.min(line.len().saturating_sub(1));
        let mut seen = HashMap::new();
        let mut hints = Vec::new();

        if backwards {
            let mut scan = cursor;
            if line.get(cursor).is_some_and(|&ch| is_word(ch)) {
                while scan > 0 && is_word(line[scan - 1]) {
                    scan -= 1;
                }
                for &ch in line[scan..cursor].iter().rev() {
                    *seen.entry(ch).or_insert(0usize) += 1;
                }
            }
            'words: while let Some(mut col) = scan.checked_sub(1) {
                while !is_word(line[col]) {
                    *seen.entry(line[col]).or_insert(0usize) += 1;
                    let Some(previous) = col.checked_sub(1) else {
                        break 'words;
                    };
                    col = previous;
                }
                let end = col + 1;
                while col > 0 && is_word(line[col - 1]) {
                    col -= 1;
                }
                hints.push(Self::best_word_hint(
                    line, row, cursor, col, end, true, &mut seen,
                ));
                scan = col;
            }
        } else {
            let mut scan = cursor.saturating_add(1);
            if line.get(cursor).is_some_and(|&ch| is_word(ch)) {
                while scan < line.len() && is_word(line[scan]) {
                    *seen.entry(line[scan]).or_insert(0usize) += 1;
                    scan += 1;
                }
            }
            while scan < line.len() {
                while scan < line.len() && !is_word(line[scan]) {
                    *seen.entry(line[scan]).or_insert(0usize) += 1;
                    scan += 1;
                }
                if scan == line.len() {
                    break;
                }
                let start = scan;
                while scan < line.len() && is_word(line[scan]) {
                    scan += 1;
                }
                hints.push(Self::best_word_hint(
                    line, row, cursor, start, scan, false, &mut seen,
                ));
            }
        }

        hints.sort_unstable();
        hints
    }

    pub(super) fn find_char(&mut self, target: char, direction: isize, till: bool, n: usize) {
        let row = self.cursor.row;
        let line = &self.lines[row];
        let mut search_from = self.cursor.col;
        let mut last_target = None;
        for _ in 0..n {
            let target_col = if direction > 0 {
                (search_from.saturating_add(1)..line.len()).find(|&col| line[col] == target)
            } else {
                (0..search_from).rev().find(|&col| line[col] == target)
            };
            let Some(target_col) = target_col else {
                break;
            };
            last_target = Some(target_col);
            search_from = target_col;
        }

        if let Some(target_col) = last_target {
            self.cursor.col = if till {
                target_col.saturating_add_signed(-direction)
            } else {
                target_col
            };
            self.char_find_highlight = Some(Pos {
                row,
                col: target_col,
            });
            self.last_char_find = Some(CharFind {
                target,
                direction,
                till,
            });
            self.preferred_col = None;
        }
    }

    pub(super) fn repeat_char_find(&mut self, n: usize) {
        if let Some(last) = self.last_char_find {
            self.find_char(last.target, last.direction, last.till, n);
        }
    }

    pub(super) fn repeat_char_find_opposite(&mut self, n: usize) {
        if let Some(last) = self.last_char_find {
            self.find_char(last.target, -last.direction, last.till, n);
            self.last_char_find = Some(last);
        }
    }
}
