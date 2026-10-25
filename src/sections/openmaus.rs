use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;
use crate::integrations::openmaus::{load_usage_report, UsageReport};

pub struct OpenMausSection {
    pub report: UsageReport,
    last_fetch_secs: u64,
}

impl OpenMausSection {
    pub fn new() -> Self {
        Self {
            report: load_usage_report(),
            last_fetch_secs: 0,
        }
    }

    pub fn refresh_if_needed(&mut self) {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now_secs.saturating_sub(self.last_fetch_secs) >= 5 {
            self.last_fetch_secs = now_secs;
            self.report = load_usage_report();
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

        // 1. Section Title: "OpenMaus" (#fff) + "Bot" (#7b4bff) at left: 15px, top: 19px (vc)
        let title_w_rect = D2D_RECT_F {
            left: left + 15.0,
            top: top + 9.0,
            right: left + 95.0,
            bottom: top + 29.0,
        };
        dwrite.draw_text(&d2d.context, "OpenMaus", title_w_rect, white_brush, &dwrite.format_title);

        if let Ok(violet_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 123.0 / 255.0,
            g: 75.0 / 255.0,
            b: 1.0,
            a: 1.0,
        }) {
            let title_v_rect = D2D_RECT_F {
                left: left + 90.0,
                top: top + 9.0,
                right: left + 135.0,
                bottom: top + 29.0,
            };
            dwrite.draw_text(&d2d.context, "Bot", title_v_rect, &violet_b, &dwrite.format_title);
        }

        // 2. Left Column: Total Tokens (30px bold) + Turns Subtitle + Active Status Pill
        let tokens_str = format_tokens(self.report.total_tokens);
        let tok_rect = D2D_RECT_F {
            left: left + 44.0,
            top: top + 30.0,
            right: left + 195.0,
            bottom: top + 64.0,
        };
        dwrite.draw_text(&d2d.context, &tokens_str, tok_rect, white_brush, &dwrite.format_big);

        let sub_str = format!("{} Requests · Tokens", self.report.total_turns);
        let sub_rect = D2D_RECT_F {
            left: left + 44.0,
            top: top + 66.0,
            right: left + 195.0,
            bottom: top + 84.0,
        };
        dwrite.draw_text(&d2d.context, &sub_str, sub_rect, dim_brush, &dwrite.format_sub);

        // Status Pill at left: 44px, top: 93px, height: 28px
        let badge_rect = D2D_RECT_F {
            left: left + 44.0,
            top: top + 93.0,
            right: left + 146.0,
            bottom: top + 121.0,
        };
        d2d.fill_rounded_rect(badge_rect, 14.0, 14.0, pill_bg_brush);
        if let Ok(green_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 51.0 / 255.0,
            g: 224.0 / 255.0,
            b: 122.0 / 255.0,
            a: 1.0,
        }) {
            d2d.fill_ellipse(left + 58.0, top + 107.0, 4.0, 4.0, &green_b);
        }
        let badge_txt_rect = D2D_RECT_F {
            left: left + 68.0,
            top: top + 93.0,
            right: left + 142.0,
            bottom: top + 121.0,
        };
        dwrite.draw_text(&d2d.context, "Active Sync", badge_txt_rect, white_brush, &dwrite.format_sub);

        // Vertical Divider at left: 198px
        if let Ok(div_b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.12 }) {
            d2d.fill_rect(
                D2D_RECT_F {
                    left: left + 198.0,
                    top: top + 24.0,
                    right: left + 199.0,
                    bottom: top + 122.0,
                },
                &div_b,
            );
        }

        // 3. Right Column: 3 Info Rows (Bots, Models, Providers) at left: 208px, width: 170px, height: 28px
        let bots_str = if self.report.bots.is_empty() {
            "Bots: None".to_string()
        } else {
            let parts: Vec<String> = self
                .report
                .bots
                .iter()
                .take(2)
                .map(|b| format!("{} ({})", b.name, format_tokens(b.total_tokens)))
                .collect();
            parts.join(" · ")
        };

        let models_str = if self.report.models.is_empty() {
            "Model: Default".to_string()
        } else {
            let parts: Vec<String> = self
                .report
                .models
                .iter()
                .take(2)
                .map(|m| m.name.clone())
                .collect();
            parts.join(" · ")
        };

        let prov_str = if self.report.providers.is_empty() {
            "Provider: Local".to_string()
        } else {
            let parts: Vec<String> = self
                .report
                .providers
                .iter()
                .take(2)
                .map(|p| format!("{} ({})", p.name, p.turns))
                .collect();
            parts.join(" · ")
        };

        let rows = [&bots_str, &models_str, &prov_str];
        let row_tops = [top + 34.0, top + 65.0, top + 96.0];
        let row_colors = [
            D2D1_COLOR_F { r: 25.0 / 255.0, g: 184.0 / 255.0, b: 1.0, a: 1.0 },
            D2D1_COLOR_F { r: 123.0 / 255.0, g: 75.0 / 255.0, b: 1.0, a: 1.0 },
            D2D1_COLOR_F { r: 51.0 / 255.0, g: 224.0 / 255.0, b: 122.0 / 255.0, a: 1.0 },
        ];

        for i in 0..3 {
            let ry = row_tops[i];
            let r_rect = D2D_RECT_F {
                left: left + 208.0,
                top: ry,
                right: left + 378.0,
                bottom: ry + 28.0,
            };
            d2d.fill_rounded_rect(r_rect, 9.0, 9.0, pill_bg_brush);

            if let Ok(dot_b) = d2d.create_solid_brush(row_colors[i]) {
                d2d.fill_ellipse(left + 220.0, ry + 14.0, 4.0, 4.0, &dot_b);
            }

            let txt_rect = D2D_RECT_F {
                left: left + 230.0,
                top: ry,
                right: left + 372.0,
                bottom: ry + 28.0,
            };
            dwrite.draw_text(&d2d.context, rows[i], txt_rect, white_brush, &dwrite.format_sub);
        }
    }
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000_000 {
        format!("{:.1}B", tokens as f64 / 1_000_000_000.0)
    } else if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}
