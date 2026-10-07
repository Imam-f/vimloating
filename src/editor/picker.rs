use super::{BufferAction, Editor};
use std::path::PathBuf;

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
    Buffer { buffer_id: usize },
    BufferLine { buffer_id: usize, row: usize },
    Prompt { kind: PickerPrompt, text: String },
    HelpTopic { path: PathBuf, row: usize },
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
}

impl PickerState {
    pub(super) fn new(source: PickerSource, entries: Vec<PickerEntry>) -> Self {
        let mut state = Self {
            source,
            query: String::new(),
            entries,
            matches: Vec::new(),
            selected: 0,
        };
        state.refilter();
        state
    }

    fn refilter(&mut self) {
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
        self.selected = 0;
    }

    fn output(&self) -> String {
        let mut output = format!(
            "{}  {} matches\nFilter: {} · Up/Down or Ctrl+N/P move\nEnter/Ctrl+J/M open · Esc close\n",
            self.source.title(),
            self.matches.len(),
            self.query
        );
        if self.matches.is_empty() {
            output.push_str("\nNo matches");
            return output;
        }

        const VISIBLE_ITEMS: usize = 14;
        let start = self.selected.saturating_sub(VISIBLE_ITEMS / 2);
        let end = (start + VISIBLE_ITEMS).min(self.matches.len());
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
            picker.refilter();
        }
        self.refresh_picker_output();
    }

    pub fn picker_backspace(&mut self) {
        if let Some(picker) = self.picker_state.as_mut() {
            picker.query.pop();
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
}
