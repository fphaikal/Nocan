use std::f32::consts::PI;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_BEZIER_SEGMENT, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED,
    D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_CLOSED, D2D1_FIGURE_END_OPEN,
    D2D_POINT_2F, D2D_RECT_F,
};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;

use crate::graphics::d2d::D2DContext;

const BLINK_SECONDS: f32 = 0.18;

pub struct TigerMascot {
    elapsed: f32,
    look: (f32, f32),
    blink_in: f32,
    blink_remaining: f32,
    reaction_remaining: f32,
    hover_elapsed: f32,
    music_mix: f32,
    random: u32,
}

impl Default for TigerMascot {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            look: (0.0, 0.0),
            blink_in: 3.2,
            blink_remaining: 0.0,
            reaction_remaining: 0.0,
            hover_elapsed: 0.0,
            music_mix: 0.0,
            random: 0x4e4f4341,
        }
    }
}

impl TigerMascot {
    pub fn update(&mut self, dt: f32, look: (f32, f32), hovered: bool, playing: bool) {
        let dt = dt.clamp(0.0, 0.05);
        self.elapsed += dt;
        let follow = 1.0 - (-12.0 * dt).exp();
        self.look.0 += (look.0.clamp(-1.0, 1.0) - self.look.0) * follow;
        self.look.1 += (look.1.clamp(-1.0, 1.0) - self.look.1) * follow;
        self.music_mix += (f32::from(playing) - self.music_mix) * follow;
        self.reaction_remaining = (self.reaction_remaining - dt).max(0.0);
        self.hover_elapsed = if hovered { self.hover_elapsed + dt } else { 0.0 };

        if self.blink_remaining > 0.0 {
            self.blink_remaining = (self.blink_remaining - dt).max(0.0);
        } else {
            self.blink_in -= dt;
            if self.blink_in <= 0.0 {
                self.blink_remaining = BLINK_SECONDS;
                self.random ^= self.random << 13;
                self.random ^= self.random >> 17;
                self.random ^= self.random << 5;
                self.blink_in = 2.8 + (self.random % 2800) as f32 / 1000.0;
            }
        }
    }

    pub fn react(&mut self) {
        self.reaction_remaining = 0.9;
        self.hover_elapsed = 0.0;
        self.blink_remaining = 0.0;
        self.blink_in = self.blink_in.max(1.0);
    }

    pub fn should_expand(&self) -> bool {
        self.hover_elapsed >= 0.55 && !self.is_reacting()
    }

    pub fn is_reacting(&self) -> bool {
        self.reaction_remaining > 0.0
    }

    fn eye_open(&self, right: bool) -> f32 {
        let mut open = if self.blink_remaining > 0.0 {
            let progress = 1.0 - self.blink_remaining / BLINK_SECONDS;
            1.0 - (progress * PI).sin()
        } else { 1.0 };
        if right && self.reaction_remaining > 0.55 {
            let progress = (0.9 - self.reaction_remaining) / 0.35;
            open *= 1.0 - (progress * PI).sin();
        }
        open.clamp(0.0, 1.0)
    }

