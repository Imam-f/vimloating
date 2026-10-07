use super::{BufferAction, Editor};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

const PICKER_VISIBLE_ITEMS: usize = 14;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerSource {
    Files,
    GitFiles,
    ModifiedGitFiles,
    Buffers,
    BufferLines,
    Lines,
    Marks,
    FileHistory,
    CommandHistory,
    SearchHistory,
    Commands,
    Help,
    Ripgrep,
}

impl PickerSource {
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Files => ":Files",
            Self::GitFiles => ":Gfiles",
            Self::ModifiedGitFiles => ":Gfiles?",
            Self::Buffers => ":Buffer",
            Self::BufferLines => ":Blines",
            Self::Lines => ":Lines",
            Self::Marks => ":Marks",
            Self::FileHistory => ":History",
            Self::CommandHistory => ":History:",
            Self::SearchHistory => ":History/",
            Self::Commands => ":Command",
            Self::Help => ":Help",
            Self::Ripgrep => ":Rg",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerPrompt {
    Command,
    Search,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickerTarget {
    File(PathBuf),
    FileLine {
        path: PathBuf,
        row: usize,
        col: usize,
    },
    Buffer {
        buffer_id: usize,
    },
    BufferLine {
        buffer_id: usize,
        row: usize,
    },
    Prompt {
        kind: PickerPrompt,
        text: String,
    },
    HelpTopic {
        path: PathBuf,
        row: usize,
    },
}

#[derive(Clone)]
pub(crate) struct PickerEntry {
    pub label: String,
    pub target: PickerTarget,
}

pub(super) struct PickerState {
    source: PickerSource,
    query: String,
    entries: Vec<PickerEntry>,
    matches: Vec<usize>,
    selected: usize,
    window_start: usize,
    rg_cwd: Option<PathBuf>,
    error: Option<String>,
}

impl PickerState {
    pub(super) fn new(source: PickerSource, entries: Vec<PickerEntry>) -> Self {
        let mut state = Self {
            source,
            query: String::new(),
            entries,
            matches: Vec::new(),
            selected: 0,
            window_start: 0,
            rg_cwd: None,
            error: None,
        };
        state.refilter();
        state
    }

    fn refilter(&mut self) {
        if self.source == PickerSource::Ripgrep {
            self.matches = (0..self.entries.len()).collect();
        } else {
            let mut matches: Vec<_> = self
                .entries
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| {
                    fuzzy_score(&self.query, &entry.label).map(|score| (index, score))
                })
                .collect();
            matches.sort_by(|(left_index, left_score), (right_index, right_score)| {
                right_score
                    .cmp(left_score)
                    .then_with(|| left_index.cmp(right_index))
            });
            self.matches = matches.into_iter().map(|(index, _)| index).collect();
        }
        self.selected = 0;
        self.window_start = 0;
    }

    fn output(&self) -> String {
        let mut output = format!(
            "{}  {} matches\n{}: {} · Up/Down or Ctrl+N/P move\nEnter/Ctrl+J/M open · Esc close\n",
            self.source.title(),
            self.matches.len(),
            if self.source == PickerSource::Ripgrep {
                "Search"
            } else {
                "Filter"
            },
            self.query
        );
        if self.matches.is_empty() {
            if let Some(error) = &self.error {
                output.push_str(&format!("\n{error}"));
            } else if self.source == PickerSource::Ripgrep && self.query.is_empty() {
                output.push_str("\nType a pattern to search with rg");
            } else {
                output.push_str("\nNo matches");
            }
            return output;
        }

        let start = self.window_start;
        let end = (start + PICKER_VISIBLE_ITEMS).min(self.matches.len());
        if start > 0 {
            output.push_str("  …\n");
        }
        for index in start..end {
            let entry = &self.entries[self.matches[index]];
            let marker = if index == self.selected { "> " } else { "  " };
            output.push_str(marker);
            output.push_str(&entry.label);
            output.push('\n');
        }
        if end < self.matches.len() {
            output.push_str("  …\n");
        }
        output
    }

    fn selected_target(&self) -> Option<PickerTarget> {
        self.matches
            .get(self.selected)
            .map(|&index| self.entries[index].target.clone())
    }
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    let query: Vec<_> = query.to_lowercase().chars().collect();
    if query.is_empty() {
        return Some(0);
    }
    let candidate: Vec<_> = candidate.to_lowercase().chars().collect();
    let mut cursor = 0;
    let mut previous = None;
    let mut score = 0;
    for needle in query {
        let offset = candidate[cursor..].iter().position(|&ch| ch == needle)?;
        let index = cursor + offset;
        let boundary = index == 0
            || candidate[index - 1].is_whitespace()
            || matches!(candidate[index - 1], '/' | '\\' | '_' | '-' | ':' | '.');
        score += 10
            + i32::from(boundary) * 12
            + i32::from(previous.is_some_and(|previous| previous + 1 == index)) * 8
            - (offset as i32).min(10);
        previous = Some(index);
        cursor = index + 1;
    }
    Some(score)
}

impl Editor {
    pub(super) fn start_picker(&mut self, source: PickerSource, entries: Vec<PickerEntry>) {
        let picker = PickerState::new(source, entries);
        self.mode = super::Mode::BufferList;
        self.output_view = Some(picker.output());
        self.picker_state = Some(picker);
    }

