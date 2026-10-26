use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;
use crate::integrations::ai_usage::{format_tokens, AiTab, AiUsageService, AiUsageSummary};

pub struct AiUsageSection {
    pub service: AiUsageService,
    pub active_tab: AiTab,
    pub summary: AiUsageSummary,
}

impl AiUsageSection {
    pub fn new() -> Self {
        let service = AiUsageService::new();
        let summary = service.get_summary();
        Self {
            service,
            active_tab: AiTab::Antigravity,
            summary,
        }
    }

    pub fn update(&mut self) {
        self.summary = self.service.get_summary();
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;

        // 1. Check 4 Logo Buttons in dedicated Logo Dock (y: top + 27.0 .. top + 53.0)
        if y >= top + 27.0 && y <= top + 53.0 {
            // Button 0 (Antigravity): left + 118.0 .. left + 154.0
            if x >= left + 118.0 && x <= left + 154.0 {
                self.active_tab = AiTab::Antigravity;
                return true;
            }
            // Button 1 (Claude): left + 158.0 .. left + 194.0
            if x >= left + 158.0 && x <= left + 194.0 {
                self.active_tab = AiTab::Claude;
                return true;
            }
            // Button 2 (Codex): left + 198.0 .. left + 234.0
            if x >= left + 198.0 && x <= left + 234.0 {
                self.active_tab = AiTab::Codex;
                return true;
            }
            // Button 3 (OpenCode): left + 238.0 .. left + 274.0
            if x >= left + 238.0 && x <= left + 274.0 {
                self.active_tab = AiTab::OpenCode;
                return true;
            }
        }

        // 2. Body card click -> cycle to next provider tab
        if x >= left + 40.0 && x <= left + 384.0 && y >= top + 54.0 && y <= top + 120.0 {
            self.active_tab = self.active_tab.next();
            return true;
        }

        false
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

        // Current provider and brand palette based on active tab
        let (current_prov, brand_color, brand_glow_color, active_brand_name) = match self.active_tab {
            AiTab::Antigravity => (
                &self.summary.antigravity,
                D2D1_COLOR_F { r: 66.0 / 255.0, g: 133.0 / 255.0, b: 244.0 / 255.0, a: 1.0 },
                D2D1_COLOR_F { r: 66.0 / 255.0, g: 133.0 / 255.0, b: 244.0 / 255.0, a: 0.25 },
                "Antigravity",
            ),
            AiTab::Claude => (
                &self.summary.claude,
                D2D1_COLOR_F { r: 217.0 / 255.0, g: 119.0 / 255.0, b: 87.0 / 255.0, a: 1.0 },
                D2D1_COLOR_F { r: 217.0 / 255.0, g: 119.0 / 255.0, b: 87.0 / 255.0, a: 0.25 },
                "Claude",
            ),
            AiTab::Codex => (
                &self.summary.codex,
                D2D1_COLOR_F { r: 16.0 / 255.0, g: 163.0 / 255.0, b: 127.0 / 255.0, a: 1.0 },
                D2D1_COLOR_F { r: 16.0 / 255.0, g: 163.0 / 255.0, b: 127.0 / 255.0, a: 0.25 },
                "Codex",
            ),
            AiTab::OpenCode => (
                &self.summary.opencode,
                D2D1_COLOR_F { r: 6.0 / 255.0, g: 182.0 / 255.0, b: 212.0 / 255.0, a: 1.0 },
                D2D1_COLOR_F { r: 6.0 / 255.0, g: 182.0 / 255.0, b: 212.0 / 255.0, a: 0.25 },
                "OpenCode",
            ),
        };

        // 1. Header Title: "AI Usage" at left: 15px, top: 8px (vc with global date/clock)
        let title_rect = D2D_RECT_F {
            left: left + 15.0,
            top: top + 8.0,
            right: left + 115.0,
            bottom: top + 28.0,
        };
        dwrite.draw_text(&d2d.context, "AI Usage", title_rect, white_brush, &dwrite.format_title);

        // 2. Logo-Only Selector Dock (top + 28.0 .. top + 52.0)
        // Positioned below top bar so date/clock time (y: 8..28) is never obstructed!
        let dock_bg_rect = D2D_RECT_F {
            left: left + 114.0,
            top: top + 28.0,
            right: left + 278.0,
            bottom: top + 52.0,
        };
        d2d.fill_rounded_rect(dock_bg_rect, 12.0, 12.0, pill_bg_brush);

        // 4 Logo Buttons: (Tab, Icon, left, right, brand_color)
        let logo_tabs = [
            (AiTab::Antigravity, "✦", left + 118.0, left + 154.0, D2D1_COLOR_F { r: 66.0 / 255.0, g: 133.0 / 255.0, b: 244.0 / 255.0, a: 1.0 }),
            (AiTab::Claude, "✳", left + 158.0, left + 194.0, D2D1_COLOR_F { r: 217.0 / 255.0, g: 119.0 / 255.0, b: 87.0 / 255.0, a: 1.0 }),
            (AiTab::Codex, "⬡", left + 198.0, left + 234.0, D2D1_COLOR_F { r: 16.0 / 255.0, g: 163.0 / 255.0, b: 127.0 / 255.0, a: 1.0 }),
            (AiTab::OpenCode, ">_", left + 238.0, left + 274.0, D2D1_COLOR_F { r: 6.0 / 255.0, g: 182.0 / 255.0, b: 212.0 / 255.0, a: 1.0 }),
        ];

        let btn_top = top + 30.0;
        let btn_bottom = top + 50.0;

        for (tab, icon, btn_l, btn_r, b_col) in logo_tabs {
            let is_active = tab == self.active_tab;
            let btn_rect = D2D_RECT_F {
                left: btn_l,
                top: btn_top,
                right: btn_r,
                bottom: btn_bottom,
            };

            if is_active {
                // Active logo button: vibrant brand glow background + crisp border
                if let Ok(glow_b) = d2d.create_solid_brush(brand_glow_color) {
                    d2d.fill_rounded_rect(btn_rect, 8.0, 8.0, &glow_b);
                }
                if let Ok(brand_b) = d2d.create_solid_brush(b_col) {
                    d2d.draw_rounded_rect(btn_rect, 8.0, 8.0, &brand_b, 1.2);
                }
                dwrite.draw_text(&d2d.context, icon, btn_rect, white_brush, &dwrite.format_button_center);
            } else {
                // Inactive logo button: clean translucent icon
                dwrite.draw_text(&d2d.context, icon, btn_rect, dim_brush, &dwrite.format_button_center);
            }
        }

        // Active Provider Label badge next to Logo Dock (right-aligned at left + 284..382)
        let prov_label_rect = D2D_RECT_F {
            left: left + 284.0,
            top: top + 29.0,
            right: left + 382.0,
            bottom: top + 51.0,
        };
        dwrite.draw_text(&d2d.context, active_brand_name, prov_label_rect, white_brush, &dwrite.format_regular_semibold_right);

        // 3. Left Column: Primary Metrics
        // Shifted to start at left + 46.0 to give 13px clearance from left navigation arrow (at left + 22.0)!
        let tokens_str = if current_prov.today_tokens > 0 {
            format_tokens(current_prov.today_tokens)
        } else if current_prov.total_tokens > 0 {
            format_tokens(current_prov.total_tokens)
        } else {
            "0".to_string()
        };

        let tok_rect = D2D_RECT_F {
            left: left + 46.0,
            top: top + 56.0,
            right: left + 190.0,
            bottom: top + 83.0,
        };
        dwrite.draw_text(&d2d.context, &tokens_str, tok_rect, white_brush, &dwrite.format_big);

        let sub_str = if current_prov.today_tokens > 0 {
            format!("{} Requests · Today", current_prov.today_turns)
        } else if current_prov.total_tokens > 0 {
            format!("{} Requests · All-Time", current_prov.total_turns)
        } else {
            "Ready · Zero Today".to_string()
        };
        let sub_rect = D2D_RECT_F {
            left: left + 47.0,
            top: top + 83.0,
            right: left + 190.0,
            bottom: top + 98.0,
        };
        dwrite.draw_text(&d2d.context, &sub_str, sub_rect, dim_brush, &dwrite.format_small);

        // Status Pill at left: 46px, top: 100px, height: 18px
        let badge_rect = D2D_RECT_F {
            left: left + 46.0,
            top: top + 100.0,
            right: left + 175.0,
            bottom: top + 118.0,
        };
        d2d.fill_rounded_rect(badge_rect, 9.0, 9.0, pill_bg_brush);

        // Indicator dot color
        let dot_color = if current_prov.is_active {
            D2D1_COLOR_F { r: 52.0 / 255.0, g: 211.0 / 255.0, b: 153.0 / 255.0, a: 1.0 }
        } else if current_prov.today_tokens > 0 {
            D2D1_COLOR_F { r: 96.0 / 255.0, g: 165.0 / 255.0, b: 250.0 / 255.0, a: 1.0 }
        } else {
            D2D1_COLOR_F { r: 148.0 / 255.0, g: 163.0 / 255.0, b: 184.0 / 255.0, a: 0.8 }
        };
        if let Ok(dot_b) = d2d.create_solid_brush(dot_color) {
            d2d.fill_ellipse(left + 56.0, top + 109.0, 3.0, 3.0, &dot_b);
        }

        let badge_txt_rect = D2D_RECT_F {
            left: left + 65.0,
            top: top + 100.0,
            right: left + 170.0,
            bottom: top + 118.0,
        };
        dwrite.draw_text(&d2d.context, &current_prov.status_text, badge_txt_rect, white_brush, &dwrite.format_small);

        // Vertical Divider at left: 194px
        if let Ok(div_b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.10 }) {
            d2d.fill_rect(
                D2D_RECT_F {
                    left: left + 194.0,
                    top: top + 57.0,
                    right: left + 195.0,
                    bottom: top + 118.0,
                },
                &div_b,
            );
        }

