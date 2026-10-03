use super::motion::advance_vertical_motion_input;
use super::repeat::repeat_due;
use super::*;

struct KeyboardHarness {
    font_size: u16,
    wrap: bool,
    motion: Option<VerticalMotion>,
    repeat: Option<KeyRepeat>,
}

impl KeyboardHarness {
    fn new() -> Self {
        Self {
            font_size: vimloating::config::BASE_FONT_SIZE,
            wrap: true,
            motion: None,
            repeat: None,
        }
    }

    fn frame(
        &mut self,
        editor: &mut Editor,
        now: f64,
        down: &[KeyCode],
        pressed: &[KeyCode],
        chars: &str,
    ) {
        let input = KeyboardInput {
            pressed: pressed.iter().copied().collect(),
            down: down.iter().copied().collect(),
            chars: chars.chars().collect(),
            now,
            clipboard: None,
        };
        handle_keyboard_input(
            editor,
            &mut self.font_size,
            &mut self.wrap,
            &mut self.motion,
            &mut self.repeat,
            &input,
        );
        let repeat_held = input.ctrl()
            && self
                .motion
                .as_ref()
                .is_some_and(|motion| input.down.contains(&motion.repeat_key));
        advance_vertical_motion_input(editor, &mut self.motion, now, repeat_held);
    }

    fn hold(&mut self, editor: &mut Editor, key: KeyCode, modifiers: &[KeyCode]) {
        let mut down = modifiers.to_vec();
        down.push(key);
        let initial_press = [key];
        for (time, pressed) in [(0.0, true), (0.1, false), (0.35, false), (0.42, false)] {
            self.frame(
                editor,
                time,
                &down,
                if pressed { &initial_press } else { &[] },
                "",
            );
        }
        self.frame(editor, 0.5, &[], &[], "");
    }
}

mod completion;
mod editing;
mod navigation;
mod repeat;
