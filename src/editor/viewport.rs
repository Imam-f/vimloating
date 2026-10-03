use super::{Editor, Pos, VerticalColumn};

impl Editor {
    /// Virtual indentation and usable text width of a rendered row.
    pub fn display_row_layout(
        &self,
        row: usize,
        start: usize,
        cols: usize,
        wrap: bool,
    ) -> (usize, usize) {
        let cols = cols.max(1);
        let indent = if wrap && start > 0 {
            self.lines[row]
                .iter()
                .take(cols.saturating_sub(1))
                .take_while(|ch| ch.is_whitespace())
                .count()
                .saturating_add(4)
                .min(cols - 1)
        } else {
            0
        };
        (indent, cols - indent)
    }

    /// Source column where a position's rendered row begins.
    pub fn display_segment_start(&self, pos: Pos, cols: usize, wrap: bool) -> usize {
        let cols = cols.max(1);
        if !wrap || pos.col < cols || self.folded_range(pos.row).is_some() {
            0
        } else {
            let (_, width) = self.display_row_layout(pos.row, cols, cols, true);
            cols + (pos.col - cols) / width * width
        }
    }

    /// Map a rendered text column back to the buffer, excluding virtual indent.
    pub fn position_at_display_column(
        &self,
        row: usize,
        start: usize,
        column: usize,
        cols: usize,
        wrap: bool,
    ) -> Pos {
        let (indent, width) = self.display_row_layout(row, start, cols, wrap);
        Pos {
            row,
            col: start
                + if wrap {
                    column.saturating_sub(indent).min(width - 1)
                } else {
                    column
                },
        }
    }

    #[cfg(test)]
    pub fn display_rows(&self, cols: usize, wrap: bool) -> Vec<(usize, usize)> {
        let cols = cols.max(1);
        let mut rows = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            if self.hidden_fold(row).is_some() {
                continue;
            }
            if self.folded_range(row).is_some() {
                rows.push((row, 0));
                continue;
            }
            let len = self.display_line_length(row);
            if !wrap || len == 0 {
                rows.push((row, 0));
            } else {
                let mut start = 0;
                while start < len {
                    rows.push((row, start));
                    start += self.display_row_layout(row, start, cols, true).1;
                }
                if self.mode == super::Mode::Insert && start == line.len() {
                    rows.push((row, start));
                }
            }
        }
        rows
    }

    fn line_segment_count(&self, row: usize, cols: usize, wrap: bool, insert_mode: bool) -> usize {
        if self.hidden_fold(row).is_some() {
            return 0;
        }
        if self.folded_range(row).is_some() {
            return 1;
        }
        let len = self.display_line_length(row);
        if !wrap || len == 0 {
            1
        } else {
            let len = len.saturating_add(usize::from(insert_mode));
            let (_, width) = self.display_row_layout(row, cols, cols, true);
            1 + len.saturating_sub(cols).div_ceil(width)
        }
    }

    fn display_line_length(&self, row: usize) -> usize {
        let len = self.lines[row].len();
        if self.mode == super::Mode::Visual && self.visual_blockwise {
            let (a, b) = self.selection();
            if a.row <= row && row <= b.row {
                return len.max(b.col + 1);
            }
        }
        len
    }

    fn ensure_display(&self, cols: usize, wrap: bool) {
        let cols = cols.max(1);
        let insert_mode = self.mode == super::Mode::Insert;
        let block_selection =
            (self.mode == super::Mode::Visual && self.visual_blockwise).then(|| self.selection());
        {
            let cache = self.display_cache.borrow();
            if cache.revision == self.structural_revision
                && cache.cols == cols
                && cache.wrap == wrap
                && cache.insert_mode == insert_mode
                && cache.line_count == self.lines.len()
                && cache.block_selection == block_selection
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
            block_selection,
            starts,
        };
    }

    /// Absolute display-row index of a cursor position, without scanning the whole buffer.
    pub fn display_index(&self, pos: Pos, cols: usize, wrap: bool) -> usize {
        let pos = if let Some(range) = self.hidden_fold(pos.row) {
            Pos {
                row: range.start,
                col: 0,
            }
        } else if self.folded_range(pos.row).is_some() {
            Pos { col: 0, ..pos }
        } else {
            pos
        };
        let cols = cols.max(1);
        self.ensure_display(cols, wrap);
        let cache = self.display_cache.borrow();
        let base = cache
            .starts
            .get(pos.row.min(self.lines.len()))
            .copied()
            .unwrap_or(0);
        if wrap && pos.col >= cols {
            let (_, width) = self.display_row_layout(pos.row, cols, cols, true);
            base + 1 + (pos.col - cols) / width
        } else {
            base
        }
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
            let segments = self.line_segment_count(row, cols, wrap, insert_mode);
            while segment < segments && out.len() < count {
                let start = if !wrap || self.display_line_length(row) == 0 || segment == 0 {
                    0
                } else {
                    let (_, width) = self.display_row_layout(row, cols, cols, true);
                    cols + (segment - 1) * width
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
        self.reveal_fold();
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

    pub(super) fn move_by_wrapped_rows(&mut self, direction: isize, amount: usize, cols: usize) {
        let cols = cols.max(1);
        self.insert_completion = None;
        self.insert_completion_pending = false;
        let start = self.display_segment_start(self.cursor, cols, true);
        let indent = self
            .display_row_layout(self.cursor.row, start, cols, true)
            .0;
        let column = match self.preferred_col {
            Some(VerticalColumn::Display(column)) => column,
            _ => indent + self.cursor.col.saturating_sub(start),
        };
        let index = self.display_index(self.cursor, cols, true);
        let total = self.display_total(cols, true);
        let target = if direction < 0 {
            index.saturating_sub(amount)
        } else {
            index.saturating_add(amount).min(total.saturating_sub(1))
        };
        if target != index
            && let Some(&(row, start)) = self.display_window(target, 1, cols, true).first()
        {
            self.cursor = self.position_at_display_column(row, start, column, cols, true);
            self.clamp();
        }
        self.preferred_col = Some(VerticalColumn::Display(column));
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
        let current_segment = self.display_segment_start(self.cursor, cols, wrap);
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
        let indent = self
            .display_row_layout(self.cursor.row, current_segment, cols, wrap)
            .0;
        let column_offset = indent + self.cursor.col.saturating_sub(current_segment);
        self.cursor = self.position_at_display_column(row, start, column_offset, cols, wrap);
        self.cursor.col = self.cursor.col.min(self.lines[row].len());
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
        let width = self.display_row_layout(row, start, cols, wrap).1;
        let end = (start + width).min(self.lines[row].len());
        let col = (start..end)
            .find(|&col| !self.lines[row][col].is_whitespace())
            .unwrap_or(start);

        self.cursor = Pos { row, col };
        self.preferred_col = None;
        self.horizontal_scroll_hold = false;
        self.clamp();
    }

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
            let current_start = self.display_segment_start(self.cursor, cols, wrap);
            let indent = self
                .display_row_layout(self.cursor.row, current_start, cols, wrap)
                .0;
            let column = indent + self.cursor.col.saturating_sub(current_start);
            self.cursor = self.position_at_display_column(row, start, column, cols, wrap);
            self.preferred_col = None;
            self.clamp();
        }
    }
}
