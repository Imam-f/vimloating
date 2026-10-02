use super::{Editor, Mode};

fn word_class(ch: char, big: bool) -> u8 {
    if ch.is_whitespace() {
        0
    } else if big || ch.is_alphanumeric() || ch == '_' {
        1
    } else {
        2
    }
}

/// (opening start, opening end exclusive, closing start, closing end exclusive).
fn tag_pairs(chars: &[(super::Pos, char)]) -> Vec<(usize, usize, usize, usize)> {
    let mut stack: Vec<(String, usize, usize)> = Vec::new();
    let mut pairs = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index].1 != '<' {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        let closing = chars.get(index).is_some_and(|entry| entry.1 == '/');
        if closing {
            index += 1;
        }
        let name_start = index;
        while index < chars.len()
            && (chars[index].1.is_alphanumeric() || matches!(chars[index].1, ':' | '-' | '_'))
        {
            index += 1;
        }
        let name: String = chars[name_start..index]
            .iter()
            .map(|entry| entry.1)
            .collect();
        let mut quote = None;
        while index < chars.len() {
            let ch = chars[index].1;
            if let Some(delimiter) = quote {
                if ch == delimiter {
                    quote = None;
                }
            } else if matches!(ch, '\'' | '"') {
                quote = Some(ch);
            } else if ch == '>' {
                break;
            }
            index += 1;
        }
        if index == chars.len() {
            break;
        }
        let self_closing = index > start && chars[index - 1].1 == '/';
        index += 1;
        if name.is_empty() || self_closing {
            continue;
        }
        if closing {
            if stack.last().is_some_and(|entry| entry.0 == name) {
                let (_, open, open_end) = stack.pop().unwrap();
                pairs.push((open, open_end, start, index));
            }
        } else if !matches!(
            name.as_str(),
            "area"
                | "base"
                | "br"
                | "col"
                | "embed"
                | "hr"
                | "img"
                | "input"
                | "link"
                | "meta"
                | "param"
                | "source"
                | "track"
                | "wbr"
        ) {
            stack.push((name, start, index));
        }
    }
    pairs
}

