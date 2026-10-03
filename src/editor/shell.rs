use super::{Editor, Mode, files::containing_directory};
use std::{
    io::Write,
    process::{Command, Stdio},
};

impl Editor {
    pub(super) fn run_shell(&mut self, shell_command: &str) {
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

    pub(super) fn filter_current_line(&mut self, shell_command: &str) {
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