    pub fn picker_active(&self) -> bool {
        self.picker_state.is_some()
    }

    pub fn picker_type(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        if let Some(picker) = self.picker_state.as_mut() {
            picker.query.push(ch);
            picker.update_rg_entries();
            picker.refilter();
        }
        self.refresh_picker_output();
    }

    pub fn picker_backspace(&mut self) {
        if let Some(picker) = self.picker_state.as_mut() {
            picker.query.pop();
            picker.update_rg_entries();
            picker.refilter();
        }
        self.refresh_picker_output();
    }

    pub fn picker_move(&mut self, direction: isize) {
        if let Some(picker) = self.picker_state.as_mut() {
            if direction < 0 {
                picker.selected = picker.selected.saturating_sub(direction.unsigned_abs());
            } else if !picker.matches.is_empty() {
                picker.selected =
                    (picker.selected + direction as usize).min(picker.matches.len() - 1);
            }
            if picker.selected < picker.window_start {
                picker.window_start = picker.selected;
            } else if picker.selected >= picker.window_start + PICKER_VISIBLE_ITEMS {
                picker.window_start = picker.selected + 1 - PICKER_VISIBLE_ITEMS;
            }
        }
        self.refresh_picker_output();
    }

    pub fn accept_picker(&mut self) {
        let target = self
            .picker_state
            .as_ref()
            .and_then(PickerState::selected_target);
        self.picker_state = None;
        self.output_view = None;
        self.mode = super::Mode::Normal;
        if let Some(target) = target {
            self.buffer_action = Some(BufferAction::PickerSelect(target));
        } else {
            self.message = "No picker match selected".into();
        }
    }

    fn refresh_picker_output(&mut self) {
        if let Some(picker) = self.picker_state.as_ref() {
            self.output_view = Some(picker.output());
        }
    }

    pub(super) fn start_rg_picker(&mut self, query: String) {
        let mut picker = PickerState::new(PickerSource::Ripgrep, Vec::new());
        picker.query = query;
        picker.rg_cwd = match std::env::current_dir() {
            Ok(cwd) => Some(cwd),
            Err(err) => {
                picker.error = Some(format!("Could not determine working directory: {err}"));
                None
            }
        };
        picker.update_rg_entries();
        picker.refilter();
        self.mode = super::Mode::BufferList;
        self.output_view = Some(picker.output());
        self.picker_state = Some(picker);
    }
}

impl PickerState {
    fn update_rg_entries(&mut self) {
        if self.source != PickerSource::Ripgrep {
            return;
        }
        self.error = None;
        if self.query.is_empty() {
            self.entries.clear();
            return;
        }
        let Some(cwd) = self.rg_cwd.as_deref() else {
            self.error = Some("Could not determine the working directory".into());
            self.entries.clear();
            return;
        };
        match rg_entries(cwd, &self.query) {
            Ok(entries) => self.entries = entries,
            Err(error) => {
                self.entries.clear();
                self.error = Some(error);
            }
        }
    }
}

fn rg_entries(cwd: &Path, query: &str) -> Result<Vec<PickerEntry>, String> {
    let output = Command::new("rg")
        .args([
            "--vimgrep",
            "--no-heading",
            "--color=never",
            "--smart-case",
            "--",
            query,
        ])
        .current_dir(cwd)
        .output()
        .map_err(|err| format!("Could not start rg: {err}"))?;
    if output.status.code() == Some(1) {
        return Ok(Vec::new());
    }
    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if details.is_empty() {
            format!("rg failed with {}", output.status)
        } else {
            details
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(4, ':');
            let relative_path = fields.next()?;
            let row = fields.next()?.parse::<usize>().ok()?.saturating_sub(1);
            let byte_col = fields.next()?.parse::<usize>().ok()?.saturating_sub(1);
            let text = fields.next()?.to_owned();
            let col = text
                .char_indices()
                .take_while(|(offset, _)| *offset < byte_col)
                .count();
            Some(PickerEntry {
                label: format!("{relative_path}:{}:{}: {text}", row + 1, col + 1),
                target: PickerTarget::FileLine {
                    path: cwd.join(relative_path),
                    row,
                    col,
                },
            })
        })
        .collect())
}
