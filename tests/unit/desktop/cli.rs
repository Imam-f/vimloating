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
