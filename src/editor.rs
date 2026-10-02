use crate::config::Theme;
use std::cell::RefCell;
use std::path::PathBuf;

pub mod buffers;
mod editing;
mod files;
pub mod folds;
mod history;
mod normal;
mod replace;
mod viewport;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    Command,
    CommandWindow,
    Search,
    ShellOutput,
    BufferList,
}

pub enum BufferAction {
    Open { path: PathBuf, replace: bool },
    Next,
    Previous,
    Last,
    Select(String),
    List,
    Delete { target: Option<String>, force: bool },
}

struct CompletionCycle {
    candidates: Vec<String>,
    index: usize,
    last: String,
}

#[derive(Clone, Copy)]
struct CharFind {
    target: char,
    direction: isize,
    till: bool,
}

#[derive(Clone)]
pub(super) enum InsertAction {
    Character(char),
    Newline,
    Backspace,
    DeleteForward,
    DeletePreviousWord,
    Tab,
}

#[derive(Clone)]
enum RepeatChange {
    Normal(Vec<char>),
    Insert {
        entry: char,
        actions: Vec<InsertAction>,
    },
    Indent(bool),
    MoveLine(isize),
    Number(i128),
    VisualDelete {
        linewise: bool,
        row_delta: usize,
        col_delta: isize,
    },
}

struct DisplayCache {
    revision: u64,
    cols: usize,
    wrap: bool,
    insert_mode: bool,
    line_count: usize,
    /// Absolute display row index where each source line begins; one extra sentinel
    /// entry holds the total display row count.
    starts: Vec<usize>,
}

struct SearchTask {
    needle: Vec<char>,
    backwards: bool,
    origin: Pos,
    candidate: Pos,
    wrapped: bool,
    compare_offset: Option<usize>,
    advance_candidate: bool,
    remaining: usize,
    last_match: Option<Pos>,
}

#[derive(Clone)]
struct Snapshot {
    lines: Vec<Vec<char>>,
    cursor: Pos,
    anchor: Pos,
}

pub struct Editor {
    pub lines: Vec<Vec<char>>,
    pub cursor: Pos,
    pub mode: Mode,
    pub anchor: Pos,
    pub visual_linewise: bool,
    pub path: Option<PathBuf>,
    pub message: String,
    pub prompt: String,
    pub search: String,
    pub search_backwards: bool,
    pub search_prompt_backwards: bool,
    pub char_find_highlight: Option<Pos>,
    pub char_find_hints: Vec<Pos>,
    pub output_view: Option<String>,
    pub theme: Theme,
    pub buffer_action: Option<BufferAction>,
    pub top: usize,
    pub left: usize,
    horizontal_scroll_hold: bool,
    pub pending: Option<char>,
    pub count: String,
    marks: std::collections::HashMap<char, Pos>,
    fold_provider: Box<dyn folds::FoldProvider>,
    closed_folds: Vec<folds::FoldRange>,
    command_history: Vec<String>,
    search_history: Vec<String>,
    history_cursor: Option<usize>,
    history_draft: String,
    pub command_window: Option<Box<Editor>>,
    command_window_search: bool,
    pub quit: bool,
    pub force_quit: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    saved: String,
    register: Vec<Vec<char>>,
    linewise: bool,
    preferred_col: Option<usize>,
    last_char_find: Option<CharFind>,
    last_change: Option<RepeatChange>,
    insert_recording: Option<(char, Vec<InsertAction>)>,
    replaying_change: bool,
    search_task: Option<SearchTask>,
    directory_entries: Option<Vec<PathBuf>>,
    completion_cycle: Option<CompletionCycle>,
    structural_revision: u64,
    display_cache: RefCell<DisplayCache>,
}

impl Editor {
    pub fn new(text: &str, path: Option<PathBuf>) -> Self {
        let text = text.replace("\r\n", "\n");
        Self {
            lines: text.split('\n').map(|s| s.chars().collect()).collect(),
            cursor: Pos::default(),
            mode: Mode::Normal,
            anchor: Pos::default(),
            visual_linewise: false,
            path,
            message: "Ready · :help for controls".into(),
            prompt: String::new(),
            search: String::new(),
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
            count: String::new(),
            marks: std::collections::HashMap::new(),
            fold_provider: Box::new(folds::IndentFoldProvider),
            closed_folds: Vec::new(),
            command_history: Vec::new(),
            search_history: Vec::new(),
            history_cursor: None,
            history_draft: String::new(),
            command_window: None,
            command_window_search: false,
            quit: false,
            force_quit: false,
            undo: vec![],
            redo: vec![],
            saved: text,
            register: vec![],
            linewise: true,
            preferred_col: None,
            last_char_find: None,
            last_change: None,
            insert_recording: None,
            replaying_change: false,
            search_task: None,
            directory_entries: None,
            completion_cycle: None,
            structural_revision: 0,
            display_cache: RefCell::new(DisplayCache {
                revision: u64::MAX,
                cols: 0,
                wrap: false,
                insert_mode: false,
                line_count: 0,
                starts: Vec::new(),
            }),
        }
    }

    /// Marks the buffer contents as changed so the display-row index is rebuilt lazily.
    pub(super) fn touch(&mut self) {
        // Edits open folds so source ranges cannot become stale after line changes.
        self.closed_folds.clear();
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

    fn checkpoint(&mut self) {
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
        self.cursor.col = self.cursor.col.min(max);
    }

    pub fn escape(&mut self) {
        self.command_window = None;
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
        self.pending = None;
        self.count.clear();
        self.prompt.clear();
        self.completion_cycle = None;
        self.history_cursor = None;
        self.preferred_col = None;
        self.clamp();
    }

    pub fn move_by(&mut self, dx: isize, dy: isize, n: usize) {
        self.horizontal_scroll_hold = false;
        self.search_task = None;
        self.char_find_hints.clear();
        if dy != 0 {
            let col = *self.preferred_col.get_or_insert(self.cursor.col);
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
        (self.anchor.min(self.cursor), self.anchor.max(self.cursor))
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

#[cfg(test)]
mod tests;
