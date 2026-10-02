use super::{Editor, Mode};

impl Editor {
    pub fn control_key(&mut self, key: char) {
        match (self.mode, key) {
            (Mode::CommandWindow, 'c') => self.finish_command_window(false),
            (_, 'c') => self.escape(),
            (Mode::Normal | Mode::Visual, 'n' | 'p') => {
                let n = self.count.parse::<usize>().unwrap_or(1).clamp(1, 10000);
                self.count.clear();
                self.pending = None;
                self.move_by(0, if key == 'n' { 1 } else { -1 }, n);
            }
            (Mode::Command | Mode::Search, 'n' | 'p') => self.browse_history(key == 'p'),
            _ => {}
        }
    }

    pub(super) fn open_command_window(&mut self, searching: bool) {
        let history = if searching {
            &self.search_history
        } else {
            &self.command_history
        };
        let text = format!("{}\n", history.join("\n"));
        let mut window = Editor::new(&text, None);
        window.cursor.row = window.lines.len() - 1;
        window.theme = self.theme;
        window.message =
            "Command history · i to edit · Enter to execute · Ctrl+C to prompt · Esc to close"
                .into();
        self.command_window = Some(Box::new(window));
        self.command_window_search = searching;
        self.mode = Mode::CommandWindow;
    }

    pub fn finish_command_window(&mut self, execute: bool) {
        let Some(window) = self.command_window.take() else {
            return;
        };
        let prompt = window.lines[window.cursor.row].iter().collect::<String>();
        self.mode = if self.command_window_search {
            Mode::Search
        } else {
            Mode::Command
        };
        self.search_prompt_backwards = false;
        self.prompt = prompt;
        self.history_cursor = None;
        if execute {
            self.submit_prompt();
        }
    }

    pub(super) fn remember_prompt(&mut self, prompt: &str) {
        if prompt.is_empty() {
            return;
        }
        let history = if self.mode == Mode::Search {
            &mut self.search_history
        } else {
            &mut self.command_history
        };
        if history.last().is_none_or(|entry| entry != prompt) {
            history.push(prompt.to_owned());
            if history.len() > 200 {
                history.remove(0);
            }
        }
    }

    fn browse_history(&mut self, backwards: bool) {
        let history = if self.mode == Mode::Search {
            &self.search_history
        } else {
            &self.command_history
        };
        if self.history_cursor.is_none() {
            self.history_draft = self.prompt.clone();
        }
        let current = self.history_cursor.unwrap_or(history.len());
        let index = if backwards {
            (0..current)
                .rev()
                .find(|&index| history[index].starts_with(&self.history_draft))
        } else {
            (current.saturating_add(1)..history.len())
                .find(|&index| history[index].starts_with(&self.history_draft))
        };
        if let Some(index) = index {
            self.prompt = history[index].clone();
            self.history_cursor = Some(index);
        } else if !backwards {
            self.prompt = self.history_draft.clone();
            self.history_cursor = None;
        }
        self.completion_cycle = None;
    }

    pub fn edit_prompt(&mut self, character: Option<char>) {
        if let Some(ch) = character {
            self.prompt.push(ch);
        } else {
            self.prompt.pop();
        }
        self.history_cursor = None;
        self.completion_cycle = None;
    }
}