        // 4. Right Column: Detailed Breakdown (3 Cards at top + 57, 78, 99)
        let row1_txt = format!("Model: {}", shorten_model(&current_prov.active_model));
        let row2_txt = match self.active_tab {
            AiTab::Antigravity => {
                if current_prov.input_tokens_today > 0 {
                    format!("In: {} · Out: {}", format_tokens(current_prov.input_tokens_today), format_tokens(current_prov.output_tokens_today))
                } else {
                    format!("Cache: {} read", format_tokens(current_prov.cached_tokens_today))
                }
            }
            AiTab::Codex => {
                if let Some(qp) = current_prov.quota_percent {
                    format!("Quota: {}% used · Monthly", qp)
                } else {
                    format!("In: {} · Turns: {}", format_tokens(current_prov.input_tokens_today), current_prov.today_turns)
                }
            }
            AiTab::Claude => {
                if current_prov.today_tokens > 0 {
                    format!("Today: {} tokens", format_tokens(current_prov.today_tokens))
                } else {
                    "Local Bridge · Loopback".to_string()
                }
            }
            AiTab::OpenCode => {
                if current_prov.today_tokens > 0 {
                    format!("Today: {} tokens", format_tokens(current_prov.today_tokens))
                } else {
                    format!("In: {} · Turns: {}", format_tokens(current_prov.input_tokens_today), current_prov.today_turns)
                }
            }
        };
        let row3_txt = format!("Lifetime: {} tokens", format_tokens(current_prov.total_tokens));

