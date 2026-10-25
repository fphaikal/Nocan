use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;

pub struct WaterSection {
    pub intake_ml: u32,
    pub goal_ml: u32,
    pub reminder_on: bool,
    pub hourly_ml: [u32; 8], // 09..16
    pub anim_progress: f32,
}

impl WaterSection {
    pub fn new() -> Self {
        Self {
            intake_ml: 1500,
            goal_ml: 1500,
            reminder_on: true,
            hourly_ml: [300, 200, 200, 150, 150, 200, 200, 200],
            anim_progress: 1.0,
        }
    }

    pub fn trigger_grow_anim(&mut self) {
        self.anim_progress = 0.0;
    }

    pub fn update(&mut self, dt: f32) {
        if self.anim_progress < 1.0 {
            self.anim_progress = (self.anim_progress + dt * 2.2).min(1.0);
        }
    }

    pub fn drink(&mut self, amount_ml: u32) {
        self.intake_ml = (self.intake_ml + amount_ml).min(self.goal_ml * 2);
        // Add to current hour slot (or last slot)
        let hour = unsafe { windows::Win32::System::SystemInformation::GetLocalTime().wHour };
        let idx = if hour >= 9 && hour <= 16 {
            (hour - 9) as usize
        } else {
            7
        };
        self.hourly_ml[idx] = (self.hourly_ml[idx] + amount_ml).min(400);
    }

    pub fn cycle_settings(&mut self) {
        if self.goal_ml == 1500 {
            self.goal_ml = 2000;
        } else if self.goal_ml == 2000 {
            self.goal_ml = 2500;
        } else {
            self.goal_ml = 1500;
            self.intake_ml = 0;
            self.hourly_ml = [0; 8];
        }
    }

    pub fn draw(
        &self,
        d2d: &D2DContext,
        dwrite: &DWriteContext,
        bounds: D2D_RECT_F,
        white_brush: &ID2D1SolidColorBrush,
        dim_brush: &ID2D1SolidColorBrush,
        pill_bg_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;

        // 1. Section Title: "Water " (#fff) + "Reminder" (#56c8ff) at left: 15px, top: 19px (vc)
        let title_w_rect = D2D_RECT_F {
            left: left + 15.0,
            top: top + 9.0,
            right: left + 65.0,
            bottom: top + 29.0,
        };
        dwrite.draw_text(&d2d.context, "Water", title_w_rect, white_brush, &dwrite.format_title);

        if let Ok(cyan_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 86.0 / 255.0,
            g: 200.0 / 255.0,
            b: 1.0,
            a: 1.0,
        }) {
            let title_r_rect = D2D_RECT_F {
                left: left + 61.0,
                top: top + 9.0,
                right: left + 145.0,
                bottom: top + 29.0,
            };
            dwrite.draw_text(&d2d.context, "Reminder", title_r_rect, &cyan_b, &dwrite.format_title);
        }

        // 2. Left Column: Intake "1.5" (30px bold) + " / 1.5 L" (13px dim) at left: 44px, top: 47px (vc)
        let liters = self.intake_ml as f32 / 1000.0;
        let goal_liters = self.goal_ml as f32 / 1000.0;
        let intake_str = format!("{:.1}", liters);
        let intake_rect = D2D_RECT_F {
            left: left + 44.0,
            top: top + 30.0,
            right: left + 100.0,
            bottom: top + 64.0,
        };
        dwrite.draw_text(&d2d.context, &intake_str, intake_rect, white_brush, &dwrite.format_big);

        let goal_unit_str = format!("/ {:.1} L", goal_liters);
        let goal_unit_rect = D2D_RECT_F {
            left: left + 92.0,
            top: top + 39.0,
            right: left + 155.0,
            bottom: top + 59.0,
        };
        dwrite.draw_text(&d2d.context, &goal_unit_str, goal_unit_rect, dim_brush, &dwrite.format_regular);

