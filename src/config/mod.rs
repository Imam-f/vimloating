mod theme;
mod user;
mod welcome;

#[cfg(feature = "graphics")]
pub use macroquad::prelude::Color;
#[cfg(feature = "graphics")]
use macroquad::prelude::Conf;
pub use theme::{Theme, ThemePalette};
pub use user::{UserConfig, load_user_config};
pub use welcome::WELCOME;

/// RGBA theme color for builds without the graphics backend.
#[cfg(not(feature = "graphics"))]
#[derive(Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[cfg(not(feature = "graphics"))]
impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

pub const TEX_W: u32 = 1600;
pub const TEX_H: u32 = 1000;
pub const MIN_RENDER_WIDTH: u32 = TEX_W * 2;
pub const MAX_RENDER_WIDTH: u32 = 8192;
pub const BOARD_W: f32 = 12.0;
pub const BOARD_H: f32 = 7.5;
pub const CELL: f32 = 17.0;
pub const LINE: f32 = 36.0;
pub const TEXT_X: f32 = 112.0;
pub const TEXT_Y: f32 = 0.0;
pub const BASE_FONT_SIZE: u16 = 27;
pub const ZOOM_SENSITIVITY: f32 = 0.004;
pub const ZOOM_SMOOTHING: f32 = 3.5;
pub const VERTICAL_MOTION_STEP: f64 = 0.025;
pub const INSERT_IDLE_TIMEOUT: f64 = 15.0;
#[cfg(feature = "graphics")]
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
