use macroquad::prelude::{Color, Conf};

pub const TEX_W: u32 = 1600;
pub const TEX_H: u32 = 1000;
pub const MIN_RENDER_WIDTH: u32 = TEX_W * 2;
pub const MAX_RENDER_WIDTH: u32 = 8192;
pub const BOARD_W: f32 = 12.0;
pub const BOARD_H: f32 = 7.5;
pub const CELL: f32 = 17.0;
pub const LINE: f32 = 36.0;
pub const TEXT_X: f32 = 112.0;
pub const TEXT_Y: f32 = 70.0;
pub const BASE_FONT_SIZE: u16 = 27;
pub const ZOOM_SENSITIVITY: f32 = 0.004;
pub const ZOOM_SMOOTHING: f32 = 3.5;
pub const VERTICAL_MOTION_STEP: f64 = 0.055;
pub const INSERT_IDLE_TIMEOUT: f64 = 15.0;
pub const INK: Color = Color::new(0.80, 0.85, 0.90, 1.0);
pub const MUTED: Color = Color::new(0.39, 0.48, 0.57, 1.0);
pub const ACCENT: Color = Color::new(0.39, 0.89, 0.77, 1.0);

pub const WELCOME: &str = r#"// vimloating — a little room for your thoughts

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

pub fn window_conf() -> Conf {
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
