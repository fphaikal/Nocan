use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;

pub const DEFAULT_TOTAL_SECTIONS: usize = 8;
const INACTIVE_WIDTH: f32 = 4.5;
const ACTIVE_WIDTH: f32 = 16.0;
const BAR_GAP: f32 = 7.0;

pub struct DotsIndicator {
    pub active_index: usize,
    pub total_sections: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn footer_has_one_clickable_bar_per_section() {
        let mut dots = DotsIndicator::new();
        dots.set_total_sections(8);
        for index in 0..8 {
            let x = 10.0 + dots.bar_left(448.0, index) + dots.bar_width(index) * 0.5;
            assert_eq!(dots.hit_test(10.0, 448.0, 180.0, x, 166.0), Some(index));
        }
        assert_eq!(dots.hit_test(10.0, 448.0, 180.0, 234.0, 140.0), None);
        assert!(dots.hit_test_prev(10.0, 448.0, 180.0, dots.prev_center(10.0, 448.0, 180.0).0, 162.0));
        assert!(dots.hit_test_next(10.0, 448.0, 180.0, dots.next_center(10.0, 448.0, 180.0).0, 162.0));
    }
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

    fn bar_width(&self, index: usize) -> f32 {
        if index == self.active_index { ACTIVE_WIDTH } else { INACTIVE_WIDTH }
    }

    pub fn total_bar_width(&self) -> f32 {
        self.bar_widths().iter().sum::<f32>()
            + self.total_sections.saturating_sub(1) as f32 * BAR_GAP
    }

    fn bar_left(&self, body_w: f32, index: usize) -> f32 {
        let total = self.total_bar_width();
        let start = (body_w - total) * 0.5;
        start + (0..index).map(|i| self.bar_width(i) + BAR_GAP).sum::<f32>()
    }

    fn bar_widths(&self) -> Vec<f32> {
        (0..self.total_sections).map(|i| self.bar_width(i)).collect()
    }

    pub fn prev_center(&self, body_left: f32, body_w: f32, body_h: f32) -> (f32, f32) {
        let total = self.total_bar_width();
        let start = (body_w - total) * 0.5;
        (body_left + start - 16.0, body_h - 18.0)
    }

    pub fn next_center(&self, body_left: f32, body_w: f32, body_h: f32) -> (f32, f32) {
        let total = self.total_bar_width();
        let start = (body_w - total) * 0.5;
        (body_left + start + total + 16.0, body_h - 18.0)
    }

    pub fn hit_test_prev(&self, body_left: f32, body_w: f32, body_h: f32, x: f32, y: f32) -> bool {
        if self.total_sections <= 1 { return false; }
        let (cx, cy) = self.prev_center(body_left, body_w, body_h);
        (x - cx).abs() <= 12.0 && (y - cy).abs() <= 12.0
    }

    pub fn hit_test_next(&self, body_left: f32, body_w: f32, body_h: f32, x: f32, y: f32) -> bool {
        if self.total_sections <= 1 { return false; }
        let (cx, cy) = self.next_center(body_left, body_w, body_h);
        (x - cx).abs() <= 12.0 && (y - cy).abs() <= 12.0
    }

    pub fn hit_test(&self, body_left: f32, body_w: f32, body_h: f32, x: f32, y: f32) -> Option<usize> {
        if y < body_h - 28.0 || y > body_h - 8.0 { return None; }
        (0..self.total_sections).find(|&index| {
            let left = body_left + self.bar_left(body_w, index) - 4.0;
            let right = left + self.bar_width(index) + 8.0;
            x >= left && x <= right
        })
    }

    pub fn draw_icons(
        &self,
        d2d: &D2DContext,
        _dwrite: &crate::graphics::dwrite::DWriteContext,
        body_left: f32,
        body_w: f32,
        body_h: f32,
        page_pos: f32,
        sections: &[crate::notch::carousel::SectionId],
    ) {
        let _ = (_dwrite, sections);
        if self.total_sections == 0 { return; }

        let widths: Vec<f32> = (0..self.total_sections).map(|index| {
            let active = (1.0 - (index as f32 - page_pos).abs()).clamp(0.0, 1.0);
            let smooth = active * active * (3.0 - 2.0 * active);
            INACTIVE_WIDTH + (ACTIVE_WIDTH - INACTIVE_WIDTH) * smooth
        }).collect();

        let total = widths.iter().sum::<f32>() + self.total_sections.saturating_sub(1) as f32 * BAR_GAP;
        let start = (body_w - total) * 0.5;
        let mut x = body_left + start;
        let y = body_h - 19.25;
        let arrow_cy = body_h - 18.0;

        // Draw Left Navigation Arrow (‹)
        let prev_cx = body_left + start - 16.0;
        if let Ok(arrow_b) = d2d.create_solid_brush(
            windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F {
                r: 0.78, g: 0.83, b: 0.91, a: 0.65,
            }
        ) {
            d2d.draw_polyline(
                &[(prev_cx + 2.0, arrow_cy - 3.5), (prev_cx - 2.0, arrow_cy), (prev_cx + 2.0, arrow_cy + 3.5)],
                &arrow_b,
                1.4,
                false,
            );
        }

        // Draw Section Indicator Bars
        for (index, width) in widths.iter().copied().enumerate() {
            let active = (1.0 - (index as f32 - page_pos).abs()).clamp(0.0, 1.0);
            let alpha = 0.30 + 0.62 * active;
            if let Ok(bar) = d2d.create_solid_brush(
                windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F {
                    r: 0.78, g: 0.83, b: 0.91, a: alpha,
                }
            ) {
                d2d.fill_rounded_rect(
                    D2D_RECT_F { left: x, top: y, right: x + width, bottom: y + 2.5 },
                    1.25,
                    1.25,
                    &bar,
                );
            }
            x += width + BAR_GAP;
        }

        // Draw Right Navigation Arrow (›)
        let next_cx = body_left + start + total + 16.0;
        if let Ok(arrow_b) = d2d.create_solid_brush(
            windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F {
                r: 0.78, g: 0.83, b: 0.91, a: 0.65,
            }
        ) {
            d2d.draw_polyline(
                &[(next_cx - 2.0, arrow_cy - 3.5), (next_cx + 2.0, arrow_cy), (next_cx - 2.0, arrow_cy + 3.5)],
                &arrow_b,
                1.4,
                false,
            );
        }
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
