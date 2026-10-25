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
        bar_bg_brush: &ID2D1SolidColorBrush,
        art_bg_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;

        // Album Art Box on the left: x=44, y=22, w=62, h=62
        let art_rect = D2D_RECT_F {
            left: left + 44.0,
            top: top + 20.0,
            right: left + 44.0 + 62.0,
            bottom: top + 20.0 + 62.0,
        };
        d2d.fill_rounded_rect(art_rect, 10.0, 10.0, art_bg_brush);

        let icon_rect = D2D_RECT_F {
            left: art_rect.left,
            top: art_rect.top + 18.0,
            right: art_rect.right,
            bottom: art_rect.bottom,
        };
        dwrite.draw_text(&d2d.context, "🎵", icon_rect, dim_brush, &dwrite.format_center);

        // Track Info on the right of art box
        let text_left = left + 120.0;
        let text_right = left + 375.0;

        let title_rect = D2D_RECT_F {
            left: text_left,
            top: top + 26.0,
            right: text_right,
            bottom: top + 44.0,
        };
        let title_text = if track.title.is_empty() {
            "No media playing"
        } else {
            &track.title
        };
        dwrite.draw_text(&d2d.context, title_text, title_rect, white_brush, &dwrite.format_title);

        let artist_rect = D2D_RECT_F {
            left: text_left,
            top: top + 46.0,
            right: text_right,
            bottom: top + 62.0,
        };
        let artist_text = if track.artist.is_empty() {
            "Start playing music on Windows"
        } else {
            &track.artist
        };
        dwrite.draw_text(&d2d.context, artist_text, artist_rect, dim_brush, &dwrite.format_regular);

        // Progress Bar: x=92..328, y=82
        let bar_left = left + 92.0;
        let bar_right = left + 328.0;
        let bar_top = top + 80.0;
        let bar_h = 4.0;

        // Elapsed time on left
        let pos_str = format_ms(track.position_ms);
        let pos_rect = D2D_RECT_F {
            left: left + 44.0,
            top: bar_top - 5.0,
            right: bar_left - 8.0,
            bottom: bar_top + 12.0,
        };
        dwrite.draw_text(&d2d.context, &pos_str, pos_rect, dim_brush, &dwrite.format_small);

        // Track background
        let bg_bar_rect = D2D_RECT_F {
            left: bar_left,
            top: bar_top,
            right: bar_right,
            bottom: bar_top + bar_h,
        };
        d2d.fill_rounded_rect(bg_bar_rect, 2.0, 2.0, bar_bg_brush);

        // Filled progress
        if track.duration_ms > 0 {
            let progress = (track.position_ms as f32 / track.duration_ms as f32).clamp(0.0, 1.0);
            let fill_w = (bar_right - bar_left) * progress;
            let fill_bar_rect = D2D_RECT_F {
                left: bar_left,
                top: bar_top,
                right: bar_left + fill_w,
                bottom: bar_top + bar_h,
            };
            d2d.fill_rounded_rect(fill_bar_rect, 2.0, 2.0, white_brush);
        }

        // Remaining time on right
        let dur_str = if track.duration_ms > track.position_ms {
            format!("-{}", format_ms(track.duration_ms - track.position_ms))
        } else {
            format_ms(track.duration_ms)
        };
        let dur_rect = D2D_RECT_F {
            left: bar_right + 8.0,
            top: bar_top - 5.0,
            right: left + 380.0,
            bottom: bar_top + 12.0,
        };
        dwrite.draw_text(&d2d.context, &dur_str, dur_rect, dim_brush, &dwrite.format_small);

        // Control Buttons at y=98..120
        let ctrl_center_x = left + 210.0;
        let ctrl_y = top + 98.0;

        let prev_rect = D2D_RECT_F {
            left: ctrl_center_x - 55.0,
            top: ctrl_y,
            right: ctrl_center_x - 30.0,
            bottom: ctrl_y + 20.0,
        };
        dwrite.draw_text(&d2d.context, "⏮", prev_rect, white_brush, &dwrite.format_center);

        let play_rect = D2D_RECT_F {
            left: ctrl_center_x - 14.0,
            top: ctrl_y - 2.0,
            right: ctrl_center_x + 14.0,
            bottom: ctrl_y + 22.0,
        };
        let play_icon = if track.is_playing { "⏸" } else { "▶" };
        dwrite.draw_text(&d2d.context, play_icon, play_rect, white_brush, &dwrite.format_center);

        let next_rect = D2D_RECT_F {
            left: ctrl_center_x + 30.0,
            top: ctrl_y,
            right: ctrl_center_x + 55.0,
            bottom: ctrl_y + 20.0,
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
        let ctrl_center_x = left + 210.0;
        let ctrl_y = top + 98.0;

        if y >= ctrl_y - 6.0 && y <= ctrl_y + 26.0 {
            if x >= ctrl_center_x - 60.0 && x <= ctrl_center_x - 25.0 {
                media.skip_prev();
                return true;
            }
            if x >= ctrl_center_x - 20.0 && x <= ctrl_center_x + 20.0 {
                media.toggle_play_pause();
                return true;
            }
            if x >= ctrl_center_x + 25.0 && x <= ctrl_center_x + 60.0 {
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
