use super::{BufferAction, Editor, Mode, Pos};

impl Editor {
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
        if self.is_directory_browser() {
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
}
