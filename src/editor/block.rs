use super::Editor;

impl Editor {
    pub(super) fn block_action(&mut self, delete: bool) {
        let (a, b) = self.selection();
        let width = b.col - a.col + 1;
        self.register = (a.row..=b.row)
            .map(|row| {
                let line = &self.lines[row];
                let mut slice = line[a.col.min(line.len())..(b.col + 1).min(line.len())].to_vec();
                slice.resize(width, ' ');
                slice
            })
            .collect();
        self.linewise = false;
        self.register_block_width = Some(width);
        if delete {
            self.checkpoint();
            self.touch();
            for row in a.row..=b.row {
                let len = self.lines[row].len();
                self.lines[row].drain(a.col.min(len)..(b.col + 1).min(len));
            }
        } else {
            self.blink_yank(
                (a.row..=b.row).map(|row| (row, a.col..b.col + 1)).collect(),
                false,
            );
        }
        self.cursor = a;
        self.message = format!(
            "{} block: {} × {}",
            if delete { "Deleted" } else { "Yanked" },
            width,
            b.row - a.row + 1
        );
        self.escape();
    }

    pub(super) fn paste_block(&mut self, before: bool, count: usize, width: usize) {
        let col = self.cursor.col + usize::from(!before && !self.lines[self.cursor.row].is_empty());
        for (offset, contents) in self.register.iter().enumerate() {
            let row = self.cursor.row + offset;
            while self.lines.len() <= row {
                self.lines.push(Vec::new());
            }
            let line = &mut self.lines[row];
            if line.len() < col {
                line.resize(col, ' ');
            }
            let mut text = contents.clone();
            text.resize(width, ' ');
            let repeated: Vec<_> = (0..count).flat_map(|_| text.iter().copied()).collect();
            line.splice(col..col, repeated);
        }
        self.cursor.col = col;
    }
}
