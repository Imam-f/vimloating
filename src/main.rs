mod editor;

use editor::{Editor, Mode, Pos};
use macroquad::camera::Camera;
use macroquad::prelude::*;
use std::path::PathBuf;

const TEX_W: u32 = 1600;
const TEX_H: u32 = 1000;
const MIN_RENDER_WIDTH: u32 = TEX_W * 2;
const MAX_RENDER_WIDTH: u32 = 8192;
const BOARD_W: f32 = 12.0;
const BOARD_H: f32 = 7.5;
const CELL: f32 = 17.0;
const LINE: f32 = 36.0;
const TEXT_X: f32 = 112.0;
const TEXT_Y: f32 = 70.0;
const BASE_FONT_SIZE: u16 = 27;
const ZOOM_SENSITIVITY: f32 = 0.004;
const ZOOM_SMOOTHING: f32 = 3.5;
const VERTICAL_MOTION_STEP: f64 = 0.055;
const INSERT_IDLE_TIMEOUT: f64 = 15.0;
const INK: Color = Color::new(0.80, 0.85, 0.90, 1.0);
const MUTED: Color = Color::new(0.39, 0.48, 0.57, 1.0);
const ACCENT: Color = Color::new(0.39, 0.89, 0.77, 1.0);

const WELCOME: &str = r#"// vimloating — a little room for your thoughts

fn main() {
    let idea = "Text doesn't have to live in a window.";
    println!("{idea}");
}

// Start exploring
//   i              enter insert mode
//   Esc            return to normal mode
//   h j k l        move left, down, up, right
//   w b e          move by word
//   gg / G         first / last line
//   dd / yy / p    delete / yank / paste a line
//   u / Ctrl-R     undo / redo
//   v              select text, then y or d
//   /text          search; n / N for next / previous
//   :w notes.rs    save this buffer
//   :e notes.rs    open a UTF-8 file

// Space is yours
//   Wheel          gentle zoom
//   Right drag     orbit (F4 turns orbit off)
//   Middle drag    pan the camera
//   F2             return to the straight-on view
//   F3             toggle the floor grid
//   F4             toggle flat-only mode (default on)
//   Ctrl+- / =     change font size
//   Ctrl+W / Ctrl+Backspace delete previous word in Insert mode
//   Ctrl+J / K     animate five-line movement
//   Idle 15s       return to Normal mode
//   Enter Enter    toggle word wrap in Normal mode
//   Ctrl+H / L     scroll horizontally (turns wrap off)
//   F1             toggle the controls overlay

// Make something worth keeping.
"#;

fn window_conf() -> Conf {
    Conf {
        window_title: "vimloating".into(),
        window_width: 1120,
        window_height: 760,
        high_dpi: true,
        sample_count: 4,
        window_resizable: true,
        ..Default::default()
    }
}

struct View {
    distance: f32,
    desired_distance: f32,
    yaw: f32,
    desired_yaw: f32,
    pitch: f32,
    desired_pitch: f32,
    center: Vec3,
    desired_center: Vec3,
    last_mouse: Vec2,
    flat_only: bool,
    right_pan: bool,
}

struct VerticalMotion {
    direction: isize,
    remaining: usize,
    next_step: f64,
}

impl View {
    fn new() -> Self {
        Self {
            distance: 12.8,
            desired_distance: 12.8,
            yaw: 0.0,
            desired_yaw: 0.0,
            pitch: 0.0,
            desired_pitch: 0.0,
            center: Vec3::ZERO,
            desired_center: Vec3::ZERO,
            last_mouse: vec2(mouse_position().0, mouse_position().1),
            flat_only: true,
            right_pan: false,
        }
    }

