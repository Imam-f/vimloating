use super::{Editor, InsertAction, Mode, RepeatChange};

impl Editor {
    pub(super) fn remember_normal_change(&mut self, count: usize, keys: &[char]) {
        if self.replaying_change {
            return;
        }
        let mut sequence = Vec::new();
        if count > 1 {
            sequence.extend(count.to_string().chars());
        }
        sequence.extend_from_slice(keys);
        self.last_change = Some(RepeatChange::Normal(sequence));
    }

    pub(super) fn repeat_last_change(&mut self, count: usize) {
        let Some(change) = self.last_change.clone() else {
            return;
        };
        for _ in 0..count {
            self.replaying_change = true;
            match &change {
                RepeatChange::Normal(keys) => {
                    for &key in keys {
                        self.normal_key(key);
                    }
                }
                RepeatChange::Insert { entry, actions } => {
                    self.begin_insert(*entry);
                    for action in actions {
                        match action {
                            InsertAction::Character(ch) => self.insert_char(*ch),
                            InsertAction::Newline => self.newline(),
                            InsertAction::Backspace => self.backspace(),
                            InsertAction::MoveBack => self.move_back_character(),
                            InsertAction::DeleteForward => self.delete_forward(),
                            InsertAction::DeletePreviousWord => self.delete_prev_word(),
                            InsertAction::Tab => self.insert_text("\t"),
                        }
                    }
                    self.escape();
                }
                RepeatChange::Indent(increase) => self.change_indent(*increase),
                RepeatChange::MoveLine(direction) => self.move_line(*direction),
                RepeatChange::Number(delta) => {
                    self.adjust_number(*delta);
                }
                RepeatChange::VisualCase {
                    linewise,
                    blockwise,
                    row_delta,
                    col_delta,
                    operation,
                } => {
                    self.mode = Mode::Visual;
                    self.visual_linewise = *linewise;
                    self.visual_blockwise = *blockwise;
                    self.anchor = self.cursor;
                    self.cursor.row = (self.cursor.row + row_delta).min(self.lines.len() - 1);
                    self.cursor.col = self.cursor.col.saturating_add_signed(*col_delta);
                    self.clamp();
                    self.change_case(*operation, 1, false);
                }
                RepeatChange::VisualDelete {
                    linewise,
                    blockwise,
                    row_delta,
                    col_delta,
                } => {
                    self.mode = Mode::Visual;
                    self.visual_linewise = *linewise;
                    self.visual_blockwise = *blockwise;
                    self.anchor = self.cursor;
                    self.cursor.row = self
                        .cursor
                        .row
                        .saturating_add(*row_delta)
                        .min(self.lines.len() - 1);
                    if !linewise {
                        self.cursor.col = self.cursor.col.saturating_add_signed(*col_delta);
                    }
                    self.clamp();
                    self.normal_key('d');
                }
            }
            self.replaying_change = false;
        }
    }
}
