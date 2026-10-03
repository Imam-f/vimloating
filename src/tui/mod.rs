mod help;
mod input;
mod render;
mod session;

use crate::{
    config::INSERT_IDLE_TIMEOUT,
    editor::{Editor, Mode, buffers::BufferList},
};
use crossterm::event::{self, Event, KeyEventKind};
use help::HELP;
pub use help::WELCOME;
use input::{handle_key, handle_paste};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};
use render::draw;
use session::TerminalSession;
use std::{
    io::{self, IsTerminal},
    time::{Duration, Instant},
};
#[derive(Default)]
struct Ui {
    wrap: bool,
    help: bool,
    output_top: usize,
    last_enter: Option<Instant>,
}

fn grid(area: Rect, editor: &Editor) -> (usize, usize, usize) {
    let gutter =
        (editor.lines.len().to_string().len() + 2).min(area.width.saturating_sub(1) as usize);
    (
        area.height.saturating_sub(2).max(1) as usize,
        (area.width as usize).saturating_sub(gutter).max(1),
        gutter,
    )
}

pub fn run(editor: Editor) -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "an interactive terminal is required",
        ));
    }
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut buffers = BufferList::new(editor);
    let mut ui = Ui {
        wrap: true,
        ..Ui::default()
    };
    let mut last_activity = Instant::now();
    let mut redraw = true;
    loop {
        buffers.process_pending();
        buffers.protect_quit();
        if buffers.active().quit {
            break;
        }
        let size = terminal.size()?;
        let viewport = buffers.active();
        let viewport = if viewport.mode == Mode::CommandWindow {
            viewport.command_window.as_deref().unwrap_or(viewport)
        } else {
            viewport
        };
        let (rows, cols, _) = grid(Rect::new(0, 0, size.width, size.height), viewport);
        let editor = buffers.active_mut();
        let (changed, animating) = {
            let active = if editor.mode == Mode::CommandWindow {
                editor
                    .command_window
                    .as_deref_mut()
                    .expect("history editor")
            } else {
                &mut *editor
            };
            let previous = (active.cursor, active.mode, active.message.clone());
            active.advance_search();
            if active.mode == Mode::Insert
                && last_activity.elapsed().as_secs_f64() >= INSERT_IDLE_TIMEOUT
            {
                active.escape();
                active.message = "Idle timeout · returned to Normal mode".into();
            }
            active.reveal_cursor(rows, cols, ui.wrap);
            (
                previous != (active.cursor, active.mode, active.message.clone()),
                active.yank_blink_active(),
            )
        };
        if redraw || changed || animating {
            terminal.draw(|frame| draw(frame, editor, &ui))?;
            redraw = false;
        }
        if event::poll(Duration::from_millis(30))? {
            last_activity = Instant::now();
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    handle_key(editor, &mut ui, key, rows, cols);
                    redraw = true;
                }
                Event::Paste(text) if !ui.help => {
                    handle_paste(editor, &text);
                    redraw = true;
                }
                Event::Resize(_, _) => redraw = true,
                _ => {}
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