        let rows = [&row1_txt, &row2_txt, &row3_txt];
        let row_tops = [top + 57.0, top + 78.0, top + 99.0];

        for i in 0..3 {
            let ry = row_tops[i];
            let r_rect = D2D_RECT_F {
                left: left + 202.0,
                top: ry,
                right: left + 382.0,
                bottom: ry + 18.0,
            };
            d2d.fill_rounded_rect(r_rect, 6.0, 6.0, pill_bg_brush);

            // Colored indicator dot
            let c = if i == 0 {
                brand_color
            } else if i == 1 {
                D2D1_COLOR_F { r: 96.0 / 255.0, g: 165.0 / 255.0, b: 250.0 / 255.0, a: 1.0 }
            } else {
                D2D1_COLOR_F { r: 168.0 / 255.0, g: 85.0 / 255.0, b: 247.0 / 255.0, a: 1.0 }
            };
            if let Ok(cb) = d2d.create_solid_brush(c) {
                d2d.fill_ellipse(left + 211.0, ry + 9.0, 2.5, 2.5, &cb);
            }

            let txt_rect = D2D_RECT_F {
                left: left + 219.0,
                top: ry,
                right: left + 378.0,
                bottom: ry + 18.0,
            };
            dwrite.draw_text(&d2d.context, rows[i], txt_rect, white_brush, &dwrite.format_small);
        }

