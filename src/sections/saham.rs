use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;

#[derive(Clone, Debug)]
pub struct StockItem {
    pub symbol: &'static str,
    pub name: &'static str,
    pub price: f64,
    pub currency: &'static str,
    pub change_pct: f64,
    pub change_val: f64,
    pub high: f64,
    pub low: f64,
    pub volume: &'static str,
    pub sparkline: Vec<f32>,
    pub is_market_open: bool,
}

pub struct SahamSection {
    pub stocks: Vec<StockItem>,
    pub selected_index: usize,
    pub tick_timer: f32,
    pub pulse_anim: f32,
}

impl SahamSection {
    pub fn new() -> Self {
        let stocks = vec![
            StockItem {
                symbol: "BBCA",
                name: "Bank Central Asia",
                price: 10250.0,
                currency: "Rp",
                change_pct: 1.48,
                change_val: 150.0,
                high: 10350.0,
                low: 10125.0,
                volume: "84.5M",
                sparkline: vec![
                    0.35, 0.40, 0.38, 0.45, 0.52, 0.50, 0.62, 0.58, 0.65, 0.70, 0.75, 0.72, 0.82,
                    0.88, 0.85, 0.92,
                ],
                is_market_open: true,
            },
            StockItem {
                symbol: "BBRI",
                name: "Bank Rakyat Indonesia",
                price: 5125.0,
                currency: "Rp",
                change_pct: 0.98,
                change_val: 50.0,
                high: 5200.0,
                low: 5075.0,
                volume: "112.3M",
                sparkline: vec![
                    0.45, 0.42, 0.48, 0.50, 0.47, 0.55, 0.60, 0.58, 0.62, 0.66, 0.70, 0.74, 0.71,
                    0.78, 0.80, 0.84,
                ],
                is_market_open: true,
            },
            StockItem {
                symbol: "TLKM",
                name: "Telkom Indonesia",
                price: 3180.0,
                currency: "Rp",
                change_pct: -0.62,
                change_val: -20.0,
                high: 3240.0,
                low: 3160.0,
                volume: "45.1M",
                sparkline: vec![
                    0.85, 0.82, 0.78, 0.75, 0.79, 0.72, 0.68, 0.65, 0.67, 0.60, 0.58, 0.52, 0.48,
                    0.45, 0.42, 0.38,
                ],
                is_market_open: true,
            },
            StockItem {
                symbol: "ASII",
                name: "Astra International",
                price: 5200.0,
                currency: "Rp",
                change_pct: 0.48,
                change_val: 25.0,
                high: 5250.0,
                low: 5150.0,
                volume: "28.7M",
                sparkline: vec![
                    0.50, 0.48, 0.52, 0.55, 0.53, 0.56, 0.54, 0.58, 0.62, 0.60, 0.64, 0.63, 0.65,
                    0.67, 0.68, 0.70,
                ],
                is_market_open: true,
            },
            StockItem {
                symbol: "GOTO",
                name: "GoTo Gojek Tokopedia",
                price: 74.0,
                currency: "Rp",
                change_pct: 2.78,
                change_val: 2.0,
                high: 76.0,
                low: 72.0,
                volume: "1.42B",
                sparkline: vec![
                    0.30, 0.28, 0.35, 0.42, 0.40, 0.55, 0.50, 0.62, 0.70, 0.68, 0.78, 0.82, 0.80,
                    0.88, 0.92, 0.95,
                ],
                is_market_open: true,
            },
            StockItem {
                symbol: "NVDA",
                name: "NVIDIA Corp",
                price: 141.25,
                currency: "$",
                change_pct: 3.12,
                change_val: 4.27,
                high: 142.80,
                low: 138.50,
                volume: "52.8M",
                sparkline: vec![
                    0.25, 0.32, 0.30, 0.45, 0.50, 0.58, 0.54, 0.68, 0.75, 0.72, 0.82, 0.85, 0.83,
                    0.90, 0.94, 0.98,
                ],
                is_market_open: true,
            },
            StockItem {
                symbol: "AAPL",
                name: "Apple Inc.",
                price: 232.80,
                currency: "$",
                change_pct: -0.35,
                change_val: -0.82,
                high: 234.50,
                low: 231.20,
                volume: "38.6M",
                sparkline: vec![
                    0.75, 0.72, 0.78, 0.70, 0.68, 0.65, 0.67, 0.63, 0.60, 0.62, 0.58, 0.55, 0.57,
                    0.54, 0.51, 0.48,
                ],
                is_market_open: true,
            },
            StockItem {
                symbol: "BTC",
                name: "Bitcoin / USD",
                price: 68950.0,
                currency: "$",
                change_pct: 2.40,
                change_val: 1615.0,
                high: 69400.0,
                low: 67200.0,
                volume: "24.1B",
                sparkline: vec![
                    0.40, 0.45, 0.42, 0.52, 0.60, 0.58, 0.68, 0.72, 0.70, 0.80, 0.85, 0.82, 0.89,
                    0.92, 0.90, 0.96,
                ],
                is_market_open: true,
            },
        ];

        Self {
            stocks,
            selected_index: 0,
            tick_timer: 0.0,
            pulse_anim: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.pulse_anim = (self.pulse_anim + dt * 2.5) % 6.28318;
        self.tick_timer += dt;

        // Subtle realistic price wiggle every 3.5 seconds
        if self.tick_timer >= 3.5 {
            self.tick_timer = 0.0;
            if let Some(stock) = self.stocks.get_mut(self.selected_index) {
                let factor = if (self.pulse_anim.sin()) > 0.0 { 1.0006 } else { 0.9995 };
                let old_price = stock.price;
                stock.price = (stock.price * factor * 100.0).round() / 100.0;
                let diff = stock.price - old_price;
                stock.change_val += diff;
                stock.change_pct = ((stock.change_val / (stock.price - stock.change_val)) * 10000.0).round() / 100.0;

                // Adjust latest sparkline point
                if let Some(last) = stock.sparkline.last_mut() {
                    *last = (*last + if diff >= 0.0 { 0.03 } else { -0.03 }).clamp(0.08, 0.98);
                }
            }
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
        let body_w = bounds.right - bounds.left;

        // 1. Header: "📈 Saham" + Market Badge + "IHSG 7.245 ▲ +0.42%"
        let title_rect = D2D_RECT_F {
            left: left + 34.0,
            top: top + 8.0,
            right: left + 110.0,
            bottom: top + 26.0,
        };
        dwrite.draw_text(&d2d.context, "Saham", title_rect, white_brush, &dwrite.format_title);

        // Market active status pill
        let badge_rect = D2D_RECT_F {
            left: left + 88.0,
            top: top + 9.5,
            right: left + 160.0,
            bottom: top + 24.5,
        };
        if let Ok(badge_bg) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 0.06,
            g: 0.35,
            b: 0.20,
            a: 0.85,
        }) {
            d2d.fill_rounded_rect(badge_rect, 7.5, 7.5, &badge_bg);
        }
        // Glowing dot
        let pulse_alpha = 0.6 + 0.4 * self.pulse_anim.sin().abs();
        if let Ok(dot_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 0.20,
            g: 0.85,
            b: 0.45,
            a: pulse_alpha,
        }) {
            d2d.fill_ellipse(badge_rect.left + 8.0, badge_rect.top + 7.5, 3.0, 3.0, &dot_b);
            dwrite.draw_text(
                &d2d.context,
                "BURSA BUKA",
                D2D_RECT_F {
                    left: badge_rect.left + 15.0,
                    top: badge_rect.top,
                    right: badge_rect.right - 4.0,
                    bottom: badge_rect.bottom,
                },
                &dot_b,
                &dwrite.format_small,
            );
        }

