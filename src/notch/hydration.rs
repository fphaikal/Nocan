use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use crate::graphics::{d2d::D2DContext, dwrite::DWriteContext};

const DURATION: f32 = 2.2;

pub struct DrinkFeedback {
    elapsed: f32,
    before: u32,
    after: u32,
    goal: u32,
}

impl DrinkFeedback {
    pub fn new(before: u32, after: u32, goal: u32) -> Self {
        Self { elapsed: 0.0, before, after, goal: goal.max(1) }
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        self.elapsed = (self.elapsed + dt.max(0.0)).min(DURATION);
        self.elapsed < DURATION
    }

    fn amount(&self) -> u32 {
        let t = (self.elapsed / 0.65).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(3);
        (self.before as f32 + self.after.saturating_sub(self.before) as f32 * eased).round() as u32
    }

    pub fn draw(&self, d: &D2DContext, w: &DWriteContext, bounds: D2D_RECT_F) {
        let height = bounds.bottom - bounds.top;
        let width = bounds.right - bounds.left;
        let cy = (bounds.top + bounds.bottom) * 0.5;
        let cx = bounds.left + 28.0;
        let fade = ((DURATION - self.elapsed) / 0.3).clamp(0.0, 1.0);
        let Ok(white) = d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: fade }) else { return; };
        let Ok(dim) = d.create_solid_brush(D2D1_COLOR_F { r: 0.76, g: 0.91, b: 1.0, a: fade * 0.85 }) else { return; };
        let Ok(blue) = d.create_solid_brush(D2D1_COLOR_F { r: 0.22, g: 0.78, b: 1.0, a: fade }) else { return; };
        let Ok(wave) = d.create_solid_brush(D2D1_COLOR_F { r: 0.04, g: 0.56, b: 0.86, a: fade * 0.38 }) else { return; };

        let fill = (self.elapsed / 0.7).clamp(0.0, 1.0);
        let surface = bounds.bottom - height * 0.72 * (1.0 - (1.0 - fill).powi(3));
        let mut points = vec![(bounds.left, bounds.bottom)];
        for i in 0..=32 {
            let x = bounds.left + width * i as f32 / 32.0;
            let y = surface + (i as f32 * 0.35 - self.elapsed * 6.0).sin() * 4.5;
            points.push((x, y));
        }
        points.push((bounds.right, bounds.bottom));
        d.fill_polygon(&points, &wave);

        for i in 0..6 {
            let t = ((self.elapsed - i as f32 * 0.07) / 1.5).clamp(0.0, 1.0);
            if t > 0.0 && t < 1.0 {
                let x = bounds.right - 22.0 - i as f32 * 16.0 + (t * 5.0 + i as f32).sin() * 3.0;
                let y = bounds.bottom - 4.0 - t * (height - 10.0);
                if let Ok(bubble) = d.create_solid_brush(D2D1_COLOR_F { r: 0.25, g: 0.80, b: 1.0, a: (1.0 - t) * 0.5 * fade }) {
                    let radius = 2.3 + (i % 3) as f32 * 0.9;
                    d.draw_ellipse(x, y, radius, radius, &bubble, 1.0);
                }
            }
        }

        let pop = 1.0 + 0.16 * (self.elapsed * 13.0).sin() * (-self.elapsed * 4.0).exp();
        d.draw_progress_ring(cx, cy, 14.0 * pop, 2.4,
            (self.amount() as f32 / self.goal as f32).clamp(0.0, 1.0), &dim, &blue);
        let check = ((self.elapsed - 0.08) / 0.4).clamp(0.0, 1.0);
        let first = (check * 2.0).min(1.0);
        d.draw_line(cx - 6.0, cy, cx - 6.0 + 5.0 * first, cy + 5.0 * first, &white, 2.1, true);
        if check > 0.5 {
            let second = (check - 0.5) * 2.0;
            d.draw_line(cx - 1.0, cy + 5.0, cx - 1.0 + 9.0 * second, cy + 5.0 - 11.0 * second, &white, 2.1, true);
        }

        let title = if self.after >= self.goal { "Daily goal reached!" } else { "Water logged!" };
        w.draw_text(&d.context, title, D2D_RECT_F {
            left: bounds.left + 52.0, top: cy - 21.0, right: bounds.right - 18.0, bottom: cy + 1.0,
        }, &white, &w.format_title);
        let detail = format!("+{} ml  ·  {} / {} ml", self.after.saturating_sub(self.before), self.amount(), self.goal);
        w.draw_text(&d.context, &detail, D2D_RECT_F {
            left: bounds.left + 52.0, top: cy + 2.0, right: bounds.right - 18.0, bottom: cy + 22.0,
        }, &dim, &w.format_sub);
    }
}

