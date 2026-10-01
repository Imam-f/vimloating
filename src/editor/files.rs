use super::{BufferAction, Editor, Mode, Pos, SearchTask};
use crate::config::Theme;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

enum SearchStep {
    Continue,
    Found(Pos),
    Done,
}

const SEARCH_STEPS_PER_FRAME: usize = 4096;

impl SearchTask {
    fn finish(&self) -> SearchStep {
        self.last_match.map_or(SearchStep::Done, SearchStep::Found)
    }

    fn wrap_or_finish(&mut self, line_count: usize) -> SearchStep {
        if self.wrapped {
            return self.finish();
        }
        self.wrapped = true;
        self.candidate = if self.backwards {
            Pos {
                row: line_count - 1,
                col: usize::MAX,
            }
        } else {
            Pos::default()
        };
        self.compare_offset = None;
        self.advance_candidate = false;
        SearchStep::Continue
    }

    fn skip_row(&mut self, line_count: usize) -> SearchStep {
        if self.backwards {
            if self.candidate.row == 0 {
                return self.wrap_or_finish(line_count);
            }
            self.candidate.row -= 1;
            self.candidate.col = usize::MAX;
        } else {
            self.candidate.row += 1;
            self.candidate.col = 0;
        }
        SearchStep::Continue
    }

