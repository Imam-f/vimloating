use super::Editor;

const INDENT_WIDTH: usize = 4;

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

    pub fn change_indent(&mut self, increase: bool) {
        let (start, end) = if self.mode == super::Mode::Visual {
            let (start, end) = self.selection();
            (start.row, end.row)
        } else {
            (self.cursor.row, self.cursor.row)
        };
        let changes: Vec<_> = (start..=end)
            .filter_map(|row| {
                if increase {
                    Some((row, INDENT_WIDTH))
                } else {
                    let leading = self.lines[row]
                        .iter()
                        .take_while(|ch| ch.is_whitespace())
                        .count();
                    if leading == 0 {
                        None
                    } else {
                        Some((
                            row,
                            if self.lines[row][0] == '\t' {
                                1
                            } else {
                                leading.min(INDENT_WIDTH)
                            },
                        ))
                    }
                }
            })
            .collect();
        if changes.is_empty() {
            return;
        }

        self.checkpoint();
        self.horizontal_scroll_hold = false;
        self.preferred_col = None;
        for (row, amount) in changes {
            if increase {
                self.lines[row].splice(0..0, std::iter::repeat_n(' ', amount));
            } else {
                self.lines[row].drain(0..amount);
            }
            if !self.visual_linewise {
                if self.cursor.row == row {
                    if increase {
                        self.cursor.col += amount;
                    } else {
                        self.cursor.col = self.cursor.col.saturating_sub(amount);
                    }
                }
                if self.mode == super::Mode::Visual && self.anchor.row == row {
                    if increase {
                        self.anchor.col += amount;
                    } else {
                        self.anchor.col = self.anchor.col.saturating_sub(amount);
                    }
                }
            }
        }
        self.clamp();
        self.message = if increase {
            "Indented line(s)".into()
        } else {
            "Unindented line(s)".into()
        };
    }

    pub fn move_line(&mut self, direction: isize) {
        if self.mode == super::Mode::Visual {
            self.move_visual_lines(direction);
            return;
        }

        let row = self.cursor.row;
        let Some(target) = row.checked_add_signed(direction) else {
            return;
        };
        if target >= self.lines.len() {
            return;
        }

        self.checkpoint();
        self.lines.swap(row, target);
        self.cursor.row = target;
        self.preferred_col = None;
        self.horizontal_scroll_hold = false;

        let previous_indent = (0..target)
            .rev()
            .find(|&previous| self.lines[previous].iter().any(|ch| !ch.is_whitespace()))
            .map(|previous| {
                self.lines[previous]
                    .iter()
                    .take_while(|ch| ch.is_whitespace())
                    .copied()
                    .collect::<Vec<_>>()
            });
        if let Some(previous_indent) = previous_indent {
            let line = &mut self.lines[target];
            let old_indent_len = line.iter().take_while(|ch| ch.is_whitespace()).count();
            if old_indent_len < line.len() {
                let content = line[old_indent_len..].to_vec();
                line.clear();
                line.extend(previous_indent.iter().copied());
                line.extend(content);
                self.cursor.col =
                    previous_indent.len() + self.cursor.col.saturating_sub(old_indent_len);
            }
        }

        self.clamp();
        self.message = if direction > 0 {
            "Moved line down"
        } else {
            "Moved line up"
        }
        .into();
    }

    fn move_visual_lines(&mut self, direction: isize) {
        let direction = direction.signum();
        if direction == 0 {
            return;
        }
        let (selection_start, selection_end) = self.selection();
        let start = selection_start.row;
        let end = selection_end.row;
        if (direction < 0 && start == 0) || (direction > 0 && end + 1 >= self.lines.len()) {
            return;
        }

        self.checkpoint();
        if direction > 0 {
            self.lines[start..=end + 1].rotate_right(1);
        } else {
            self.lines[start - 1..=end].rotate_left(1);
        }
        self.cursor.row = self.cursor.row.saturating_add_signed(direction);
        self.anchor.row = self.anchor.row.saturating_add_signed(direction);
        self.preferred_col = None;
        self.horizontal_scroll_hold = false;
        self.message = if direction > 0 {
            "Moved selected lines down"
        } else {
            "Moved selected lines up"
        }
        .into();

        let moved_start = start.saturating_add_signed(direction);
        let moved_end = end.saturating_add_signed(direction);
        let previous_indent = (0..moved_start)
            .rev()
            .find(|&row| self.lines[row].iter().any(|ch| !ch.is_whitespace()))
            .map(|row| {
                self.lines[row]
                    .iter()
                    .take_while(|ch| ch.is_whitespace())
                    .count()
            });
        let Some(previous_indent) = previous_indent else {
            self.clamp();
            return;
        };
        let Some(base_indent) = (moved_start..=moved_end)
            .find(|&row| self.lines[row].iter().any(|ch| !ch.is_whitespace()))
            .map(|row| {
                self.lines[row]
                    .iter()
                    .take_while(|ch| ch.is_whitespace())
                    .count()
            })
        else {
            self.clamp();
            return;
        };

        for row in moved_start..=moved_end {
            let line = &mut self.lines[row];
            let old_indent = line.iter().take_while(|ch| ch.is_whitespace()).count();
            if old_indent == line.len() {
                continue;
            }
            let adjustment = if previous_indent >= base_indent {
                let add = previous_indent - base_indent;
                line.splice(0..0, std::iter::repeat_n(' ', add));
                add as isize
            } else {
                let remove = old_indent.min(base_indent - previous_indent);
                line.drain(0..remove);
                -(remove as isize)
            };
            if !self.visual_linewise {
                if self.cursor.row == row {
                    self.cursor.col = self.cursor.col.saturating_add_signed(adjustment);
                }
                if self.anchor.row == row {
                    self.anchor.col = self.anchor.col.saturating_add_signed(adjustment);
                }
            }
        }

        self.clamp();
    }
}
