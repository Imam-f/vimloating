use super::Editor;

/// Inclusive source-line range, with a visible header at `start`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoldRange {
    pub start: usize,
    pub end: usize,
}

/// A syntax provider can return the same ranges without changing folding or rendering.
pub trait FoldProvider {
    fn ranges(&self, lines: &[Vec<char>]) -> Vec<FoldRange>;
}

pub struct IndentFoldProvider;

impl FoldProvider for IndentFoldProvider {
    fn ranges(&self, lines: &[Vec<char>]) -> Vec<FoldRange> {
        let mut ranges = Vec::new();
        let mut stack = Vec::new();
        let mut previous = None;
        for (row, line) in lines.iter().enumerate() {
            if line.iter().all(|ch| ch.is_whitespace()) {
                continue;
            }
            let indent = line
                .iter()
                .take_while(|ch| ch.is_whitespace())
                .map(|ch| if *ch == '\t' { 4 } else { 1 })
                .sum::<usize>();
            while stack.last().is_some_and(|&(_, level)| indent <= level) {
                let (start, _) = stack.pop().unwrap();
                ranges.push(FoldRange {
                    start,
                    end: row - 1,
                });
            }
            if let Some((start, level)) = previous
                && indent > level
            {
                stack.push((start, level));
            }
            previous = Some((row, indent));
        }
        for (start, _) in stack {
            ranges.push(FoldRange {
                start,
                end: lines.len() - 1,
            });
        }
        ranges.sort_by_key(|range| (range.start, range.end));
        ranges
    }
}

impl Editor {
    /// Replacing the provider opens existing folds; no syntax engine is required.
    pub fn set_fold_provider(&mut self, provider: Box<dyn FoldProvider>) {
        self.fold_provider = provider;
        self.closed_folds.clear();
        self.visible_folds.clear();
        self.structural_revision = self.structural_revision.wrapping_add(1);
    }

    pub fn folded_range(&self, row: usize) -> Option<FoldRange> {
        let index = self
            .visible_folds
            .partition_point(|range| range.start < row);
        self.visible_folds
            .get(index)
            .copied()
            .filter(|range| range.start == row)
    }

    pub(super) fn hidden_fold(&self, row: usize) -> Option<FoldRange> {
        let index = self
            .visible_folds
            .partition_point(|range| range.start < row)
            .checked_sub(1)?;
        self.visible_folds
            .get(index)
            .copied()
            .filter(|range| row <= range.end)
    }

    fn index_closed_folds(&mut self) {
        self.closed_folds
            .retain(|range| range.start < range.end && range.end < self.lines.len());
        self.closed_folds
            .sort_by_key(|range| (range.start, std::cmp::Reverse(range.end)));
        self.closed_folds.dedup();
        self.visible_folds.clear();
        for &range in &self.closed_folds {
            if self
                .visible_folds
                .last()
                .is_none_or(|outer| outer.end < range.start)
            {
                self.visible_folds.push(range);
            }
        }
    }

    pub(super) fn reveal_fold(&mut self) {
        let row = self.cursor.row;
        let before = self.closed_folds.len();
        self.closed_folds
            .retain(|range| !(range.start < row && row <= range.end));
        if before != self.closed_folds.len() {
            self.index_closed_folds();
            self.structural_revision = self.structural_revision.wrapping_add(1);
        }
    }

    pub(super) fn fold_command(&mut self, key: char) {
        let ranges = self.fold_provider.ranges(&self.lines);
        let row = self.cursor.row;
        let current = self
            .closed_folds
            .iter()
            .copied()
            .filter(|range| range.start <= row && row <= range.end)
            .max_by_key(|range| range.end - range.start);
        match key {
            'R' => self.closed_folds.clear(),
            'M' => {
                self.closed_folds = ranges;
            }
            'o' | 'a' if current.is_some() => {
                let range = current.unwrap();
                self.closed_folds.retain(|closed| *closed != range);
            }
            'c' | 'a' => {
                if let Some(range) = ranges
                    .into_iter()
                    .filter(|range| {
                        range.start <= row && row <= range.end && !self.closed_folds.contains(range)
                    })
                    .min_by_key(|range| range.end - range.start)
                {
                    self.closed_folds.push(range);
                    self.cursor.row = range.start;
                } else {
                    self.message = "No indentation fold here".into();
                }
            }
            _ => return,
        }
        self.index_closed_folds();
        if let Some(range) = self.hidden_fold(self.cursor.row) {
            self.cursor.row = range.start;
        }
        if self.folded_range(self.cursor.row).is_some() {
            self.cursor.col = 0;
        }
        self.structural_revision = self.structural_revision.wrapping_add(1);
        self.clamp();
    }
}
