use super::{Editor, Mode, Pos};
use std::{fs, path::PathBuf};

impl Editor {
    pub fn find(&mut self, backwards: bool) {
        self.horizontal_scroll_hold = false;
        let needle: Vec<char> = self.search.chars().collect();
        if needle.is_empty() {
            return;
        }
        let mut hits = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            if line.len() >= needle.len() {
                for col in 0..=line.len() - needle.len() {
                    if line[col..col + needle.len()] == needle {
                        hits.push(Pos { row, col });
                    }
                }
            }
        }
        let hit = if backwards {
            hits.iter()
                .rev()
                .find(|p| **p < self.cursor)
                .or_else(|| hits.last())
        } else {
            hits.iter()
                .find(|p| **p > self.cursor)
                .or_else(|| hits.first())
        };
        if let Some(p) = hit {
            self.cursor = *p;
            self.message = format!("/{} · {} matches", self.search, hits.len());
        } else {
            self.message = format!("Pattern not found: {}", self.search);
        }
    }

    pub fn submit_prompt(&mut self) {
        let prompt = self.prompt.clone();
        let searching = self.mode == Mode::Search;
        self.escape();
        if searching {
            self.search = prompt;
            self.find(false);
        } else {
            self.command(&prompt);
        }
    }

    pub fn save(&mut self, path: Option<&str>) -> bool {
        let destination = path
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| self.path.clone());
        let Some(destination) = destination else {
            self.message = "No filename · use :w path/to/file".into();
            return false;
        };
        let text = self.text();
        match fs::write(&destination, &text) {
            Ok(()) => {
                self.path = Some(destination);
                self.saved = text;
                self.message = format!("Written {} · {} lines", self.name(), self.lines.len());
                true
            }
            Err(err) => {
                self.message = format!("Write failed: {err}");
                false
            }
        }
    }

    pub fn command(&mut self, command: &str) {
        let command = command.trim();
        let (cmd, arg) = command
            .split_once(char::is_whitespace)
            .unwrap_or((command, ""));
        let arg = arg.trim();
        match cmd {
            "w" => {
                self.save(Some(arg));
            }
            "wq" | "x" => {
                if self.save(Some(arg)) {
                    self.quit = true;
                }
            }
            "q" => {
                if self.dirty() {
                    self.message = "Unsaved changes · :w to save, :q! to discard".into();
                } else {
                    self.quit = true;
                }
            }
            "q!" => self.quit = true,
            "e" | "e!" => {
                if cmd == "e" && self.dirty() {
                    self.message = "Unsaved changes · :w first or :e! path".into();
                } else if arg.is_empty() {
                    self.message = "Usage: :e path/to/file".into();
                } else {
                    match fs::read_to_string(arg) {
                        Ok(text) => {
                            *self = Self::new(&text, Some(PathBuf::from(arg)));
                            self.message = "File opened".into();
                        }
                        Err(err) => self.message = format!("Open failed: {err}"),
                    }
                }
            }
            "help" => {
                self.message =
                    "hjkl · w/b/e · gg/G · i/a/o · v · dd/yy/p · u/Ctrl-R · /search · :w/:e/:q"
                        .into()
            }
            "noh" | "nohlsearch" => self.search.clear(),
            _ => {
                if let Ok(line) = command.parse::<usize>() {
                    self.cursor = Pos {
                        row: line.saturating_sub(1).min(self.lines.len() - 1),
                        col: 0,
                    };
                } else {
                    self.message = format!("Unknown command: {command}");
                }
            }
        }
    }
}
