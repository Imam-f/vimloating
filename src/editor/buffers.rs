use super::{BufferAction, Editor, PickerEntry, PickerPrompt, PickerSource, PickerTarget};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

struct BufferSlot {
    id: usize,
    editor: Editor,
}

pub struct BufferList {
    slots: Vec<BufferSlot>,
    active: usize,
    next_id: usize,
    last_active: Option<usize>,
    file_history: Vec<PathBuf>,
}

impl BufferList {
    pub fn new(editor: Editor) -> Self {
        let file_history = editor
            .path
            .as_deref()
            .filter(|path| path.is_file())
            .and_then(|path| path.canonicalize().ok())
            .into_iter()
            .collect();
        Self {
            slots: vec![BufferSlot { id: 1, editor }],
            active: 0,
            next_id: 2,
            last_active: None,
            file_history,
        }
    }

    pub fn active(&self) -> &Editor {
        &self.slots[self.active].editor
    }

    pub fn active_mut(&mut self) -> &mut Editor {
        &mut self.slots[self.active].editor
    }

    pub fn has_modified_buffers(&self) -> bool {
        self.slots.iter().any(|slot| slot.editor.dirty())
    }

    pub fn protect_quit(&mut self) {
        if self.active().quit && !self.active().force_quit && self.has_modified_buffers() {
            let editor = self.active_mut();
            editor.quit = false;
            editor.message = "Modified buffers · :ls to review, :w to save, :q! to discard".into();
        }
    }

    pub fn process_pending(&mut self) {
        let action = self.active_mut().buffer_action.take();
        match action {
            Some(BufferAction::Open { path, replace }) => self.open(path, replace),
            Some(BufferAction::Help { path, query }) => {
                self.open(path.clone(), false);
                if !query.is_empty() && same_path(self.active().path.as_deref(), Some(&path)) {
                    let editor = self.active_mut();
                    editor.search = query;
                    editor.search_whole_word = false;
                    editor.find(false);
                }
            }
            Some(BufferAction::OpenAddress { path, position }) => {
                if !same_path(self.active().path.as_deref(), Some(&path)) {
                    self.open(path.clone(), false);
                }
                if same_path(self.active().path.as_deref(), Some(&path))
                    && let Some(position) = position
                {
                    let editor = self.active_mut();
                    editor.cursor = position;
                    editor.clamp();
                    editor.preferred_col = None;
                    editor.follow_cursor_horizontally();
                }
            }
            Some(BufferAction::Next) => self.step(1),
            Some(BufferAction::Previous) => self.step(-1),
            Some(BufferAction::Last) => self.switch_last(),
            Some(BufferAction::Select(query)) => self.select(&query),
            Some(BufferAction::List) => self.show_list(),
            Some(BufferAction::Delete { target, force }) => self.delete(target.as_deref(), force),
            Some(BufferAction::StartPicker(source)) => self.start_picker(source),
            Some(BufferAction::StartRgPicker(query)) => self.active_mut().start_rg_picker(query),
            Some(BufferAction::PickerSelect(target)) => self.select_picker(target),
            None => {}
        }
    }

    fn start_picker(&mut self, source: PickerSource) {
        match self.picker_entries(source) {
            Ok(entries) => self.active_mut().start_picker(source, entries),
            Err(error) => self.active_mut().message = error,
        }
    }

