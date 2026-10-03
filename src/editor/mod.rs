use crate::config::Theme;
use std::cell::RefCell;
use std::path::PathBuf;

mod block;
pub mod buffers;
mod case;
mod char_find;
mod commands;
mod completion;
mod core;
pub use completion::{CompletionPopup, CompletionPopupLayout};
mod delimiters;
mod editing;
mod files;
pub mod folds;
mod history;
mod motions;
mod normal;
mod operators;
mod paths;
mod registers;
mod repeat;
mod replace;
mod search;
mod shell;
mod text_objects;
mod viewport;
mod yank;

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
    Open {
        path: PathBuf,
        replace: bool,
    },
    OpenAddress {
        path: PathBuf,
        position: Option<Pos>,
    },
    Next,
    Previous,
    Last,
    Select(String),
    List,
    Delete {
        target: Option<String>,
        force: bool,
    },
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
    MoveBack,
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
    VisualCase {
        linewise: bool,
        blockwise: bool,
        row_delta: usize,
        col_delta: isize,
        operation: case::CaseChange,
    },
    VisualDelete {
        linewise: bool,
        blockwise: bool,
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
    block_selection: Option<(Pos, Pos)>,
    /// Absolute display row index where each source line begins; one extra sentinel
    /// entry holds the total display row count.
    starts: Vec<usize>,
}

struct SearchTask {
    needle: Vec<char>,
    whole_word: bool,
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
    pub visual_blockwise: bool,
    pub path: Option<PathBuf>,
    pub message: String,
    pub prompt: String,
    pub search: String,
    pub search_whole_word: bool,
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
    pending_operator: Option<(operators::TextOperator, usize)>,
    pub count: String,
    marks: std::collections::HashMap<char, Pos>,
    fold_provider: Box<dyn folds::FoldProvider>,
    closed_folds: Vec<folds::FoldRange>,
    visible_folds: Vec<folds::FoldRange>,
    command_history: Vec<String>,
    search_history: Vec<String>,
    history_cursor: Option<usize>,
    history_draft: String,
    pub command_window: Option<Box<Editor>>,
    command_window_search: bool,
    yank_highlight: Option<yank::YankHighlight>,
    pub quit: bool,
    pub force_quit: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    saved: String,
    register: Vec<Vec<char>>,
    linewise: bool,
    register_block_width: Option<usize>,
    preferred_col: Option<usize>,
    last_char_find: Option<CharFind>,
    last_change: Option<RepeatChange>,
    insert_recording: Option<(char, Vec<InsertAction>)>,
    replaying_change: bool,
    search_task: Option<SearchTask>,
    directory_entries: Option<Vec<PathBuf>>,
    completion_cycle: Option<CompletionCycle>,
    insert_completion: Option<completion::InsertCompletion>,
    insert_completion_pending: bool,
    structural_revision: u64,
    display_cache: RefCell<DisplayCache>,
}

#[cfg(test)]
mod tests;
