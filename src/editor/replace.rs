use super::{Editor, Mode};

impl Editor {
    pub(super) fn replace_char(&mut self, ch: char, count: usize) {
        if ch.is_control() && !matches!(ch, '\n' | '\t') {
            return;
        }
        if self.mode == Mode::Visual {
            if ch == '\n' {
                return;
            }
            self.checkpoint();
            self.touch();
            let (a, b) = self.selection();
            for row in a.row..=b.row {
                let len = self.lines[row].len();
                let start = if self.visual_linewise || row != a.row {
                    0
                } else {
                    a.col.min(len)
                };
                let end = if self.visual_linewise || row != b.row {
                    len
                } else {
                    (b.col + 1).min(len)
                };
                self.lines[row][start..end].fill(ch);
            }
            self.cursor = a;
            self.escape();
            return;
        }
        let row = self.cursor.row;
        let col = self.cursor.col;
        if self.lines[row].len().saturating_sub(col) < count {
            self.message = "Not enough characters to replace".into();
            return;
        }
        self.remember_normal_change(count, &['r', ch]);
        self.checkpoint();
        self.touch();
        if ch == '\n' {
            let tail = self.lines[row].split_off(col + count);
            self.lines[row].truncate(col);
            self.lines.insert(row + 1, tail);
            self.cursor.row += 1;
            self.cursor.col = 0;
        } else {
            self.lines[row][col..col + count].fill(ch);
            self.cursor.col += count - 1;
        }
        self.clamp();
    }
}