    fn update(&mut self) {
        if is_mouse_button_pressed(MouseButton::Right) {
            self.right_pan = self.pick().is_none();
        } else if !is_mouse_button_down(MouseButton::Right) {
            self.right_pan = false;
        }
        let mouse = vec2(mouse_position().0, mouse_position().1);
        let delta = mouse - self.last_mouse;
        self.last_mouse = mouse;
        let (_, wheel) = mouse_wheel();
        self.desired_distance =
            (self.desired_distance * (-wheel * ZOOM_SENSITIVITY).exp()).clamp(3.0, 42.0);
        if is_mouse_button_down(MouseButton::Right) && !self.right_pan {
            self.desired_yaw = (self.desired_yaw - delta.x * 0.004).clamp(-1.1, 1.1);
            self.desired_pitch = (self.desired_pitch + delta.y * 0.004).clamp(-0.85, 0.85);
        }
        if is_mouse_button_down(MouseButton::Middle)
            || (is_mouse_button_down(MouseButton::Right) && self.right_pan)
        {
            let scale = self.distance * 0.0009;
            self.desired_center += vec3(-delta.x * scale, delta.y * scale, 0.0);
        }
        if is_key_pressed(KeyCode::F2) {
            self.desired_distance = 12.8;
            self.desired_yaw = 0.0;
            self.desired_pitch = 0.0;
            self.desired_center = Vec3::ZERO;
        }
        if is_key_pressed(KeyCode::F4) {
            self.flat_only = !self.flat_only;
            if self.flat_only {
                self.yaw = 0.0;
                self.pitch = 0.0;
                self.desired_yaw = 0.0;
                self.desired_pitch = 0.0;
            }
        }
        if self.flat_only {
            // Keep the text plane face-on while allowing its view position to pan.
            self.desired_yaw = 0.0;
            self.desired_pitch = 0.0;
        }
        // Frame-rate independent exponential convergence: no wheel-step snapping.
        let blend = 1.0 - (-12.0 * get_frame_time().min(0.1)).exp();
        let zoom_blend = 1.0 - (-ZOOM_SMOOTHING * get_frame_time().min(0.1)).exp();
        self.distance += (self.desired_distance - self.distance) * zoom_blend;
        self.yaw += (self.desired_yaw - self.yaw) * blend;
        self.pitch += (self.desired_pitch - self.pitch) * blend;
        self.center = self.center.lerp(self.desired_center, blend);
    }

    fn camera(&self) -> Camera3D {
        let offset = vec3(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        let aspect = screen_width() / screen_height().max(1.0);
        let fit = (1.35 / aspect).max(1.0);
        Camera3D {
            position: self.center + offset * self.distance * fit,
            target: self.center,
            up: vec3(0.0, 1.0, 0.0),
            fovy: 45.0_f32.to_radians(),
            z_near: 0.1,
            z_far: 150.0,
            ..Default::default()
        }
    }

    fn pick(&self) -> Option<Vec2> {
        let inverse = self.camera().matrix().inverse();
        let (mx, my) = mouse_position();
        let ndc = vec2(
            2.0 * mx / screen_width() - 1.0,
            1.0 - 2.0 * my / screen_height(),
        );
        let near = inverse.project_point3(vec3(ndc.x, ndc.y, -1.0));
        let far = inverse.project_point3(vec3(ndc.x, ndc.y, 1.0));
        let ray = far - near;
        if ray.z.abs() < 0.0001 {
            return None;
        }
        let t = -near.z / ray.z;
        if t < 0.0 {
            return None;
        }
        let p = near + ray * t;
        if p.x.abs() > BOARD_W / 2.0 || p.y.abs() > BOARD_H / 2.0 {
            return None;
        }
        Some(vec2(
            (p.x / BOARD_W + 0.5) * TEX_W as f32,
            (0.5 - p.y / BOARD_H) * TEX_H as f32,
        ))
    }
}

fn system_font() -> Option<Font> {
    // Prefer a real monospace font with broad Unicode coverage, with a built-in fallback.
    for path in [
        "C:/Windows/Fonts/consola.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
    ] {
        if let Ok(bytes) = std::fs::read(path)
            && let Ok(font) = load_ttf_font_from_bytes(&bytes)
        {
            return Some(font);
        }
    }
    None
}

fn label(text: &str, x: f32, y: f32, size: u16, color: Color, font: Option<&Font>) {
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font,
            font_size: size,
            color,
            ..Default::default()
        },
    );
}

fn ui_label(text: &str, x: f32, y: f32, size: u16, color: Color, font: Option<&Font>, scale: f32) {
    label(
        text,
        x * scale,
        y * scale,
        (size as f32 * scale).round().clamp(1.0, u16::MAX as f32) as u16,
        color,
        font,
    );
}

fn ui_line(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color, scale: f32) {
    draw_line(
        x1 * scale,
        y1 * scale,
        x2 * scale,
        y2 * scale,
        thickness * scale,
        color,
    );
}

