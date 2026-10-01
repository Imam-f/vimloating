use crate::config::{BOARD_H, BOARD_W, TEX_H, TEX_W, ZOOM_SENSITIVITY, ZOOM_SMOOTHING};
use macroquad::camera::Camera;
use macroquad::prelude::*;

pub struct View {
    distance: f32,
    desired_distance: f32,
    yaw: f32,
    desired_yaw: f32,
    pitch: f32,
    desired_pitch: f32,
    center: Vec3,
    desired_center: Vec3,
    pub last_mouse: Vec2,
    pub two_d_only: bool,
    pub flat_only: bool,
    right_pan: bool,
}

impl View {
    pub fn new(two_d_only: bool) -> Self {
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
            two_d_only,
            flat_only: true,
            right_pan: false,
        }
    }

    pub fn update(&mut self) {
        if self.two_d_only {
            self.last_mouse = vec2(mouse_position().0, mouse_position().1);
            return;
        }
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
            self.desired_yaw = 0.0;
            self.desired_pitch = 0.0;
        }
        let blend = 1.0 - (-12.0 * get_frame_time().min(0.1)).exp();
        let zoom_blend = 1.0 - (-ZOOM_SMOOTHING * get_frame_time().min(0.1)).exp();
        self.distance += (self.desired_distance - self.distance) * zoom_blend;
        self.yaw += (self.desired_yaw - self.yaw) * blend;
        self.pitch += (self.desired_pitch - self.pitch) * blend;
        self.center = self.center.lerp(self.desired_center, blend);
    }

    pub fn camera(&self) -> Camera3D {
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

    pub fn follow_cursor(&mut self, cursor: Vec2) {
        if self.two_d_only {
            return;
        }
        let projected = self
            .camera()
            .matrix()
            .project_point3(vec3(cursor.x, cursor.y, 0.0));
        let screen = vec2(
            (projected.x + 1.0) * 0.5 * screen_width(),
            (1.0 - projected.y) * 0.5 * screen_height(),
        );
        let margin = 24.0;
        if screen.x < margin
            || screen.x > screen_width() - margin
            || screen.y < margin
            || screen.y > screen_height() - margin
        {
            self.desired_center = vec3(cursor.x, cursor.y, 0.0);
        }
    }

    pub fn pick(&self) -> Option<Vec2> {
        if self.two_d_only {
            let (mouse_x, mouse_y) = mouse_position();
            return Some(vec2(
                mouse_x / screen_width() * TEX_W as f32,
                mouse_y / screen_height() * TEX_H as f32,
            ));
        }
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