pub fn reminder_motion(seconds: f32) -> (f32, f32, f32) {
    let cycle = (seconds.max(0.0) / 2.4).fract();
    let breath = (cycle * std::f32::consts::TAU).sin();
    (breath * 2.8, 15.0 + cycle * 8.0, (1.0 - cycle).powi(2) * 0.7)
}

pub fn draw_reminder_water(d: &D2DContext, bounds: D2D_RECT_F, seconds: f32, hover: bool) {
    let seconds = seconds.rem_euclid(2.4);
    let width = bounds.right - bounds.left;
    let height = bounds.bottom - bounds.top;
    let depth = if hover { 19.0 } else { 12.0 };
    for layer in 0..2 {
        let phase = seconds * std::f32::consts::TAU / 2.4 * if layer == 0 { 1.0 } else { -1.0 };
        let level = bounds.bottom - depth + layer as f32 * 5.0;
        let mut points = vec![(bounds.left, bounds.bottom)];
        for i in 0..=40 {
            let x = bounds.left + width * i as f32 / 40.0;
            let y = level + (i as f32 * 0.23 + phase).sin() * (3.5 + layer as f32);
            points.push((x, y));
        }
        points.push((bounds.right, bounds.bottom));
        if let Ok(water) = d.create_solid_brush(D2D1_COLOR_F {
            r: 0.02, g: 0.48 + layer as f32 * 0.18, b: 0.90, a: 0.30 - layer as f32 * 0.06,
        }) { d.fill_polygon(&points, &water); }
    }
    for i in 0..7 {
        let life = (seconds / 2.4 + i as f32 / 7.0).fract();
        let x = bounds.left + 18.0 + i as f32 * (width - 36.0) / 6.0 + (life * 5.0).sin() * 3.0;
        let y = bounds.bottom - 2.0 - life * (height * 0.5);
        if let Ok(bubble) = d.create_solid_brush(D2D1_COLOR_F {
            r: 0.3, g: 0.84, b: 1.0, a: (1.0 - life) * 0.6,
        }) {
            let radius = 1.6 + (i % 3) as f32 * 0.6;
            d.draw_ellipse(x, y, radius, radius, &bubble, 1.2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acknowledgement_counts_up_once_then_finishes() {
        let mut feedback = DrinkFeedback::new(500, 750, 1500);
        assert_eq!(feedback.amount(), 500);
        assert!(feedback.advance(0.3));
        assert!(feedback.amount() > 500 && feedback.amount() < 750);
        assert!(feedback.advance(0.4));
        assert_eq!(feedback.amount(), 750);
        assert!(!feedback.advance(2.0));
        assert!(!feedback.advance(1.0));
        assert_eq!(feedback.amount(), 750);
    }

    #[test]
    fn reminder_repeats_without_moving_the_controls() {
        let a = reminder_motion(0.6);
        let b = reminder_motion(3.0);
        assert!((a.0 - b.0).abs() < 0.0001);
        assert!((a.1 - b.1).abs() < 0.0001);
        assert!((a.2 - b.2).abs() < 0.0001);
        assert_eq!(reminder_motion(0.0).0, 0.0);
    }
}
