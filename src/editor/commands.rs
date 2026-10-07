use super::{BufferAction, Editor, Mode, PickerSource, Pos, files::containing_directory};
use crate::config::Theme;
use std::path::Path;

pub(super) const COMMAND_NAMES: &[&str] = &[
    "!",
    "b",
    "bd",
    "bd!",
    "bdelete",
    "bdelete!",
    "bn",
    "bnext",
    "bp",
    "bprevious",
    "buffer",
    "buffers",
    "e",
    "e!",
    "Ex",
    "Explore",
    "Files",
    "Gfiles",
    "Gfiles?",
    "help",
    "Help",
    "History",
    "History:",
    "History/",
    "Buffer",
    "Blines",
    "Lines",
    "Marks",
    "Command",
    "ls",
    "noh",
    "nohlsearch",
    "q",
    "q!",
    "theme",
    "w",
    "wq",
    "x",
];

impl Editor {
    pub fn submit_prompt(&mut self) {
        let prompt = self.prompt.clone();
        self.remember_prompt(&prompt);
        let searching = self.mode == Mode::Search;
        let search_backwards = self.search_prompt_backwards;
        self.escape();
        if searching {
            if !prompt.is_empty() {
                self.search = prompt;
                self.search_whole_word = false;
            }
            self.search_backwards = search_backwards;
            self.find(search_backwards);
        } else {
            self.command(&prompt);
        }
    }

    pub fn complete_command(&mut self) {
        if let Some(mut cycle) = self.completion_cycle.take()
            && self.prompt == cycle.last
        {
            cycle.index = (cycle.index + 1) % cycle.candidates.len();
            let completed = cycle.candidates[cycle.index].clone();
            self.prompt = completed.clone();
            cycle.last = completed;
            self.completion_cycle = Some(cycle);
            return;
        }

        let prefix = self.prompt.clone();
        let candidates = command_completions(&prefix);
        if candidates.is_empty() {
            self.message = "No command or path completions".into();
            return;
        }
        let completed = candidates[0].clone();
        if candidates.len() == 1 && completed == prefix {
            self.message = "Already complete".into();
            return;
        }
        self.prompt = completed.clone();
        self.completion_cycle = Some(super::CompletionCycle {
            candidates,
            index: 0,
            last: completed,
        });
    }

