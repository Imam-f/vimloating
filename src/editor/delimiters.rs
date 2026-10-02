use super::{Editor, Pos};

impl Editor {
    pub(super) fn source_chars(&self) -> Vec<(Pos, char)> {
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
        chars
    }

    /// Matched delimiter offsets, excluding quoted text and C-style comments.
    pub(super) fn delimiter_pairs(&self, chars: &[(Pos, char)]) -> Vec<(usize, usize)> {
        let mut stack = Vec::new();
        let mut pairs = Vec::new();
        let mut index = 0;
        while index < chars.len() {
            let ch = chars[index].1;
            let next = chars.get(index + 1).map(|entry| entry.1);
            if ch == '/' && next == Some('/') {
                while index < chars.len() && chars[index].1 != '\n' {
                    index += 1;
                }
                continue;
            }
            if ch == '/' && next == Some('*') {
                index += 2;
                let mut depth = 1;
                while index < chars.len() && depth > 0 {
                    let pair = (chars[index].1, chars.get(index + 1).map(|entry| entry.1));
                    if pair == ('/', Some('*')) {
                        depth += 1;
                        index += 2;
                    } else if pair == ('*', Some('/')) {
                        depth -= 1;
                        index += 2;
                    } else {
                        index += 1;
                    }
                }
                continue;
            }
            if matches!(ch, '"' | '\'' | '`') {
                let mut end = index + 1;
                while end < chars.len() {
                    if chars[end].1 == '\\' {
                        end += 2;
                        continue;
                    }
                    if chars[end].1 == ch {
                        break;
                    }
                    if ch == '\'' && chars[end].1 == '\n' {
                        break;
                    }
                    end += 1;
                }
                if end < chars.len() && chars[end].1 == ch {
                    index = end + 1;
                    continue;
                }
            }
            match ch {
                '(' | '[' | '{' => stack.push((index, ch)),
                ')' | ']' | '}' => {
                    let open = match ch {
                        ')' => '(',
                        ']' => '[',
                        _ => '{',
                    };
                    if stack.last().is_some_and(|&(_, ch)| ch == open) {
                        pairs.push((stack.pop().unwrap().0, index));
                    }
                }
                _ => {}
            }
            index += 1;
        }
        pairs
    }

    pub(super) fn matching_delimiter(&mut self) {
        let chars = self.source_chars();
        let pairs = self.delimiter_pairs(&chars);
        let target = chars.iter().position(|&(pos, ch)| {
            pos.row == self.cursor.row
                && pos.col >= self.cursor.col
                && matches!(ch, '(' | ')' | '[' | ']' | '{' | '}')
        });
        if let Some(target) = target {
            if let Some(&(a, b)) = pairs.iter().find(|&&(a, b)| a == target || b == target) {
                self.cursor = chars[if a == target { b } else { a }].0;
                self.preferred_col = None;
                self.clamp();
            } else {
                self.message = "No matching delimiter".into();
            }
        }
    }
}