    fn picker_entries(&self, source: PickerSource) -> Result<Vec<PickerEntry>, String> {
        match source {
            PickerSource::Files => {
                let cwd = std::env::current_dir().map_err(|err| format!("Files: {err}"))?;
                let output = picker_command("rg", &["--files", "--hidden", "-g", "!.git"], &cwd)?;
                Ok(output
                    .split(|byte| *byte == b'\n')
                    .filter(|path| !path.is_empty())
                    .map(|path| {
                        let path = String::from_utf8_lossy(path)
                            .trim_end_matches('\r')
                            .to_owned();
                        PickerEntry {
                            label: path.clone(),
                            target: PickerTarget::File(cwd.join(path)),
                        }
                    })
                    .collect())
            }
            PickerSource::GitFiles | PickerSource::ModifiedGitFiles => {
                let root = git_root()?;
                let paths = if source == PickerSource::GitFiles {
                    picker_command("git", &["ls-files", "--cached", "-z"], &root)?
                        .split(|byte| *byte == 0)
                        .filter(|path| !path.is_empty())
                        .map(|path| String::from_utf8_lossy(path).into_owned())
                        .collect()
                } else {
                    git_status_paths(&root)?
                };
                Ok(paths
                    .into_iter()
                    .map(|path| PickerEntry {
                        label: path.clone(),
                        target: PickerTarget::File(root.join(path)),
                    })
                    .collect())
            }
            PickerSource::Buffers => Ok(self
                .slots
                .iter()
                .map(|slot| PickerEntry {
                    label: format!(
                        "{} [{}]{}",
                        slot.editor.name(),
                        slot.id,
                        if slot.editor.dirty() { " [+]" } else { "" }
                    ),
                    target: PickerTarget::Buffer { buffer_id: slot.id },
                })
                .collect()),
            PickerSource::Lines | PickerSource::BufferLines => {
                let slots: Vec<_> = if source == PickerSource::Lines {
                    vec![&self.slots[self.active]]
                } else {
                    self.slots.iter().collect()
                };
                let mut entries = Vec::new();
                for slot in slots {
                    for (row, line) in slot.editor.lines.iter().enumerate() {
                        let text: String = line.iter().collect();
                        entries.push(PickerEntry {
                            label: if source == PickerSource::Lines {
                                format!("{:>5}  {text}", row + 1)
                            } else {
                                format!("{}:{}: {}", slot.editor.name(), row + 1, text)
                            },
                            target: PickerTarget::BufferLine {
                                buffer_id: slot.id,
                                row,
                            },
                        });
                    }
                }
                Ok(entries)
            }
            PickerSource::Marks => {
                let mut entries = Vec::new();
                for slot in &self.slots {
                    let mut marks: Vec<_> = slot.editor.marks.iter().collect();
                    marks.sort_by_key(|(name, _)| **name);
                    for (name, position) in marks {
                        let row = position.row.min(slot.editor.lines.len() - 1);
                        let text: String = slot.editor.lines[row].iter().collect();
                        entries.push(PickerEntry {
                            label: format!("'{name}  {}:{}: {text}", slot.editor.name(), row + 1),
                            target: PickerTarget::BufferLine {
                                buffer_id: slot.id,
                                row,
                            },
                        });
                    }
                }
                Ok(entries)
            }
            PickerSource::FileHistory => Ok(self
                .file_history
                .iter()
                .map(|path| PickerEntry {
                    label: path.display().to_string(),
                    target: PickerTarget::File(path.clone()),
                })
                .collect()),
            PickerSource::CommandHistory => Ok(self
                .active()
                .command_history
                .iter()
                .rev()
                .map(|command| PickerEntry {
                    label: command.clone(),
                    target: PickerTarget::Prompt {
                        kind: PickerPrompt::Command,
                        text: command.clone(),
                    },
                })
                .collect()),
            PickerSource::SearchHistory => Ok(self
                .active()
                .search_history
                .iter()
                .rev()
                .map(|query| PickerEntry {
                    label: query.clone(),
                    target: PickerTarget::Prompt {
                        kind: PickerPrompt::Search,
                        text: query.clone(),
                    },
                })
                .collect()),
            PickerSource::Commands => Ok(super::commands::COMMAND_NAMES
                .iter()
                .copied()
                .map(|command| PickerEntry {
                    label: command.to_owned(),
                    target: PickerTarget::Prompt {
                        kind: PickerPrompt::Command,
                        text: command.to_owned(),
                    },
                })
                .collect()),
            PickerSource::Help => {
                let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
                let readme = std::fs::read_to_string(&path)
                    .map_err(|err| format!("Help topics unavailable: {err}"))?;
                Ok(readme
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| line.starts_with('#'))
                    .map(|(row, line)| PickerEntry {
                        label: line.trim_start_matches('#').trim().to_owned(),
                        target: PickerTarget::HelpTopic {
                            path: path.clone(),
                            row,
                        },
                    })
                    .collect())
            }
            PickerSource::Ripgrep => Ok(Vec::new()),
        }
    }

    fn select_picker(&mut self, target: PickerTarget) {
        match target {
            PickerTarget::File(path) => {
                if same_path(self.active().path.as_deref(), Some(&path)) {
                    self.remember_file(&path);
                } else {
                    self.open(path, false);
                }
            }
            PickerTarget::FileLine { path, row, col } => {
                if !same_path(self.active().path.as_deref(), Some(&path)) {
                    self.open(path.clone(), false);
                }
                if same_path(self.active().path.as_deref(), Some(&path)) {
                    let editor = self.active_mut();
                    editor.cursor.row = row.min(editor.lines.len() - 1);
                    editor.cursor.col = col;
                    editor.clamp();
                    editor.follow_cursor_horizontally();
                }
            }
            PickerTarget::Buffer { buffer_id } => {
                if let Some(index) = self.slots.iter().position(|slot| slot.id == buffer_id) {
                    self.switch_to(index);
                }
            }
            PickerTarget::BufferLine { buffer_id, row } => {
                if let Some(index) = self.slots.iter().position(|slot| slot.id == buffer_id) {
                    self.switch_to(index);
                    let editor = self.active_mut();
                    editor.cursor.row = row.min(editor.lines.len() - 1);
                    editor.cursor.col = 0;
                    editor.clamp();
                    editor.follow_cursor_horizontally();
                }
            }
            PickerTarget::Prompt { kind, text } => {
                let editor = self.active_mut();
                editor.prompt = text;
                editor.mode = match kind {
                    PickerPrompt::Command => super::Mode::Command,
                    PickerPrompt::Search => {
                        editor.search_prompt_backwards = false;
                        super::Mode::Search
                    }
                };
            }
            PickerTarget::HelpTopic { path, row } => {
                if !same_path(self.active().path.as_deref(), Some(&path)) {
                    self.open(path.clone(), false);
                }
                if same_path(self.active().path.as_deref(), Some(&path)) {
                    let editor = self.active_mut();
                    editor.cursor.row = row.min(editor.lines.len() - 1);
                    editor.cursor.col = 0;
                    editor.clamp();
                }
            }
        }
    }

    fn remember_file(&mut self, path: &Path) {
        if !path.is_file() {
            return;
        }
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.file_history
            .retain(|recent| !same_path(Some(recent), Some(&path)));
        self.file_history.insert(0, path);
        self.file_history.truncate(100);
    }

    fn open(&mut self, path: PathBuf, replace: bool) {
        let theme = self.active().theme;
        if !replace
            && let Some(index) = self
                .slots
                .iter()
                .position(|slot| same_path(slot.editor.path.as_deref(), Some(&path)))
        {
            if index == self.active {
                if self.active().dirty() {
                    self.active_mut().message = "Already open · unsaved changes kept".into();
                } else {
                    match Editor::open_path(path.clone()) {
                        Ok(mut editor) => {
                            editor.theme = theme;
                            self.slots[index].editor = editor;
                        }
                        Err(err) => self.active_mut().message = format!("Open failed: {err}"),
                    }
                }
            } else {
                self.switch_to(index);
            }
            self.remember_file(&path);
            return;
        }

        match Editor::open_path(path.clone()) {
            Ok(mut editor) => {
                editor.theme = theme;
                if replace {
                    self.slots[self.active].editor = editor;
                    self.active_mut().message = format!("Opened {}", path.display());
                } else {
                    self.last_active = Some(self.slots[self.active].id);
                    let id = self.next_id;
                    self.next_id += 1;
                    self.slots.push(BufferSlot { id, editor });
                    self.active = self.slots.len() - 1;
                    self.active_mut().message = format!("Opened {} · buffer {id}", path.display());
                }
                self.remember_file(&path);
            }
            Err(err) => self.active_mut().message = format!("Open failed: {err}"),
        }
    }

    fn step(&mut self, direction: isize) {
        if self.slots.len() < 2 {
            self.active_mut().message = "Only one buffer is open".into();
            return;
        }
        let next =
            (self.active as isize + direction).rem_euclid(self.slots.len() as isize) as usize;
        self.switch_to(next);
    }

    fn switch_to(&mut self, index: usize) {
        if index == self.active {
            return;
        }
        let theme = self.active().theme;
        self.last_active = Some(self.slots[self.active].id);
        self.active = index;
        let id = self.slots[index].id;
        let name = self.slots[index].editor.name();
        let editor = &mut self.slots[index].editor;
        editor.theme = theme;
        editor.message = format!("Buffer {id}: {name}");
    }

    fn switch_last(&mut self) {
        let Some(id) = self.last_active else {
            self.active_mut().message = "No previous buffer".into();
            return;
        };
        let Some(index) = self.slots.iter().position(|slot| slot.id == id) else {
            self.last_active = None;
            self.active_mut().message = "No previous buffer".into();
            return;
        };
        self.switch_to(index);
    }

    fn select(&mut self, query: &str) {
        let query = query.trim();
        let matching = if let Ok(id) = query.parse::<usize>() {
            self.slots.iter().position(|slot| slot.id == id)
        } else {
            let query_lower = query.to_lowercase();
            let exact: Vec<_> = self
                .slots
                .iter()
                .enumerate()
                .filter(|(_, slot)| slot_matches(slot, &query_lower, false))
                .map(|(index, _)| index)
                .collect();
            if exact.len() == 1 {
                Some(exact[0])
            } else if exact.is_empty() {
                let prefix: Vec<_> = self
                    .slots
                    .iter()
                    .enumerate()
                    .filter(|(_, slot)| slot_matches(slot, &query_lower, true))
                    .map(|(index, _)| index)
                    .collect();
                if prefix.len() == 1 {
                    Some(prefix[0])
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(index) = matching {
            self.switch_to(index);
        } else {
            self.active_mut().message = format!("No unique buffer matches: {query}");
        }
    }

    fn show_list(&mut self) {
        let mut output = String::from("Buffers · % active · + modified\n\n");
        for (index, slot) in self.slots.iter().enumerate() {
            let active = if index == self.active { '%' } else { ' ' };
            let modified = if slot.editor.dirty() { '+' } else { ' ' };
            output.push_str(&format!(
                "{active}{modified} {:>3}  {}\n",
                slot.id,
                slot.editor.name()
            ));
        }
        output.push_str(
            "\n:b {id|name}  switch    :bn/:bp  next/previous\nCtrl+6  last active    :bd  delete current buffer",
        );
        self.active_mut().show_buffer_list(output);
    }

    fn delete(&mut self, target: Option<&str>, force: bool) {
        let index = if let Some(query) = target {
            match self.find_buffer(query) {
                Some(index) => index,
                None => {
                    self.active_mut().message = format!("No unique buffer matches: {query}");
                    return;
                }
            }
        } else {
            self.active
        };
        if self.slots[index].editor.dirty() && !force {
            self.active_mut().message = "Unsaved changes · use :bd! to discard".into();
            return;
        }

        let id = self.slots[index].id;
        let theme = self.active().theme;
        let was_active = index == self.active;
        if self.slots.len() == 1 {
            self.last_active = None;
            let mut editor = Editor::new("", None);
            editor.theme = theme;
            editor.message = format!("Deleted buffer {id} · empty buffer ready");
            self.slots[0].editor = editor;
            return;
        }

        self.slots.remove(index);
        if was_active || self.last_active == Some(id) {
            self.last_active = None;
        }
        if index < self.active {
            self.active -= 1;
        } else if index == self.active {
            self.active = index.min(self.slots.len() - 1);
        }
        let active = &mut self.slots[self.active].editor;
        active.theme = theme;
        active.message = if was_active {
            format!("Deleted buffer {id} · now on {}", active.name())
        } else {
            format!("Deleted buffer {id}")
        };
    }

    fn find_buffer(&self, query: &str) -> Option<usize> {
        if let Ok(id) = query.parse::<usize>() {
            return self.slots.iter().position(|slot| slot.id == id);
        }
        let query_lower = query.to_lowercase();
        let exact: Vec<_> = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot_matches(slot, &query_lower, false))
            .map(|(index, _)| index)
            .collect();
        if exact.len() == 1 {
            Some(exact[0])
        } else {
            let prefix: Vec<_> = self
                .slots
                .iter()
                .enumerate()
                .filter(|(_, slot)| slot_matches(slot, &query_lower, true))
                .map(|(index, _)| index)
                .collect();
            if prefix.len() == 1 {
                Some(prefix[0])
            } else {
                None
            }
        }
    }
}

fn picker_command(program: &str, args: &[&str], cwd: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|err| format!("Could not start {program}: {err}"))?;
    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if details.is_empty() {
            format!("{program} failed with {}", output.status)
        } else {
            details
        });
    }
    Ok(output.stdout)
}

