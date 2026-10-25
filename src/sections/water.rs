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
        water_brush: &ID2D1SolidColorBrush,
        btn_dark_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;

        // Section Title: "WaterReminder"
        let title_rect = D2D_RECT_F {
            left: left + 24.0,
            top: top + 10.0,
            right: left + 160.0,
            bottom: top + 26.0,
        };
        dwrite.draw_text(&d2d.context, "WaterReminder", title_rect, white_brush, &dwrite.format_title);

        // Circular Water Graphic on Left: center (left + 78, top + 66), radius 32
        let water_center_x = left + 44.0 + 34.0;
        let water_center_y = top + 32.0 + 34.0;
        d2d.fill_ellipse(water_center_x, water_center_y, 32.0, 32.0, water_brush);

        let icon_rect = D2D_RECT_F {
            left: water_center_x - 16.0,
            top: water_center_y - 14.0,
            right: water_center_x + 16.0,
            bottom: water_center_y + 14.0,
        };
        dwrite.draw_text(&d2d.context, "💧", icon_rect, white_brush, &dwrite.format_center);

        // Right side: Intake and buttons
        let percent = ((self.intake_ml as f32 / self.goal_ml as f32) * 100.0) as u32;
        let intake_str = format!("{} ml", self.intake_ml);

        let intake_rect = D2D_RECT_F {
            left: left + 215.0,
            top: top + 30.0,
            right: left + 360.0,
            bottom: top + 56.0,
        };
        dwrite.draw_text(&d2d.context, &intake_str, intake_rect, white_brush, &dwrite.format_big);

        let goal_str = format!("Goal: {}% • Stay hydrated!", percent);
        let goal_rect = D2D_RECT_F {
            left: left + 217.0,
            top: top + 58.0,
            right: left + 375.0,
            bottom: top + 74.0,
        };
        dwrite.draw_text(&d2d.context, &goal_str, goal_rect, dim_brush, &dwrite.format_regular);

        // Buttons at y=84
        let btn_y = top + 84.0;

        let drink_btn_rect = D2D_RECT_F {
            left: left + 215.0,
            top: btn_y,
            right: left + 295.0,
            bottom: btn_y + 26.0,
        };
        d2d.fill_rounded_rect(drink_btn_rect, 13.0, 13.0, water_brush);

        let drink_text_rect = D2D_RECT_F {
            left: drink_btn_rect.left,
            top: drink_btn_rect.top + 3.0,
            right: drink_btn_rect.right,
            bottom: drink_btn_rect.bottom,
        };
        dwrite.draw_text(&d2d.context, "+250 ml", drink_text_rect, white_brush, &dwrite.format_button);

        let reset_btn_rect = D2D_RECT_F {
            left: left + 302.0,
            top: btn_y,
            right: left + 362.0,
            bottom: btn_y + 26.0,
        };
        d2d.fill_rounded_rect(reset_btn_rect, 13.0, 13.0, btn_dark_brush);

        let reset_text_rect = D2D_RECT_F {
            left: reset_btn_rect.left,
            top: reset_btn_rect.top + 3.0,
            right: reset_btn_rect.right,
            bottom: reset_btn_rect.bottom,
        };
        dwrite.draw_text(&d2d.context, "Reset", reset_text_rect, white_brush, &dwrite.format_button);
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let btn_y = top + 84.0;

        if y >= btn_y - 2.0 && y <= btn_y + 28.0 {
            if x >= left + 215.0 && x <= left + 295.0 {
                self.drink(250);
                return true;
            }
            if x >= left + 302.0 && x <= left + 362.0 {
                self.reset();
                return true;
            }
        }
        false
    }
}
