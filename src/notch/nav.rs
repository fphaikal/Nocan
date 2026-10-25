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

    pub fn get_prev_rect(notch_width: f32, notch_height: f32) -> D2D_RECT_F {
        let center_x = notch_width / 2.0;
        let y_pos = notch_height - 11.0;
        D2D_RECT_F {
            left: center_x - 62.0,
            top: y_pos - 10.0,
            right: center_x - 42.0,
            bottom: y_pos + 10.0,
        }
    }

    pub fn get_next_rect(notch_width: f32, notch_height: f32) -> D2D_RECT_F {
        let center_x = notch_width / 2.0;
        let y_pos = notch_height - 11.0;
        D2D_RECT_F {
            left: center_x + 42.0,
            top: y_pos - 10.0,
            right: center_x + 62.0,
            bottom: y_pos + 10.0,
        }
    }

    pub fn hit_test_prev(&self, notch_width: f32, notch_height: f32, x: f32, y: f32) -> bool {
        let r = Self::get_prev_rect(notch_width, notch_height);
        x >= r.left && x <= r.right && y >= r.top && y <= r.bottom
    }

    pub fn hit_test_next(&self, notch_width: f32, notch_height: f32, x: f32, y: f32) -> bool {
        let r = Self::get_next_rect(notch_width, notch_height);
        x >= r.left && x <= r.right && y >= r.top && y <= r.bottom
    }

    pub fn draw(
        &self,
        d2d: &D2DContext,
        dwrite: &DWriteContext,
        notch_width: f32,
        notch_height: f32,
        text_brush: &ID2D1SolidColorBrush,
        hover_brush: &ID2D1SolidColorBrush,
    ) {
        let prev_rect = Self::get_prev_rect(notch_width, notch_height);
        let next_rect = Self::get_next_rect(notch_width, notch_height);

        if self.hovered_prev {
            d2d.fill_rounded_rect(prev_rect, 4.0, 4.0, hover_brush);
        }
        if self.hovered_next {
            d2d.fill_rounded_rect(next_rect, 4.0, 4.0, hover_brush);
        }

        dwrite.draw_text(&d2d.context, "‹", prev_rect, text_brush, &dwrite.format_nav);
        dwrite.draw_text(&d2d.context, "›", next_rect, text_brush, &dwrite.format_nav);
    }
}
