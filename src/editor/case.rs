use super::{Editor, Mode, Pos, RepeatChange};

#[derive(Clone, Copy)]
pub(super) enum CaseChange {
    Toggle,
}

impl CaseChange {
    fn apply(self, ch: char) -> Vec<char> {
        match self {
            Self::Toggle if ch.is_uppercase() => ch.to_lowercase().collect(),
            Self::Toggle => ch.to_uppercase().collect(),
        }
    }
}

impl Editor {
    pub(super) fn change_case(&mut self, operation: CaseChange, count: usize, whole_lines: bool) {
        if self.is_directory_browser() {
            self.message = "Directory listing is read-only".into();
            return;
        }
        let visual = self.mode == Mode::Visual;
        let (a, b) = if visual {
            self.selection()
        } else {
            (
                self.cursor,
                Pos {
                    row: if whole_lines {
                        (self.cursor.row + count - 1).min(self.lines.len() - 1)
                    } else {
                        self.cursor.row
                    },
                    col: self.cursor.col.saturating_add(count - 1),
                },
            )
        };
        let mut replacements = Vec::new();
        let mut cursor_advance = 0;
        for row in a.row..=b.row {
            let line = &self.lines[row];
            let full = whole_lines || (visual && self.visual_linewise);
            let start = if full {
                0
            } else if !visual || self.visual_blockwise || row == a.row {
                a.col.min(line.len())
            } else {
                0
            };
            let end = if full {
                line.len()
            } else if !visual || self.visual_blockwise || row == b.row {
                (b.col + 1).min(line.len())
            } else {
                line.len()
            };
            let replacement: Vec<_> = line[start..end]
                .iter()
                .flat_map(|&ch| operation.apply(ch))
                .collect();
            if row == a.row {
                cursor_advance = replacement.len();
            }
            if replacement != line[start..end] {
                replacements.push((row, start, end, replacement));
            }
        }
        if !replacements.is_empty() {
            self.checkpoint();
            self.touch();
            for (row, start, end, replacement) in replacements {
                self.lines[row].splice(start..end, replacement);
            }
            if visual && !self.replaying_change {
                self.last_change = Some(RepeatChange::VisualCase {
                    linewise: self.visual_linewise,
                    blockwise: self.visual_blockwise,
                    row_delta: b.row - a.row,
                    col_delta: b.col as isize - a.col as isize,
                    operation,
                });
            } else if !visual {
                self.remember_normal_change(
                    count,
                    if whole_lines {
                        &['g', '~', '~']
                    } else {
                        &['~']
                    },
                );
            }
        }
        if visual {
            self.cursor = a;
            self.escape();
        } else if whole_lines {
            self.cursor.col = self.lines[a.row]
                .iter()
                .position(|ch| !ch.is_whitespace())
                .unwrap_or(0);
        } else {
            self.cursor.col = a.col.saturating_add(cursor_advance);
        }
        self.preferred_col = None;
        self.clamp();
    }
}