    pub fn draw(&self, d2d: &D2DContext, bounds: D2D_RECT_F) {
        self.draw_background(d2d, bounds);
        let Ok(white) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }) else { return; };
        let Ok(ink) = d2d.create_solid_brush(D2D1_COLOR_F { r: 0.10, g: 0.09, b: 0.08, a: 1.0 }) else { return; };
        let scale = ((bounds.right - bounds.left) / 150.0).min((bounds.bottom - bounds.top) / 40.0).min(1.1);
        let cx = (bounds.left + bounds.right) * 0.5;
        let breathe = (self.elapsed * 1.7).sin() * 0.3;
        let groove = (self.elapsed * 5.0).sin() * self.music_mix;
        let top = bounds.top + ((bounds.bottom - bounds.top) - 40.0 * scale) * 0.5;
        let point = |x: f32, y: f32| D2D_POINT_2F { x: cx + x * scale, y: top + (y + breathe + groove * 0.6) * scale };

        for (x, height) in [(-8.0, 6.5), (0.0, 9.0), (8.0, 6.5)] {
            stripe(d2d, &white, point(x - 2.2, 3.0), point(x + 2.2, 3.0), point(x, 3.0 + height));
        }
        for side in [-1.0f32, 1.0] {
            let twitch = (self.elapsed * 19.0).sin() * self.reaction_remaining * 2.0 + groove * 0.6;
            for (y, end_y) in [(24.0, 20.0), (27.0, 27.0), (30.0, 34.0)] {
                let start = point(side * 38.0, y);
                let end = point(side * 57.0, end_y + twitch);
                d2d.draw_line(start.x, start.y, end.x, end.y, &white, 1.25 * scale, true);
            }
        }

        for (side, right) in [(-1.0f32, false), (1.0, true)] {
            let eye = point(side * 17.0, 23.0);
            let open = self.eye_open(right);
            if open < 0.18 {
                happy_lid(d2d, &white, eye, 6.6 * scale);
                continue;
            }
            let rx = 6.4 * scale;
            let ry = 7.2 * open * scale;
            d2d.fill_ellipse(eye.x, eye.y, rx, ry, &white);
            let pupil_rx = 2.5 * scale;
            let pupil_ry = 4.6 * open * scale;
            let px = eye.x + self.look.0 * 2.0 * scale;
            let py = eye.y + self.look.1 * 1.3 * open * scale;
            d2d.fill_ellipse(px, py, pupil_rx, pupil_ry, &ink);
            d2d.fill_ellipse(px - 0.65 * scale, py - 1.7 * open * scale, 0.8 * scale, 1.1 * open * scale, &white);
        }
    }

    fn draw_background(&self, d2d: &D2DContext, bounds: D2D_RECT_F) {
        let width = bounds.right - bounds.left;
        let height = bounds.bottom - bounds.top;
        let cx = (bounds.left + bounds.right) * 0.5;
        let opacity = 0.75 + 0.12 * (self.elapsed * 1.6).sin();
        let shift = (self.elapsed * 0.60).sin() * width * 0.12;
        crate::graphics::ambient::draw(d2d,bounds,14.0,opacity,shift);

        let Ok(dot) = d2d.create_solid_brush(D2D1_COLOR_F { r: 0.55, g: 0.78, b: 1.0, a: 1.0 }) else { return; };
        let scale = (width / 150.0).min(height / 40.0).min(1.1);
        let cy = bounds.top + height * 0.5;
        let spacing = 3.5 * scale;
        let columns = (width * 0.5 / spacing).ceil() as i32;
        let rows = (height * 0.5 / spacing).ceil() as i32;
        let swell = 1.0 + (self.elapsed * 0.65).sin() * 0.035;
        // One shared radial field forms the central donut and continues to both edges.
        for row in -rows..=rows {
            for column in -columns..=columns {
                let dx = (column as f32 + (row & 1) as f32 * 0.5) * spacing;
                let dy = row as f32 * spacing;
                let x = cx + dx;
                let y = cy + dy;
                if x < bounds.left || x > bounds.right || y < bounds.top || y > bounds.bottom {
                    continue;
                }
                let radius = circular_radius(dx, dy, height * 0.40);
                let ring = (1.0 - ((radius - swell) / 0.34).abs()).clamp(0.0, 1.0);
                let hole = ((radius - 0.52) / 0.22).clamp(0.0, 1.0);
                let opacity = hole * (0.18 + 0.38 * ring);
                let dot_radius = (0.5 + 0.24 * ring) * scale;
                unsafe { dot.SetOpacity(opacity); }
                d2d.fill_ellipse(x, y, dot_radius, dot_radius, &dot);
            }
        }
    }
}

fn circular_radius(dx: f32, dy: f32, radius: f32) -> f32 {
    dx.hypot(dy) / radius
}

