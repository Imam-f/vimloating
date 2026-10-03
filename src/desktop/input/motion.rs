use macroquad::prelude::*;
use vimloating::{config::VERTICAL_MOTION_STEP, editor::Editor};

pub struct VerticalMotion {
    pub(super) direction: isize,
    pub(super) remaining: usize,
    pub(super) next_step: f64,
    pub(super) repeat_key: KeyCode,
}

pub fn advance_vertical_motion(editor: &mut Editor, vertical_motion: &mut Option<VerticalMotion>) {
    let now = get_time();
    let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let repeat_held = ctrl
        && vertical_motion
            .as_ref()
            .is_some_and(|motion| is_key_down(motion.repeat_key));
    advance_vertical_motion_input(editor, vertical_motion, now, repeat_held);
}

pub(super) fn advance_vertical_motion_input(
    editor: &mut Editor,
    vertical_motion: &mut Option<VerticalMotion>,
    now: f64,
    repeat_held: bool,
) {
    let Some(motion) = vertical_motion.as_mut() else {
        return;
    };
    while motion.remaining > 0 && now >= motion.next_step {
        editor.move_by(0, motion.direction, 1);
        motion.remaining -= 1;
        motion.next_step += VERTICAL_MOTION_STEP;
    }
    if motion.remaining == 0 {
        if repeat_held {
            motion.remaining = 5;
        } else {
            *vertical_motion = None;
        }
    }
}
