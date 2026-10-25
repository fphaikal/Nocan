use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;

pub struct WaterSection {
    pub intake_ml: u32,
    pub goal_ml: u32,
    pub glasses_count: u32,
}

impl WaterSection {
    pub fn new() -> Self {
        Self {
            intake_ml: 1250,
            goal_ml: 2000,
            glasses_count: 5,
        }
    }

    pub fn drink(&mut self, amount_ml: u32) {
        self.intake_ml = (self.intake_ml + amount_ml).min(self.goal_ml * 2);
        self.glasses_count += 1;
    }

    pub fn reset(&mut self) {
        self.intake_ml = 0;
        self.glasses_count = 0;
    }

    pub fn draw(
        &self,
        d2d: &D2DContext,
        dwrite: &DWriteContext,
        bounds: D2D_RECT_F,
        white_brush: &ID2D1SolidColorBrush,
        dim_brush: &ID2D1SolidColorBrush,
        card_brush: &ID2D1SolidColorBrush,
        water_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;
        let right = bounds.right;
        let center_x = (left + right) / 2.0;

        // Header on top left
        let header_rect = D2D_RECT_F {
            left: left + 24.0,
            top: top + 10.0,
            right: left + 150.0,
            bottom: top + 24.0,
        };
        dwrite.draw_text(&d2d.context, "💧 Water Intake", header_rect, dim_brush, &dwrite.format_small);

        // Percentage on top right
        let percent = ((self.intake_ml as f32 / self.goal_ml as f32) * 100.0) as u32;
        let pct_rect = D2D_RECT_F {
            left: right - 120.0,
            top: top + 10.0,
            right: right - 24.0,
            bottom: top + 24.0,
        };
        let pct_str = format!("Goal: {}%", percent);
        dwrite.draw_text(&d2d.context, &pct_str, pct_rect, dim_brush, &dwrite.format_small);

        // Main Intake Display: "1,250 ml / 2,000 ml"
        let intake_str = format!("{} / {} ml", self.intake_ml, self.goal_ml);
        let intake_rect = D2D_RECT_F {
            left: center_x - 120.0,
            top: top + 24.0,
            right: center_x + 120.0,
            bottom: top + 52.0,
        };
        dwrite.draw_text(&d2d.context, &intake_str, intake_rect, white_brush, &dwrite.format_big);

        // Progress Bar
        let bar_left = left + 40.0;
        let bar_right = right - 40.0;
        let bar_top = top + 54.0;
        let bar_h = 4.0;

        let bg_bar_rect = D2D_RECT_F {
            left: bar_left,
            top: bar_top,
            right: bar_right,
            bottom: bar_top + bar_h,
        };
        d2d.fill_rounded_rect(bg_bar_rect, 2.0, 2.0, card_brush);

        let progress = (self.intake_ml as f32 / self.goal_ml as f32).clamp(0.0, 1.0);
        let fill_w = (bar_right - bar_left) * progress;
        let fill_bar_rect = D2D_RECT_F {
            left: bar_left,
            top: bar_top,
            right: bar_left + fill_w,
            bottom: bar_top + bar_h,
        };
        d2d.fill_rounded_rect(fill_bar_rect, 2.0, 2.0, water_brush);

        // Action Buttons
        let btn_y = top + 66.0;
        let drink_btn_rect = D2D_RECT_F {
            left: center_x - 65.0,
            top: btn_y,
            right: center_x + 10.0,
            bottom: btn_y + 22.0,
        };
        d2d.fill_rounded_rect(drink_btn_rect, 4.0, 4.0, water_brush);

        let drink_label_rect = D2D_RECT_F {
            left: drink_btn_rect.left,
            top: drink_btn_rect.top + 2.0,
            right: drink_btn_rect.right,
            bottom: drink_btn_rect.bottom,
        };
        dwrite.draw_text(&d2d.context, "+250 ml Drink", drink_label_rect, white_brush, &dwrite.format_center);

        let reset_btn_rect = D2D_RECT_F {
            left: center_x + 18.0,
            top: btn_y,
            right: center_x + 65.0,
            bottom: btn_y + 22.0,
        };
        d2d.fill_rounded_rect(reset_btn_rect, 4.0, 4.0, card_brush);

        let reset_label_rect = D2D_RECT_F {
            left: reset_btn_rect.left,
            top: reset_btn_rect.top + 2.0,
            right: reset_btn_rect.right,
            bottom: reset_btn_rect.bottom,
        };
        dwrite.draw_text(&d2d.context, "Reset", reset_label_rect, white_brush, &dwrite.format_center);
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let right = bounds.right;
        let center_x = (left + right) / 2.0;

        let btn_y = top + 66.0;
        if y >= btn_y && y <= btn_y + 22.0 {
            if x >= center_x - 65.0 && x <= center_x + 10.0 {
                self.drink(250);
                return true;
            }
            if x >= center_x + 18.0 && x <= center_x + 65.0 {
                self.reset();
                return true;
            }
        }
        false
    }
}
