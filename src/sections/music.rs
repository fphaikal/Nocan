use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;
use crate::integrations::media::{MediaService, MediaTrackInfo};

pub struct MusicSection;

impl MusicSection {
    pub fn draw(
        d2d: &D2DContext,
        dwrite: &DWriteContext,
        bounds: D2D_RECT_F,
        track: &MediaTrackInfo,
        white_brush: &ID2D1SolidColorBrush,
        dim_brush: &ID2D1SolidColorBrush,
        card_brush: &ID2D1SolidColorBrush,
        accent_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;
        let right = bounds.right;
        let _bottom = bounds.bottom;

        // Album Art Box on the left
        let art_size = 56.0;
        let art_rect = D2D_RECT_F {
            left: left + 20.0,
            top: top + 14.0,
            right: left + 20.0 + art_size,
            bottom: top + 14.0 + art_size,
        };
        d2d.fill_rounded_rect(art_rect, 8.0, 8.0, card_brush);

        // Music note icon inside art box if no image
        let art_icon_rect = D2D_RECT_F {
            left: art_rect.left,
            top: art_rect.top + 16.0,
            right: art_rect.right,
            bottom: art_rect.bottom,
        };
        dwrite.draw_text(&d2d.context, "🎵", art_icon_rect, dim_brush, &dwrite.format_center);

        // Track Info
        let text_left = art_rect.right + 14.0;
        let text_right = right - 20.0;

        let title_rect = D2D_RECT_F {
            left: text_left,
            top: top + 12.0,
            right: text_right,
            bottom: top + 28.0,
        };
        let title_text = if track.title.is_empty() {
            "No media playing"
        } else {
            &track.title
        };
        dwrite.draw_text(&d2d.context, title_text, title_rect, white_brush, &dwrite.format_title);

        let artist_rect = D2D_RECT_F {
            left: text_left,
            top: top + 30.0,
            right: text_right,
            bottom: top + 44.0,
        };
        let artist_text = if track.artist.is_empty() {
            "Start playing music on your PC"
        } else {
            &track.artist
        };
        dwrite.draw_text(&d2d.context, artist_text, artist_rect, dim_brush, &dwrite.format_regular);

        // Progress Bar
        let bar_left = text_left;
        let bar_right = text_right;
        let bar_top = top + 52.0;
        let bar_h = 3.5;

        let bg_bar_rect = D2D_RECT_F {
            left: bar_left,
            top: bar_top,
            right: bar_right,
            bottom: bar_top + bar_h,
        };
        d2d.fill_rounded_rect(bg_bar_rect, 1.5, 1.5, card_brush);

        if track.duration_ms > 0 {
            let progress = (track.position_ms as f32 / track.duration_ms as f32).clamp(0.0, 1.0);
            let fill_w = (bar_right - bar_left) * progress;
            let fill_bar_rect = D2D_RECT_F {
                left: bar_left,
                top: bar_top,
                right: bar_left + fill_w,
                bottom: bar_top + bar_h,
            };
            d2d.fill_rounded_rect(fill_bar_rect, 1.5, 1.5, accent_brush);
        }

        // Timestamps
        let pos_str = format_ms(track.position_ms);
        let pos_rect = D2D_RECT_F {
            left: bar_left,
            top: bar_top + 5.0,
            right: bar_left + 40.0,
            bottom: bar_top + 18.0,
        };
        dwrite.draw_text(&d2d.context, &pos_str, pos_rect, dim_brush, &dwrite.format_small);

        let dur_str = if track.duration_ms > track.position_ms {
            format!("-{}", format_ms(track.duration_ms - track.position_ms))
        } else {
            format_ms(track.duration_ms)
        };
        let dur_rect = D2D_RECT_F {
            left: bar_right - 45.0,
            top: bar_top + 5.0,
            right: bar_right,
            bottom: bar_top + 18.0,
        };
        dwrite.draw_text(&d2d.context, &dur_str, dur_rect, dim_brush, &dwrite.format_small);

        // Control Buttons
        let ctrl_center_x = (bar_left + bar_right) / 2.0;
        let ctrl_y = bar_top + 16.0;

        let prev_rect = D2D_RECT_F {
            left: ctrl_center_x - 36.0,
            top: ctrl_y,
            right: ctrl_center_x - 16.0,
            bottom: ctrl_y + 16.0,
        };
        dwrite.draw_text(&d2d.context, "⏮", prev_rect, white_brush, &dwrite.format_center);

        let play_rect = D2D_RECT_F {
            left: ctrl_center_x - 10.0,
            top: ctrl_y - 2.0,
            right: ctrl_center_x + 10.0,
            bottom: ctrl_y + 18.0,
        };
        let play_icon = if track.is_playing { "⏸" } else { "▶" };
        dwrite.draw_text(&d2d.context, play_icon, play_rect, white_brush, &dwrite.format_center);

        let next_rect = D2D_RECT_F {
            left: ctrl_center_x + 16.0,
            top: ctrl_y,
            right: ctrl_center_x + 36.0,
            bottom: ctrl_y + 16.0,
        };
        dwrite.draw_text(&d2d.context, "⏭", next_rect, white_brush, &dwrite.format_center);
    }

    pub fn hit_test_controls(
        bounds: D2D_RECT_F,
        x: f32,
        y: f32,
        media: &MediaService,
    ) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let right = bounds.right;
        let text_left = left + 20.0 + 56.0 + 14.0;
        let text_right = right - 20.0;
        let ctrl_center_x = (text_left + text_right) / 2.0;
        let ctrl_y = top + 52.0 + 16.0;

        if y >= ctrl_y - 4.0 && y <= ctrl_y + 22.0 {
            if x >= ctrl_center_x - 40.0 && x <= ctrl_center_x - 14.0 {
                media.skip_prev();
                return true;
            }
            if x >= ctrl_center_x - 12.0 && x <= ctrl_center_x + 12.0 {
                media.toggle_play_pause();
                return true;
            }
            if x >= ctrl_center_x + 14.0 && x <= ctrl_center_x + 40.0 {
                media.skip_next();
                return true;
            }
        }
        false
    }
}

fn format_ms(ms: u64) -> String {
    let secs = ms / 1000;
    let m = secs / 60;
    let s = secs % 60;
    format!("{:02}:{:02}", m, s)
}
