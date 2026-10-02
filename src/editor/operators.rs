use super::{Editor, case::CaseChange};

#[derive(Clone, Copy)]
pub(super) enum TextOperator {
    Delete,
    Yank,
    Case(CaseChange),
}

impl TextOperator {
    pub fn line_key(self) -> char {
        match self {
            Self::Delete => 'd',
            Self::Yank => 'y',
            Self::Case(CaseChange::Lower) => 'u',
            Self::Case(CaseChange::Upper) => 'U',
            Self::Case(CaseChange::Toggle) => '~',
        }
    }

    fn keys(self) -> &'static [char] {
        match self {
            Self::Delete => &['d'],
            Self::Yank => &['y'],
            Self::Case(CaseChange::Lower) => &['g', 'u'],
            Self::Case(CaseChange::Upper) => &['g', 'U'],
            Self::Case(CaseChange::Toggle) => &['g', '~'],
        }
    }
}

impl Editor {
    pub(super) fn apply_line_operator(&mut self, operator: TextOperator, count: usize) {
        match operator {
            TextOperator::Delete => {
                self.remember_normal_change(count, &['d', 'd']);
                self.line_action(true, count);
            }
            TextOperator::Yank => self.line_action(false, count),
            TextOperator::Case(operation) => {
                self.change_case(operation, count, true);
            }
        }
    }

    pub(super) fn apply_text_object_operator(
        &mut self,
        operator: TextOperator,
        around: bool,
        object: char,
        count: usize,
    ) {
        if self.is_directory_browser() {
            self.message = "Directory listing is read-only".into();
            return;
        }
        let origin = self.cursor;
        if !self.select_text_object(object, around, count) {
            return;
        }
        let changed = match operator {
            TextOperator::Delete => {
                self.visual_action(true);
                true
            }
            TextOperator::Yank => {
                self.visual_action(false);
                self.cursor = origin;
                self.clamp();
                false
            }
            TextOperator::Case(operation) => self.change_case(operation, 1, false),
        };
        if changed {
            let mut keys = operator.keys().to_vec();
            keys.extend([if around { 'a' } else { 'i' }, object]);
            self.remember_normal_change(count, &keys);
        }
    }
}