        // 5. Bottom Proportional Usage Segment Bar (top + 122.0)
        let bar_left = left + 18.0;
        let bar_top = top + 122.0;
        let bar_w = 364.0;
        let bar_h = 3.0;

        // Background track
        let track_rect = D2D_RECT_F {
            left: bar_left,
            top: bar_top,
            right: bar_left + bar_w,
            bottom: bar_top + bar_h,
        };
        d2d.fill_rounded_rect(track_rect, 1.5, 1.5, pill_bg_brush);

        // Calculate distribution shares across 4 providers
        let ag_val = self.summary.antigravity.today_tokens.max(self.summary.antigravity.total_tokens / 100);
        let cl_val = self.summary.claude.today_tokens.max(self.summary.claude.total_tokens / 100);
        let cx_val = self.summary.codex.today_tokens.max(self.summary.codex.total_tokens / 100);
        let oc_val = self.summary.opencode.today_tokens.max(self.summary.opencode.total_tokens / 100);
        let sum_val = (ag_val + cl_val + cx_val + oc_val).max(1);

        let ag_w = (ag_val as f32 / sum_val as f32) * bar_w;
        let cl_w = (cl_val as f32 / sum_val as f32) * bar_w;
        let cx_w = (cx_val as f32 / sum_val as f32) * bar_w;
        let oc_w = (bar_w - ag_w - cl_w - cx_w).max(0.0);

        let mut seg_x = bar_left;

        // Segment 1: Antigravity (Blue)
        if ag_w > 1.0 {
            if let Ok(b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 66.0 / 255.0, g: 133.0 / 255.0, b: 244.0 / 255.0, a: 0.9 }) {
                d2d.fill_rect(
                    D2D_RECT_F {
                        left: seg_x,
                        top: bar_top,
                        right: seg_x + ag_w,
                        bottom: bar_top + bar_h,
                    },
                    &b,
                );
            }
            seg_x += ag_w;
        }

        // Segment 2: Claude (Coral)
        if cl_w > 1.0 {
            if let Ok(b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 217.0 / 255.0, g: 119.0 / 255.0, b: 87.0 / 255.0, a: 0.9 }) {
                d2d.fill_rect(
                    D2D_RECT_F {
                        left: seg_x,
                        top: bar_top,
                        right: seg_x + cl_w,
                        bottom: bar_top + bar_h,
                    },
                    &b,
                );
            }
            seg_x += cl_w;
        }

        // Segment 3: Codex (Green)
        if cx_w > 1.0 {
            if let Ok(b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 16.0 / 255.0, g: 163.0 / 255.0, b: 127.0 / 255.0, a: 0.9 }) {
                d2d.fill_rect(
                    D2D_RECT_F {
                        left: seg_x,
                        top: bar_top,
                        right: seg_x + cx_w,
                        bottom: bar_top + bar_h,
                    },
                    &b,
                );
            }
            seg_x += cx_w;
        }

        // Segment 4: OpenCode (Cyan)
        if oc_w > 1.0 {
            if let Ok(b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 6.0 / 255.0, g: 182.0 / 255.0, b: 212.0 / 255.0, a: 0.9 }) {
                d2d.fill_rect(
                    D2D_RECT_F {
                        left: seg_x,
                        top: bar_top,
                        right: bar_left + bar_w,
                        bottom: bar_top + bar_h,
                    },
                    &b,
                );
            }
        }
    }
}

fn shorten_model(m: &str) -> String {
    let s = m.trim();
    if s.is_empty() {
        return "Default".to_string();
    }
    if let Some(pos) = s.rfind('/') {
        return s[pos + 1..].to_string();
    }
    if let Some(pos) = s.rfind("--") {
        return s[pos + 2..].to_string();
    }
    s.to_string()
}
