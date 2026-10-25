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

    pub fn hit_test(&self, notch_width: f32, notch_height: f32, x: f32, y: f32) -> Option<usize> {
        let center_x = notch_width / 2.0;
        let y_pos = notch_height - 9.0;

        if y < y_pos - 8.0 || y > y_pos + 8.0 {
            return None;
        }

        let total_w = 76.0;
        let start_x = center_x - (total_w / 2.0);
        if x < start_x || x > start_x + total_w {
            return None;
        }

        let rel_x = x - start_x;
        let idx = ((rel_x / total_w) * TOTAL_SECTIONS as f32) as usize;
        Some(idx.min(TOTAL_SECTIONS - 1))
    }

    pub fn draw(
        &self,
        d2d: &D2DContext,
        notch_width: f32,
        notch_height: f32,
        active_brush: &ID2D1SolidColorBrush,
        inactive_brush: &ID2D1SolidColorBrush,
    ) {
        let center_x = notch_width / 2.0;
        let y_pos = notch_height - 9.0;

        let inactive_w = 4.0;
        let active_w = 16.0;
        let dot_h = 4.0;
        let gap = 6.0;

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

            d2d.fill_rounded_rect(rect, 2.0, 2.0, brush);
            cur_x += w + gap;
        }
    }
}