        // IHSG Market benchmark summary on right
        let ihsg_rect = D2D_RECT_F {
            left: left + body_w - 170.0,
            top: top + 8.0,
            right: left + body_w - 34.0,
            bottom: top + 26.0,
        };
        dwrite.draw_text(
            &d2d.context,
            "IHSG 7.245 ▲ +0.42%",
            ihsg_rect,
            dim_brush,
            &dwrite.format_small_center,
        );

        // 2. Horizontal Ticker Pills Row
        let pill_count = self.stocks.len();
        let pill_w = 40.0f32;
        let pill_h = 18.0f32;
        let pill_gap = 4.0f32;
        let total_pills_w = pill_count as f32 * pill_w + (pill_count - 1) as f32 * pill_gap;
        let pills_start_x = left + (body_w - total_pills_w) * 0.5;
        let pills_y = top + 29.0;

        for (idx, stock) in self.stocks.iter().enumerate() {
            let px = pills_start_x + idx as f32 * (pill_w + pill_gap);
            let pill_rect = D2D_RECT_F {
                left: px,
                top: pills_y,
                right: px + pill_w,
                bottom: pills_y + pill_h,
            };

            let is_selected = idx == self.selected_index;
            if is_selected {
                let accent_color = if stock.change_pct >= 0.0 {
                    D2D1_COLOR_F { r: 0.10, g: 0.65, b: 0.38, a: 0.95 }
                } else {
                    D2D1_COLOR_F { r: 0.85, g: 0.22, b: 0.26, a: 0.95 }
                };
                if let Ok(b) = d2d.create_solid_brush(accent_color) {
                    d2d.fill_rounded_rect(pill_rect, 6.0, 6.0, &b);
                }
                dwrite.draw_text(
                    &d2d.context,
                    stock.symbol,
                    pill_rect,
                    white_brush,
                    &dwrite.format_card_name,
                );
            } else {
                d2d.fill_rounded_rect(pill_rect, 6.0, 6.0, pill_bg_brush);
                dwrite.draw_text(
                    &d2d.context,
                    stock.symbol,
                    pill_rect,
                    dim_brush,
                    &dwrite.format_card_name,
                );
            }
        }

