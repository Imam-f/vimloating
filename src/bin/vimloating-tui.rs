use std::{io, path::PathBuf, process::ExitCode};
use vimloating::{config::load_user_config, editor::Editor, tui};

fn run() -> io::Result<()> {
    let mut path = None;
    let mut positional = false;
    for arg in std::env::args_os().skip(1) {
        if !positional && (arg == "--help" || arg == "-h") {
            println!(
                "vimloating-tui [file or directory]\n\ni: insert · Esc: normal · :help: bindings · :w path: save · :q: quit\nTerminal paste is supported in Insert mode. F1 shows terminal controls."
            );
            return Ok(());
        }
        if !positional && arg == "--" {
            positional = true;
            continue;
        }
        if path.is_some() || (!positional && arg.to_string_lossy().starts_with('-')) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Usage: vimloating-tui [file or directory]",
            ));
        }
        path = Some(PathBuf::from(arg));
    }
    let mut editor = if let Some(path) = path {
        match Editor::open_path(path.clone()) {
            Ok(editor) => editor,
            Err(_) if !path.try_exists()? => Editor::new("", Some(path)),
            Err(err) => return Err(io::Error::other(err)),
        }
    } else {
        Editor::new(tui::WELCOME, None)
    };
    editor.theme = load_user_config().theme.unwrap_or_default();
    tui::run(editor)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("vimloating-tui: {err}");
            ExitCode::FAILURE
        }
    }
}
