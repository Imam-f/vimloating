use super::render::rgb;
use super::*;
use crate::editor::Pos;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;

fn press(editor: &mut Editor, ui: &mut Ui, code: KeyCode) {
    handle_key(editor, ui, KeyEvent::new(code, KeyModifiers::NONE), 8, 20);
}

fn type_keys(editor: &mut Editor, ui: &mut Ui, keys: &str) {
    for ch in keys.chars() {
        press(editor, ui, KeyCode::Char(ch));
    }
}

fn control(editor: &mut Editor, ui: &mut Ui, ch: char) {
    handle_key(
        editor,
        ui,
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL),
        8,
        20,
    );
}

mod completion;
mod editing;
mod history;
mod navigation;
mod render;
