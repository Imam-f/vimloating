use macroquad::prelude::*;

pub fn system_font() -> Option<Font> {
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

pub(super) fn label(text: &str, x: f32, y: f32, size: u16, color: Color, font: Option<&Font>) {
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

pub(super) fn ui_label(
    text: &str,
    x: f32,
    y: f32,
    size: u16,
    color: Color,
    font: Option<&Font>,
    scale: f32,
) {
    label(
        text,
        x * scale,
        y * scale,
        (size as f32 * scale).round().clamp(1.0, u16::MAX as f32) as u16,
        color,
        font,
    );
}

pub(super) fn ui_line(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    thickness: f32,
    color: Color,
    scale: f32,
) {
    draw_line(
        x1 * scale,
        y1 * scale,
        x2 * scale,
        y2 * scale,
        thickness * scale,
        color,
    );
}

pub(super) fn ui_rectangle(x: f32, y: f32, width: f32, height: f32, color: Color, scale: f32) {
    draw_rectangle(x * scale, y * scale, width * scale, height * scale, color);
}