    pub fn command(&mut self, command: &str) {
        let command = command.trim();
        if let Some(shell_command) = command.strip_prefix(".!") {
            self.filter_current_line(shell_command.trim());
            return;
        }
        if let Some(shell_command) = command.strip_prefix('!') {
            self.run_shell(shell_command.trim());
            return;
        }
        if let Some(id) = command
            .strip_prefix('b')
            .filter(|suffix| !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()))
        {
            self.buffer_action = Some(BufferAction::Select(id.to_owned()));
            return;
        }
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
            "q!" => {
                self.quit = true;
                self.force_quit = true;
            }
            "e" | "e!" => {
                if arg.is_empty() {
                    self.message = "Usage: :e path/to/file".into();
                } else {
                    self.buffer_action = Some(BufferAction::Open {
                        path: super::paths::expand_home(arg),
                        replace: cmd == "e!",
                    });
                    self.message = format!("Opening {arg}…");
                }
            }
            "Ex" | "Explore" => {
                let directory = if !arg.is_empty() {
                    super::paths::expand_home(arg)
                } else {
                    self.path
                        .as_deref()
                        .map(containing_directory)
                        .or_else(|| std::env::current_dir().ok())
                        .unwrap_or_default()
                };
                self.buffer_action = Some(BufferAction::Open {
                    path: directory,
                    replace: false,
                });
                self.message = "Opening directory…".into();
            }
            "b" | "buffer" => match arg {
                "" => self.buffer_action = Some(BufferAction::List),
                "delete" => {
                    self.buffer_action = Some(BufferAction::Delete {
                        target: None,
                        force: false,
                    })
                }
                "delete!" => {
                    self.buffer_action = Some(BufferAction::Delete {
                        target: None,
                        force: true,
                    })
                }
                _ => self.buffer_action = Some(BufferAction::Select(arg.to_owned())),
            },
            "buffers" | "ls" => self.buffer_action = Some(BufferAction::List),
            "bn" | "bnext" => self.buffer_action = Some(BufferAction::Next),
            "bp" | "bprevious" => self.buffer_action = Some(BufferAction::Previous),
            "bd" | "bdelete" | "bd!" | "bdelete!" => {
                self.buffer_action = Some(BufferAction::Delete {
                    target: (!arg.is_empty()).then(|| arg.to_owned()),
                    force: cmd.ends_with('!'),
                });
            }
            "theme" => {
                if arg.is_empty() {
                    self.message = format!(
                        "Theme: {} · :theme everforest | solarized-blue | default",
                        self.theme.name()
                    );
                } else if let Some(theme) = Theme::parse(arg) {
                    self.theme = theme;
                    self.message = format!("Theme: {}", self.theme.name());
                } else {
                    self.message = format!(
                        "Unknown theme: {arg} · use everforest, solarized-blue, or default"
                    );
                }
            }
            "help" => {
                self.buffer_action = Some(BufferAction::Help {
                    path: Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"),
                    query: arg.to_owned(),
                });
            }
            "Files" => self.buffer_action = Some(BufferAction::StartPicker(PickerSource::Files)),
            "Gfiles" => {
                self.buffer_action = Some(BufferAction::StartPicker(PickerSource::GitFiles))
            }
            "Gfiles?" => {
                self.buffer_action = Some(BufferAction::StartPicker(PickerSource::ModifiedGitFiles))
            }
            "Buffer" => self.buffer_action = Some(BufferAction::StartPicker(PickerSource::Buffers)),
            "Blines" => {
                self.buffer_action = Some(BufferAction::StartPicker(PickerSource::BufferLines))
            }
            "Lines" => self.buffer_action = Some(BufferAction::StartPicker(PickerSource::Lines)),
            "Marks" => self.buffer_action = Some(BufferAction::StartPicker(PickerSource::Marks)),
            "History" => {
                self.buffer_action = Some(BufferAction::StartPicker(PickerSource::FileHistory))
            }
            "History:" => {
                self.buffer_action = Some(BufferAction::StartPicker(PickerSource::CommandHistory))
            }
            "History/" => {
                self.buffer_action = Some(BufferAction::StartPicker(PickerSource::SearchHistory))
            }
            "Command" => {
                self.buffer_action = Some(BufferAction::StartPicker(PickerSource::Commands))
            }
            "Help" => self.buffer_action = Some(BufferAction::StartPicker(PickerSource::Help)),
            "noh" | "nohlsearch" => self.search.clear(),
            _ => {
                if let Ok(line) = command.parse::<usize>() {
                    self.search_task = None;
                    self.char_find_highlight = None;
                    self.char_find_hints.clear();
                    self.preferred_col = None;
                    self.horizontal_scroll_hold = false;
                    let row = line.saturating_sub(1).min(self.lines.len() - 1);
                    self.cursor = Pos {
                        row,
                        col: self.lines[row]
                            .iter()
                            .position(|ch| !ch.is_whitespace())
                            .unwrap_or(0),
                    };
                    self.clamp();
                } else {
                    self.message = format!("Unknown command: {command}");
                }
            }
        }
    }
}

fn command_completions(prefix: &str) -> Vec<String> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let Some(command_end) = prefix.find(char::is_whitespace) else {
        let mut matches: Vec<_> = COMMAND_NAMES
            .iter()
            .filter(|command| {
                command
                    .to_ascii_lowercase()
                    .starts_with(&prefix.to_ascii_lowercase())
            })
            .map(|command| (*command).to_owned())
            .collect();
        if matches.iter().any(|candidate| candidate == prefix) {
            return matches
                .into_iter()
                .filter(|candidate| candidate == prefix)
                .collect();
        }
        let case_matches: Vec<_> = matches
            .iter()
            .filter(|candidate| candidate.starts_with(prefix))
            .cloned()
            .collect();
        if !case_matches.is_empty() {
            matches = case_matches;
        }
        if let Some(exact) = matches
            .iter()
            .find(|candidate| candidate.eq_ignore_ascii_case(prefix))
        {
            return vec![exact.clone()];
        }
        if matches.len() > 1 {
            matches.sort_by_key(|candidate| candidate.to_ascii_lowercase());
        }
        return matches;
    };

    let command = &prefix[..command_end];
    if !matches!(command, "e" | "e!" | "w" | "wq" | "x" | "Ex" | "Explore") {
        return Vec::new();
    }
    let rest = &prefix[command_end..];
    let arg_offset = rest
        .find(|ch: char| !ch.is_whitespace())
        .map_or(prefix.len(), |offset| command_end + offset);
    let arg_prefix = &prefix[arg_offset..];
    super::paths::path_completions(arg_prefix, Path::new("."))
        .into_iter()
        .map(|candidate| format!("{}{candidate}", &prefix[..arg_offset]))
        .collect()
}
