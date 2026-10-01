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
pub const TEXT_Y: f32 = 0.0;
pub const BASE_FONT_SIZE: u16 = 27;
pub const ZOOM_SENSITIVITY: f32 = 0.004;
pub const ZOOM_SMOOTHING: f32 = 3.5;
pub const VERTICAL_MOTION_STEP: f64 = 0.025;
pub const INSERT_IDLE_TIMEOUT: f64 = 15.0;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Vimfloating,
    Everforest,
    SolarizedBlue,
}

#[derive(Clone, Copy)]
pub struct ThemePalette {
    pub background: Color,
    pub world_background: Color,
    pub divider: Color,
    pub gutter: Color,
    pub current_line: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub insert: Color,
    pub visual: Color,
    pub command: Color,
    pub status: Color,
    pub line_selection: Color,
    pub selection: Color,
    pub search: Color,
    pub comment: Color,
    pub string: Color,
    pub number: Color,
    pub punctuation: Color,
    pub cursor_text: Color,
    pub board_frame: Color,
    pub board_border: Color,
    pub floor: Color,
    pub floor_axis: Color,
    pub help_background: Color,
    pub help_border: Color,
}

fn color(hex: u32) -> Color {
    Color::new(
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
        1.0,
    )
}

impl Theme {
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "vimfloating" | "default" => Some(Self::Vimfloating),
            "everforest" => Some(Self::Everforest),
            "solarized-blue" | "solarized-dark-blue" | "solarized" => Some(Self::SolarizedBlue),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Vimfloating => "Vimfloating",
            Self::Everforest => "Everforest",
            Self::SolarizedBlue => "Solarized Dark Blue",
        }
    }

    pub fn palette(self) -> ThemePalette {
        match self {
            Self::Vimfloating => ThemePalette {
                background: color(0x101923),
                world_background: color(0x080e17),
                divider: color(0x24313e),
                gutter: color(0x2f4352),
                current_line: color(0x182735),
                text: color(0xccd9e6),
                muted: color(0x637b91),
                accent: color(0x63e3c5),
                insert: color(0x83b9f5),
                visual: color(0xc7a0f4),
                command: color(0xeec181),
                status: color(0x1c2b39),
                line_selection: color(0x342c4a),
                selection: color(0x4e3e6b),
                search: color(0x544729),
                comment: color(0x708b91),
                string: color(0xc7d69b),
                number: color(0xd5abf1),
                punctuation: color(0x8c9cae),
                cursor_text: color(0x10232a),
                board_frame: color(0x233743),
                board_border: color(0x41606a),
                floor: color(0x14222e),
                floor_axis: color(0x1d3943),
                help_background: Color::new(0.035, 0.065, 0.095, 0.92),
                help_border: color(0x2a3d48),
            },
            Self::Everforest => ThemePalette {
                background: color(0x2d353b),
                world_background: color(0x232a2e),
                divider: color(0x475258),
                gutter: color(0x859289),
                current_line: color(0x343f44),
                text: color(0xd3c6aa),
                muted: color(0x859289),
                accent: color(0xa7c080),
                insert: color(0x7fbbb3),
                visual: color(0xd699b6),
                command: color(0xdbbc7f),
                status: color(0x343f44),
                line_selection: color(0x414b50),
                selection: color(0x4b565b),
                search: color(0x514a3d),
                comment: color(0x859289),
                string: color(0xa7c080),
                number: color(0xd699b6),
                punctuation: color(0x9da9a0),
                cursor_text: color(0x2d353b),
                board_frame: color(0x3d484d),
                board_border: color(0x7fbbb3),
                floor: color(0x343f44),
                floor_axis: color(0xa7c080),
                help_background: Color::new(0.08, 0.10, 0.11, 0.94),
                help_border: color(0x475258),
            },
            Self::SolarizedBlue => ThemePalette {
                background: color(0x002b36),
                world_background: color(0x001e26),
                divider: color(0x073642),
                gutter: color(0x586e75),
                current_line: color(0x073642),
                text: color(0x839496),
                muted: color(0x586e75),
                accent: color(0x268bd2),
                insert: color(0x2aa198),
                visual: color(0x6c71c4),
                command: color(0xb58900),
                status: color(0x073642),
                line_selection: color(0x10404b),
                selection: color(0x164b59),
                search: color(0x4a3b00),
                comment: color(0x586e75),
                string: color(0x2aa198),
                number: color(0xd33682),
                punctuation: color(0x657b83),
                cursor_text: color(0x002b36),
                board_frame: color(0x073642),
                board_border: color(0x268bd2),
                floor: color(0x073642),
                floor_axis: color(0x268bd2),
                help_background: Color::new(0.0, 0.10, 0.13, 0.94),
                help_border: color(0x164b59),
            },
        }
    }
}

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
//   > / <          indent / unindent current line or visual selection
//   Alt+J / K      move line(s) down / up; match previous nonblank indentation
//   gg / G         first / last line
//   dd / yy / p    delete / yank / paste a line
//   u / Ctrl-R     undo / redo
//   v              select text, then y or d
//   /text, ?text   search forward / backward; n / N repeat / reverse
//   :w notes.rs    save this buffer
//   :e notes.rs    open a UTF-8 file
//   :Explore       browse files; Enter opens, - goes to the parent
//   :!git status   run a shell command (Esc closes its output)
//   :.!pwd         replace the current line with command output
//   :theme everforest / solarized-blue  change the colors
//   :ls             list buffers · :b 2 switches · :bn / :bp cycles · :bd deletes
//   Ctrl+6          switch to the last active buffer
//   Tab             complete command names and file paths in the command prompt

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
//   Ctrl+E / Y     scroll one display line
//   Ctrl+D / U     scroll half a page; Ctrl+F / B one page
//   Idle 15s       return to Normal mode
//   Enter Enter    toggle word wrap in Normal mode
//   Ctrl+H / L     scroll horizontally (turns wrap off)
//   F1             toggle the controls overlay

// Make something worth keeping.
"#;

#[derive(Default)]
pub struct UserConfig {
    pub theme: Option<Theme>,
    pub two_d_only: Option<bool>,
}

pub fn load_user_config() -> UserConfig {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return UserConfig::default();
    };
    let Ok(contents) = std::fs::read_to_string(std::path::PathBuf::from(home).join(".vimfloating"))
    else {
        return UserConfig::default();
    };
    parse_user_config(&contents)
}

fn parse_user_config(contents: &str) -> UserConfig {
    let mut settings = UserConfig::default();
    for line in contents.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "theme" => settings.theme = Theme::parse(value),
            "2d_only" => settings.two_d_only = value.trim().parse().ok(),
            _ => {}
        }
    }
    settings
}

#[cfg(test)]
mod tests {
    use super::{Theme, parse_user_config};

    #[test]
    fn parses_theme_and_2d_only_setting() {
        let settings = parse_user_config("theme=everforest\n2d_only=true");
        assert_eq!(settings.theme, Some(Theme::Everforest));
        assert_eq!(settings.two_d_only, Some(true));
    }
}

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
