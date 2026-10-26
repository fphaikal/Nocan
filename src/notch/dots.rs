use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;

pub const DEFAULT_TOTAL_SECTIONS: usize = 8;

pub struct DotsIndicator {
    pub active_index: usize,
    pub total_sections: usize,
}

impl DotsIndicator {
    pub fn new() -> Self {
        Self {
            active_index: 0,
            total_sections: DEFAULT_TOTAL_SECTIONS,
        }
    }

    pub fn set_total_sections(&mut self, total: usize) {
        self.total_sections = total;
        if self.active_index >= total && total > 0 {
            self.active_index = total - 1;
        }
    }

    pub fn set_index(&mut self, index: usize) {
        if index < self.total_sections {
            self.active_index = index;
        }
    }

    pub fn hit_test(&self, body_left: f32, body_w: f32, _body_h: f32, x: f32, y: f32) -> Option<usize> {
        if self.total_sections <= 1 {
            return None;
        }

        let center_x = body_left + body_w * 0.5;
        let y_pos = 133.0;

        if y < y_pos - 7.0 || y > y_pos + 7.0 {
            return None;
        }

        let inactive_w = 3.6;
        let active_w = 15.0;
        let gap = 4.5;
        let total_w = (self.total_sections - 1) as f32 * (inactive_w + gap) + active_w;
        let start_x = center_x - total_w * 0.5;

        if x < start_x - 4.0 || x > start_x + total_w + 4.0 {
            return None;
        }

        let mut cur_x = start_x;
        for i in 0..self.total_sections {
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
        page_pos: f32,
        _active_brush: &ID2D1SolidColorBrush,
        _inactive_brush: &ID2D1SolidColorBrush,
    ) {
        if self.total_sections <= 1 {
            return;
        }

        let center_x = body_left + body_w * 0.5;
        let y_pos = 133.0;

        let inactive_w = 3.6f32;
        let active_w = 15.0f32;
        let dot_h = 3.6f32;
        let gap = 4.5f32;

        let mut widths = vec![inactive_w; self.total_sections];
        let mut alphas = vec![0.45f32; self.total_sections];
        let mut total_w = (self.total_sections - 1) as f32 * gap;

        for i in 0..self.total_sections {
            let t = (1.0 - ((i as f32) - page_pos).abs()).clamp(0.0, 1.0);
            let smooth_t = t * t * (3.0 - 2.0 * t);
            widths[i] = inactive_w + (active_w - inactive_w) * smooth_t;
            alphas[i] = 0.45 + 0.55 * smooth_t;
            total_w += widths[i];
        }

        let mut cur_x = center_x - (total_w * 0.5);

        for i in 0..self.total_sections {
            let w = widths[i];
            let rect = D2D_RECT_F {
                left: cur_x,
                top: y_pos - (dot_h * 0.5),
                right: cur_x + w,
                bottom: y_pos + (dot_h * 0.5),
            };

            if let Ok(dot_b) = d2d.create_solid_brush(
                windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: alphas[i],
                },
            ) {
                d2d.fill_rounded_rect(rect, 1.8, 1.8, &dot_b);
            }

            cur_x += w + gap;
        }
    }
}
