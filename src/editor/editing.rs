use super::Editor;

impl Editor {
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
                self.cursor = super::Pos {
                    row: next,
                    col: indent.len(),
                };
                self.lines.insert(next, indent);
            }
            _ => {}
        }
        self.mode = super::Mode::Insert;
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
}
