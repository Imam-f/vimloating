use crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io;
/// Restores terminal state on normal return, I/O errors, and unwinding panics.
pub(super) struct TerminalSession;

impl TerminalSession {
    pub(super) fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let session = Self;
        execute!(io::stdout(), EnterAlternateScreen)?;
        // These enhancements are optional on older Windows console hosts.
        let _ = execute!(io::stdout(), EnableBracketedPaste);
        let _ = execute!(io::stdout(), SetCursorStyle::BlinkingBar);
        Ok(session)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableBracketedPaste);
        let _ = execute!(io::stdout(), SetCursorStyle::DefaultUserShape);
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}
