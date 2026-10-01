mod config;
mod editor;
mod input;
mod render;
mod view;

use config::window_conf;
use config::*;
use editor::{Editor, Mode, Pos, buffers::BufferList};
use input::{ScrollRepeat, VerticalMotion, advance_vertical_motion, handle_keyboard};
use macroquad::prelude::*;
use std::path::PathBuf;
use view::View;

#[macroquad::main(window_conf)]
async fn main() {
    let mut args = std::env::args_os().skip(1);
    let mut path = None;
    let mut screenshot = None;
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            let Some(destination) = args.next() else {
                eprintln!("Usage: vimloating [file] [--screenshot output.png]");
                return;
            };
            screenshot = Some(PathBuf::from(destination));
        } else if arg == "--help" || arg == "-h" {
            println!(
                "vimloating [file] [--screenshot output.png]\n\nWheel: zoom · right drag: orbit · middle drag: pan\ni: insert · Esc: normal · :help: bindings · :w path: save"
            );
            return;
        } else if path.is_none() {
            path = Some(PathBuf::from(arg));
        } else {
            eprintln!("Unexpected argument: {}", arg.to_string_lossy());
            return;
        }
    }

    let mut editor = if let Some(path) = path {
        match Editor::open_path(path.clone()) {
            Ok(editor) => editor,
            Err(_) if !path.exists() => Editor::new("", Some(path)),
            Err(err) => {
                eprintln!("Cannot open {}: {err}", path.display());
                return;
            }
        }
    } else {
        Editor::new(WELCOME, None)
    };
    let user_config = load_user_config();
    editor.theme = user_config.theme.unwrap_or_default();
    let mut buffers = BufferList::new(editor);

    let font = render::system_font();
    let (target_width, target_height) = render::editor_target_dimensions();
    let mut target = render::create_editor_target(target_width, target_height);
    let mut board = render::board_mesh(target.texture.clone());
    let mut view = View::new(user_config.two_d_only.unwrap_or(true));
    let mut help = false;
    let mut show_floor = false;
    let mut word_wrap = true;
    let mut font_size = BASE_FONT_SIZE;
    let mut last_normal_enter = None;
    let mut vertical_motion: Option<VerticalMotion> = None;
    let mut scroll_repeat: Option<ScrollRepeat> = None;
    let mut last_insert_activity = get_time();
    let mut frame = 0;
    prevent_quit();

    loop {
        buffers.process_pending();
        let editor = buffers.active_mut();
        let (target_width, target_height) = render::editor_target_dimensions();
        if target.texture.width() != target_width as f32
            || target.texture.height() != target_height as f32
        {
            target = render::create_editor_target(target_width, target_height);
            board = render::board_mesh(target.texture.clone());
        }
        let mouse_position = vec2(mouse_position().0, mouse_position().1);
        let mouse_moved = mouse_position.distance_squared(view.last_mouse) > 0.25;
        let (wheel_x, wheel_y) = mouse_wheel();
        let input_activity = is_any_key_down()
            || mouse_moved
            || is_mouse_button_down(MouseButton::Left)
            || is_mouse_button_down(MouseButton::Middle)
            || is_mouse_button_down(MouseButton::Right)
            || wheel_x != 0.0
            || wheel_y != 0.0;
        view.update();

        let cursor_before_keys = editor.cursor;
        let mode_before_keys = editor.mode;
        let directory_before_keys = editor.is_directory_browser();
        handle_keyboard(
            editor,
            &mut font_size,
            &mut word_wrap,
            &mut vertical_motion,
            &mut scroll_repeat,
        );
        editor.advance_search();
        advance_vertical_motion(editor, &mut vertical_motion);
        if editor.mode == Mode::Insert {
            let now = get_time();
            if mode_before_keys != Mode::Insert || input_activity {
                last_insert_activity = now;
            } else if now - last_insert_activity >= INSERT_IDLE_TIMEOUT {
                editor.escape();
                editor.message = "Idle timeout · returned to Normal mode".into();
            }
        } else {
            last_insert_activity = get_time();
        }
        if mode_before_keys == Mode::Normal
            && editor.mode == Mode::Normal
            && !directory_before_keys
            && is_key_pressed(KeyCode::Enter)
        {
            let now = get_time();
            if last_normal_enter.is_some_and(|last| now - last <= 0.45) {
                word_wrap = !word_wrap;
                editor.left = 0;
                editor.message = format!("Word wrap {}", if word_wrap { "on" } else { "off" });
                last_normal_enter = None;
            } else {
                last_normal_enter = Some(now);
            }
        }
        if is_quit_requested() {
            editor.command("q");
        }
        if is_key_pressed(KeyCode::F1) {
            help = !help;
        }
        if is_key_pressed(KeyCode::F3) {
            show_floor = !show_floor;
        }

        let (visible_rows, visible_cols, cell_width, line_height) = render::text_grid(font_size);
        if is_mouse_button_pressed(MouseButton::Left)
            && !matches!(
                editor.mode,
                Mode::Command | Mode::Search | Mode::ShellOutput | Mode::BufferList
            )
            && let Some(point) = view.pick()
            && point.x >= TEXT_X
            && point.y >= TEXT_Y
            && point.y < TEXT_Y + visible_rows as f32 * line_height
        {
            vertical_motion = None;
            editor.follow_cursor_horizontally();
            let display_index = editor.top + ((point.y - TEXT_Y) / line_height) as usize;
            if let Some(&(row, segment_start)) = editor
                .display_window(display_index, 1, visible_cols, word_wrap)
                .first()
            {
                let horizontal_offset = if word_wrap { 0 } else { editor.left };
                editor.cancel_search();
                editor.cursor = Pos {
                    row,
                    col: segment_start
                        + horizontal_offset
                        + ((point.x - TEXT_X) / cell_width) as usize,
                };
                editor.clamp();
                editor.char_find_highlight =
                    (editor.cursor.col < editor.lines[row].len()).then_some(editor.cursor);
                editor.char_find_hints.clear();
            }
        }
        editor.reveal_cursor(visible_rows, visible_cols, word_wrap);
        if editor.cursor != cursor_before_keys {
            view.follow_cursor(render::cursor_board_position(editor, font_size, word_wrap));
        }
        buffers.protect_quit();
        let editor = buffers.active();
        if editor.quit {
            break;
        }

        let render_scale = target.texture.width() / TEX_W as f32;
        render::draw_buffer(
            editor,
            &target,
            font.as_ref(),
            font_size,
            word_wrap,
            render_scale,
        );
        if view.two_d_only {
            render::draw_2d_only(&target.texture, editor.theme);
        } else {
            render::draw_world(&view, &board, show_floor, editor.theme);
            render::draw_overlay(help, view.flat_only, font.as_ref(), editor.theme);
        }
        frame += 1;
        if frame == 10
            && let Some(path) = &screenshot
        {
            get_screen_data().export_png(&path.to_string_lossy());
            println!("OpenGL frame captured: {}", path.display());
            break;
        }
        next_frame().await;
    }
}
