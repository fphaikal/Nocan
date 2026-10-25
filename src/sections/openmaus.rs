use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
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
        bot_circle_brush: &ID2D1SolidColorBrush,
        green_bg_brush: &ID2D1SolidColorBrush,
        green_text_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;

        // Section Title: "⚡ OpenMausBot"
        let title_rect = D2D_RECT_F {
            left: left + 24.0,
            top: top + 10.0,
            right: left + 180.0,
            bottom: top + 26.0,
        };
        dwrite.draw_text(&d2d.context, "⚡ OpenMausBot", title_rect, white_brush, &dwrite.format_title);

        // Status Badge: "● ACTIVE" on the top right
        let badge_rect = D2D_RECT_F {
            left: left + 312.0,
            top: top + 9.0,
            right: left + 375.0,
            bottom: top + 25.0,
        };
        d2d.fill_rounded_rect(badge_rect, 8.0, 8.0, green_bg_brush);

        let badge_text_rect = D2D_RECT_F {
            left: badge_rect.left,
            top: badge_rect.top + 2.0,
            right: badge_rect.right,
            bottom: badge_rect.bottom,
        };
        dwrite.draw_text(&d2d.context, "● ACTIVE", badge_text_rect, green_text_brush, &dwrite.format_center);

        // Circular Bot Avatar on Left at x=44, y=32, radius=32
        let bot_center_x = left + 44.0 + 34.0;
        let bot_center_y = top + 32.0 + 34.0;
        d2d.fill_ellipse(bot_center_x, bot_center_y, 32.0, 32.0, bot_circle_brush);

        let icon_rect = D2D_RECT_F {
            left: bot_center_x - 16.0,
            top: bot_center_y - 14.0,
            right: bot_center_x + 16.0,
            bottom: bot_center_y + 14.0,
        };
        dwrite.draw_text(&d2d.context, "🤖", icon_rect, white_brush, &dwrite.format_center);

        // Right side data rows (x=120..380)
        let text_left = left + 120.0;
        let text_right = left + 380.0;

        // Row 1: Total Turns & Tokens
        let row1_str = format!(
            "{} Reqs • {} Tokens",
            self.report.total_turns,
            format_tokens(self.report.total_tokens)
        );
        let row1_rect = D2D_RECT_F {
            left: text_left,
            top: top + 28.0,
            right: text_right,
            bottom: top + 46.0,
        };
        dwrite.draw_text(&d2d.context, &row1_str, row1_rect, white_brush, &dwrite.format_title);

        // Row 2: Bots breakdown
        let mut bots_str = String::from("Bots: ");
        if self.report.bots.is_empty() {
            bots_str.push_str("None active");
        } else {
            let bot_parts: Vec<String> = self.report.bots.iter().take(3).map(|b| {
                format!("{} ({})", b.name, format_tokens(b.total_tokens))
            }).collect();
            bots_str.push_str(&bot_parts.join(" • "));
        }
        let row2_rect = D2D_RECT_F {
            left: text_left,
            top: top + 48.0,
            right: text_right,
            bottom: top + 64.0,
        };
        dwrite.draw_text(&d2d.context, &bots_str, row2_rect, dim_brush, &dwrite.format_regular);

        // Row 3: Models
        let top_models: Vec<String> = self.report.models.iter().take(2).map(|m| {
            format!("{} ({})", m.name, format_tokens(m.total_tokens))
        }).collect();
        let models_str = if top_models.is_empty() {
            "Model: Default".to_string()
        } else {
            format!("🧠 {}", top_models.join(" • "))
        };
        let row3_rect = D2D_RECT_F {
            left: text_left,
            top: top + 66.0,
            right: text_right,
            bottom: top + 80.0,
        };
        dwrite.draw_text(&d2d.context, &models_str, row3_rect, dim_brush, &dwrite.format_small);

        // Row 4: Providers
        let top_providers: Vec<String> = self.report.providers.iter().take(2).map(|p| p.name.clone()).collect();
        let prov_str = if top_providers.is_empty() {
            "Provider: Local".to_string()
        } else {
            format!("🔌 {}", top_providers.join(" • "))
        };
        let row4_rect = D2D_RECT_F {
            left: text_left,
            top: top + 82.0,
            right: text_right,
            bottom: top + 96.0,
        };
        dwrite.draw_text(&d2d.context, &prov_str, row4_rect, dim_brush, &dwrite.format_small);
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
