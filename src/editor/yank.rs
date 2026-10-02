use super::{Editor, Pos};
use std::{ops::Range, time::Instant};

pub(super) struct YankHighlight {
    pub ranges: Vec<(usize, Range<usize>)>,
    pub linewise: bool,
    started: Instant,
}

impl YankHighlight {
    pub fn visible_at(&self, elapsed: f64) -> bool {
        (0.0..0.6).contains(&elapsed) && (elapsed / 0.1) as usize % 2 == 0
    }

    fn visible(&self) -> bool {
        self.visible_at(self.started.elapsed().as_secs_f64())
    }
}

impl Editor {
    pub fn yank_blink_active(&self) -> bool {
        self.yank_highlight
            .as_ref()
            .is_some_and(|highlight| highlight.started.elapsed().as_secs_f64() < 0.6)
    }

    pub(super) fn blink_yank(&mut self, ranges: Vec<(usize, Range<usize>)>, linewise: bool) {
        self.yank_highlight = Some(YankHighlight {
            ranges,
            linewise,
            started: Instant::now(),
        });
    }

    pub fn yank_blink_cell(&self, pos: Pos) -> bool {
        self.yank_highlight.as_ref().is_some_and(|highlight| {
            highlight.visible()
                && highlight
                    .ranges
                    .iter()
                    .any(|(row, cols)| *row == pos.row && cols.contains(&pos.col))
        })
    }

    pub fn yank_blink_line(&self, row: usize) -> bool {
        self.yank_highlight.as_ref().is_some_and(|highlight| {
            highlight.linewise
                && highlight.visible()
                && highlight
                    .ranges
                    .iter()
                    .any(|(source_row, _)| *source_row == row)
        })
    }
}
