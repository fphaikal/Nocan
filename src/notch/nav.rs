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

    pub fn prev_center(body_left: f32, _body_w: f32, body_h: f32) -> (f32, f32) {
        (body_left + 26.0, body_h - 19.0)
    }

    pub fn next_center(body_left: f32, body_w: f32, body_h: f32) -> (f32, f32) {
        (body_left + body_w - 26.0, body_h - 19.0)
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
        dx.abs() <= 13.0 && dy.abs() <= 12.0
    }

    pub fn hit_test_next(&self, body_left: f32, body_w: f32, body_h: f32, x: f32, y: f32, show_next: bool) -> bool {
        if !show_next {
            return false;
        }
        let (cx, cy) = Self::next_center(body_left, body_w, body_h);
        let dx = x - cx;
        let dy = y - cy;
        dx.abs() <= 13.0 && dy.abs() <= 12.0
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
        let _=(d2d,body_left,body_w,body_h,prev_alpha,next_alpha);
    }
}
