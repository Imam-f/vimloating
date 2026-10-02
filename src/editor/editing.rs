use super::{Editor, InsertAction, RepeatChange};

const INDENT_WIDTH: usize = 4;

impl Editor {
    pub fn adjust_number(&mut self, delta: i128) -> bool {
        if self.mode != super::Mode::Normal || self.is_directory_browser() {
            return false;
        }
        let row = self.cursor.row;
        let line = &self.lines[row];
        let Some(mut digit_start) =
            (self.cursor.col.min(line.len())..line.len()).find(|&col| line[col].is_ascii_digit())
        else {
            self.message = "No number under cursor".into();
            return false;
        };
        while digit_start > 0 && line[digit_start - 1].is_ascii_digit() {
            digit_start -= 1;
        }
        let mut start = digit_start;
        if start > 0
            && matches!(line[start - 1], '+' | '-')
            && (start == 1 || !(line[start - 2].is_alphanumeric() || line[start - 2] == '_'))
        {
            start -= 1;
        }
        let mut end = digit_start;
        while end < line.len() && line[end].is_ascii_digit() {
            end += 1;
        }
        let original: String = line[start..end].iter().collect();
        let Ok(value) = original.parse::<i128>() else {
            self.message = "Number is too large to adjust".into();
            return false;
        };
        let Some(adjusted) = value.checked_add(delta) else {
            self.message = "Number is too large to adjust".into();
            return false;
        };
        let digit_width = end - digit_start;
        let mut digits = adjusted.unsigned_abs().to_string();
        if digits.len() < digit_width {
            digits.insert_str(0, &"0".repeat(digit_width - digits.len()));
        }
        let replacement = if adjusted < 0 {
            format!("-{digits}")
        } else {
            digits
        };

        if !self.replaying_change {
            self.last_change = Some(RepeatChange::Number(delta));
        }
        self.checkpoint();
        self.touch();
        self.horizontal_scroll_hold = false;
        self.preferred_col = None;
        self.lines[row].splice(start..end, replacement.chars());
        let replacement_len = replacement.chars().count();
        self.cursor.col = start
            + self
                .cursor
                .col
                .saturating_sub(start)
                .min(replacement_len.saturating_sub(1));
        self.clamp();
        self.message = if delta > 0 {
            "Incremented number".into()
        } else {
            "Decremented number".into()
        };
        true
    }

    pub fn begin_insert(&mut self, key: char) {
        self.horizontal_scroll_hold = false;
        if !self.replaying_change {
            self.insert_recording = Some((key, Vec::new()));
        }
        self.touch();
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
            self.record_insert_action(InsertAction::Character(ch));
            self.touch();
            self.lines[self.cursor.row].insert(self.cursor.col, ch);
            self.cursor.col += 1;
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        self.touch();
        let normalized = text.replace("\r\n", "\n");
        for ch in normalized.chars() {
            match ch {
                '\n' => {
                    self.record_insert_action(InsertAction::Newline);
                    let tail = self.lines[self.cursor.row].split_off(self.cursor.col);
                    self.cursor.row += 1;
                    self.cursor.col = 0;
                    self.lines.insert(self.cursor.row, tail);
                }
                '\t' => {
                    self.record_insert_action(InsertAction::Tab);
                    self.lines[self.cursor.row].insert(self.cursor.col, '\t');
                    self.cursor.col += 1;
                }
                _ => self.insert_char(ch),
            }
        }
    }

    pub fn newline(&mut self) {
        self.horizontal_scroll_hold = false;
        self.record_insert_action(InsertAction::Newline);
        self.touch();
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
        self.record_insert_action(InsertAction::Backspace);
        self.touch();
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
        self.record_insert_action(InsertAction::DeletePreviousWord);
        self.touch();
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
        self.record_insert_action(InsertAction::DeleteForward);
        self.touch();
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

        if !self.replaying_change {
            self.last_change = Some(RepeatChange::Indent(increase));
        }

        self.checkpoint();
        self.touch();
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
        self.touch();
        self.lines.swap(row, target);
        self.cursor.row = target;
        self.preferred_col = None;
        self.horizontal_scroll_hold = false;
        if !self.replaying_change {
            self.last_change = Some(RepeatChange::MoveLine(direction));
        }

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
        self.touch();
        if direction > 0 {
            self.lines[start..=end + 1].rotate_right(1);
        } else {
            self.lines[start - 1..=end].rotate_left(1);
        }
        if !self.replaying_change {
            self.last_change = Some(RepeatChange::MoveLine(direction));
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
