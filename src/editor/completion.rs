use super::{Editor, Mode, Pos, paths};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CompletionKind {
    Word,
    Line,
    Path,
}

impl CompletionKind {
    fn name(self) -> &'static str {
        match self {
            Self::Word => "Word",
            Self::Line => "Line",
            Self::Path => "Path",
        }
    }
}

pub(super) struct InsertCompletion {
    kind: CompletionKind,
    candidates: Vec<String>,
    index: usize,
    start: usize,
    cursor: Pos,
    revision: u64,
}

/// A borrowed view of the current matches shared by both frontends.
pub struct CompletionPopup<'a> {
    pub kind: &'static str,
    pub candidates: &'a [String],
    pub selected: usize,
    pub anchor: Pos,
}

pub struct CompletionPopupLayout {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub first: usize,
    pub visible: usize,
}

impl CompletionPopup<'_> {
    pub fn title(&self) -> String {
        format!(
            "{} {}/{}",
            self.kind,
            self.selected + 1,
            self.candidates.len()
        )
    }

    pub fn candidate_text(&self, index: usize, cols: usize) -> String {
        let candidate = &self.candidates[index];
        let prefix: Vec<_> = candidate.chars().take(cols.saturating_add(1)).collect();
        if prefix.len() <= cols {
            return prefix.into_iter().collect();
        }
        if cols == 0 {
            return String::new();
        }
        if matches!(self.kind, "Path" | "Command") {
            // Keep the filename visible when a long directory prefix is shared.
            let head = (cols - 1) / 2;
            let mut tail: Vec<_> = candidate.chars().rev().take(cols - 1 - head).collect();
            tail.reverse();
            prefix[..head]
                .iter()
                .chain(std::iter::once(&'…'))
                .chain(tail.iter())
                .collect()
        } else {
            prefix[..cols - 1]
                .iter()
                .chain(std::iter::once(&'…'))
                .collect()
        }
    }

    /// Place a bounded, scrolling list below the cursor, or above near the bottom.
    /// Coordinates and dimensions are in text cells, with a one-cell border.
    pub fn layout(
        &self,
        anchor_col: usize,
        cursor_row: usize,
        cols: usize,
        rows: usize,
    ) -> Option<CompletionPopupLayout> {
        if cols < 3 || self.candidates.is_empty() {
            return None;
        }
        let cursor_row = cursor_row.min(rows);
        let below = rows.saturating_sub(cursor_row + 1);
        let above = cursor_row;
        let desired = self.candidates.len().min(8) + 2;
        let use_below = below >= desired || (above < desired && below >= above);
        let available = if use_below { below } else { above };
        if available < 3 {
            return None;
        }
        let visible = self.candidates.len().min(8).min(available - 2);
        let height = visible + 2;
        let longest = self
            .candidates
            .iter()
            .map(|s| s.chars().take(46).count())
            .max()
            .unwrap_or(0);
        let width = (longest.max(self.title().chars().count()) + 2)
            .min(48)
            .min(cols)
            .max(3);
        Some(CompletionPopupLayout {
            x: anchor_col.min(cols - width),
            y: if use_below {
                cursor_row + 1
            } else {
                cursor_row - height
            },
            width,
            height,
            first: self
                .selected
                .saturating_sub(visible / 2)
                .min(self.candidates.len() - visible),
            visible,
        })
    }
}

