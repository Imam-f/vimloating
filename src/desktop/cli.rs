use std::{ffi::OsString, path::PathBuf};

pub const USAGE: &str = "vimloating [--screenshot output.png] [--] [file or directory]";
pub const HELP: &str = "Usage: vimloating [--screenshot output.png] [--] [file or directory]

Open a UTF-8 file or browse a directory. A missing file starts an empty buffer.
Without a path, open the introduction buffer.

  -h, --help               Show this help without opening a window
  --screenshot output.png  Capture a frame and exit
  --                       Treat remaining arguments as paths, even with a leading '-'

Wheel: zoom · right drag: orbit · middle drag: pan
i: insert · Esc: normal · :help: bindings · :w path: save";

#[derive(Debug, PartialEq, Eq)]
pub enum DesktopArgs {
    Help,
    Run {
        path: Option<PathBuf>,
        screenshot: Option<PathBuf>,
    },
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<DesktopArgs, String> {
    let mut args = args.into_iter();
    let mut path = None;
    let mut screenshot = None;
    let mut positional = false;
    while let Some(arg) = args.next() {
        if !positional {
            if arg == "--help" || arg == "-h" {
                return Ok(DesktopArgs::Help);
            }
            if arg == "--" {
                positional = true;
                continue;
            }
            if arg == "--screenshot" {
                if screenshot.is_some() {
                    return Err("--screenshot may only be specified once".into());
                }
                let destination = args
                    .next()
                    .filter(|value| !value.is_empty() && !value.to_string_lossy().starts_with('-'))
                    .ok_or(
                        "--screenshot requires an output path; use ./ for a path starting with '-'",
                    )?;
                screenshot = Some(PathBuf::from(destination));
                continue;
            }
            if arg.to_string_lossy().starts_with('-') {
                return Err(format!(
                    "Unknown option: {} · use -- before a path starting with '-'",
                    arg.to_string_lossy()
                ));
            }
        }
        if arg.is_empty() {
            return Err("File or directory path cannot be empty".into());
        }
        if path.is_some() {
            return Err(format!(
                "Only one file or directory may be opened at startup; unexpected argument: {}",
                arg.to_string_lossy()
            ));
        }
        path = Some(PathBuf::from(arg));
    }
    Ok(DesktopArgs::Run { path, screenshot })
}

#[cfg(test)]
#[path = "../../tests/unit/desktop/cli.rs"]
mod tests;
