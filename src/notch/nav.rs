use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;

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

    pub fn get_prev_rect(_notch_width: f32, notch_height: f32) -> D2D_RECT_F {
        let center_y = notch_height / 2.0;
        D2D_RECT_F {
            left: 10.0,
            top: center_y - 14.0,
            right: 32.0,
            bottom: center_y + 14.0,
        }
    }

    pub fn get_next_rect(notch_width: f32, notch_height: f32) -> D2D_RECT_F {
        let center_y = notch_height / 2.0;
        D2D_RECT_F {
            left: notch_width - 32.0,
            top: center_y - 14.0,
            right: notch_width - 10.0,
            bottom: center_y + 14.0,
        }
    }

    pub fn hit_test_prev(&self, notch_width: f32, notch_height: f32, x: f32, y: f32) -> bool {
        let r = Self::get_prev_rect(notch_width, notch_height);
        x >= r.left - 4.0 && x <= r.right + 4.0 && y >= r.top - 4.0 && y <= r.bottom + 4.0
    }

    pub fn hit_test_next(&self, notch_width: f32, notch_height: f32, x: f32, y: f32) -> bool {
        let r = Self::get_next_rect(notch_width, notch_height);
        x >= r.left - 4.0 && x <= r.right + 4.0 && y >= r.top - 4.0 && y <= r.bottom + 4.0
    }

    pub fn draw(
        &self,
        d2d: &D2DContext,
        dwrite: &DWriteContext,
        notch_width: f32,
        notch_height: f32,
        normal_brush: &ID2D1SolidColorBrush,
        hover_brush: &ID2D1SolidColorBrush,
    ) {
        let prev_rect = Self::get_prev_rect(notch_width, notch_height);
        let next_rect = Self::get_next_rect(notch_width, notch_height);

        let p_brush = if self.hovered_prev { hover_brush } else { normal_brush };
        let n_brush = if self.hovered_next { hover_brush } else { normal_brush };

        dwrite.draw_text(&d2d.context, "‹", prev_rect, p_brush, &dwrite.format_nav);
        dwrite.draw_text(&d2d.context, "›", next_rect, n_brush, &dwrite.format_nav);
    }
}