fn keyword(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

impl Editor {
    pub fn completion_popup(&self) -> Option<CompletionPopup<'_>> {
        if self.mode == Mode::Insert {
            let cycle = self.insert_completion.as_ref()?;
            if cycle.cursor != self.cursor || cycle.revision != self.structural_revision {
                return None;
            }
            Some(CompletionPopup {
                kind: cycle.kind.name(),
                candidates: &cycle.candidates,
                selected: cycle.index,
                anchor: Pos {
                    row: self.cursor.row,
                    col: cycle.start,
                },
            })
        } else if self.mode == Mode::Command {
            let cycle = self.completion_cycle.as_ref()?;
            (self.prompt == cycle.last).then_some(CompletionPopup {
                kind: "Command",
                candidates: &cycle.candidates,
                selected: cycle.index,
                anchor: Pos::default(),
            })
        } else {
            None
        }
    }

    fn active_completion_kind(&self) -> Option<CompletionKind> {
        self.insert_completion
            .as_ref()
            .filter(|cycle| {
                cycle.cursor == self.cursor && cycle.revision == self.structural_revision
            })
            .map(|cycle| cycle.kind)
    }

    /// Returns true when an Insert-mode control key was handled by completion.
    pub fn insert_control_key(&mut self, key: char) -> bool {
        if self.mode != Mode::Insert {
            return false;
        }
        let pending = std::mem::take(&mut self.insert_completion_pending);
        let active = self.active_completion_kind();
        match key {
            'x' => {
                self.insert_completion = None;
                self.insert_completion_pending = true;
                self.message = "Completion · Ctrl+N/P words · Ctrl+L lines · Ctrl+F paths".into();
            }
            'f' if pending || active == Some(CompletionKind::Path) => self.complete_path(false),
            'l' if pending || active == Some(CompletionKind::Line) => self.complete_line(false),
            'n' | 'p' => {
                let kind = if pending {
                    CompletionKind::Word
                } else {
                    active.unwrap_or(CompletionKind::Word)
                };
                self.complete_insert(kind, key == 'p');
            }
            _ => return false,
        }
        true
    }

    pub fn complete_path(&mut self, backwards: bool) {
        self.complete_insert(CompletionKind::Path, backwards);
    }

    pub fn complete_word(&mut self, backwards: bool) {
        self.complete_insert(CompletionKind::Word, backwards);
    }

    pub fn complete_line(&mut self, backwards: bool) {
        self.complete_insert(CompletionKind::Line, backwards);
    }

    fn completion_candidates(&self, kind: CompletionKind) -> (usize, Vec<String>) {
        let line = &self.lines[self.cursor.row];
        let start = match kind {
            CompletionKind::Word => {
                let mut start = self.cursor.col;
                while start > 0 && keyword(line[start - 1]) {
                    start -= 1;
                }
                start
            }
            CompletionKind::Line => line
                .iter()
                .take(self.cursor.col)
                .take_while(|ch| ch.is_whitespace())
                .count(),
            CompletionKind::Path => self
                .cursor
                .col
                .checked_sub(1)
                .and_then(|col| paths::path_range(line, col))
                .map_or(self.cursor.col, |range| range.start),
        };
        let prefix: String = line[start..self.cursor.col].iter().collect();
        if kind == CompletionKind::Path {
            return (
                start,
                paths::path_completions(&prefix, &self.path_directory()),
            );
        }
        let mut candidates = BTreeSet::new();
        for (row, line) in self.lines.iter().enumerate() {
            match kind {
                CompletionKind::Word => {
                    let mut col = 0;
                    while col < line.len() {
                        if !keyword(line[col]) {
                            col += 1;
                            continue;
                        }
                        let word_start = col;
                        while col < line.len() && keyword(line[col]) {
                            col += 1;
                        }
                        // The word being completed must not suggest its own suffix.
                        if row == self.cursor.row && word_start == start {
                            continue;
                        }
                        let word: String = line[word_start..col].iter().collect();
                        if word != prefix && word.starts_with(&prefix) {
                            candidates.insert(word);
                        }
                    }
                }
                CompletionKind::Line if row != self.cursor.row => {
                    let candidate: String =
                        line.iter().skip_while(|ch| ch.is_whitespace()).collect();
                    if !candidate.is_empty()
                        && candidate != prefix
                        && candidate.starts_with(&prefix)
                    {
                        candidates.insert(candidate);
                    }
                }
                _ => {}
            }
        }
        (start, candidates.into_iter().collect())
    }

    fn complete_insert(&mut self, kind: CompletionKind, backwards: bool) {
        if self.mode != Mode::Insert || self.is_directory_browser() {
            return;
        }
        self.insert_completion_pending = false;
        let mut cycle = match self.insert_completion.take() {
            Some(mut cycle)
                if cycle.kind == kind
                    && cycle.cursor == self.cursor
                    && cycle.revision == self.structural_revision =>
            {
                cycle.index = if backwards {
                    (cycle.index + cycle.candidates.len() - 1) % cycle.candidates.len()
                } else {
                    (cycle.index + 1) % cycle.candidates.len()
                };
                cycle
            }
            _ => {
                let (start, candidates) = self.completion_candidates(kind);
                if candidates.is_empty() {
                    self.message = match kind {
                        CompletionKind::Path => "No file or path completions".into(),
                        _ => format!("No {} completions", kind.name().to_lowercase()),
                    };
                    return;
                }
                let index = if backwards { candidates.len() - 1 } else { 0 };
                InsertCompletion {
                    kind,
                    candidates,
                    index,
                    start,
                    cursor: self.cursor,
                    revision: self.structural_revision,
                }
            }
        };
        // Ordinary edits make all completion types part of Insert undo and dot-repeat.
        while self.cursor.col > cycle.start {
            self.backspace();
        }
        self.insert_text(&cycle.candidates[cycle.index]);
        cycle.cursor = self.cursor;
        cycle.revision = self.structural_revision;
        self.message = format!(
            "{} completion {}/{} · Ctrl+N / Ctrl+P to cycle",
            kind.name(),
            cycle.index + 1,
            cycle.candidates.len()
        );
        self.insert_completion = Some(cycle);
    }
}
