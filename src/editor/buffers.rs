use super::{BufferAction, Editor};
use std::path::{Path, PathBuf};

struct BufferSlot {
    id: usize,
    editor: Editor,
}

pub struct BufferList {
    slots: Vec<BufferSlot>,
    active: usize,
    next_id: usize,
    last_active: Option<usize>,
}

impl BufferList {
    pub fn new(editor: Editor) -> Self {
        Self {
            slots: vec![BufferSlot { id: 1, editor }],
            active: 0,
            next_id: 2,
            last_active: None,
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
            None => {}
        }
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
                    match Editor::open_path(path) {
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
