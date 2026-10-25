use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;

pub const TOTAL_SECTIONS: usize = 6;

pub struct DotsIndicator {
    pub active_index: usize,
}

impl DotsIndicator {
    pub fn new() -> Self {
        Self { active_index: 0 }
    }

    pub fn set_index(&mut self, index: usize) {
        if index < TOTAL_SECTIONS {
            self.active_index = index;
        }
    }

    pub fn hit_test(&self, body_left: f32, body_w: f32, _body_h: f32, x: f32, y: f32) -> Option<usize> {
        let center_x = body_left + body_w * 0.5;
        let y_pos = 133.0;

        if y < y_pos - 7.0 || y > y_pos + 7.0 {
            return None;
        }

        let inactive_w = 3.6;
        let active_w = 15.0;
        let gap = 4.5;
        let total_w = (TOTAL_SECTIONS - 1) as f32 * (inactive_w + gap) + active_w;
        let start_x = center_x - total_w * 0.5;

        if x < start_x - 4.0 || x > start_x + total_w + 4.0 {
            return None;
        }

        let mut cur_x = start_x;
        for i in 0..TOTAL_SECTIONS {
            let w = if i == self.active_index { active_w } else { inactive_w };
            if x >= cur_x - 2.2 && x <= cur_x + w + 2.2 {
                return Some(i);
            }
            cur_x += w + gap;
        }
        None
    }

    pub fn draw(
        &self,
        d2d: &D2DContext,
        body_left: f32,
        body_w: f32,
        _body_h: f32,
        active_brush: &ID2D1SolidColorBrush,
        inactive_brush: &ID2D1SolidColorBrush,
    ) {
        let center_x = body_left + body_w * 0.5;
        let y_pos = 133.0;

        let inactive_w = 3.6;
        let active_w = 15.0;
        let dot_h = 3.6;
        let gap = 4.5;

        let total_w = (TOTAL_SECTIONS - 1) as f32 * (inactive_w + gap) + active_w;
        let mut cur_x = center_x - (total_w / 2.0);

        for i in 0..TOTAL_SECTIONS {
            let is_active = i == self.active_index;
            let w = if is_active { active_w } else { inactive_w };
            let rect = D2D_RECT_F {
                left: cur_x,
                top: y_pos - (dot_h / 2.0),
                right: cur_x + w,
                bottom: y_pos + (dot_h / 2.0),
            };

            let brush = if is_active {
                active_brush
            } else {
                inactive_brush
            };

            d2d.fill_rounded_rect(rect, 1.8, 1.8, brush);
            cur_x += w + gap;
        }
    }
}
