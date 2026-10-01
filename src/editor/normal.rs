use super::{BufferAction, CharFind, Editor, Mode, Pos};
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

    fn word_find_hints(&self, backwards: bool) -> Vec<Pos> {
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

    fn find_char(&mut self, target: char, direction: isize, till: bool, n: usize) {
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

    fn repeat_char_find(&mut self, n: usize) {
        if let Some(last) = self.last_char_find {
            self.find_char(last.target, last.direction, last.till, n);
        }
    }

    fn visual_action(&mut self, delete: bool) {
        let (a, b) = self.selection();
        self.register.clear();
        if self.visual_linewise {
            self.register
                .extend(self.lines[a.row..=b.row].iter().cloned());
            self.linewise = true;
            if delete {
                self.checkpoint();
                self.lines.drain(a.row..=b.row);
                if self.lines.is_empty() {
                    self.lines.push(vec![]);
                }
            }
            self.cursor = Pos {
                row: (if delete { a.row } else { self.cursor.row }).min(self.lines.len() - 1),
                col: 0,
            };
            self.message = format!(
                "{} {} line(s)",
                if delete { "Deleted" } else { "Yanked" },
                self.register.len()
            );
            self.escape();
            self.clamp();
            return;
        }
        for row in a.row..=b.row {
            let start = if row == a.row { a.col } else { 0 };
            let end = if row == b.row {
                b.col + 1
            } else {
                self.lines[row].len()
            };
            self.register.push(
                self.lines[row][start.min(self.lines[row].len())..end.min(self.lines[row].len())]
                    .to_vec(),
            );
        }
        self.linewise = false;
        if delete {
            self.checkpoint();
            let mut merged = self.lines[a.row][..a.col.min(self.lines[a.row].len())].to_vec();
            merged
                .extend_from_slice(&self.lines[b.row][(b.col + 1).min(self.lines[b.row].len())..]);
            self.lines.splice(a.row..=b.row, [merged]);
        }
        self.message = format!("{} selection", if delete { "Deleted" } else { "Yanked" });
        self.cursor = a;
        self.escape();
    }

    fn line_action(&mut self, delete: bool, n: usize) {
        let end = (self.cursor.row + n).min(self.lines.len());
        self.register = self.lines[self.cursor.row..end].to_vec();
        self.linewise = true;
        if delete {
            self.checkpoint();
            self.lines.drain(self.cursor.row..end);
            if self.lines.is_empty() {
                self.lines.push(vec![]);
            }
            self.clamp();
        }
        self.message = format!(
            "{} {} line(s)",
            if delete { "Deleted" } else { "Yanked" },
            self.register.len()
        );
    }

    fn paste(&mut self, before: bool, n: usize) {
        if self.register.is_empty() {
            return;
        }
        self.checkpoint();
        for _ in 0..n {
            if self.linewise {
                let row = self.cursor.row + usize::from(!before);
                self.lines.splice(row..row, self.register.clone());
                self.cursor = Pos { row, col: 0 };
            } else {
                let row = self.cursor.row;
                let col = (self.cursor.col + usize::from(!before)).min(self.lines[row].len());
                let tail = self.lines[row].split_off(col);
                self.lines[row].extend(&self.register[0]);
                for i in 1..self.register.len() {
                    self.lines.insert(row + i, self.register[i].clone());
                }
                self.lines[row + self.register.len() - 1].extend(tail);
                self.cursor.col = col;
            }
        }
        self.clamp();
    }

    pub fn normal_key(&mut self, key: char) {
        self.horizontal_scroll_hold = false;
        self.char_find_highlight = None;
        self.char_find_hints.clear();
        self.search_task = None;
        if self.is_directory_browser() && !matches!(self.pending, Some('f' | 'F' | 't' | 'T')) {
            if key == '-' {
                if let Some(parent) = self.path.as_deref().and_then(std::path::Path::parent) {
                    self.buffer_action = Some(BufferAction::Open {
                        path: parent.to_path_buf(),
                        replace: false,
                    });
                }
                return;
            }
            if matches!(
                key,
                'i' | 'a' | 'I' | 'A' | 'o' | 'O' | 'x' | 'D' | 'd' | 'y' | 'v' | 'V' | 'p' | 'P'
            ) {
                self.message = "Directory listing is read-only · Enter opens entries".into();
                self.pending = None;
                self.count.clear();
                return;
            }
        }
        if let Some(motion @ ('f' | 'F' | 't' | 'T')) = self.pending {
            let n = self.count.parse::<usize>().unwrap_or(1).clamp(1, 10000);
            self.pending = None;
            self.count.clear();
            self.find_char(
                key,
                if matches!(motion, 'f' | 't') { 1 } else { -1 },
                matches!(motion, 't' | 'T'),
                n,
            );
            return;
        }
        if key.is_ascii_digit() && (key != '0' || !self.count.is_empty()) {
            if self.count.len() < 5 {
                self.count.push(key);
            }
            return;
        }
        let has_count = !self.count.is_empty();
        let n = self.count.parse::<usize>().unwrap_or(1).clamp(1, 10000);
        if let Some(pending) = self.pending.take() {
            self.count.clear();
            if pending == key {
                match key {
                    'g' => {
                        self.cursor = Pos {
                            row: n.saturating_sub(1).min(self.lines.len() - 1),
                            col: 0,
                        };
                        self.clamp();
                    }
                    'd' => self.line_action(true, n),
                    'y' => self.line_action(false, n),
                    _ => {}
                }
            }
            return;
        }
        if self.mode == Mode::Visual && matches!(key, 'd' | 'x' | 'y') {
            self.visual_action(key != 'y');
            return;
        }
        if self.mode == Mode::Visual && key == 'o' {
            std::mem::swap(&mut self.anchor, &mut self.cursor);
            self.preferred_col = None;
            return;
        }
        match key {
            'h' => self.move_by(-1, 0, n),
            'j' => self.move_by(0, 1, n),
            'k' => self.move_by(0, -1, n),
            'l' => self.move_by(1, 0, n),
            'w' | 'b' | 'e' => self.word(key, n),
            '{' | '}' => self.paragraph(key == '}', n),
            '(' | ')' => self.sentence(key == ')', n),
            '0' => {
                self.cursor.col = 0;
                self.preferred_col = None;
            }
            '$' => {
                self.cursor.col = self.lines[self.cursor.row].len().saturating_sub(1);
                self.preferred_col = None;
            }
            '^' => {
                self.cursor.col = self.lines[self.cursor.row]
                    .iter()
                    .position(|c| !c.is_whitespace())
                    .unwrap_or(0)
            }
            'G' => {
                self.cursor.row = if has_count {
                    n.saturating_sub(1).min(self.lines.len() - 1)
                } else {
                    self.lines.len() - 1
                };
                self.clamp();
            }
            'g' | 'd' | 'y' | 'z' => {
                self.pending = Some(key);
                return;
            }
            '<' => self.change_indent(false),
            '>' => self.change_indent(true),
            'f' | 't' | 'F' | 'T' => {
                self.pending = Some(key);
                self.char_find_hints = self.word_find_hints(matches!(key, 'F' | 'T'));
                return;
            }
            'i' | 'a' | 'I' | 'A' | 'o' | 'O' => self.begin_insert(key),
            'x' => {
                self.checkpoint();
                let row = self.cursor.row;
                let end = (self.cursor.col + n).min(self.lines[row].len());
                self.register = vec![self.lines[row].drain(self.cursor.col..end).collect()];
                self.linewise = false;
                self.clamp();
            }
            'X' => {
                if self.cursor.col > 0 {
                    self.checkpoint();
                    let row = self.cursor.row;
                    let start = self.cursor.col.saturating_sub(n);
                    self.register = vec![self.lines[row].drain(start..self.cursor.col).collect()];
                    self.linewise = false;
                    self.cursor.col = start;
                    self.clamp();
                }
            }
            'D' => {
                self.checkpoint();
                self.register = vec![self.lines[self.cursor.row].split_off(self.cursor.col)];
                self.linewise = false;
                self.clamp();
            }
            'p' | 'P' => self.paste(key == 'P', n),
            'u' => {
                for _ in 0..n {
                    self.undo(false);
                }
            }
            'v' => {
                if self.mode == Mode::Visual && !self.visual_linewise {
                    self.escape();
                } else {
                    self.mode = Mode::Visual;
                    self.anchor = self.cursor;
                    self.visual_linewise = false;
                }
            }
            'V' => {
                if self.mode == Mode::Visual && self.visual_linewise {
                    self.escape();
                } else {
                    self.mode = Mode::Visual;
                    self.anchor = self.cursor;
                    self.visual_linewise = true;
                }
            }
            ':' | '/' | '?' => {
                self.mode = if key == ':' {
                    Mode::Command
                } else {
                    Mode::Search
                };
                self.search_prompt_backwards = key == '?';
                self.prompt.clear();
            }
            'n' | 'N' => {
                let backwards = if key == 'N' {
                    !self.search_backwards
                } else {
                    self.search_backwards
                };
                self.find_repeat(backwards, n);
            }
            ';' => self.repeat_char_find(n),
            _ => {}
        }
        self.count.clear();
    }
}
