use super::{
    BufferAction, DisplayCache, Editor, InsertAction, Mode, Pos, RepeatChange, Snapshot,
    VerticalColumn, folds,
};
use crate::config::Theme;
use std::{cell::RefCell, path::PathBuf};

impl Editor {
    pub fn new(text: &str, path: Option<PathBuf>) -> Self {
        let text = text.replace("\r\n", "\n");
        Self {
            lines: text.split('\n').map(|s| s.chars().collect()).collect(),
            cursor: Pos::default(),
            mode: Mode::Normal,
            anchor: Pos::default(),
            visual_linewise: false,
            visual_blockwise: false,
            path,
            message: "Ready · :help for controls".into(),
            prompt: String::new(),
            search: String::new(),
            search_whole_word: false,
            search_backwards: false,
            search_prompt_backwards: false,
            char_find_highlight: None,
            char_find_hints: Vec::new(),
            output_view: None,
            theme: Theme::default(),
            buffer_action: None,
            top: 0,
            left: 0,
            horizontal_scroll_hold: false,
            pending: None,
            pending_operator: None,
            count: String::new(),
            marks: std::collections::HashMap::new(),
            fold_provider: Box::new(folds::IndentFoldProvider),
            closed_folds: Vec::new(),
            visible_folds: Vec::new(),
            command_history: Vec::new(),
            search_history: Vec::new(),
            picker_state: None,
            history_cursor: None,
            history_draft: String::new(),
            command_window: None,
            command_window_search: false,
            yank_highlight: None,
            quit: false,
            force_quit: false,
            undo: vec![],
            redo: vec![],
            saved: text,
            register: vec![],
            linewise: true,
            register_block_width: None,
            preferred_col: None,
            last_char_find: None,
            last_change: None,
            insert_recording: None,
            replaying_change: false,
            search_task: None,
            directory_entries: None,
            completion_cycle: None,
            insert_completion: None,
            insert_completion_pending: false,
            structural_revision: 0,
            display_cache: RefCell::new(DisplayCache {
                revision: u64::MAX,
                cols: 0,
                wrap: false,
                insert_mode: false,
                line_count: 0,
                block_selection: None,
                starts: Vec::new(),
            }),
        }
    }

