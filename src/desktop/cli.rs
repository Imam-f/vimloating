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
mod tests {
    use super::*;

    fn arguments(values: &[&str]) -> Result<DesktopArgs, String> {
        parse(values.iter().map(OsString::from))
    }

    #[test]
    fn startup_paths_and_screenshots_can_be_combined_in_either_order() {
        assert_eq!(
            arguments(&[]).unwrap(),
            DesktopArgs::Run {
                path: None,
                screenshot: None
            }
        );
        for values in [
            &["日本語 notes.rs", "--screenshot", "my preview.png"][..],
            &["--screenshot", "my preview.png", "日本語 notes.rs"][..],
        ] {
            assert_eq!(
                arguments(values).unwrap(),
                DesktopArgs::Run {
                    path: Some(PathBuf::from("日本語 notes.rs")),
                    screenshot: Some(PathBuf::from("my preview.png")),
                }
            );
        }
        assert_eq!(
            arguments(&["."]).unwrap(),
            DesktopArgs::Run {
                path: Some(PathBuf::from(".")),
                screenshot: None
            }
        );
        assert_eq!(
            arguments(&["--screenshot", "preview.png"]).unwrap(),
            DesktopArgs::Run {
                path: None,
                screenshot: Some(PathBuf::from("preview.png"))
            }
        );
    }

    #[test]
    fn help_and_option_terminator_are_distinguished_from_paths() {
        for values in [&["--help"][..], &["-h"][..], &["notes.rs", "--help"][..]] {
            assert_eq!(arguments(values).unwrap(), DesktopArgs::Help);
        }
        for path in ["--help", "--screenshot", "--", "-notes.rs"] {
            assert_eq!(
                arguments(&["--", path]).unwrap(),
                DesktopArgs::Run {
                    path: Some(PathBuf::from(path)),
                    screenshot: None
                }
            );
        }
        assert_eq!(
            arguments(&["--screenshot", "preview.png", "--", "-notes.rs"]).unwrap(),
            DesktopArgs::Run {
                path: Some(PathBuf::from("-notes.rs")),
                screenshot: Some(PathBuf::from("preview.png")),
            }
        );
    }

    #[test]
    fn invalid_arguments_are_rejected_instead_of_becoming_filenames() {
        for values in [
            &["--unknown"][..],
            &["notes.rs", "--unknown"][..],
            &["one.rs", "two.rs"][..],
            &["--", "one.rs", "two.rs"][..],
            &[""][..],
        ] {
            assert!(arguments(values).is_err(), "{values:?}");
        }
        for values in [
            &["--screenshot"][..],
            &["--screenshot", ""][..],
            &["--screenshot", "--help"][..],
            &["--screenshot", "--"][..],
            &["--screenshot", "one.png", "--screenshot", "two.png"][..],
        ] {
            assert!(arguments(values).unwrap_err().contains("--screenshot"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_paths_are_preserved() {
        use std::os::unix::ffi::OsStringExt;
        let path = OsString::from_vec(b"notes-\xff.rs".to_vec());
        let screenshot = OsString::from_vec(b"preview-\xfe.png".to_vec());
        assert_eq!(
            parse([path.clone(), "--screenshot".into(), screenshot.clone()]).unwrap(),
            DesktopArgs::Run {
                path: Some(PathBuf::from(path)),
                screenshot: Some(PathBuf::from(screenshot)),
            }
        );
    }
}