        // 3. Main Stock Card
        let stock = match self.stocks.get(self.selected_index) {
            Some(s) => s,
            None => return,
        };

        let card_rect = D2D_RECT_F {
            left: left + 34.0,
            top: top + 51.0,
            right: left + body_w - 34.0,
            bottom: top + 125.0,
        };

        if let Ok(card_bg) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 0.10,
            g: 0.10,
            b: 0.12,
            a: 0.65,
        }) {
            d2d.fill_rounded_rect(card_rect, 9.0, 9.0, &card_bg);
        }
        if let Ok(card_border) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.08,
        }) {
            d2d.draw_rounded_rect(card_rect, 9.0, 9.0, &card_border, 1.0);
        }

        // Left Column: Ticker + Name
        let name_rect = D2D_RECT_F {
            left: card_rect.left + 12.0,
            top: card_rect.top + 6.0,
            right: card_rect.left + 195.0,
            bottom: card_rect.top + 22.0,
        };
        let label_text = format!("{}  ·  {}", stock.symbol, stock.name);
        dwrite.draw_text(&d2d.context, &label_text, name_rect, white_brush, &dwrite.format_regular_bold);

        // Big Price Display
        let price_rect = D2D_RECT_F {
            left: card_rect.left + 12.0,
            top: card_rect.top + 23.0,
            right: card_rect.left + 195.0,
            bottom: card_rect.top + 50.0,
        };
        let price_str = if stock.currency == "Rp" {
            let p_int = stock.price as i64;
            format!("Rp {}", format_thousands(p_int))
        } else {
            format!("$ {:.2}", stock.price)
        };
        dwrite.draw_text(&d2d.context, &price_str, price_rect, white_brush, &dwrite.format_big);

        // Bottom line: Change Badge & High/Low
        let is_positive = stock.change_pct >= 0.0;
        let change_color = if is_positive {
            D2D1_COLOR_F { r: 0.20, g: 0.85, b: 0.45, a: 1.0 }
        } else {
            D2D1_COLOR_F { r: 0.95, g: 0.32, b: 0.35, a: 1.0 }
        };

        let badge_bg_color = if is_positive {
            D2D1_COLOR_F { r: 0.08, g: 0.36, b: 0.20, a: 0.8 }
        } else {
            D2D1_COLOR_F { r: 0.42, g: 0.12, b: 0.14, a: 0.8 }
        };

        let change_pill_rect = D2D_RECT_F {
            left: card_rect.left + 12.0,
            top: card_rect.top + 52.0,
            right: card_rect.left + 78.0,
            bottom: card_rect.top + 68.0,
        };
        if let Ok(pill_b) = d2d.create_solid_brush(badge_bg_color) {
            d2d.fill_rounded_rect(change_pill_rect, 5.0, 5.0, &pill_b);
        }

        let change_sign = if is_positive { "+" } else { "" };
        let change_str = format!("{}{:.2}%", change_sign, stock.change_pct);
        if let Ok(change_b) = d2d.create_solid_brush(change_color) {
            dwrite.draw_text(
                &d2d.context,
                &change_str,
                change_pill_rect,
                &change_b,
                &dwrite.format_small_center,
            );
        }

        // High / Low stats
        let stats_rect = D2D_RECT_F {
            left: card_rect.left + 84.0,
            top: card_rect.top + 52.0,
            right: card_rect.left + 200.0,
            bottom: card_rect.top + 68.0,
        };
        let stats_str = format!("H: {:.0}  L: {:.0}  Vol: {}", stock.high, stock.low, stock.volume);
        dwrite.draw_text(&d2d.context, &stats_str, stats_rect, dim_brush, &dwrite.format_small);

        // 4. Right Column: Sparkline Trend Chart
        let chart_x = card_rect.right - 142.0;
        let chart_w = 130.0f32;
        let chart_y = card_rect.top + 8.0;
        let chart_h = card_rect.bottom - card_rect.top - 16.0;

        if stock.sparkline.len() >= 2 {
            let pts_count = stock.sparkline.len();
            let mut chart_points = Vec::with_capacity(pts_count);

            for (i, &val) in stock.sparkline.iter().enumerate() {
                let px = chart_x + (i as f32 / (pts_count - 1) as f32) * chart_w;
                let py = chart_y + (1.0 - val) * chart_h;
                chart_points.push((px, py));
            }

            // Fill area under sparkline
            let mut fill_polygon_pts = Vec::with_capacity(pts_count + 2);
            fill_polygon_pts.push((chart_x, chart_y + chart_h));
            fill_polygon_pts.extend_from_slice(&chart_points);
            fill_polygon_pts.push((chart_x + chart_w, chart_y + chart_h));

            let area_color = if is_positive {
                D2D1_COLOR_F { r: 0.12, g: 0.75, b: 0.40, a: 0.12 }
            } else {
                D2D1_COLOR_F { r: 0.90, g: 0.25, b: 0.28, a: 0.12 }
            };
            if let Ok(area_b) = d2d.create_solid_brush(area_color) {
                d2d.fill_polygon(&fill_polygon_pts, &area_b);
            }

            // Draw line stroke
            if let Ok(stroke_b) = d2d.create_solid_brush(change_color) {
                d2d.draw_polyline(&chart_points, &stroke_b, 2.0, false);
            }

            // Highlight pulsing end dot on latest price point
            if let Some(&(last_x, last_y)) = chart_points.last() {
                if let Ok(white_b) = d2d.create_solid_brush(D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.95,
                }) {
                    d2d.fill_ellipse(last_x, last_y, 2.8, 2.8, &white_b);
                }
                if let Ok(ring_b) = d2d.create_solid_brush(change_color) {
                    d2d.draw_ellipse(last_x, last_y, 4.5, 4.5, &ring_b, 1.2);
                }
            }
        }
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let body_w = bounds.right - bounds.left;

        // Check ticker pills click
        let pill_count = self.stocks.len();
        let pill_w = 40.0f32;
        let pill_h = 18.0f32;
        let pill_gap = 4.0f32;
        let total_pills_w = pill_count as f32 * pill_w + (pill_count - 1) as f32 * pill_gap;
        let pills_start_x = left + (body_w - total_pills_w) * 0.5;
        let pills_y = top + 29.0;

        if y >= pills_y && y <= pills_y + pill_h {
            for idx in 0..pill_count {
                let px = pills_start_x + idx as f32 * (pill_w + pill_gap);
                if x >= px && x <= px + pill_w {
                    self.selected_index = idx;
                    return true;
                }
            }
        }

        // Check main card click -> cycle to next stock
        let card_rect = D2D_RECT_F {
            left: left + 34.0,
            top: top + 51.0,
            right: left + body_w - 34.0,
            bottom: top + 125.0,
        };

        if x >= card_rect.left && x <= card_rect.right && y >= card_rect.top && y <= card_rect.bottom {
            self.selected_index = (self.selected_index + 1) % self.stocks.len();
            return true;
        }

        false
    }

    pub fn is_interactive(&self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let body_w = bounds.right - bounds.left;
        let interactive_rect = D2D_RECT_F {
            left: left + 34.0,
            top: top + 28.0,
            right: left + body_w - 34.0,
            bottom: top + 126.0,
        };
        x >= interactive_rect.left
            && x <= interactive_rect.right
            && y >= interactive_rect.top
            && y <= interactive_rect.bottom
    }
}

fn format_thousands(val: i64) -> String {
    let s = val.abs().to_string();
    let mut result = String::new();
    let mut count = 0;
    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.push('.');
        }
        result.push(c);
        count += 1;
    }
    let res: String = result.chars().rev().collect();
    if val < 0 {
        format!("-{}", res)
    } else {
        res
    }
}
