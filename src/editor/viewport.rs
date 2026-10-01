use super::{Editor, Pos};

impl Editor {
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

    pub(super) fn word(&mut self, key: char, n: usize) {
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
                if self.mode == super::Mode::Insert && line.len() % cols == 0 {
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

    #[allow(dead_code)]
    pub fn scroll_vertical(
        &mut self,
        direction: isize,
        amount: usize,
        rows: usize,
        cols: usize,
        wrap: bool,
    ) {
        let display_rows = self.display_rows(cols, wrap);
        let max_top = display_rows.len().saturating_sub(1);
        self.top = self.top.min(max_top);
        self.top = if direction < 0 {
            self.top.saturating_sub(amount)
        } else {
            self.top.saturating_add(amount).min(max_top)
        };

        let cols = cols.max(1);
        let segment = if wrap {
            self.cursor.col / cols * cols
        } else {
            0
        };
        let cursor_index = display_rows
            .iter()
            .position(|&(row, start)| row == self.cursor.row && start == segment)
            .unwrap_or(0);
        let visible_rows = rows.max(1);
        let visible_end = self
            .top
            .saturating_add(visible_rows)
            .min(display_rows.len());
        let target_index = if cursor_index < self.top {
            Some(self.top)
        } else if cursor_index >= visible_end {
            Some(visible_end.saturating_sub(1))
        } else {
            None
        };

        if let Some(target_index) = target_index
            && let Some(&(row, start)) = display_rows.get(target_index)
        {
            let current_start = display_rows[cursor_index].1;
            self.cursor = Pos {
                row,
                col: start + self.cursor.col.saturating_sub(current_start),
            };
            self.preferred_col = None;
            self.clamp();
        }
    }

    pub fn reveal_cursor_after_motion(
        &mut self,
        previous_cursor: Pos,
        rows: usize,
        cols: usize,
        wrap: bool,
    ) {
        if self.cursor != previous_cursor {
            self.reveal_cursor(rows, cols, wrap);
        }
    }
}