fn ui_rectangle(x: f32, y: f32, width: f32, height: f32, color: Color, scale: f32) {
    draw_rectangle(x * scale, y * scale, width * scale, height * scale, color);
}

fn mode_color(mode: Mode) -> Color {
    match mode {
        Mode::Insert => Color::from_hex(0x83b9f5),
        Mode::Visual => Color::from_hex(0xc7a0f4),
        Mode::Command | Mode::Search => Color::from_hex(0xeec181),
        Mode::Normal => ACCENT,
    }
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "INSERT",
        Mode::Visual => "VISUAL",
        Mode::Command => "COMMAND",
        Mode::Search => "SEARCH",
    }
}

fn text_grid(font_size: u16) -> (usize, usize, f32, f32) {
    let scale = font_size as f32 / BASE_FONT_SIZE as f32;
    let cell_width = CELL * scale;
    let line_height = LINE * scale;
    let rows = ((890.0 - TEXT_Y - 8.0) / line_height).floor().max(1.0) as usize;
    let cols = ((TEX_W as f32 - TEXT_X - 30.0) / cell_width)
        .floor()
        .max(1.0) as usize;
    (rows, cols, cell_width, line_height)
}

fn draw_buffer(
    editor: &Editor,
    target: &RenderTarget,
    font: Option<&Font>,
    font_size: u16,
    word_wrap: bool,
    scale: f32,
) {
    set_camera(&Camera2D {
        render_target: Some(target.clone()),
        ..Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            target.texture.width(),
            target.texture.height(),
        ))
    });
    clear_background(Color::from_hex(0x101923));
    ui_line(
        90.0,
        55.0,
        90.0,
        871.0,
        1.0,
        Color::from_hex(0x24313e),
        scale,
    );

    let (rows, cols, cell_width, line_height) = text_grid(font_size);
    let baseline = line_height * 0.78;
    let display_rows = editor.display_rows(cols, word_wrap);
    let search: Vec<char> = editor.search.chars().collect();
    for visible in 0..rows {
        let display_index = editor.top + visible;
        let y = TEXT_Y + visible as f32 * line_height;
        let Some(&(row, segment_start)) = display_rows.get(display_index) else {
            ui_label(
                "~",
                48.0,
                y + baseline,
                23,
                Color::from_hex(0x2f4352),
                font,
                scale,
            );
            continue;
        };
        let line = &editor.lines[row];
        let visible_start = segment_start + if word_wrap { 0 } else { editor.left };
        let visible_end = (visible_start + cols).min(line.len());
        let cursor_segment = if word_wrap {
            editor.cursor.col / cols * cols
        } else {
            0
        };
        if row == editor.cursor.row && segment_start == cursor_segment {
            ui_rectangle(
                94.0,
                y - 1.0,
                1480.0,
                line_height,
                Color::from_hex(0x182735),
                scale,
            );
        }
        let num = if segment_start == 0 {
            format!("{:>4}", row + 1)
        } else {
            "   ↪".into()
        };
        ui_label(
            &num,
            18.0,
            y + baseline,
            (20.0 * font_size as f32 / BASE_FONT_SIZE as f32)
                .round()
                .max(10.0) as u16,
            if row == editor.cursor.row && segment_start == cursor_segment {
                ACCENT
            } else {
                MUTED
            },
            font,
            scale,
        );
        let comment = line
            .iter()
            .skip_while(|c| c.is_whitespace())
            .take(2)
            .collect::<String>()
            == "//";
        let mut quoted = false;
        for (col, &ch) in line.iter().enumerate() {
            if ch == '"' {
                quoted = !quoted;
            }
            if col < visible_start || col >= visible_end {
                continue;
            }
            let x = TEXT_X + (col - visible_start) as f32 * cell_width;
            if !search.is_empty() {
                let starts = col.saturating_sub(search.len() - 1)..=col;
                if starts.clone().any(|start| {
                    start + search.len() <= line.len()
                        && line[start..start + search.len()] == search
                }) {
                    ui_rectangle(
                        x,
                        y,
                        cell_width,
                        line_height - 1.0,
                        Color::from_hex(0x544729),
                        scale,
                    );
                }
            }
            if editor.mode == Mode::Visual {
                let (a, b) = editor.selection();
                let pos = Pos { row, col };
                if a <= pos && pos <= b {
                    ui_rectangle(
                        x,
                        y,
                        cell_width,
                        line_height - 1.0,
                        Color::from_hex(0x4e3e6b),
                        scale,
                    );
                }
            }
            let color = if comment {
                Color::from_hex(0x708b91)
            } else if quoted || ch == '"' {
                Color::from_hex(0xc7d69b)
            } else if ch.is_ascii_digit() {
                Color::from_hex(0xd5abf1)
            } else if "{}()[];:,.!".contains(ch) {
                Color::from_hex(0x8c9cae)
            } else {
                INK
            };
            // Fixed cells keep cursor, Unicode character indices, and hit testing aligned.
            if ch != '\t' {
                ui_label(
                    &ch.to_string(),
                    x,
                    y + baseline,
                    font_size,
                    color,
                    font,
                    scale,
                );
            } else {
                ui_label(
                    "→",
                    x,
                    y + baseline,
                    (font_size as f32 * 0.74) as u16,
                    MUTED,
                    font,
                    scale,
                );
            }
        }
    }

    let cursor_segment = if word_wrap {
        editor.cursor.col / cols * cols
    } else {
        0
    };
    if let Some(cursor_row) = display_rows
        .iter()
        .position(|&(row, segment)| row == editor.cursor.row && segment == cursor_segment)
        .filter(|&index| {
            index >= editor.top
                && index < editor.top + rows
                && (word_wrap
                    || (editor.cursor.col >= editor.left && editor.cursor.col < editor.left + cols))
        })
    {
        let visible_start = cursor_segment + if word_wrap { 0 } else { editor.left };
        let x = TEXT_X + editor.cursor.col.saturating_sub(visible_start) as f32 * cell_width;
        let y = TEXT_Y + (cursor_row - editor.top) as f32 * line_height;
        if editor.mode == Mode::Insert {
            let alpha = 0.65 + 0.35 * (get_time() as f32 * 4.0).sin().abs();
            ui_rectangle(
                x - 1.0,
                y,
                2.5,
                line_height - 1.0,
                Color {
                    a: alpha,
                    ..mode_color(editor.mode)
                },
                scale,
            );
        } else if matches!(editor.mode, Mode::Normal | Mode::Visual) {
            ui_rectangle(
                x,
                y,
                cell_width,
                line_height - 1.0,
                mode_color(editor.mode),
                scale,
            );
            if let Some(ch) = editor.lines[editor.cursor.row].get(editor.cursor.col) {
                ui_label(
                    &ch.to_string(),
                    x,
                    y + baseline,
                    font_size,
                    Color::from_hex(0x10232a),
                    font,
                    scale,
                );
            }
        }
    }

    ui_rectangle(
        0.0,
        890.0,
        TEX_W as f32,
        49.0,
        Color::from_hex(0x1c2b39),
        scale,
    );
    ui_rectangle(0.0, 890.0, 170.0, 49.0, mode_color(editor.mode), scale);
    ui_label(
        mode_name(editor.mode),
        25.0,
        922.0,
        23,
        Color::from_hex(0x101923),
        font,
        scale,
    );
    let pending = format!("{}{}", editor.count, editor.pending.unwrap_or(' '));
    ui_label(&pending, 196.0, 922.0, 23, ACCENT, font, scale);
    let wrap_label = if word_wrap { "WRAP ON" } else { "WRAP OFF" };
    let status = format!("{font_size}px  ·  {wrap_label}");
    ui_label(&status, 295.0, 921.0, 19, MUTED, font, scale);
    let position = format!(
        "Ln {}, Col {}    {} lines",
        editor.cursor.row + 1,
        editor.cursor.col + 1,
        editor.lines.len()
    );
    ui_label(&position, 1040.0, 922.0, 22, INK, font, scale);
    let prompt = if matches!(editor.mode, Mode::Command | Mode::Search) {
        format!(
            "{}{}|",
            if editor.mode == Mode::Command {
                ':'
            } else {
                '/'
            },
            editor.prompt
        )
    } else {
        editor.message.clone()
    };
    // Keep long filenames and command lines on the text surface.
    let prompt: String = prompt
        .chars()
        .rev()
        .take(112)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    ui_label(
        &prompt,
        30.0,
        977.0,
        22,
        if matches!(editor.mode, Mode::Command | Mode::Search) {
            ACCENT
        } else {
            MUTED
        },
        font,
        scale,
    );
    set_default_camera();
}

