use super::{Editor, Pos, RepeatChange};

impl Editor {
    pub(super) fn visual_action(&mut self, delete: bool) {
        self.touch();
        let (a, b) = self.selection();
        if !delete {
            let ranges = (a.row..=b.row)
                .map(|row| {
                    let start = if self.visual_linewise || row != a.row {
                        0
                    } else {
                        a.col
                    };
                    let end = if self.visual_linewise || row != b.row {
                        self.lines[row].len().max(1)
                    } else {
                        b.col + 1
                    };
                    (row, start..end)
                })
                .collect();
            self.blink_yank(ranges, self.visual_linewise);
        }
        if delete && !self.replaying_change {
            self.last_change = Some(RepeatChange::VisualDelete {
                linewise: self.visual_linewise,
                blockwise: self.visual_blockwise,
                row_delta: b.row - a.row,
                col_delta: b.col as isize - a.col as isize,
            });
        }
        if self.visual_blockwise {
            self.block_action(delete);
            return;
        }
        self.register.clear();
        self.register_block_width = None;
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

    pub(super) fn line_action(&mut self, delete: bool, n: usize) {
        self.touch();
        let end = (self.cursor.row + n).min(self.lines.len());
        if !delete {
            let ranges = (self.cursor.row..end)
                .map(|row| (row, 0..self.lines[row].len().max(1)))
                .collect();
            self.blink_yank(ranges, true);
        }
        self.register = self.lines[self.cursor.row..end].to_vec();
        self.register_block_width = None;
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

    pub(super) fn paste(&mut self, before: bool, n: usize) {
        if self.register.is_empty() {
            return;
        }
        self.checkpoint();
        self.touch();
        if let Some(width) = self.register_block_width {
            self.paste_block(before, n, width);
            self.clamp();
            return;
        }
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
}
