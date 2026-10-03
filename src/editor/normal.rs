use super::{BufferAction, Editor, Mode, Pos, operators::TextOperator};

impl Editor {
    pub fn normal_key(&mut self, key: char) {
        self.normal_key_with_viewport(key, 1, false);
    }

    /// Process a Vim key with the frontend's current wrapping layout.
    pub fn normal_key_with_viewport(&mut self, key: char, cols: usize, wrap: bool) {
        self.horizontal_scroll_hold = false;
        self.char_find_highlight = None;
        self.char_find_hints.clear();
        self.search_task = None;
        if let Some(command @ ('m' | '\'' | '`')) = self.pending {
            self.pending = None;
            self.count.clear();
            if !key.is_ascii_alphabetic() {
                self.message = "Marks use letters a-z or A-Z".into();
            } else if command == 'm' {
                self.marks.insert(key, self.cursor);
                self.message = format!("Mark {key} set");
            } else if let Some(&position) = self.marks.get(&key) {
                self.cursor = position;
                self.clamp();
                if command == '\'' {
                    self.cursor.col = self.lines[self.cursor.row]
                        .iter()
                        .position(|ch| !ch.is_whitespace())
                        .unwrap_or(0);
                }
                self.preferred_col = None;
            } else {
                self.message = format!("Mark {key} is not set");
            }
            return;
        }
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
                'i' | 'a'
                    | 'I'
                    | 'A'
                    | 'o'
                    | 'O'
                    | 'x'
                    | 'D'
                    | 'd'
                    | 'y'
                    | 'v'
                    | 'V'
                    | 'p'
                    | 'P'
                    | 'r'
            ) {
                self.message = "Directory listing is read-only · Enter opens entries".into();
                self.pending = None;
                self.pending_operator = None;
                self.count.clear();
                return;
            }
        }
        if self.pending == Some('r') {
            let n = self.count.parse::<usize>().unwrap_or(1).clamp(1, 10000);
            self.pending = None;
            self.count.clear();
            self.replace_char(key, n);
            return;
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
            if let Some((operator, operator_count)) = self.pending_operator.take() {
                let total = operator_count.saturating_mul(n).min(10000);
                if matches!(pending, 'i' | 'a') && self.mode == Mode::Normal {
                    self.apply_text_object_operator(operator, pending == 'a', key, total);
                } else if key == operator.line_key() {
                    self.apply_line_operator(operator, total);
                } else if matches!(key, 'i' | 'a') {
                    self.pending = Some(key);
                    self.pending_operator = Some((operator, total));
                }
                return;
            }
            if pending == 'g' && matches!(key, 'u' | 'U') {
                let operation = if key == 'u' {
                    super::case::CaseChange::Lower
                } else {
                    super::case::CaseChange::Upper
                };
                if self.mode == Mode::Visual {
                    self.change_case(operation, n, false);
                } else {
                    self.pending = Some(key);
                    self.pending_operator = Some((TextOperator::Case(operation), n));
                }
                return;
            }
            if pending == 'g' && key == '~' {
                if self.mode == Mode::Visual {
                    self.change_case(super::case::CaseChange::Toggle, n, false);
                } else {
                    self.pending = Some('~');
                    self.pending_operator =
                        Some((TextOperator::Case(super::case::CaseChange::Toggle), n));
                }
                return;
            }
            if self.mode == Mode::Visual && matches!(pending, 'i' | 'a') {
                self.select_text_object(key, pending == 'a', n);
                return;
            }
            if pending == 'z' {
                self.fold_command(key);
                return;
            }
            if pending == 'g' && key == 'g' {
                self.cursor = Pos {
                    row: n.saturating_sub(1).min(self.lines.len() - 1),
                    col: 0,
                };
                self.clamp();
            }
            if pending == 'g' && matches!(key, 'j' | 'k') {
                self.move_by(0, if key == 'j' { 1 } else { -1 }, n);
            }
            if pending == 'g' && key == 'e' {
                self.previous_word_end(n);
            }
            if pending == 'g' && matches!(key, 'f' | 'F') && self.mode == Mode::Normal {
                self.open_cursor_path();
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
            '*' | '#' => self.search_cursor_word(key == '#', n),
            '~' => {
                self.change_case(super::case::CaseChange::Toggle, n, false);
            }
            '%' => self.matching_delimiter(),
            'h' => self.move_by(-1, 0, n),
            'j' | 'k' => {
                let direction = if key == 'j' { 1 } else { -1 };
                if wrap {
                    self.move_by_wrapped_rows(direction, n, cols);
                } else {
                    self.move_by(0, direction, n);
                }
            }
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
            'd' | 'y' => {
                self.pending = Some(key);
                self.pending_operator = Some((
                    if key == 'd' {
                        TextOperator::Delete
                    } else {
                        TextOperator::Yank
                    },
                    n,
                ));
                self.count.clear();
                return;
            }
            'g' | 'z' | 'm' | '\'' | '`' | 'r' => {
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
            'i' | 'a' if self.mode == Mode::Visual => {
                self.pending = Some(key);
                return;
            }
            'i' | 'a' | 'I' | 'A' | 'o' | 'O' => self.begin_insert(key),
            'x' => {
                self.register_block_width = None;
                self.remember_normal_change(n, &['x']);
                self.checkpoint();
                self.touch();
                let row = self.cursor.row;
                let end = (self.cursor.col + n).min(self.lines[row].len());
                self.register = vec![self.lines[row].drain(self.cursor.col..end).collect()];
                self.linewise = false;
                self.clamp();
            }
            'X' => {
                self.register_block_width = None;
                if self.cursor.col > 0 {
                    self.remember_normal_change(n, &['X']);
                    self.checkpoint();
                    self.touch();
                    let row = self.cursor.row;
                    let start = self.cursor.col.saturating_sub(n);
                    self.register = vec![self.lines[row].drain(start..self.cursor.col).collect()];
                    self.linewise = false;
                    self.cursor.col = start;
                    self.clamp();
                }
            }
            'D' => {
                self.register_block_width = None;
                self.remember_normal_change(n, &['D']);
                self.checkpoint();
                self.touch();
                self.register = vec![self.lines[self.cursor.row].split_off(self.cursor.col)];
                self.linewise = false;
                self.clamp();
            }
            'p' | 'P' => {
                if !self.register.is_empty() {
                    self.remember_normal_change(n, &[key]);
                }
                self.paste(key == 'P', n);
            }
            'u' => {
                for _ in 0..n {
                    self.undo(false);
                }
            }
            'v' => {
                if self.mode == Mode::Visual && !self.visual_linewise && !self.visual_blockwise {
                    self.escape();
                } else {
                    self.mode = Mode::Visual;
                    self.anchor = self.cursor;
                    self.visual_linewise = false;
                    self.visual_blockwise = false;
                }
            }
            'V' => {
                if self.mode == Mode::Visual && self.visual_linewise {
                    self.escape();
                } else {
                    self.mode = Mode::Visual;
                    self.anchor = self.cursor;
                    self.visual_linewise = true;
                    self.visual_blockwise = false;
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
            ',' => self.repeat_char_find_opposite(n),
            '.' => self.repeat_last_change(n),
            _ => {}
        }
        self.count.clear();
    }
}