    fn step(&mut self, lines: &[Vec<char>]) -> SearchStep {
        if let Some(offset) = self.compare_offset {
            if lines[self.candidate.row][self.candidate.col + offset] == self.needle[offset] {
                if offset + 1 == self.needle.len() {
                    let found = self.candidate;
                    self.last_match = Some(found);
                    if self.remaining > 1 {
                        self.remaining -= 1;
                        self.origin = found;
                        self.candidate = found;
                        self.wrapped = false;
                        self.compare_offset = None;
                        self.advance_candidate = true;
                        return SearchStep::Continue;
                    }
                    return SearchStep::Found(found);
                }
                self.compare_offset = Some(offset + 1);
            } else {
                self.compare_offset = None;
                self.advance_candidate = true;
            }
            return SearchStep::Continue;
        }

        if self.advance_candidate {
            self.advance_candidate = false;
            if self.backwards {
                if self.candidate.col > 0 {
                    self.candidate.col -= 1;
                } else if self.candidate.row > 0 {
                    self.candidate.row -= 1;
                    self.candidate.col = usize::MAX;
                } else {
                    return self.wrap_or_finish(lines.len());
                }
            } else {
                self.candidate.col = self.candidate.col.saturating_add(1);
            }
        }

        if self.candidate.row >= lines.len() {
            return self.wrap_or_finish(lines.len());
        }
        if self.wrapped
            && ((!self.backwards && self.candidate.row > self.origin.row)
                || (self.backwards && self.candidate.row < self.origin.row))
        {
            return self.finish();
        }

        let Some(max_start) = lines[self.candidate.row]
            .len()
            .checked_sub(self.needle.len())
        else {
            return self.skip_row(lines.len());
        };
        if self.backwards {
            self.candidate.col = self.candidate.col.min(max_start);
        } else if self.candidate.col > max_start {
            return self.skip_row(lines.len());
        }

        if self.wrapped && self.candidate.row == self.origin.row {
            let boundary = self.origin.col.min(max_start);
            if (!self.backwards && self.candidate.col > boundary)
                || (self.backwards && self.candidate.col < boundary)
            {
                return self.finish();
            }
        }

        self.compare_offset = Some(0);
        SearchStep::Continue
    }
}

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
        self.find_count(backwards, 1);
    }

    pub(super) fn find_count(&mut self, backwards: bool, count: usize) {
        self.start_find(backwards, count, true);
    }

    pub(super) fn find_repeat(&mut self, backwards: bool, count: usize) {
        self.start_find(backwards, count, false);
    }

    fn start_find(&mut self, backwards: bool, count: usize, remember_direction: bool) {
        self.horizontal_scroll_hold = false;
        if remember_direction {
            self.search_backwards = backwards;
        }
        self.search_task = None;
        let needle: Vec<char> = self.search.chars().collect();
        if needle.is_empty() {
            return;
        }
        self.search_task = Some(SearchTask {
            needle,
            backwards,
            origin: self.cursor,
            candidate: self.cursor,
            wrapped: false,
            compare_offset: None,
            advance_candidate: true,
            remaining: count.max(1),
            last_match: None,
        });
    }

    pub fn advance_search(&mut self) {
        for _ in 0..SEARCH_STEPS_PER_FRAME {
            let Some(mut task) = self.search_task.take() else {
                return;
            };
            match task.step(&self.lines) {
                SearchStep::Continue => self.search_task = Some(task),
                SearchStep::Found(position) => {
                    self.cursor = position;
                    self.preferred_col = None;
                    self.char_find_highlight = None;
                    self.message = if task.backwards {
                        "Backward search match".into()
                    } else {
                        "Forward search match".into()
                    };
                    return;
                }
                SearchStep::Done => {
                    self.message = "Pattern not found".into();
                    return;
                }
            }
        }
    }

    pub fn submit_prompt(&mut self) {
        let prompt = self.prompt.clone();
        let searching = self.mode == Mode::Search;
        let search_backwards = self.search_prompt_backwards;
        self.escape();
        if searching {
            if !prompt.is_empty() {
                self.search = prompt;
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
        if let Some(shell_command) = command.strip_prefix(".!") {
            self.filter_current_line(shell_command.trim());
            return;
        }
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
                if arg.is_empty() {
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
                self.message = "hjkl · w/b/e · f/F/t/T + char (word hint) · ; repeat · gg/G · zz · i/a/o · / ? search · n/N repeat · :.!cmd · :bn/:bp".into()
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
        let output = match self.shell_process(shell_command).output() {
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

    fn filter_current_line(&mut self, shell_command: &str) {
        if shell_command.is_empty() {
            self.message = "Usage: :.!command".into();
            return;
        }
        let row = self.cursor.row;
        let mut input: String = self.lines[row].iter().collect();
        input.push('\n');
        let child = self
            .shell_process(shell_command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match child {
            Ok(child) => child,
            Err(err) => {
                self.message = format!("Could not start shell: {err}");
                return;
            }
        };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(err) => {
                self.message = format!("Shell failed: {err}");
                return;
            }
        };

        let mut replacement_text = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        let replacement = if replacement_text.is_empty() {
            Vec::new()
        } else {
            if replacement_text.ends_with('\n') {
                replacement_text.pop();
            }
            replacement_text
                .split('\n')
                .map(|line| line.chars().collect::<Vec<_>>())
                .collect()
        };
        self.checkpoint();
        self.touch();
        self.lines.splice(row..=row, replacement);
        if self.lines.is_empty() {
            self.lines.push(Vec::new());
        }
        self.cursor.row = row.min(self.lines.len() - 1);
        self.cursor.col = 0;
        self.clamp();
        let exit_code = output
            .status
            .code()
            .map(|code| format!("exit {code}"))
            .unwrap_or_else(|| "terminated".into());
        let stderr = String::from_utf8_lossy(&output.stderr);
        self.message = if stderr.is_empty() {
            format!("Filtered line through {shell_command} · {exit_code}")
        } else {
            format!("Filter {exit_code}: {}", stderr.trim())
        };
    }

    fn shell_process(&self, shell_command: &str) -> Command {
        let working_directory = self.path.as_deref().map(containing_directory);
        #[cfg(windows)]
        let mut process = {
            let mut command = Command::new(std::env::var_os("COMSPEC").unwrap_or("cmd.exe".into()));
            let windows_command = if shell_command.trim() == "pwd" {
                "cd"
            } else {
                shell_command
            };
            command.args(["/C", windows_command]);
            command
        };
        #[cfg(not(windows))]
        let mut process = {
            let mut command = Command::new(std::env::var_os("SHELL").unwrap_or("/bin/sh".into()));
            command.args(["-c", shell_command]);
            command
        };
        if let Some(directory) = working_directory {
            process.current_dir(directory);
        }
        process
    }
}

fn command_completions(prefix: &str) -> Vec<String> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let commands = [
        "!",
        "b",
        "bd",
        "bdelete",
        "bn",
        "bp",
        "buffer",
        "buffers",
        "e",
        "e!",
        "Ex",
        "Explore",
        "help",
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
    let Some(command_end) = prefix.find(char::is_whitespace) else {
        let mut matches: Vec<_> = commands
            .iter()
            .filter(|command| {
                command
                    .to_ascii_lowercase()
                    .starts_with(&prefix.to_ascii_lowercase())
            })
            .map(|command| (*command).to_owned())
            .collect();
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
    if !matches!(command, "e" | "e!" | "w" | "Ex" | "Explore") {
        return Vec::new();
    }
    let rest = &prefix[command_end..];
    let arg_offset = rest
        .find(|ch: char| !ch.is_whitespace())
        .map_or(prefix.len(), |offset| command_end + offset);
    let arg_prefix = &prefix[arg_offset..];
    let separator = arg_prefix.rfind(['/', '\\']);
    let (directory_prefix, name_prefix) = match separator {
        Some(index) if index + 1 == arg_prefix.len() => (arg_prefix, ""),
        Some(index) => (&arg_prefix[..=index], &arg_prefix[index + 1..]),
        None => ("", arg_prefix),
    };
    let directory = if directory_prefix.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(directory_prefix)
    };
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_lowercase().starts_with(&name_prefix.to_lowercase()) {
            continue;
        }
        let mut candidate = format!("{directory_prefix}{name}");
        if entry.path().is_dir() {
            candidate.push(if directory_prefix.contains('\\') {
                '\\'
            } else if directory_prefix.contains('/') {
                '/'
            } else {
                std::path::MAIN_SEPARATOR
            });
        }
        matches.push(format!("{}{candidate}", &prefix[..arg_offset]));
    }
    matches.sort_by_key(|candidate| candidate.to_lowercase());
    matches
}