fn board_mesh(texture: Texture2D) -> Mesh {
    let w = BOARD_W / 2.0;
    let h = BOARD_H / 2.0;
    Mesh {
        vertices: vec![
            Vertex::new(-w, h, 0.0, 0.0, 1.0, WHITE),
            Vertex::new(w, h, 0.0, 1.0, 1.0, WHITE),
            Vertex::new(w, -h, 0.0, 1.0, 0.0, WHITE),
            Vertex::new(-w, -h, 0.0, 0.0, 0.0, WHITE),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: Some(texture),
    }
}

fn editor_target_dimensions() -> (u32, u32) {
    let viewport_width = screen_width() * screen_dpi_scale();
    let width = (viewport_width * 2.0)
        .round()
        .clamp(MIN_RENDER_WIDTH as f32, MAX_RENDER_WIDTH as f32) as u32;
    let height = (width as f32 * TEX_H as f32 / TEX_W as f32)
        .round()
        .max(1.0) as u32;
    (width, height)
}

fn create_editor_target(width: u32, height: u32) -> RenderTarget {
    let target = render_target(width, height);
    target.texture.set_filter(FilterMode::Linear);
    target
}

fn draw_world(view: &View, board: &Mesh, show_floor: bool) {
    clear_background(Color::from_hex(0x080e17));
    set_camera(&view.camera());
    // Toggle the perspective floor independently from the floating editor surface.
    if show_floor {
        for i in -20..=20 {
            let p = i as f32 * 1.5;
            let color = if i == 0 {
                Color::from_hex(0x1d3943)
            } else {
                Color::from_hex(0x14222e)
            };
            draw_line_3d(vec3(p, -4.55, -30.0), vec3(p, -4.55, 12.0), color);
            draw_line_3d(vec3(-30.0, -4.55, p), vec3(30.0, -4.55, p), color);
        }
    }
    if !view.flat_only {
        draw_cube(
            vec3(0.0, 0.0, -0.13),
            vec3(BOARD_W + 0.13, BOARD_H + 0.13, 0.20),
            None,
            Color::from_hex(0x233743),
        );
        draw_cube_wires(
            vec3(0.0, 0.0, -0.13),
            vec3(BOARD_W + 0.14, BOARD_H + 0.14, 0.22),
            Color::from_hex(0x41606a),
        );
        draw_line_3d(vec3(-6.0, 3.81, 0.01), vec3(-3.4, 3.81, 0.01), ACCENT);
    }
    draw_mesh(board);
    set_default_camera();
}

fn draw_overlay(help: bool, flat_only: bool, font: Option<&Font>) {
    let height = screen_height();
    if flat_only {
        return;
    }
    if help {
        let x = (screen_width() - 357.0).max(20.0);
        let y = (height - 275.0).max(95.0);
        draw_rectangle(x, y, 329.0, 220.0, Color::new(0.035, 0.065, 0.095, 0.92));
        draw_rectangle_lines(x, y, 329.0, 220.0, 1.0, Color::from_hex(0x2a3d48));
        label("MOVE THROUGH SPACE", x + 17.0, y + 28.0, 15, ACCENT, font);
        for (i, text) in [
            "wheel          zoom",
            "RMB outside pan; F4 unlocks orbit",
            "middle drag    pan",
            "F2             straight home view",
            "F4             flat-only / orbit",
            "F3             toggle floor grid",
            "double Enter   toggle word wrap",
            "Ctrl+H / L     scroll horizontally; disables wrap",
            "F1             hide controls",
        ]
        .iter()
        .enumerate()
        {
            label(text, x + 17.0, y + 53.0 + i as f32 * 20.0, 14, INK, font);
        }
    }
    label(
        "F1 controls  ·  F4 flat-only/orbit  ·  Ctrl +/- font",
        29.0,
        height - 24.0,
        14,
        MUTED,
        font,
    );
}

fn handle_keyboard(
    editor: &mut Editor,
    font_size: &mut u16,
    word_wrap: &mut bool,
    vertical_motion: &mut Option<VerticalMotion>,
) {
    let mut chars = Vec::new();
    while let Some(ch) = get_char_pressed() {
        chars.push(ch);
    }
    let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let scroll_left = ctrl && is_key_pressed(KeyCode::H);
    let scroll_right = ctrl && is_key_pressed(KeyCode::L);
    if ctrl && !scroll_left && !scroll_right {
        editor.follow_cursor_horizontally();
    }
    if is_key_pressed(KeyCode::Escape) {
        *vertical_motion = None;
        editor.escape();
        return;
    }
    if ctrl {
        if is_key_pressed(KeyCode::J) || is_key_pressed(KeyCode::K) {
            *vertical_motion = Some(VerticalMotion {
                direction: if is_key_pressed(KeyCode::J) { 1 } else { -1 },
                remaining: 5,
                next_step: get_time(),
            });
            return;
        }
        if [
            KeyCode::Minus,
            KeyCode::Equal,
            KeyCode::S,
            KeyCode::Backspace,
            KeyCode::W,
            KeyCode::V,
            KeyCode::R,
            KeyCode::D,
            KeyCode::U,
            KeyCode::H,
            KeyCode::L,
            KeyCode::C,
        ]
        .into_iter()
        .any(is_key_pressed)
        {
            *vertical_motion = None;
        }
        if is_key_pressed(KeyCode::Minus) {
            *font_size = font_size.saturating_sub(2).max(12);
            editor.message = format!("Font size: {font_size}px");
        }
        if is_key_pressed(KeyCode::Equal) {
            *font_size = (*font_size + 2).min(42);
            editor.message = format!("Font size: {font_size}px");
        }
        if is_key_pressed(KeyCode::S) {
            editor.save(None);
        }
        if is_key_pressed(KeyCode::Backspace) && editor.mode == Mode::Insert {
            editor.delete_prev_word();
        }
        if is_key_pressed(KeyCode::W) && editor.mode == Mode::Insert {
            editor.delete_prev_word();
        }
        if is_key_pressed(KeyCode::V)
            && editor.mode == Mode::Insert
            && let Some(text) = macroquad::miniquad::window::clipboard_get()
        {
            editor.insert_text(&text);
        }
        if is_key_pressed(KeyCode::R) && editor.mode == Mode::Normal {
            editor.undo(true);
        }
        if is_key_pressed(KeyCode::D) {
            editor.move_by(0, 1, text_grid(*font_size).0 / 2);
        }
        if is_key_pressed(KeyCode::U) {
            editor.move_by(0, -1, text_grid(*font_size).0 / 2);
        }
        if scroll_left || scroll_right {
            if *word_wrap {
                *word_wrap = false;
                editor.left = 0;
                editor.message = "Word wrap off · horizontal scrolling".into();
            }
            editor.scroll_horizontal(if scroll_left { -1 } else { 1 }, text_grid(*font_size).1);
        }
        if is_key_pressed(KeyCode::C) {
            editor.escape();
        }
        return;
    }
    if !chars.is_empty()
        || is_key_pressed(KeyCode::Left)
        || is_key_pressed(KeyCode::Right)
        || is_key_pressed(KeyCode::Up)
        || is_key_pressed(KeyCode::Down)
        || is_key_pressed(KeyCode::Enter)
        || is_key_pressed(KeyCode::Backspace)
        || is_key_pressed(KeyCode::Delete)
        || is_key_pressed(KeyCode::PageUp)
        || is_key_pressed(KeyCode::PageDown)
        || is_key_pressed(KeyCode::Home)
        || is_key_pressed(KeyCode::End)
    {
        *vertical_motion = None;
    }
    match editor.mode {
        Mode::Insert => {
            if is_key_pressed(KeyCode::Enter) {
                editor.newline();
            }
            if is_key_pressed(KeyCode::Backspace) {
                editor.backspace();
            }
            if is_key_pressed(KeyCode::Delete) {
                editor.delete_forward();
            }
            if is_key_pressed(KeyCode::Tab) {
                for _ in 0..4 {
                    editor.insert_char(' ');
                }
            }
        }
        Mode::Command | Mode::Search if is_key_pressed(KeyCode::Backspace) => {
            editor.prompt.pop();
        }
        _ => {}
    }
    for ch in chars {
        match editor.mode {
            Mode::Insert => editor.insert_char(ch),
            Mode::Command | Mode::Search => {
                if !ch.is_control() {
                    editor.prompt.push(ch);
                }
            }
            Mode::Normal | Mode::Visual => editor.normal_key(ch),
        }
    }
    if matches!(editor.mode, Mode::Command | Mode::Search) {
        if is_key_pressed(KeyCode::Enter) {
            editor.submit_prompt();
        }
        return;
    }
    for (key, dx, dy) in [
        (KeyCode::Left, -1, 0),
        (KeyCode::Right, 1, 0),
        (KeyCode::Up, 0, -1),
        (KeyCode::Down, 0, 1),
    ] {
        if is_key_pressed(key) {
            editor.move_by(dx, dy, 1);
        }
    }
    if is_key_pressed(KeyCode::PageDown) {
        editor.move_by(0, 1, text_grid(*font_size).0 - 2);
    }
    if is_key_pressed(KeyCode::PageUp) {
        editor.move_by(0, -1, text_grid(*font_size).0 - 2);
    }
    if is_key_pressed(KeyCode::Home) {
        editor.follow_cursor_horizontally();
        editor.cursor.col = 0;
    }
    if is_key_pressed(KeyCode::End) {
        editor.follow_cursor_horizontally();
        editor.cursor.col = editor.lines[editor.cursor.row].len();
        editor.clamp();
    }
}

fn advance_vertical_motion(editor: &mut Editor, vertical_motion: &mut Option<VerticalMotion>) {
    let now = get_time();
    let Some(motion) = vertical_motion.as_mut() else {
        return;
    };
    while motion.remaining > 0 && now >= motion.next_step {
        editor.move_by(0, motion.direction, 1);
        motion.remaining -= 1;
        motion.next_step += VERTICAL_MOTION_STEP;
    }
    if motion.remaining == 0 {
        *vertical_motion = None;
    }
}

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
        match std::fs::read_to_string(&path) {
            Ok(text) => Editor::new(&text, Some(path)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Editor::new("", Some(path)),
            Err(err) => {
                eprintln!("Cannot open {}: {err}", path.display());
                return;
            }
        }
    } else {
        Editor::new(WELCOME, None)
    };
    let font = system_font();
    let (target_width, target_height) = editor_target_dimensions();
    let mut target = create_editor_target(target_width, target_height);
    let mut board = board_mesh(target.texture.clone());
    let mut view = View::new();
    let mut help = false;
    let mut show_floor = false;
    let mut word_wrap = true;
    let mut font_size = BASE_FONT_SIZE;
    let mut last_normal_enter = None;
    let mut vertical_motion = None;
    let mut last_insert_activity = get_time();
    let mut frame = 0;
    prevent_quit();
    loop {
        let (target_width, target_height) = editor_target_dimensions();
        if target.texture.width() != target_width as f32
            || target.texture.height() != target_height as f32
        {
            target = create_editor_target(target_width, target_height);
            board = board_mesh(target.texture.clone());
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
        let mode_before_keys = editor.mode;
        handle_keyboard(
            &mut editor,
            &mut font_size,
            &mut word_wrap,
            &mut vertical_motion,
        );
        advance_vertical_motion(&mut editor, &mut vertical_motion);
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
        let (visible_rows, visible_cols, cell_width, line_height) = text_grid(font_size);
        if is_mouse_button_pressed(MouseButton::Left)
            && !matches!(editor.mode, Mode::Command | Mode::Search)
            && let Some(point) = view.pick()
            && point.x >= TEXT_X
            && point.y >= TEXT_Y
            && point.y < TEXT_Y + visible_rows as f32 * line_height
        {
            vertical_motion = None;
            editor.follow_cursor_horizontally();
            let display_index = editor.top + ((point.y - TEXT_Y) / line_height) as usize;
            let display_rows = editor.display_rows(visible_cols, word_wrap);
            if let Some(&(row, segment_start)) = display_rows.get(display_index) {
                let horizontal_offset = if word_wrap { 0 } else { editor.left };
                editor.cursor = Pos {
                    row,
                    col: segment_start
                        + horizontal_offset
                        + ((point.x - TEXT_X) / cell_width) as usize,
                };
                editor.clamp();
            }
        }
        editor.reveal_cursor(visible_rows, visible_cols, word_wrap);
        if editor.quit {
            break;
        }
        let render_scale = target.texture.width() / TEX_W as f32;
        draw_buffer(
            &editor,
            &target,
            font.as_ref(),
            font_size,
            word_wrap,
            render_scale,
        );
        draw_world(&view, &board, show_floor);
        draw_overlay(help, view.flat_only, font.as_ref());
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
