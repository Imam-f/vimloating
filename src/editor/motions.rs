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

    pub(super) fn previous_word_end(&mut self, n: usize) {
        for _ in 0..n {
            let mut pos = self.cursor;
            while let Some(previous) = self.prev_pos(pos) {
                let class = self.class(previous);
                let word_end = class != 0 && (previous.row != pos.row || class != self.class(pos));
                pos = previous;
                if word_end {
                    break;
                }
            }
            self.cursor = pos;
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
}
