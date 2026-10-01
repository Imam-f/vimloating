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

    pub(super) fn paragraph(&mut self, forward: bool, n: usize) {
        let empty_lines: Vec<_> = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.iter().all(|ch| ch.is_whitespace()))
            .map(|(row, _)| row)
            .collect();
        let mut row = self.cursor.row;
        for _ in 0..n {
            row = if forward {
                empty_lines
                    .iter()
                    .copied()
                    .find(|&empty| empty > row)
                    .unwrap_or(self.lines.len() - 1)
            } else {
                empty_lines
                    .iter()
                    .copied()
                    .rev()
                    .find(|&empty| empty < row)
                    .unwrap_or(0)
            };
        }
        self.cursor.row = row;
        self.cursor.col = self.lines[row]
            .iter()
            .position(|ch| !ch.is_whitespace())
            .unwrap_or(0);
        self.preferred_col = None;
        self.clamp();
    }

    pub(super) fn sentence(&mut self, forward: bool, n: usize) {
        let mut chars = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            chars.extend(
                line.iter()
                    .enumerate()
                    .map(|(col, &ch)| (Pos { row, col }, ch)),
            );
            if row + 1 < self.lines.len() {
                chars.push((
                    Pos {
                        row,
                        col: line.len(),
                    },
                    '\n',
                ));
            }
        }
        let mut starts = Vec::new();
        if let Some((pos, _)) = chars.iter().find(|(_, ch)| !ch.is_whitespace()) {
            starts.push(*pos);
        }
        for index in 0..chars.len() {
            if !matches!(chars[index].1, '.' | '!' | '?') {
                continue;
            }
            let mut next = index + 1;
            while next < chars.len() && matches!(chars[next].1, '\'' | '"' | ')' | ']' | '}') {
                next += 1;
            }
            if next == chars.len() || chars[next].1.is_whitespace() {
                while next < chars.len() && chars[next].1.is_whitespace() {
                    next += 1;
                }
                if let Some(&(pos, ch)) = chars.get(next)
                    && !ch.is_whitespace()
                    && starts.last() != Some(&pos)
                {
                    starts.push(pos);
                }
            }
        }

        let mut target = self.cursor;
        for _ in 0..n {
            target = if forward {
                starts
                    .iter()
                    .copied()
                    .find(|&start| start > target)
                    .or_else(|| chars.last().map(|(pos, _)| *pos))
                    .unwrap_or(target)
            } else {
                starts
                    .iter()
                    .copied()
                    .rev()
                    .find(|&start| start < target)
                    .or_else(|| starts.first().copied())
                    .unwrap_or(target)
            };
        }
        self.cursor = target;
        self.preferred_col = None;
        self.clamp();
    }

    #[cfg(test)]
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
                if self.mode == super::Mode::Insert && line.len().is_multiple_of(cols) {
                    rows.push((row, line.len()));
                }
            }
        }
        rows
    }

    fn line_segment_count(&self, row: usize, cols: usize, wrap: bool, insert_mode: bool) -> usize {
        let line = &self.lines[row];
        if !wrap || line.is_empty() {
            1
        } else {
            let mut count = line.len().div_ceil(cols);
            if insert_mode && line.len().is_multiple_of(cols) {
                count += 1;
            }
            count
        }
    }

    fn ensure_display(&self, cols: usize, wrap: bool) {
        let cols = cols.max(1);
        let insert_mode = self.mode == super::Mode::Insert;
        {
            let cache = self.display_cache.borrow();
            if cache.revision == self.structural_revision
                && cache.cols == cols
                && cache.wrap == wrap
                && cache.insert_mode == insert_mode
                && cache.line_count == self.lines.len()
            {
                return;
            }
        }
        let mut starts = Vec::with_capacity(self.lines.len() + 1);
        starts.push(0);
        let mut total = 0usize;
        for row in 0..self.lines.len() {
            total += self.line_segment_count(row, cols, wrap, insert_mode);
            starts.push(total);
        }
        *self.display_cache.borrow_mut() = super::DisplayCache {
            revision: self.structural_revision,
            cols,
            wrap,
            insert_mode,
            line_count: self.lines.len(),
            starts,
        };
    }

    /// Absolute display-row index of a cursor position, without scanning the whole buffer.
    pub fn display_index(&self, pos: Pos, cols: usize, wrap: bool) -> usize {
        let cols = cols.max(1);
        self.ensure_display(cols, wrap);
        let cache = self.display_cache.borrow();
        let base = cache
            .starts
            .get(pos.row.min(self.lines.len()))
            .copied()
            .unwrap_or(0);
        if wrap { base + pos.col / cols } else { base }
    }

    /// Total number of rendered display rows.
    pub fn display_total(&self, cols: usize, wrap: bool) -> usize {
        self.ensure_display(cols.max(1), wrap);
        let cache = self.display_cache.borrow();
        cache.starts.last().copied().unwrap_or(0)
    }

    /// Up to `count` display rows starting at absolute display index `top`.
    /// Only touches the lines intersecting that window, so cost is O(count), not O(lines).
    pub fn display_window(
        &self,
        top: usize,
        count: usize,
        cols: usize,
        wrap: bool,
    ) -> Vec<(usize, usize)> {
        let cols = cols.max(1);
        self.ensure_display(cols, wrap);
        let cache = self.display_cache.borrow();
        let mut out = Vec::with_capacity(count);
        if count == 0 || self.lines.is_empty() {
            return out;
        }
        let total = cache.starts.last().copied().unwrap_or(0);
        if top >= total {
            return out;
        }
        let insert_mode = cache.insert_mode;
        let mut row = cache
            .starts
            .partition_point(|&start| start <= top)
            .saturating_sub(1);
        let mut segment = top - cache.starts[row];
        while out.len() < count && row < self.lines.len() {
            let line = &self.lines[row];
            let segments = self.line_segment_count(row, cols, wrap, insert_mode);
            while segment < segments && out.len() < count {
                let start = if !wrap || line.is_empty() {
                    0
                } else {
                    segment * cols
                };
                out.push((row, start));
                segment += 1;
            }
            row += 1;
            segment = 0;
        }
        out
    }

    pub fn reveal_cursor(&mut self, rows: usize, cols: usize, wrap: bool) {
        let rows = rows.max(1);
        let cols = cols.max(1);
        let row_index = self.display_index(self.cursor, cols, wrap);
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

    pub fn center_cursor(&mut self, rows: usize, cols: usize, wrap: bool) {
        let rows = rows.max(1);
        let cols = cols.max(1);
        let row_index = self.display_index(self.cursor, cols, wrap);
        let max_top = self.display_total(cols, wrap).saturating_sub(rows);
        self.top = row_index.saturating_sub(rows / 2).min(max_top);
    }

    pub fn move_by_display_rows(
        &mut self,
        direction: isize,
        amount: usize,
        rows: usize,
        cols: usize,
        wrap: bool,
    ) {
        let cols = cols.max(1);
        let total = self.display_total(cols, wrap);
        let current_segment = if wrap {
            self.cursor.col / cols * cols
        } else {
            0
        };
        let current_index = self.display_index(self.cursor, cols, wrap);
        let target_index = if direction < 0 {
            current_index.saturating_sub(amount)
        } else {
            current_index
                .saturating_add(amount)
                .min(total.saturating_sub(1))
        };
        let (row, start) = self
            .display_window(target_index, 1, cols, wrap)
            .first()
            .copied()
            .unwrap_or((self.cursor.row, current_segment));
        let column_offset = self.cursor.col.saturating_sub(current_segment);
        self.cursor = Pos {
            row,
            col: start
                .saturating_add(column_offset)
                .min(self.lines[row].len()),
        };
        self.preferred_col = None;
        self.search_task = None;
        self.char_find_hints.clear();
        self.clamp();
        self.center_cursor(rows, cols, wrap);
    }

    pub fn move_to_screen_line(&mut self, offset: usize, cols: usize, wrap: bool) {
        self.search_task = None;
        self.char_find_hints.clear();
        let cols = cols.max(1);
        let total = self.display_total(cols, wrap);
        let index = self.top.saturating_add(offset).min(total.saturating_sub(1));
        let (row, start) = self
            .display_window(index, 1, cols, wrap)
            .first()
            .copied()
            .unwrap_or((0, 0));
        let end = (start + cols).min(self.lines[row].len());
        let col = (start..end)
            .find(|&col| !self.lines[row][col].is_whitespace())
            .unwrap_or(start);

        self.cursor = Pos { row, col };
        self.preferred_col = None;
        self.horizontal_scroll_hold = false;
        self.clamp();
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
        self.search_task = None;
        self.char_find_hints.clear();
        let cols = cols.max(1);
        let total = self.display_total(cols, wrap);
        let max_top = total.saturating_sub(1);
        self.top = self.top.min(max_top);
        self.top = if direction < 0 {
            self.top.saturating_sub(amount)
        } else {
            self.top.saturating_add(amount).min(max_top)
        };

        let segment = if wrap {
            self.cursor.col / cols * cols
        } else {
            0
        };
        let cursor_index = self.display_index(self.cursor, cols, wrap);
        let visible_rows = rows.max(1);
        let visible_end = self.top.saturating_add(visible_rows).min(total);
        let target_index = if cursor_index < self.top {
            Some(self.top)
        } else if cursor_index >= visible_end {
            Some(visible_end.saturating_sub(1))
        } else {
            None
        };

        if let Some(target_index) = target_index
            && let Some(&(row, start)) = self.display_window(target_index, 1, cols, wrap).first()
        {
            let current_start = if wrap {
                self.cursor.col / cols * cols
            } else {
                segment
            };
            self.cursor = Pos {
                row,
                col: start + self.cursor.col.saturating_sub(current_start),
            };
            self.preferred_col = None;
            self.clamp();
        }
    }
}
