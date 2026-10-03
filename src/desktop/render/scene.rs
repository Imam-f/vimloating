use super::{super::view::View, text::label};
use macroquad::prelude::*;
use vimloating::config::*;

pub fn board_mesh(texture: Texture2D) -> Mesh {
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

pub fn editor_target_dimensions() -> (u32, u32) {
    let viewport_width = screen_width() * screen_dpi_scale();
    let width = (viewport_width * 2.0)
        .round()
        .clamp(MIN_RENDER_WIDTH as f32, MAX_RENDER_WIDTH as f32) as u32;
    let height = (width as f32 * TEX_H as f32 / TEX_W as f32)
        .round()
        .max(1.0) as u32;
    (width, height)
}

pub fn create_editor_target(width: u32, height: u32) -> RenderTarget {
    let target = render_target(width, height);
    target.texture.set_filter(FilterMode::Linear);
    sharpen_editor_texture(&target.texture);
    target
}

fn sharpen_editor_texture(texture: &Texture2D) {
    use miniquad::{RawId, gl};

    // Favor finer mip levels while retaining trilinear transitions during zoom.
    const TEXTURE_LOD_BIAS: u32 = 0x8501;
    const TEXTURE_BINDING_2D: u32 = 0x8069;
    let texture = texture.raw_miniquad_id();
    // SAFETY: texture creation runs on the render thread with a current context.
    // Restore the binding so Miniquad's cached texture state stays valid.
    unsafe {
        let context = get_internal_gl();
        match context.quad_context.texture_raw_id(texture) {
            RawId::OpenGl(texture) => {
                let mut previous = 0;
                gl::glGetIntegerv(TEXTURE_BINDING_2D, &mut previous);
                gl::glBindTexture(gl::GL_TEXTURE_2D, texture);
                gl::glTexParameterf(gl::GL_TEXTURE_2D, TEXTURE_LOD_BIAS, -0.5);
                gl::glBindTexture(gl::GL_TEXTURE_2D, previous as u32);
            }
            #[cfg(target_vendor = "apple")]
            RawId::Metal(_) => {}
        }
    }
}

pub fn update_editor_mipmaps(target: &RenderTarget) {
    let texture = target.texture.raw_miniquad_id();
    // Finish drawing the current buffer before averaging its smaller levels.
    // Bilinear sampling alone skips thin strokes when the board is zoomed out.
    // SAFETY: this runs on Macroquad's render thread, between drawing passes.
    let mut gl = unsafe { get_internal_gl() };
    gl.flush();
    gl.quad_context.texture_generate_mipmaps(texture);
    gl.quad_context.texture_set_min_filter(
        texture,
        FilterMode::Linear,
        miniquad::MipmapFilterMode::Linear,
    );
}

pub fn draw_world(view: &View, board: &Mesh, show_floor: bool, theme: Theme) {
    let palette = theme.palette();
    clear_background(palette.world_background);
    set_camera(&view.camera());
    if show_floor {
        for i in -20..=20 {
            let p = i as f32 * 1.5;
            let color = if i == 0 {
                palette.floor_axis
            } else {
                palette.floor
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
            palette.board_frame,
        );
        draw_cube_wires(
            vec3(0.0, 0.0, -0.13),
            vec3(BOARD_W + 0.14, BOARD_H + 0.14, 0.22),
            palette.board_border,
        );
        draw_line_3d(
            vec3(-6.0, 3.81, 0.01),
            vec3(-3.4, 3.81, 0.01),
            palette.accent,
        );
    }
    draw_mesh(board);
    set_default_camera();
}

pub fn draw_2d_only(texture: &Texture2D, theme: Theme) {
    clear_background(theme.palette().background);
    draw_texture_ex(
        texture,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(screen_width(), screen_height())),
            flip_y: true,
            ..Default::default()
        },
    );
}

pub fn draw_overlay(help: bool, flat_only: bool, font: Option<&Font>, theme: Theme) {
    let palette = theme.palette();
    let height = screen_height();
    if flat_only {
        return;
    }
    if help {
        let x = (screen_width() - 357.0).max(20.0);
        let y = (height - 275.0).max(95.0);
        draw_rectangle(x, y, 329.0, 220.0, palette.help_background);
        draw_rectangle_lines(x, y, 329.0, 220.0, 1.0, palette.help_border);
        label(
            "MOVE THROUGH SPACE",
            x + 17.0,
            y + 28.0,
            15,
            palette.accent,
            font,
        );
        for (i, text) in [
            "wheel          zoom",
            "RMB outside pan; F4 unlocks orbit",
            "middle drag    pan",
            "F2 home · F5 2D-only on/off",
            "F4 flat/orbit · F5 2D-only on/off",
            "F3             toggle floor grid",
            "double Enter   toggle word wrap",
            "Ctrl+H / L     scroll horizontally; disables wrap",
            "F1             hide controls",
        ]
        .iter()
        .enumerate()
        {
            label(
                text,
                x + 17.0,
                y + 53.0 + i as f32 * 20.0,
                14,
                palette.text,
                font,
            );
        }
    }
    label(
        "F1 controls  ·  F4 flat/orbit · F5 2D-only · Ctrl +/- font",
        29.0,
        height - 24.0,
        14,
        palette.muted,
        font,
    );
}
