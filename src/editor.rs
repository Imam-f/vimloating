use crate::config::Theme;
use std::path::PathBuf;

pub mod buffers;
mod editing;
mod files;
mod normal;
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
    Search,
    ShellOutput,
    BufferList,
}

pub enum BufferAction {
    Open { path: PathBuf, replace: bool },
    Next,
    Previous,
    Select(String),
    List,
    Delete { target: Option<String>, force: bool },
}

struct CompletionCycle {
    candidates: Vec<String>,
    index: usize,
    last: String,
}

#[derive(Clone)]
struct Snapshot {
    lines: Vec<Vec<char>>,
    cursor: Pos,
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
    pub output_view: Option<String>,
    pub theme: Theme,
    pub buffer_action: Option<BufferAction>,
    pub top: usize,
    pub left: usize,
    horizontal_scroll_hold: bool,
    pub pending: Option<char>,
    pub count: String,
    pub quit: bool,
    pub force_quit: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    saved: String,
    register: Vec<Vec<char>>,
    linewise: bool,
    preferred_col: Option<usize>,
    directory_entries: Option<Vec<PathBuf>>,
    completion_cycle: Option<CompletionCycle>,
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
            output_view: None,
            theme: Theme::default(),
            buffer_action: None,
            top: 0,
            left: 0,
            horizontal_scroll_hold: false,
            pending: None,
            count: String::new(),
            quit: false,
            force_quit: false,
            undo: vec![],
            redo: vec![],
            saved: text,
            register: vec![],
            linewise: true,
            preferred_col: None,
            directory_entries: None,
            completion_cycle: None,
        }
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
        }
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, redo: bool) {
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
            self.clamp();
            self.message = if redo { "Redo" } else { "Undo" }.into();
        }
    }

    pub fn clamp(&mut self) {
        self.cursor.row = self.cursor.row.min(self.lines.len() - 1);
        let len = self.lines[self.cursor.row].len();
        let max = if self.mode == Mode::Insert {
            len
        } else {
            len.saturating_sub(1)
        };
        self.cursor.col = self.cursor.col.min(max);
    }

    pub fn escape(&mut self) {
        self.horizontal_scroll_hold = false;
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
        self.preferred_col = None;
        self.clamp();
    }

    pub fn move_by(&mut self, dx: isize, dy: isize, n: usize) {
        self.horizontal_scroll_hold = false;
        if dy != 0 {
            let col = *self.preferred_col.get_or_insert(self.cursor.col);
            self.cursor.row = self
                .cursor
                .row
                .saturating_add_signed(dy * n as isize)
                .min(self.lines.len() - 1);
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
        self.horizontal_scroll_hold = true;
    }

    pub fn follow_cursor_horizontally(&mut self) {
        self.horizontal_scroll_hold = false;
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