    /// Marks the buffer contents as changed so the display-row index is rebuilt lazily.
    pub(super) fn touch(&mut self) {
        self.insert_completion = None;
        self.insert_completion_pending = false;
        self.yank_highlight = None;
        // Edits open folds so source ranges cannot become stale after line changes.
        self.closed_folds.clear();
        self.visible_folds.clear();
        self.structural_revision = self.structural_revision.wrapping_add(1);
    }

    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn dirty(&self) -> bool {
        self.text() != self.saved
    }

    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "untitled".into())
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            cursor: self.cursor,
            anchor: self.anchor,
        }
    }

    pub(super) fn checkpoint(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub(super) fn record_insert_action(&mut self, action: InsertAction) {
        if !self.replaying_change
            && let Some((_, actions)) = self.insert_recording.as_mut()
        {
            actions.push(action);
        }
    }

    pub fn undo(&mut self, redo: bool) {
        self.search_task = None;
        let state = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(state) = state {
            let current = self.snapshot();
            if redo {
                self.undo.push(current);
            } else {
                self.redo.push(current);
            }
            self.lines = state.lines;
            self.cursor = state.cursor;
            self.anchor = state.anchor;
            self.touch();
            self.clamp();
            self.message = if redo { "Redo" } else { "Undo" }.into();
        }
    }

    pub fn clamp(&mut self) {
        self.cursor.row = self.cursor.row.min(self.lines.len() - 1);
        self.reveal_fold();
        let len = self.lines[self.cursor.row].len();
        let max = if self.mode == Mode::Insert {
            len
        } else {
            len.saturating_sub(1)
        };
        if !(self.mode == Mode::Visual && self.visual_blockwise) {
            self.cursor.col = self.cursor.col.min(max);
        }
    }

    pub fn escape(&mut self) {
        self.command_window = None;
        self.picker_state = None;
        self.horizontal_scroll_hold = false;
        if !self.replaying_change
            && let Some((entry, actions)) = self.insert_recording.take()
        {
            self.last_change = Some(RepeatChange::Insert { entry, actions });
        }
        self.search_task = None;
        self.char_find_hints.clear();
        if matches!(self.mode, Mode::ShellOutput | Mode::BufferList) {
            self.output_view = None;
        }
        if self.mode == Mode::Insert {
            self.cursor.col = self.cursor.col.saturating_sub(1);
        }
        self.mode = Mode::Normal;
        self.visual_linewise = false;
        self.visual_blockwise = false;
        self.pending = None;
        self.pending_operator = None;
        self.count.clear();
        self.prompt.clear();
        self.completion_cycle = None;
        self.insert_completion = None;
        self.insert_completion_pending = false;
        self.history_cursor = None;
        self.preferred_col = None;
        self.clamp();
    }

    pub fn move_by(&mut self, dx: isize, dy: isize, n: usize) {
        self.insert_completion = None;
        self.insert_completion_pending = false;
        self.horizontal_scroll_hold = false;
        self.search_task = None;
        self.char_find_hints.clear();
        if dy != 0 {
            let col = match self.preferred_col {
                Some(VerticalColumn::Source(col)) => col,
                _ => self.cursor.col,
            };
            self.preferred_col = Some(VerticalColumn::Source(col));
            for _ in 0..n.saturating_mul(dy.unsigned_abs()) {
                let row = if dy > 0 {
                    self.folded_range(self.cursor.row)
                        .map_or(self.cursor.row + 1, |r| r.end + 1)
                } else {
                    self.cursor.row.saturating_sub(1)
                };
                let row = row.min(self.lines.len() - 1);
                self.cursor.row = self.hidden_fold(row).map_or(row, |r| r.start);
            }
            self.cursor.col = col;
        } else {
            self.preferred_col = None;
            self.cursor.col = self.cursor.col.saturating_add_signed(dx * n as isize);
        }
        self.clamp();
    }

    pub fn scroll_horizontal(&mut self, direction: isize, cols: usize) {
        let cols = cols.max(1);
        let max_left = self
            .lines
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0)
            .saturating_sub(cols);
        let amount = (cols / 2).max(1) as isize;
        self.left = self
            .left
            .saturating_add_signed(direction * amount)
            .min(max_left);
        if self.cursor.col < self.left || self.cursor.col >= self.left + cols {
            self.cursor.col = self.left;
            self.preferred_col = None;
            self.clamp();
        }
        self.horizontal_scroll_hold = true;
    }

    pub fn follow_cursor_horizontally(&mut self) {
        self.horizontal_scroll_hold = false;
    }

    pub fn cancel_search(&mut self) {
        self.search_task = None;
    }

    pub fn selection(&self) -> (Pos, Pos) {
        if self.visual_blockwise {
            return (
                Pos {
                    row: self.anchor.row.min(self.cursor.row),
                    col: self.anchor.col.min(self.cursor.col),
                },
                Pos {
                    row: self.anchor.row.max(self.cursor.row),
                    col: self.anchor.col.max(self.cursor.col),
                },
            );
        }
        (self.anchor.min(self.cursor), self.anchor.max(self.cursor))
    }

    pub fn selected_cell(&self, pos: Pos) -> bool {
        if self.mode != Mode::Visual {
            return false;
        }
        let (a, b) = self.selection();
        if self.visual_blockwise {
            a.row <= pos.row && pos.row <= b.row && a.col <= pos.col && pos.col <= b.col
        } else if self.visual_linewise {
            a.row <= pos.row && pos.row <= b.row
        } else {
            a <= pos && pos <= b
        }
    }

    pub fn is_directory_browser(&self) -> bool {
        self.directory_entries.is_some()
    }

    pub fn open_directory_entry(&mut self) {
        let Some(entries) = &self.directory_entries else {
            return;
        };
        let Some(path) = entries.get(self.cursor.row).cloned() else {
            return;
        };
        self.buffer_action = Some(BufferAction::Open {
            path,
            replace: false,
        });
    }

    pub fn show_buffer_list(&mut self, text: String) {
        self.output_view = Some(text);
        self.mode = Mode::BufferList;
        self.message = "Buffer list · Esc to close · :b id/name to switch".into();
    }
}