        // 3. Toggle Switch (.sw) at left: 162px, top: 37px, width: 34px, height: 20px, rx: 10px
        let sw_rect = D2D_RECT_F {
            left: left + 162.0,
            top: top + 37.0,
            right: left + 196.0,
            bottom: top + 57.0,
        };
        if self.reminder_on {
            if let Ok(sw_b) = d2d.create_solid_brush(D2D1_COLOR_F {
                r: 38.0 / 255.0,
                g: 153.0 / 255.0,
                b: 1.0,
                a: 1.0,
            }) {
                d2d.fill_rounded_rect(sw_rect, 10.0, 10.0, &sw_b);
                let knob_cx = left + 186.0;
                let knob_cy = top + 47.0;
                d2d.fill_ellipse(knob_cx, knob_cy, 8.0, 8.0, white_brush);
                // Tiny blue bell inside knob
                let bell_pts = [
                    (knob_cx - 3.0, knob_cy + 2.0),
                    (knob_cx - 2.2, knob_cy - 1.5),
                    (knob_cx, knob_cy - 3.0),
                    (knob_cx + 2.2, knob_cy - 1.5),
                    (knob_cx + 3.0, knob_cy + 2.0),
                ];
                d2d.fill_polygon(&bell_pts, &sw_b);
                d2d.fill_ellipse(knob_cx, knob_cy + 2.6, 1.0, 1.0, &sw_b);
            }
        } else {
            d2d.fill_rounded_rect(sw_rect, 10.0, 10.0, pill_bg_brush);
            d2d.fill_ellipse(left + 172.0, top + 47.0, 8.0, 8.0, dim_brush);
        }

        // 4. Subtitle at left: 44px, top: 71px (vc), font-size: 11.5px
        let pct = ((self.intake_ml as f32 / self.goal_ml.max(1) as f32) * 100.0).round() as u32;
        let status_tail = if pct >= 100 { "Goal reached" } else { "Stay hydrated" };

        let sub1_rect = D2D_RECT_F {
            left: left + 44.0,
            top: top + 62.0,
            right: left + 88.0,
            bottom: top + 80.0,
        };
        dwrite.draw_text(&d2d.context, "Today ·", sub1_rect, dim_brush, &dwrite.format_sub);

