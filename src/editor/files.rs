use super::{BufferAction, Editor, Mode, Pos};
use crate::config::Theme;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn containing_directory(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    }
}

impl Editor {
    pub fn open_path(path: PathBuf) -> Result<Self, String> {
        let metadata = fs::metadata(&path).map_err(|err| err.to_string())?;
        if metadata.is_dir() {
            let path = fs::canonicalize(&path).unwrap_or(path);
            let mut directories = Vec::new();
            let mut files = Vec::new();
            for entry in fs::read_dir(&path).map_err(|err| err.to_string())? {
                let entry = entry.map_err(|err| err.to_string())?;
                let entry_path = entry.path();
                if entry.file_type().map_err(|err| err.to_string())?.is_dir() {
                    directories.push(entry_path);
                } else {
                    files.push(entry_path);
                }
            }
            let sort_entries = |entries: &mut Vec<PathBuf>| {
                entries.sort_by_key(|entry| {
                    entry
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase()
                });
            };
            sort_entries(&mut directories);
            sort_entries(&mut files);

            let parent = path.parent().filter(|parent| *parent != path);
            let has_parent = parent.is_some();
            let mut entries = Vec::new();
            if let Some(parent) = parent {
                entries.push(parent.to_path_buf());
            }
            entries.extend(directories);
            entries.extend(files);
            let listing = entries
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    let name = if has_parent && index == 0 {
                        "..".to_owned()
                    } else {
                        entry
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    };
                    if entry.is_dir() {
                        format!("{name}/")
                    } else {
                        name
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let mut editor = Self::new(&listing, Some(path.clone()));
            editor.directory_entries = Some(entries);
            editor.message = format!(
                "Directory: {} · Enter opens, - goes to parent",
                path.display()
            );
            Ok(editor)
        } else {
            let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
            Ok(Self::new(&text, Some(path)))
        }
    }

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
        if self.is_directory_browser() {
            self.message = "Cannot write a directory listing".into();
            return false;
        }
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
        if let Some(shell_command) = command.strip_prefix('!') {
            self.run_shell(shell_command.trim());
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
                if cmd == "e" && self.dirty() {
                    self.message = "Unsaved changes · :w first or :e! path".into();
                } else if arg.is_empty() {
                    self.message = "Usage: :e path/to/file".into();
                } else {
                    self.buffer_action = Some(BufferAction::Open {
                        path: PathBuf::from(arg),
                        replace: cmd == "e!",
                    });
                    self.message = format!("Opening {arg}…");
                }
            }
            "Ex" | "Explore" => {
                if self.dirty() {
                    self.message = "Unsaved changes · :w before browsing directories".into();
                    return;
                }
                let directory = if !arg.is_empty() {
                    PathBuf::from(arg)
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
                self.message =
                    "hjkl · w/b/e · gg/G · i/a/o · v/V · /search · :!cmd · :Explore · :theme".into()
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

    fn run_shell(&mut self, shell_command: &str) {
        if shell_command.is_empty() {
            self.message = "Usage: :!command".into();
            return;
        }
        let working_directory = self.path.as_deref().map(containing_directory);

        #[cfg(windows)]
        let output = {
            let mut command = Command::new(std::env::var_os("COMSPEC").unwrap_or("cmd.exe".into()));
            command.args(["/C", shell_command]);
            if let Some(directory) = working_directory {
                command.current_dir(directory);
            }
            command.output()
        };
        #[cfg(not(windows))]
        let output = {
            let mut command = Command::new(std::env::var_os("SHELL").unwrap_or("/bin/sh".into()));
            command.args(["-c", shell_command]);
            if let Some(directory) = working_directory {
                command.current_dir(directory);
            }
            command.output()
        };

        let output = match output {
            Ok(output) => {
                let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                    text.push_str(&stderr);
                }
                let status = output
                    .status
                    .code()
                    .map(|code| format!("exit {code}"))
                    .unwrap_or_else(|| "terminated".into());
                if text.is_empty() {
                    format!("$ {shell_command}\n(no output)\n{status}")
                } else {
                    format!("$ {shell_command}\n{}\n{status}", text.trim_end())
                }
            }
            Err(err) => format!("$ {shell_command}\nCould not start shell: {err}"),
        };
        self.output_view = Some(output);
        self.mode = Mode::ShellOutput;
        self.message = "Shell finished · Esc to close".into();
    }
}