impl Editor {
    pub(super) fn select_text_object(&mut self, object: char, around: bool, count: usize) -> bool {
        let chars = self.source_chars();
        let Some(origin) = chars.iter().position(|entry| entry.0 == self.cursor) else {
            self.message = "No text object here".into();
            return false;
        };
        let (selected_a, selected_b) = if self.mode == Mode::Visual {
            self.selection()
        } else {
            (self.cursor, self.cursor)
        };
        let selected_start = chars
            .iter()
            .position(|entry| entry.0 == selected_a)
            .unwrap_or(origin);
        let selected_end = chars
            .iter()
            .position(|entry| entry.0 == selected_b)
            .unwrap_or(origin);
        let mut bounds = None;
        match object {
            'w' | 'W' => {
                let class = word_class(chars[origin].1, object == 'W');
                let mut start = origin;
                let mut end = origin + 1;
                while start > 0
                    && chars[start - 1].0.row == chars[origin].0.row
                    && word_class(chars[start - 1].1, object == 'W') == class
                {
                    start -= 1;
                }
                while end < chars.len()
                    && chars[end].0.row == chars[origin].0.row
                    && word_class(chars[end].1, object == 'W') == class
                {
                    end += 1;
                }
                if around && class == 0 && end < chars.len() {
                    let next_class = word_class(chars[end].1, object == 'W');
                    while end < chars.len() && word_class(chars[end].1, object == 'W') == next_class
                    {
                        end += 1;
                    }
                }
                for _ in 1..count {
                    while end < chars.len() && chars[end].1.is_whitespace() {
                        end += 1;
                    }
                    if end == chars.len() {
                        break;
                    }
                    let class = word_class(chars[end].1, object == 'W');
                    while end < chars.len() && word_class(chars[end].1, object == 'W') == class {
                        end += 1;
                    }
                }
                if around && class != 0 {
                    let before = end;
                    let end_row = chars[end - 1].0.row;
                    while end < chars.len()
                        && chars[end].0.row == end_row
                        && chars[end].1 != '\n'
                        && chars[end].1.is_whitespace()
                    {
                        end += 1;
                    }
                    if before == end {
                        while start > 0
                            && chars[start - 1].0.row == chars[origin].0.row
                            && chars[start - 1].1.is_whitespace()
                        {
                            start -= 1;
                        }
                    }
                }
                bounds = Some((start, end));
            }
            'b' | '(' | ')' | 'B' | '{' | '}' | '[' | ']' | '<' | '>' | 't' => {
                let mut pairs = if object == 't' {
                    tag_pairs(&chars)
                } else {
                    let open = match object {
                        'b' | '(' | ')' => '(',
                        'B' | '{' | '}' => '{',
                        '[' | ']' => '[',
                        _ => '<',
                    };
                    if open == '<' {
                        let mut stack = Vec::new();
                        let mut pairs = Vec::new();
                        for (i, &(_, ch)) in chars.iter().enumerate() {
                            if ch == '<' {
                                stack.push(i);
                            }
                            if ch == '>'
                                && let Some(start) = stack.pop()
                            {
                                pairs.push((start, start + 1, i, i + 1));
                            }
                        }
                        pairs
                    } else {
                        self.delimiter_pairs(&chars)
                            .into_iter()
                            .filter(|&(a, _)| chars[a].1 == open)
                            .map(|(a, b)| (a, a + 1, b, b + 1))
                            .collect()
                    }
                };
                pairs.retain(|&(a, _, _, b)| a <= selected_start && selected_end < b);
                pairs.sort_by_key(|&(a, _, _, b)| b - a);
                let mut candidates = Vec::new();
                for (a, inner_a, inner_b, b) in pairs {
                    let candidate = if around { (a, b) } else { (inner_a, inner_b) };
                    if candidate == (selected_start, selected_end + 1)
                        && selected_start != selected_end
                    {
                        continue;
                    }
                    candidates.push(candidate);
                }
                bounds = candidates.get(count.saturating_sub(1)).copied();
            }
            '\'' | '"' | '`' => {
                let row = self.cursor.row;
                let mut opening = None;
                let mut escaped = false;
                for (i, &(pos, ch)) in chars
                    .iter()
                    .enumerate()
                    .filter(|entry| entry.1.0.row == row)
                {
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if ch == '\\' {
                        escaped = true;
                        continue;
                    }
                    if ch == object {
                        if let Some(start) = opening.take() {
                            if start <= origin && origin <= i {
                                bounds = Some(if around {
                                    (start, i + 1)
                                } else {
                                    (start + 1, i)
                                });
                                break;
                            }
                        } else {
                            opening = Some(i);
                        }
                    }
                    let _ = pos;
                }
            }
            'p' => {
                let blank = |row: usize| self.lines[row].iter().all(|ch| ch.is_whitespace());
                let mut start = self.cursor.row;
                let mut end = start;
                let empty = blank(start);
                while start > 0 && blank(start - 1) == empty {
                    start -= 1;
                }
                while end + 1 < self.lines.len() && blank(end + 1) == empty {
                    end += 1;
                }
                for _ in 1..count {
                    while end + 1 < self.lines.len() && blank(end + 1) {
                        end += 1;
                    }
                    while end + 1 < self.lines.len() && !blank(end + 1) {
                        end += 1;
                    }
                }
                if around {
                    let before = end;
                    while end + 1 < self.lines.len() && blank(end + 1) {
                        end += 1;
                    }
                    if before == end {
                        while start > 0 && blank(start - 1) {
                            start -= 1;
                        }
                    }
                }
                bounds = Some((
                    chars.partition_point(|entry| entry.0.row < start),
                    chars.partition_point(|entry| entry.0.row <= end),
                ));
            }
            's' => {
                let mut starts = vec![0];
                for i in 0..chars.len() {
                    if matches!(chars[i].1, '.' | '?' | '!')
                        && chars.get(i + 1).is_none_or(|entry| entry.1.is_whitespace())
                    {
                        let mut next = i + 1;
                        while next < chars.len() && chars[next].1.is_whitespace() {
                            next += 1;
                        }
                        if next < chars.len() {
                            starts.push(next);
                        }
                    }
                }
                let index = starts
                    .partition_point(|&start| start <= origin)
                    .saturating_sub(1);
                let mut start = starts[index];
                while start < chars.len() && chars[start].1.is_whitespace() {
                    start += 1;
                }
                let mut end = starts.get(index + count).copied().unwrap_or(chars.len());
                if !around {
                    while end > start && chars[end - 1].1.is_whitespace() {
                        end -= 1;
                    }
                }
                bounds = Some((start, end));
            }
            _ => {}
        }
        if let Some((start, end)) = bounds
            && start < end
            && end <= chars.len()
        {
            self.anchor = chars[start].0;
            self.cursor = chars[end - 1].0;
            self.mode = Mode::Visual;
            self.visual_linewise = object == 'p';
            self.visual_blockwise = false;
            self.preferred_col = None;
            self.clamp();
            true
        } else {
            self.message = "No text object here".into();
            false
        }
    }
}