fn stripe(d2d: &D2DContext, brush: &ID2D1SolidColorBrush, a: D2D_POINT_2F, b: D2D_POINT_2F, tip: D2D_POINT_2F) {
    unsafe {
        let Ok(path) = d2d.factory.CreatePathGeometry() else { return; };
        let Ok(sink) = path.Open() else { return; };
        sink.BeginFigure(a, D2D1_FIGURE_BEGIN_FILLED);
        sink.AddLine(b);
        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
            point1: b,
            point2: D2D_POINT_2F { x: (b.x + tip.x) * 0.5, y: tip.y },
            point3: tip,
        });
        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
            point1: D2D_POINT_2F { x: (a.x + tip.x) * 0.5, y: tip.y },
            point2: a,
            point3: a,
        });
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        if sink.Close().is_ok() { d2d.context.FillGeometry(&path, brush, None); }
    }
}

fn happy_lid(d2d: &D2DContext, brush: &ID2D1SolidColorBrush, center: D2D_POINT_2F, radius: f32) {
    unsafe {
        let Ok(path) = d2d.factory.CreatePathGeometry() else { return; };
        let Ok(sink) = path.Open() else { return; };
        sink.BeginFigure(D2D_POINT_2F { x: center.x - radius, y: center.y + 1.0 }, D2D1_FIGURE_BEGIN_HOLLOW);
        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
            point1: D2D_POINT_2F { x: center.x - radius * 0.4, y: center.y - radius * 0.7 },
            point2: D2D_POINT_2F { x: center.x + radius * 0.4, y: center.y - radius * 0.7 },
            point3: D2D_POINT_2F { x: center.x + radius, y: center.y + 1.0 },
        });
        sink.EndFigure(D2D1_FIGURE_END_OPEN);
        if sink.Close().is_ok() { d2d.context.DrawGeometry(&path, brush, 1.8, &d2d.round_stroke_style); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn donut_has_the_same_radius_in_every_direction() {
        for angle in [0.0f32, PI * 0.25, PI * 0.5, PI * 0.75, PI] {
            let radius = 16.0;
            let distance = circular_radius(angle.cos() * radius, angle.sin() * radius, radius);
            assert!((distance - 1.0).abs() < 0.00001);
        }
    }

    #[test]
    fn gaze_is_smoothed_and_bounded() {
        let mut tiger = TigerMascot::default();
        tiger.update(0.016, (10.0, -10.0), false, false);
        assert!(tiger.look.0 > 0.0 && tiger.look.0 < 1.0);
        for _ in 0..300 { tiger.update(0.016, (10.0, -10.0), false, false); }
        assert!(tiger.look.0 <= 1.0 && tiger.look.1 >= -1.0);
    }

    #[test]
    fn blink_closes_and_reopens_both_eyes() {
        let mut tiger = TigerMascot::default();
        tiger.blink_remaining = BLINK_SECONDS;
        tiger.update(0.045, (0.0, 0.0), false, false);
        tiger.update(0.045, (0.0, 0.0), false, false);
        assert!(tiger.eye_open(false) < 0.01 && tiger.eye_open(true) < 0.01);
        for _ in 0..5 { tiger.update(0.05, (0.0, 0.0), false, false); }
        assert_eq!(tiger.eye_open(false), 1.0);
        assert_eq!(tiger.eye_open(true), 1.0);
    }

    #[test]
    fn click_winks_before_allowing_hover_expansion() {
        let mut tiger = TigerMascot::default();
        tiger.react();
        for _ in 0..4 { tiger.update(0.04, (0.0, 0.0), true, false); }
        assert!(tiger.eye_open(true) < 0.05);
        assert_eq!(tiger.eye_open(false), 1.0);
        for _ in 0..10 { tiger.update(0.05, (0.0, 0.0), true, false); }
        assert!(!tiger.should_expand());
        for _ in 0..6 { tiger.update(0.05, (0.0, 0.0), true, false); }
        assert!(tiger.should_expand());
        tiger.update(0.016, (0.0, 0.0), false, false);
        assert!(!tiger.should_expand());
    }
}
