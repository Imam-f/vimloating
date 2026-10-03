use super::{Editor, Pos, SearchTask};

enum SearchStep {
    Continue,
    Found(Pos),
    Done,
}

const SEARCH_STEPS_PER_FRAME: usize = 4096;

fn keyword(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

impl SearchTask {
    fn finish(&self) -> SearchStep {
        self.last_match.map_or(SearchStep::Done, SearchStep::Found)
    }

    fn wrap_or_finish(&mut self, line_count: usize) -> SearchStep {
        if self.wrapped {
            return self.finish();
        }
        self.wrapped = true;
        self.candidate = if self.backwards {
            Pos {
                row: line_count - 1,
                col: usize::MAX,
            }
        } else {
            Pos::default()
        };
        self.compare_offset = None;
        self.advance_candidate = false;
        SearchStep::Continue
    }

    fn skip_row(&mut self, line_count: usize) -> SearchStep {
        if self.backwards {
            if self.candidate.row == 0 {
                return self.wrap_or_finish(line_count);
            }
            self.candidate.row -= 1;
            self.candidate.col = usize::MAX;
        } else {
            self.candidate.row += 1;
            self.candidate.col = 0;
        }
        SearchStep::Continue
    }

    fn step(&mut self, lines: &[Vec<char>]) -> SearchStep {
        if let Some(offset) = self.compare_offset {
            if lines[self.candidate.row][self.candidate.col + offset] == self.needle[offset] {
                if offset + 1 == self.needle.len() {
                    let found = self.candidate;
                    self.last_match = Some(found);
                    if self.remaining > 1 {
                        self.remaining -= 1;
                        self.origin = found;
                        self.candidate = found;
                        self.wrapped = false;
                        self.compare_offset = None;
                        self.advance_candidate = true;
                        return SearchStep::Continue;
                    }
                    return SearchStep::Found(found);
                }
                self.compare_offset = Some(offset + 1);
            } else {
                self.compare_offset = None;
                self.advance_candidate = true;
            }
            return SearchStep::Continue;
        }

        if self.advance_candidate {
            self.advance_candidate = false;
            if self.backwards {
                if self.candidate.col > 0 {
                    self.candidate.col -= 1;
                } else if self.candidate.row > 0 {
                    self.candidate.row -= 1;
                    self.candidate.col = usize::MAX;
                } else {
                    return self.wrap_or_finish(lines.len());
                }
            } else {
                self.candidate.col = self.candidate.col.saturating_add(1);
            }
        }

        if self.candidate.row >= lines.len() {
            return self.wrap_or_finish(lines.len());
        }
        if self.wrapped
            && ((!self.backwards && self.candidate.row > self.origin.row)
                || (self.backwards && self.candidate.row < self.origin.row))
        {
            return self.finish();
        }

        let Some(max_start) = lines[self.candidate.row]
            .len()
            .checked_sub(self.needle.len())
        else {
            return self.skip_row(lines.len());
        };
        if self.backwards {
            self.candidate.col = self.candidate.col.min(max_start);
        } else if self.candidate.col > max_start {
            return self.skip_row(lines.len());
        }

        if self.wrapped && self.candidate.row == self.origin.row {
            let boundary = self.origin.col.min(max_start);
            if (!self.backwards && self.candidate.col > boundary)
                || (self.backwards && self.candidate.col < boundary)
            {
                return self.finish();
            }
        }

        let line = &lines[self.candidate.row];
        let start = self.candidate.col;
        let end = start + self.needle.len();
        if self.whole_word
            && ((start > 0 && keyword(line[start - 1]))
                || line.get(end).is_some_and(|&ch| keyword(ch)))
        {
            self.advance_candidate = true;
            return SearchStep::Continue;
        }
        self.compare_offset = Some(0);
        SearchStep::Continue
    }
}

impl Editor {
    pub fn find(&mut self, backwards: bool) {
        self.start_find(backwards, 1, true);
    }

    pub(super) fn search_cursor_word(&mut self, backwards: bool, count: usize) {
        let line = &self.lines[self.cursor.row];
        let Some(mut start) = (self.cursor.col..line.len()).find(|&col| keyword(line[col])) else {
            self.message = "No word under or after cursor".into();
            return;
        };
        while start > 0 && keyword(line[start - 1]) {
            start -= 1;
        }
        let mut end = start;
        while end < line.len() && keyword(line[end]) {
            end += 1;
        }
        self.search = line[start..end].iter().collect();
        self.search_whole_word = true;
        self.cursor.col = start;
        self.start_find(backwards, count, true);
    }

    pub(super) fn find_repeat(&mut self, backwards: bool, count: usize) {
        self.start_find(backwards, count, false);
    }

    fn start_find(&mut self, backwards: bool, count: usize, remember_direction: bool) {
        self.horizontal_scroll_hold = false;
        if remember_direction {
            self.search_backwards = backwards;
        }
        self.search_task = None;
        let needle: Vec<char> = self.search.chars().collect();
        if needle.is_empty() {
            return;
        }
        self.search_task = Some(SearchTask {
            needle,
            whole_word: self.search_whole_word,
            backwards,
            origin: self.cursor,
            candidate: self.cursor,
            wrapped: false,
            compare_offset: None,
            advance_candidate: true,
            remaining: count.max(1),
            last_match: None,
        });
    }

    pub fn advance_search(&mut self) {
        for _ in 0..SEARCH_STEPS_PER_FRAME {
            let Some(mut task) = self.search_task.take() else {
                return;
            };
            match task.step(&self.lines) {
                SearchStep::Continue => self.search_task = Some(task),
                SearchStep::Found(position) => {
                    self.cursor = position;
                    self.preferred_col = None;
                    self.char_find_highlight = None;
                    self.message = if task.backwards {
                        "Backward search match".into()
                    } else {
                        "Forward search match".into()
                    };
                    return;
                }
                SearchStep::Done => {
                    self.message = "Pattern not found".into();
                    return;
                }
            }
        }
    }
}
