#![cfg(feature = "graphics")]

use std::process::Command;

fn desktop() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vimloating"));
    // CLI help and errors must work without a graphical display.
    command.env_remove("DISPLAY").env_remove("WAYLAND_DISPLAY");
    command
}

#[test]
fn help_exits_successfully_before_opening_a_path_or_renderer() {
    for args in [
        &["--help"][..],
        &["-h"][..],
        &["missing-parent/missing-file.rs", "--help"][..],
    ] {
        let output = desktop().args(args).output().unwrap();
        assert!(output.status.success(), "{args:?}: {output:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("file or directory"));
        assert!(stdout.contains("--screenshot"));
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn invalid_arguments_exit_with_diagnostics_and_failure_status() {
    for (args, message) in [
        (&["--unknown"][..], "Unknown option"),
        (&["one.rs", "two.rs"][..], "Only one file or directory"),
        (&["--screenshot"][..], "requires an output path"),
        (&["--screenshot", "--help"][..], "requires an output path"),
        (
            &["--screenshot", "one.png", "--screenshot", "two.png"][..],
            "may only be specified once",
        ),
    ] {
        let output = desktop().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(message), "{args:?}: {stderr}");
        assert!(stderr.contains("Usage: vimloating"));
    }
}

#[test]
fn invalid_utf8_file_exits_before_starting_the_renderer() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "vimloating-invalid-utf8-{}-{unique}.rs",
        std::process::id()
    ));
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    let output = desktop().arg(&path).output();
    std::fs::remove_file(&path).unwrap();
    let output = output.unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Cannot open")
    );
}
