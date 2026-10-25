use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;

pub struct NavButtons {
    pub hovered_prev: bool,
    pub hovered_next: bool,
}

impl NavButtons {
    pub fn new() -> Self {
        Self {
            hovered_prev: false,
            hovered_next: false,
        }
    }

    pub fn prev_center(body_left: f32, _body_w: f32, _body_h: f32) -> (f32, f32) {
        (body_left + 22.0, 70.0)
    }

    pub fn next_center(body_left: f32, body_w: f32, _body_h: f32) -> (f32, f32) {
        (body_left + body_w - 25.0, 70.0)
    }

    pub fn get_prev_rect(body_left: f32, body_w: f32, body_h: f32) -> D2D_RECT_F {
        let (cx, cy) = Self::prev_center(body_left, body_w, body_h);
        D2D_RECT_F {
            left: cx - 11.0,
            top: cy - 11.0,
            right: cx + 11.0,
            bottom: cy + 11.0,
        }
    }

    pub fn get_next_rect(body_left: f32, body_w: f32, body_h: f32) -> D2D_RECT_F {
        let (cx, cy) = Self::next_center(body_left, body_w, body_h);
        D2D_RECT_F {
            left: cx - 11.0,
            top: cy - 11.0,
            right: cx + 11.0,
            bottom: cy + 11.0,
        }
    }

    pub fn hit_test_prev(&self, body_left: f32, body_w: f32, body_h: f32, x: f32, y: f32, show_prev: bool) -> bool {
        if !show_prev {
            return false;
        }
        let (cx, cy) = Self::prev_center(body_left, body_w, body_h);
        let dx = x - cx;
        let dy = y - cy;
        dx * dx + dy * dy <= 14.0 * 14.0
    }

    pub fn hit_test_next(&self, body_left: f32, body_w: f32, body_h: f32, x: f32, y: f32, show_next: bool) -> bool {
        if !show_next {
            return false;
        }
        let (cx, cy) = Self::next_center(body_left, body_w, body_h);
        let dx = x - cx;
        let dy = y - cy;
        dx * dx + dy * dy <= 14.0 * 14.0
    }

    pub fn draw(
        &self,
        d2d: &D2DContext,
        body_left: f32,
        body_w: f32,
        body_h: f32,
        prev_alpha: f32,
        next_alpha: f32,
        _circle_bg: &ID2D1SolidColorBrush,
        _circle_hover_bg: &ID2D1SolidColorBrush,
        _stroke_brush: &ID2D1SolidColorBrush,
    ) {
        let scale = 0.9f32; // 9px / 10px viewBox

        let pa = prev_alpha.clamp(0.0, 1.0);
        if pa > 0.01 {
            let (cx, cy) = Self::prev_center(body_left, body_w, body_h);
            let bg_alpha = (if self.hovered_prev { 0.42 } else { 0.28 }) * pa;
            if let (Ok(bg), Ok(stroke)) = (
                d2d.create_solid_brush(
                    windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: bg_alpha,
                    },
                ),
                d2d.create_solid_brush(
                    windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: pa,
                    },
                ),
            ) {
                d2d.fill_ellipse(cx, cy, 11.0, 11.0, &bg);

                // viewBox 0 0 10 10: M6.5 1.5 L3 5 L6.5 8.5
                let pts = [
                    (cx + (6.5 - 5.0) * scale, cy + (1.5 - 5.0) * scale),
                    (cx + (3.0 - 5.0) * scale, cy + (5.0 - 5.0) * scale),
                    (cx + (6.5 - 5.0) * scale, cy + (8.5 - 5.0) * scale),
                ];
                d2d.draw_polyline(&pts, &stroke, 2.1, false);
            }
        }

        let na = next_alpha.clamp(0.0, 1.0);
        if na > 0.01 {
            let (cx, cy) = Self::next_center(body_left, body_w, body_h);
            let bg_alpha = (if self.hovered_next { 0.42 } else { 0.28 }) * na;
            if let (Ok(bg), Ok(stroke)) = (
                d2d.create_solid_brush(
                    windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: bg_alpha,
                    },
                ),
                d2d.create_solid_brush(
                    windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: na,
                    },
                ),
            ) {
                d2d.fill_ellipse(cx, cy, 11.0, 11.0, &bg);

                // viewBox 0 0 10 10: M3.5 1.5 L7 5 L3.5 8.5
                let pts = [
                    (cx + (3.5 - 5.0) * scale, cy + (1.5 - 5.0) * scale),
                    (cx + (7.0 - 5.0) * scale, cy + (5.0 - 5.0) * scale),
                    (cx + (3.5 - 5.0) * scale, cy + (8.5 - 5.0) * scale),
                ];
                d2d.draw_polyline(&pts, &stroke, 2.1, false);
            }
        }
    }
}
