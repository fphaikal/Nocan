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
        card_brush: &ID2D1SolidColorBrush,
        green_bg_brush: &ID2D1SolidColorBrush,
        green_text_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;
        let right = bounds.right;

        // Card Container
        let card_rect = D2D_RECT_F {
            left: left + 16.0,
            top: top + 4.0,
            right: right - 16.0,
            bottom: top + 86.0,
        };
        d2d.fill_rounded_rect(card_rect, 8.0, 8.0, card_brush);

        // Header: "⚡ OpenMausBot"
        let title_rect = D2D_RECT_F {
            left: card_rect.left + 12.0,
            top: card_rect.top + 6.0,
            right: card_rect.left + 160.0,
            bottom: card_rect.top + 22.0,
        };
        dwrite.draw_text(&d2d.context, "⚡ OpenMausBot", title_rect, white_brush, &dwrite.format_title);

        // Status Badge: "● ACTIVE"
        let badge_rect = D2D_RECT_F {
            left: card_rect.right - 70.0,
            top: card_rect.top + 7.0,
            right: card_rect.right - 10.0,
            bottom: card_rect.top + 21.0,
        };
        d2d.fill_rounded_rect(badge_rect, 3.5, 3.5, green_bg_brush);

        let badge_text_rect = D2D_RECT_F {
            left: badge_rect.left,
            top: badge_rect.top + 1.0,
            right: badge_rect.right,
            bottom: badge_rect.bottom,
        };
        dwrite.draw_text(&d2d.context, "● ACTIVE", badge_text_rect, green_text_brush, &dwrite.format_center);

        // Row 1: "📊 324 Total Reqs • 160.9M Total Tokens"
        let row1_rect = D2D_RECT_F {
            left: card_rect.left + 12.0,
            top: card_rect.top + 25.0,
            right: card_rect.right - 12.0,
            bottom: card_rect.top + 39.0,
        };
        let row1_text = format!(
            "📊 {} Total Reqs • {} Total Tokens",
            self.report.total_turns,
            format_tokens(self.report.total_tokens)
        );
        dwrite.draw_text(&d2d.context, &row1_text, row1_rect, white_brush, &dwrite.format_regular);

        // Row 2: Bots breakdown: "🤖 Bots: Maya (50.2M) • Kino (35.8M) • Nova (18.0M)"
        let row2_rect = D2D_RECT_F {
            left: card_rect.left + 12.0,
            top: card_rect.top + 41.0,
            right: card_rect.right - 12.0,
            bottom: card_rect.top + 55.0,
        };
        let mut bots_str = String::from("🤖 Bots: ");
        if self.report.bots.is_empty() {
            bots_str.push_str("None active yet");
        } else {
            let bot_parts: Vec<String> = self.report.bots.iter().take(3).map(|b| {
                format!("{} ({})", b.name, format_tokens(b.total_tokens))
            }).collect();
            bots_str.push_str(&bot_parts.join(" • "));
        }
        dwrite.draw_text(&d2d.context, &bots_str, row2_rect, dim_brush, &dwrite.format_small);

        // Row 3: Models & Providers breakdown
        let row3_rect = D2D_RECT_F {
            left: card_rect.left + 12.0,
            top: card_rect.top + 56.0,
            right: card_rect.right - 12.0,
            bottom: card_rect.top + 70.0,
        };
        let top_model = self.report.models.first().map(|m| format!("🧠 {} ({})", m.name, format_tokens(m.total_tokens))).unwrap_or_else(|| "🧠 No model".to_string());
        let top_prov = self.report.providers.first().map(|p| format!("🔌 {}", p.name)).unwrap_or_else(|| "🔌 None".to_string());
        let row3_text = format!("{} | {}", top_model, top_prov);
        dwrite.draw_text(&d2d.context, &row3_text, row3_rect, dim_brush, &dwrite.format_small);
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
