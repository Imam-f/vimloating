use std::{fs, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    Command,
    Search,
}

#[derive(Clone)]
struct Snapshot {
    lines: Vec<Vec<char>>,
    cursor: Pos,
}

pub struct Editor {
    pub lines: Vec<Vec<char>>,
    pub cursor: Pos,
    pub mode: Mode,
    pub anchor: Pos,
    pub path: Option<PathBuf>,
    pub message: String,
    pub prompt: String,
    pub search: String,
    pub top: usize,
    pub left: usize,
    horizontal_scroll_hold: bool,
    pub pending: Option<char>,
    pub count: String,
    pub quit: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    saved: String,
    register: Vec<Vec<char>>,
    linewise: bool,
    preferred_col: Option<usize>,
}

impl Editor {
    pub fn new(text: &str, path: Option<PathBuf>) -> Self {
        let text = text.replace("\r\n", "\n");
        Self {
            lines: text.split('\n').map(|s| s.chars().collect()).collect(),
            cursor: Pos::default(),
            mode: Mode::Normal,
            anchor: Pos::default(),
            path,
            message: "Ready · :help for controls".into(),
            prompt: String::new(),
            search: String::new(),
            top: 0,
            left: 0,
            horizontal_scroll_hold: false,
            pending: None,
            count: String::new(),
            quit: false,
            undo: vec![],
            redo: vec![],
            saved: text,
            register: vec![],
            linewise: true,
            preferred_col: None,
        }
    }

    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn dirty(&self) -> bool {
        self.text() != self.saved
    }

    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "untitled".into())
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            cursor: self.cursor,
        }
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, redo: bool) {
        let state = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(state) = state {
            let current = self.snapshot();
            if redo {
                self.undo.push(current);
            } else {
                self.redo.push(current);
            }
            self.lines = state.lines;
            self.cursor = state.cursor;
            self.clamp();
            self.message = if redo { "Redo" } else { "Undo" }.into();
        }
    }

    pub fn clamp(&mut self) {
        self.cursor.row = self.cursor.row.min(self.lines.len() - 1);
        let len = self.lines[self.cursor.row].len();
        let max = if self.mode == Mode::Insert {
            len
        } else {
            len.saturating_sub(1)
        };
        self.cursor.col = self.cursor.col.min(max);
    }

    pub fn escape(&mut self) {
        self.horizontal_scroll_hold = false;
        if self.mode == Mode::Insert {
            self.cursor.col = self.cursor.col.saturating_sub(1);
        }
        self.mode = Mode::Normal;
        self.pending = None;
        self.count.clear();
        self.prompt.clear();
        self.preferred_col = None;
        self.clamp();
    }

    pub fn move_by(&mut self, dx: isize, dy: isize, n: usize) {
        self.horizontal_scroll_hold = false;
        if dy != 0 {
            let col = *self.preferred_col.get_or_insert(self.cursor.col);
            self.cursor.row = self
                .cursor
                .row
                .saturating_add_signed(dy * n as isize)
                .min(self.lines.len() - 1);
            self.cursor.col = col;
        } else {
            self.preferred_col = None;
            self.cursor.col = self.cursor.col.saturating_add_signed(dx * n as isize);
        }
        self.clamp();
    }

    pub fn scroll_horizontal(&mut self, direction: isize, cols: usize) {
        let cols = cols.max(1);
        let max_left = self
            .lines
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0)
            .saturating_sub(cols);
        let amount = (cols / 2).max(1) as isize;
        self.left = self
            .left
            .saturating_add_signed(direction * amount)
            .min(max_left);
        self.horizontal_scroll_hold = true;
    }

    pub fn follow_cursor_horizontally(&mut self) {
        self.horizontal_scroll_hold = false;
    }

    pub fn begin_insert(&mut self, key: char) {
        self.horizontal_scroll_hold = false;
        self.checkpoint();
        let row = self.cursor.row;
        match key {
            'a' => self.cursor.col = (self.cursor.col + 1).min(self.lines[row].len()),
            'A' => self.cursor.col = self.lines[row].len(),
            'I' => {
                self.cursor.col = self.lines[row]
                    .iter()
                    .position(|c| !c.is_whitespace())
                    .unwrap_or(0)
            }
            'o' | 'O' => {
                let indent: Vec<char> = self.lines[row]
                    .iter()
                    .take_while(|c| c.is_whitespace())
                    .copied()
                    .collect();
                let next = if key == 'o' { row + 1 } else { row };
                self.cursor = Pos {
                    row: next,
                    col: indent.len(),
                };
                self.lines.insert(next, indent);
            }
            _ => {}
        }
        self.mode = Mode::Insert;
        self.preferred_col = None;
    }

    pub fn insert_char(&mut self, ch: char) {
        self.horizontal_scroll_hold = false;
        if !ch.is_control() {
            self.lines[self.cursor.row].insert(self.cursor.col, ch);
            self.cursor.col += 1;
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        let normalized = text.replace("\r\n", "\n");
        for ch in normalized.chars() {
            match ch {
                '\n' => {
                    // Pasted text keeps its own indentation instead of auto-indenting.
                    let tail = self.lines[self.cursor.row].split_off(self.cursor.col);
                    self.cursor.row += 1;
                    self.cursor.col = 0;
                    self.lines.insert(self.cursor.row, tail);
                }
                '\t' => {
                    self.lines[self.cursor.row].insert(self.cursor.col, '\t');
                    self.cursor.col += 1;
                }
                _ => self.insert_char(ch),
            }
        }
    }

    pub fn newline(&mut self) {
        self.horizontal_scroll_hold = false;
        let indent: Vec<char> = self.lines[self.cursor.row]
            .iter()
            .take(self.cursor.col)
            .take_while(|c| c.is_whitespace())
            .copied()
            .collect();
        let tail = self.lines[self.cursor.row].split_off(self.cursor.col);
        let mut next = indent.clone();
        next.extend(tail);
        self.cursor.row += 1;
        self.cursor.col = indent.len();
        self.lines.insert(self.cursor.row, next);
    }

    pub fn backspace(&mut self) {
        self.horizontal_scroll_hold = false;
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
            self.lines[self.cursor.row].remove(self.cursor.col);
        } else if self.cursor.row > 0 {
            let line = self.lines.remove(self.cursor.row);
            self.cursor.row -= 1;
            self.cursor.col = self.lines[self.cursor.row].len();
            self.lines[self.cursor.row].extend(line);
        }
    }

    pub fn delete_prev_word(&mut self) {
        let row = self.cursor.row;
        let col = self.cursor.col;
        if col == 0 {
            return;
        }
        let line = &mut self.lines[row];
        let mut start = col;
        while start > 0 && line[start - 1].is_whitespace() {
            start -= 1;
        }
        if start > 0 {
            let previous = line[start - 1];
            let word_class = if previous.is_alphanumeric() || previous == '_' {
                1
            } else {
                2
            };
            while start > 0 {
                let ch = line[start - 1];
                let class = if ch.is_whitespace() {
                    0
                } else if ch.is_alphanumeric() || ch == '_' {
                    1
                } else {
                    2
                };
                if class != word_class {
                    break;
                }
                start -= 1;
            }
        }
        line.drain(start..col);
        self.cursor.col = start;
    }

    pub fn delete_forward(&mut self) {
        self.horizontal_scroll_hold = false;
        let row = self.cursor.row;
        if self.cursor.col < self.lines[row].len() {
            self.lines[row].remove(self.cursor.col);
        } else if row + 1 < self.lines.len() {
            let next = self.lines.remove(row + 1);
            self.lines[row].extend(next);
        }
    }

    fn next_pos(&self, p: Pos) -> Option<Pos> {
        if p.col + 1 < self.lines[p.row].len() {
            Some(Pos {
                col: p.col + 1,
                ..p
            })
        } else if p.row + 1 < self.lines.len() {
            Some(Pos {
                row: p.row + 1,
                col: 0,
            })
        } else {
            None
        }
    }

    fn prev_pos(&self, p: Pos) -> Option<Pos> {
        if p.col > 0 {
            Some(Pos {
                col: p.col - 1,
                ..p
            })
        } else if p.row > 0 {
            Some(Pos {
                row: p.row - 1,
                col: self.lines[p.row - 1].len().saturating_sub(1),
            })
        } else {
            None
        }
    }

    fn class(&self, p: Pos) -> u8 {
        match self.lines[p.row].get(p.col) {
            None => 0,
            Some(c) if c.is_whitespace() => 0,
            Some(c) if c.is_alphanumeric() || *c == '_' => 1,
            _ => 2,
        }
    }

    fn word(&mut self, key: char, n: usize) {
        for _ in 0..n {
            let mut p = self.cursor;
            if key == 'b' {
                if let Some(prev) = self.prev_pos(p) {
                    p = prev;
                }
                while self.class(p) == 0 {
                    match self.prev_pos(p) {
                        Some(prev) => p = prev,
                        None => break,
                    }
                }
                let class = self.class(p);
                while let Some(prev) = self.prev_pos(p) {
                    if self.class(prev) != class || prev.row != p.row {
                        break;
                    }
                    p = prev;
                }
            } else if key == 'w' {
                let class = self.class(p);
                while let Some(next) = self.next_pos(p) {
                    let stop = self.class(next) != class || next.row != p.row;
                    p = next;
                    if stop {
                        break;
                    }
                }
                while self.class(p) == 0 {
                    match self.next_pos(p) {
                        Some(next) => p = next,
                        None => break,
                    }
                }
            } else {
                if let Some(next) = self.next_pos(p) {
                    p = next;
                }
                while self.class(p) == 0 {
                    match self.next_pos(p) {
                        Some(next) => p = next,
                        None => break,
                    }
                }
                let class = self.class(p);
                while let Some(next) = self.next_pos(p) {
                    if self.class(next) != class || next.row != p.row {
                        break;
                    }
                    p = next;
                }
            }
            self.cursor = p;
        }
        self.preferred_col = None;
        self.clamp();
    }

    pub fn selection(&self) -> (Pos, Pos) {
        (self.anchor.min(self.cursor), self.anchor.max(self.cursor))
    }

    fn visual_action(&mut self, delete: bool) {
        let (a, b) = self.selection();
        self.register.clear();
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
        match key {
            'h' => self.move_by(-1, 0, n),
            'j' => self.move_by(0, 1, n),
            'k' => self.move_by(0, -1, n),
            'l' => self.move_by(1, 0, n),
            'w' | 'b' | 'e' => self.word(key, n),
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
            'g' | 'd' | 'y' => {
                self.pending = Some(key);
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
                if self.mode == Mode::Visual {
                    self.escape();
                } else {
                    self.mode = Mode::Visual;
                    self.anchor = self.cursor;
                }
            }
            ':' | '/' => {
                self.mode = if key == ':' {
                    Mode::Command
                } else {
                    Mode::Search
                };
                self.prompt.clear();
            }
            'n' | 'N' => {
                for _ in 0..n {
                    self.find(key == 'N');
                }
            }
            _ => {}
        }
        self.count.clear();
    }

    pub fn find(&mut self, backwards: bool) {
        self.horizontal_scroll_hold = false;
        let needle: Vec<char> = self.search.chars().collect();
        if needle.is_empty() {
            return;
        }
        let mut hits = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            if line.len() >= needle.len() {
                for col in 0..=line.len() - needle.len() {
                    if line[col..col + needle.len()] == needle {
                        hits.push(Pos { row, col });
                    }
                }
            }
        }
        let hit = if backwards {
            hits.iter()
                .rev()
                .find(|p| **p < self.cursor)
                .or_else(|| hits.last())
        } else {
            hits.iter()
                .find(|p| **p > self.cursor)
                .or_else(|| hits.first())
        };
        if let Some(p) = hit {
            self.cursor = *p;
            self.message = format!("/{} · {} matches", self.search, hits.len());
        } else {
            self.message = format!("Pattern not found: {}", self.search);
        }
    }

    pub fn submit_prompt(&mut self) {
        let prompt = self.prompt.clone();
        let searching = self.mode == Mode::Search;
        self.escape();
        if searching {
            self.search = prompt;
            self.find(false);
        } else {
            self.command(&prompt);
        }
    }

    pub fn save(&mut self, path: Option<&str>) -> bool {
        let destination = path
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| self.path.clone());
        let Some(destination) = destination else {
            self.message = "No filename · use :w path/to/file".into();
            return false;
        };
        let text = self.text();
        match fs::write(&destination, &text) {
            Ok(()) => {
                self.path = Some(destination);
                self.saved = text;
                self.message = format!("Written {} · {} lines", self.name(), self.lines.len());
                true
            }
            Err(err) => {
                self.message = format!("Write failed: {err}");
                false
            }
        }
    }

    pub fn command(&mut self, command: &str) {
        let command = command.trim();
        let (cmd, arg) = command
            .split_once(char::is_whitespace)
            .unwrap_or((command, ""));
        let arg = arg.trim();
        match cmd {
            "w" => {
                self.save(Some(arg));
            }
            "wq" | "x" => {
                if self.save(Some(arg)) {
                    self.quit = true;
                }
            }
            "q" => {
                if self.dirty() {
                    self.message = "Unsaved changes · :w to save, :q! to discard".into();
                } else {
                    self.quit = true;
                }
            }
            "q!" => self.quit = true,
            "e" | "e!" => {
                if cmd == "e" && self.dirty() {
                    self.message = "Unsaved changes · :w first or :e! path".into();
                } else if arg.is_empty() {
                    self.message = "Usage: :e path/to/file".into();
                } else {
                    match fs::read_to_string(arg) {
                        Ok(text) => {
                            *self = Self::new(&text, Some(PathBuf::from(arg)));
                            self.message = "File opened".into();
                        }
                        Err(err) => self.message = format!("Open failed: {err}"),
                    }
                }
            }
            "help" => {
                self.message =
                    "hjkl · w/b/e · gg/G · i/a/o · v · dd/yy/p · u/Ctrl-R · /search · :w/:e/:q"
                        .into()
            }
            "noh" | "nohlsearch" => self.search.clear(),
            _ => {
                if let Ok(line) = command.parse::<usize>() {
                    self.cursor = Pos {
                        row: line.saturating_sub(1).min(self.lines.len() - 1),
                        col: 0,
                    };
                } else {
                    self.message = format!("Unknown command: {command}");
                }
            }
        }
    }

    pub fn display_rows(&self, cols: usize, wrap: bool) -> Vec<(usize, usize)> {
        let cols = cols.max(1);
        let mut rows = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            if !wrap || line.is_empty() {
                rows.push((row, 0));
            } else {
                for start in (0..line.len()).step_by(cols) {
                    rows.push((row, start));
                }
                if self.mode == Mode::Insert && line.len() % cols == 0 {
                    rows.push((row, line.len()));
                }
            }
        }
        rows
    }

    pub fn reveal_cursor(&mut self, rows: usize, cols: usize, wrap: bool) {
        let rows = rows.max(1);
        let cols = cols.max(1);
        let display_rows = self.display_rows(cols, wrap);
        let segment = if wrap {
            self.cursor.col / cols * cols
        } else {
            0
        };
        let row_index = display_rows
            .iter()
            .position(|&(row, start)| row == self.cursor.row && start == segment)
            .unwrap_or_else(|| {
                display_rows
                    .iter()
                    .rposition(|&(row, _)| row == self.cursor.row)
                    .unwrap_or(0)
            });
        if row_index < self.top {
            self.top = row_index;
        }
        if row_index >= self.top + rows {
            self.top = row_index + 1 - rows;
        }
        if wrap {
            self.left = 0;
        } else if !self.horizontal_scroll_hold {
            if self.cursor.col < self.left {
                self.left = self.cursor.col;
            }
            if self.cursor.col >= self.left + cols {
                self.left = self.cursor.col + 1 - cols;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn search_wraps_and_uses_character_columns() {
        let mut e = Editor::new("é猫 x 猫\n猫", None);
        e.search = "猫".into();
        e.find(false);
        assert_eq!(e.cursor, Pos { row: 0, col: 1 });
        e.find(true);
        assert_eq!(e.cursor, Pos { row: 1, col: 0 });
        e.find(false);
        assert_eq!(e.cursor, Pos { row: 0, col: 1 });
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
    fn horizontal_scroll_moves_the_buffer_view_without_moving_the_cursor() {
        let mut e = Editor::new("abcdefghijklmnopqrstuvwxyz", None);
        e.scroll_horizontal(1, 10);
        e.reveal_cursor(5, 10, false);
        assert_eq!(e.left, 5);
        assert_eq!(e.cursor.col, 0);

        e.move_by(1, 0, 1);
        e.reveal_cursor(5, 10, false);
        assert_eq!(e.left, 1);
    }

    #[test]
    fn save_round_trip_and_failed_save_keeps_dirty_state() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("vimloating-{}-{unique}.txt", std::process::id()));
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
}