        let pct_str = format!("{}%", pct);
        if let Ok(pct_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 79.0 / 255.0,
            g: 192.0 / 255.0,
            b: 1.0,
            a: 1.0,
        }) {
            let pct_rect = D2D_RECT_F {
                left: left + 86.0,
                top: top + 62.0,
                right: left + 124.0,
                bottom: top + 80.0,
            };
            dwrite.draw_text(&d2d.context, &pct_str, pct_rect, &pct_b, &dwrite.format_sub);
        }

        let tail_offset = if pct >= 100 { 118.0 } else { 112.0 };
        let sub2_rect = D2D_RECT_F {
            left: left + tail_offset,
            top: top + 62.0,
            right: left + 204.0,
            bottom: top + 80.0,
        };
        let tail_str = format!("· {}", status_tail);
        dwrite.draw_text(&d2d.context, &tail_str, sub2_rect, dim_brush, &dwrite.format_sub);

        // 5. Progress Bar (.wbar) at left: 44px, top: 82.5px, width: 152px, height: 5px, rx: 2.5px
        let wbar_track = D2D_RECT_F {
            left: left + 44.0,
            top: top + 82.5,
            right: left + 196.0,
            bottom: top + 87.5,
        };
        d2d.fill_rounded_rect(wbar_track, 2.5, 2.5, pill_bg_brush);

        let fill_ratio = (self.intake_ml as f32 / self.goal_ml.max(1) as f32).clamp(0.0, 1.0);
        if fill_ratio > 0.01 {
            let fill_w = (152.0 * fill_ratio).max(5.0);
            let wbar_fill = D2D_RECT_F {
                left: left + 44.0,
                top: top + 82.5,
                right: left + 44.0 + fill_w,
                bottom: top + 87.5,
            };
            if let Ok(grad) = d2d.create_linear_gradient(
                (left + 44.0, top + 85.0),
                (left + 196.0, top + 85.0),
                &[
                    (0.0, D2D1_COLOR_F { r: 102.0 / 255.0, g: 219.0 / 255.0, b: 1.0, a: 1.0 }),
                    (1.0, D2D1_COLOR_F { r: 45.0 / 255.0, g: 107.0 / 255.0, b: 1.0, a: 1.0 }),
                ],
            ) {
                d2d.fill_rounded_rect_linear(wbar_fill, 2.5, 2.5, &grad);
            }
        }

        // 6. Drink & Settings Buttons at top: 98px, height: 26px, rx: 13px
        let btn_top = top + 98.0;
        let btn_bot = btn_top + 26.0;
        let btn_cy = btn_top + 13.0;

        // Drink button: left: 44px, width: 63px, bg: #0e4a86
        let drink_rect = D2D_RECT_F {
            left: left + 44.0,
            top: btn_top,
            right: left + 107.0,
            bottom: btn_bot,
        };
        if let Ok(drink_bg) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 14.0 / 255.0,
            g: 74.0 / 255.0,
            b: 134.0 / 255.0,
            a: 1.0,
        }) {
            d2d.fill_rounded_rect(drink_rect, 13.0, 13.0, &drink_bg);
        }
        if let Ok(drop_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 127.0 / 255.0,
            g: 208.0 / 255.0,
            b: 1.0,
            a: 1.0,
        }) {
            d2d.fill_droplet(left + 56.5, btn_cy, 6.5, &drop_b);
        }
        let drink_txt_rect = D2D_RECT_F {
            left: left + 65.0,
            top: btn_top,
            right: left + 105.0,
            bottom: btn_bot,
        };
        dwrite.draw_text(&d2d.context, "Drink", drink_txt_rect, white_brush, &dwrite.format_button);

        // Settings button: left: 113px, width: 76px, bg: rgba(255,255,255,0.12)
        let set_rect = D2D_RECT_F {
            left: left + 113.0,
            top: btn_top,
            right: left + 189.0,
            bottom: btn_bot,
        };
        d2d.fill_rounded_rect(set_rect, 13.0, 13.0, pill_bg_brush);
        // Sliders icon at left + 125.5, btn_cy
        let sx = left + 125.5;
        d2d.draw_line(sx - 4.5, btn_cy - 2.5, sx + 4.5, btn_cy - 2.5, white_brush, 1.3, true);
        d2d.draw_line(sx - 4.5, btn_cy + 2.5, sx + 4.5, btn_cy + 2.5, white_brush, 1.3, true);
        d2d.fill_ellipse(sx - 1.5, btn_cy - 2.5, 1.6, 1.6, white_brush);
        d2d.fill_ellipse(sx + 1.5, btn_cy + 2.5, 1.6, 1.6, white_brush);

        let set_txt_rect = D2D_RECT_F {
            left: left + 135.0,
            top: btn_top,
            right: left + 187.0,
            bottom: btn_bot,
        };
        dwrite.draw_text(&d2d.context, "Settings", set_txt_rect, white_brush, &dwrite.format_button);

        // 7. Vertical Divider at left: 208px, top: 22px, height: 102px
        if let Ok(div_b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.12 }) {
            d2d.fill_rect(
                D2D_RECT_F {
                    left: left + 208.0,
                    top: top + 22.0,
                    right: left + 209.0,
                    bottom: top + 124.0,
                },
                &div_b,
            );
        }

        // 8. Right Column: Hourly Bar Chart
        // Top-right mini dots at (360, 41) and (368, 41)
        d2d.fill_ellipse(left + 360.0, top + 41.0, 2.0, 2.0, dim_brush);
        d2d.fill_ellipse(left + 368.0, top + 41.0, 2.5, 2.5, white_brush);

        let chart_x = left + 220.0;
        let chart_y = top + 56.0;
        let chart_h = 50.0;

        // Dotted grid lines at top + 56 and top + 81
        if let Ok(grid_b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.22 }) {
            d2d.draw_dotted_line(chart_x, chart_y, left + 338.0, chart_y, &grid_b, 1.1);
            d2d.draw_dotted_line(chart_x, chart_y + 25.0, left + 338.0, chart_y + 25.0, &grid_b, 1.1);
        }

        // 8 Hourly Columns
        let col_track_b = d2d
            .create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.07 })
            .ok();
        let col_grad = d2d
            .create_linear_gradient(
                (chart_x, chart_y),
                (chart_x, chart_y + chart_h),
                &[
                    (0.0, D2D1_COLOR_F { r: 92.0 / 255.0, g: 198.0 / 255.0, b: 1.0, a: 0.90 }),
                    (1.0, D2D1_COLOR_F { r: 29.0 / 255.0, g: 99.0 / 255.0, b: 224.0 / 255.0, a: 0.90 }),
                ],
            )
            .ok();

        for (i, &val) in self.hourly_ml.iter().enumerate() {
            let x = chart_x + (i as f32 + 0.5) * 14.0 - 3.5;
            if let Some(tb) = &col_track_b {
                let track_r = D2D_RECT_F {
                    left: x,
                    top: chart_y + 1.5,
                    right: x + 7.0,
                    bottom: chart_y + chart_h,
                };
                d2d.fill_rounded_rect(track_r, 3.5, 3.5, tb);
            }
            if val > 0 {
                let p = self.anim_progress.clamp(0.0, 1.0);
                let ease = 1.0 - (1.0 - p).powi(3);
                let bh = ((val as f32 / 400.0).clamp(0.0, 1.0) * 48.5 * ease).max(2.0);
                let bar_r = D2D_RECT_F {
                    left: x,
                    top: chart_y + chart_h - 1.5 - bh,
                    right: x + 7.0,
                    bottom: chart_y + chart_h - 1.5,
                };
                if let Some(cg) = &col_grad {
                    d2d.fill_rounded_rect_linear(bar_r, 3.5, 3.5, cg);
                }
            }
        }

        // Bottom & Right chart axis lines
        if let Ok(ax_x) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 80.0 / 255.0,
            g: 180.0 / 255.0,
            b: 1.0,
            a: 0.45,
        }) {
            d2d.fill_rect(
                D2D_RECT_F {
                    left: left + 216.0,
                    top: top + 106.0,
                    right: left + 338.0,
                    bottom: top + 107.0,
                },
                &ax_x,
            );
        }
        if let Ok(ax_y) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.18 }) {
            d2d.fill_rect(
                D2D_RECT_F {
                    left: left + 338.0,
                    top: top + 50.0,
                    right: left + 339.0,
                    bottom: top + 106.0,
                },
                &ax_y,
            );
        }

        // Y-axis labels: "400" (y=56), "200" (y=81), "0 ml" (y=106) at left: 344px
        dwrite.draw_text(
            &d2d.context,
            "400",
            D2D_RECT_F { left: left + 344.0, top: top + 49.0, right: left + 375.0, bottom: top + 63.0 },
            dim_brush,
            &dwrite.format_small,
        );
        dwrite.draw_text(
            &d2d.context,
            "200",
            D2D_RECT_F { left: left + 344.0, top: top + 74.0, right: left + 375.0, bottom: top + 88.0 },
            dim_brush,
            &dwrite.format_small,
        );
        dwrite.draw_text(
            &d2d.context,
            "0 ml",
            D2D_RECT_F { left: left + 344.0, top: top + 99.0, right: left + 375.0, bottom: top + 113.0 },
            dim_brush,
            &dwrite.format_small,
        );

        // X-axis labels: "09" (x=227), "12" (x=269), "15" (x=311) at y=117
        for &(cx_off, lbl) in &[(227.0f32, "09"), (269.0f32, "12"), (311.0f32, "15")] {
            dwrite.draw_text(
                &d2d.context,
                lbl,
                D2D_RECT_F {
                    left: left + cx_off - 12.0,
                    top: top + 110.0,
                    right: left + cx_off + 12.0,
                    bottom: top + 124.0,
                },
                dim_brush,
                &dwrite.format_small_center,
            );
        }
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;

        // Toggle switch at left + 162..196, top + 37..57
        if x >= left + 160.0 && x <= left + 198.0 && y >= top + 35.0 && y <= top + 59.0 {
            self.reminder_on = !self.reminder_on;
            return true;
        }

        // Drink & Settings buttons at top + 98..124
        if y >= top + 96.0 && y <= top + 126.0 {
            if x >= left + 44.0 && x <= left + 107.0 {
                self.drink(250);
                return true;
            }
            if x >= left + 113.0 && x <= left + 189.0 {
                self.cycle_settings();
                return true;
            }
        }
        false
    }

    pub fn is_interactive(&self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        if x >= left + 160.0 && x <= left + 198.0 && y >= top + 35.0 && y <= top + 59.0 {
            return true;
        }
        if y >= top + 96.0 && y <= top + 126.0 {
            if (x >= left + 44.0 && x <= left + 107.0) || (x >= left + 113.0 && x <= left + 189.0) {
                return true;
            }
        }
        false
    }
}