fn git_root() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|err| format!("Git files: {err}"))?;
    let output = picker_command("git", &["rev-parse", "--show-toplevel"], &cwd)?;
    let root = String::from_utf8_lossy(&output).trim().to_owned();
    if root.is_empty() {
        Err("Not inside a Git repository".into())
    } else {
        Ok(PathBuf::from(root))
    }
}

fn git_status_paths(root: &Path) -> Result<Vec<String>, String> {
    let output = picker_command(
        "git",
        &["status", "--short", "-z", "--untracked-files=all"],
        root,
    )?;
    let mut fields = output.split(|byte| *byte == 0).peekable();
    let mut paths = Vec::new();
    let mut seen = std::collections::HashSet::new();
    while let Some(record) = fields.next() {
        if record.len() < 4 {
            continue;
        }
        let status = &record[..2];
        let path = String::from_utf8_lossy(&record[3..]).into_owned();
        if status.contains(&b'R') || status.contains(&b'C') {
            fields.next();
        }
        if status.contains(&b'D') || path.is_empty() {
            continue;
        }
        let path_buf = root.join(&path);
        if path_buf.is_file() && seen.insert(path_buf) {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn slot_matches(slot: &BufferSlot, query: &str, prefix: bool) -> bool {
    let name = slot.editor.name().to_lowercase();
    let name_matches = if prefix {
        name.starts_with(query)
    } else {
        name == query
    };
    let filename_matches = slot
        .editor
        .path
        .as_deref()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().to_lowercase())
        .is_some_and(|name| {
            if prefix {
                name.starts_with(query)
            } else {
                name == query
            }
        });
    name_matches || filename_matches
}

fn same_path(left: Option<&Path>, right: Option<&PathBuf>) -> bool {
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}
