use super::{rgb, safe_text};
use crate::editor::{Editor, Mode};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};
pub(super) fn draw_completion_popup(
    frame: &mut Frame,
    editor: &Editor,
    content: Rect,
    gutter: usize,
    cols: usize,
    wrap: bool,
) {
    let Some(popup) = editor.completion_popup() else {
        return;
    };
    let (anchor_col, cursor_row) = if editor.mode == Mode::Command {
        (0, content.height as usize)
    } else {
        let index = editor.display_index(editor.cursor, cols, wrap);
        let Some(row) = index
            .checked_sub(editor.top)
            .filter(|&row| row < content.height as usize)
        else {
            return;
        };
        let start = if wrap {
            editor.cursor.col / cols * cols
        } else {
            editor.left
        };
        (popup.anchor.col.saturating_sub(start), row)
    };
    let Some(layout) = popup.layout(anchor_col, cursor_row, cols, content.height as usize) else {
        return;
    };
    let area = Rect::new(
        content.x + gutter as u16 + layout.x as u16,
        content.y + layout.y as u16,
        layout.width as u16,
        layout.height as u16,
    );
    let palette = editor.theme.palette();
    let base = Style::default()
        .fg(rgb(palette.text))
        .bg(rgb(palette.status));
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(
                popup
                    .title()
                    .chars()
                    .take(layout.width - 2)
                    .collect::<String>(),
            )
            .border_style(base.fg(rgb(palette.accent)))
            .style(base),
        area,
    );
    for offset in 0..layout.visible {
        let index = layout.first + offset;
        let text = safe_text(&popup.candidate_text(index, layout.width - 2));
        let style = if index == popup.selected {
            base.bg(rgb(palette.accent))
                .fg(rgb(palette.cursor_text))
                .add_modifier(Modifier::BOLD)
        } else {
            base
        };
        frame.render_widget(
            Paragraph::new(text).style(style),
            Rect::new(area.x + 1, area.y + 1 + offset as u16, area.width - 2, 1),
        );
    }
}
