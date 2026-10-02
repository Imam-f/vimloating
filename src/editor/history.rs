use super::{Editor, Mode};

impl Editor {
    pub fn control_key(&mut self, key: char) {
        match (self.mode, key) {
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
