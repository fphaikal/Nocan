use std::sync::{Arc, Mutex};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;
use crate::integrations::media::{MediaService, MediaTrackInfo};

#[derive(Clone, Debug)]
pub struct LyricLine {
    pub time_ms: u64,
    pub text: String,
}

pub struct MusicSection {
    pub show_lyrics: bool,
    last_fetched_key: String,
    lyrics: Arc<Mutex<Vec<LyricLine>>>,
    lyrics_status: Arc<Mutex<String>>,
}

impl MusicSection {
    pub fn new() -> Self {
        let default_lyrics = vec![
            LyricLine { time_ms: 0, text: "we fold the summer light".to_string() },
            LyricLine { time_ms: 4000, text: "into paper birds tonight".to_string() },
            LyricLine { time_ms: 8000, text: "and send them through the sky".to_string() },
            LyricLine { time_ms: 12000, text: "to find you by the sea".to_string() },
            LyricLine { time_ms: 16000, text: "every wing a little song".to_string() },
            LyricLine { time_ms: 20000, text: "that carries us along".to_string() },
        ];
        Self {
            show_lyrics: false,
            last_fetched_key: String::new(),
            lyrics: Arc::new(Mutex::new(default_lyrics)),
            lyrics_status: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn update_lyrics(&mut self, track: &MediaTrackInfo) {
        if !self.show_lyrics || track.title.is_empty() {
            return;
        }
        let key = format!("{}|{}", track.title, track.artist);
        if key == self.last_fetched_key {
            return;
        }
        self.last_fetched_key = key;

        let title = track.title.clone();
        let artist = track.artist.clone();
        let lyrics_arc = Arc::clone(&self.lyrics);
        let status_arc = Arc::clone(&self.lyrics_status);

        if let Ok(mut st) = status_arc.lock() {
            *st = "Loading lyrics".to_string();
        }

        std::thread::spawn(move || {
            let url = format!(
                "https://lrclib.net/api/search?track_name={}&artist_name={}",
                url_encode(&title),
                url_encode(&artist)
            );
            let mut parsed_lines = Vec::new();
            if let Ok(resp) = ureq::get(&url).call() {
                if let Ok(json) = resp.into_json::<serde_json::Value>() {
                    if let Some(arr) = json.as_array() {
                        for item in arr {
                            if let Some(synced) = item.get("syncedLyrics").and_then(|v| v.as_str()) {
                                parsed_lines = parse_lrc(synced);
                                if !parsed_lines.is_empty() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if parsed_lines.is_empty() {
                if let Ok(mut st) = status_arc.lock() {
                    *st = "Lyrics not available".to_string();
                }
            } else {
                if let Ok(mut l) = lyrics_arc.lock() {
                    *l = parsed_lines;
                }
                if let Ok(mut st) = status_arc.lock() {
                    st.clear();
                }
            }
        });
    }

    pub fn draw(
        &self,
        d2d: &D2DContext,
        dwrite: &DWriteContext,
        bounds: D2D_RECT_F,
        track: &MediaTrackInfo,
        white_brush: &ID2D1SolidColorBrush,
        dim_brush: &ID2D1SolidColorBrush,
        bar_bg_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;

        // If in lyrics mode, draw bottom ambient blue glow & right lyrics pane
        if self.show_lyrics {
            let body_w = bounds.right - bounds.left;
            let body_h = bounds.bottom - bounds.top;
            d2d.draw_ambient_glow(
                left,
                body_w,
                body_h,
                32.0,
                0.82,
                0.85,
                0.0,
                &[
                    (0.0, D2D1_COLOR_F { r: 44.0 / 255.0, g: 100.0 / 255.0, b: 184.0 / 255.0, a: 1.0 }),
                    (0.5, D2D1_COLOR_F { r: 58.0 / 255.0, g: 122.0 / 255.0, b: 208.0 / 255.0, a: 1.0 }),
                    (1.0, D2D1_COLOR_F { r: 42.0 / 255.0, g: 90.0 / 255.0, b: 168.0 / 255.0, a: 1.0 }),
                ],
            );
        }

        // Album Art Box: in normal mode x=44 (so left chevron at x=22 has space), in lyrics mode x=22
        let art_x = if self.show_lyrics { left + 22.0 } else { left + 44.0 };
        let art_y = top + 12.0;
        let art_w = 62.0;
        let art_h = 62.0;
        Self::draw_album_art(d2d, art_x, art_y, art_w, art_h, track.thumbnail_bgra.as_deref());

        // Track Title & Artist
        let text_left = if self.show_lyrics { left + 100.0 } else { left + 122.0 };
        let text_right = if self.show_lyrics { left + 244.0 } else { left + 376.0 };

        let title_text = if track.title.is_empty() {
            "Paper Sky"
        } else {
            &track.title
        };
        let title_rect = D2D_RECT_F {
            left: text_left,
            top: top + 36.0,
            right: text_right,
            bottom: top + 56.0,
        };
        dwrite.draw_text(&d2d.context, title_text, title_rect, white_brush, &dwrite.format_title);

        let artist_text = if track.artist.is_empty() {
            "Lumen Coast - Summer Flows"
        } else {
            &track.artist
        };
        let artist_rect = D2D_RECT_F {
            left: text_left,
            top: top + 55.0,
            right: text_right,
            bottom: top + 72.0,
        };
        dwrite.draw_text(&d2d.context, artist_text, artist_rect, dim_brush, &dwrite.format_sub);

        // Progress Bar Row at y = 91
        let bar_y = top + 91.0;
        let bar_h = 4.2;

        let (pos_left, bar_left, bar_right, dur_right) = if self.show_lyrics {
            (left + 25.0, left + 76.0, left + 184.0, left + 238.0)
        } else {
            (left + 44.0, left + 96.0, left + 318.0, left + 376.0)
        };

        let (pos_ms, dur_ms) = if track.title.is_empty() && track.duration_ms == 0 {
            (193_000, 256_000) // 03:13 / -01:03 preview default
        } else {
            (track.position_ms, track.duration_ms)
        };

        let pos_str = format_ms(pos_ms);
        let pos_rect = D2D_RECT_F {
            left: pos_left,
            top: bar_y - 10.0,
            right: bar_left - 6.0,
            bottom: bar_y + 10.0,
        };
        dwrite.draw_text(&d2d.context, &pos_str, pos_rect, white_brush, &dwrite.format_regular);

        let bg_bar_rect = D2D_RECT_F {
            left: bar_left,
            top: bar_y - bar_h * 0.5,
            right: bar_right,
            bottom: bar_y + bar_h * 0.5,
        };
        d2d.fill_rounded_rect(bg_bar_rect, 2.1, 2.1, bar_bg_brush);

        let progress = if dur_ms > 0 {
            (pos_ms as f32 / dur_ms as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if progress > 0.0 {
            let fill_w = (bar_right - bar_left) * progress;
            let fill_bar_rect = D2D_RECT_F {
                left: bar_left,
                top: bar_y - bar_h * 0.5,
                right: bar_left + fill_w.max(4.0),
                bottom: bar_y + bar_h * 0.5,
            };
            d2d.fill_rounded_rect(fill_bar_rect, 2.1, 2.1, white_brush);
        }

        let rem_str = if dur_ms > pos_ms {
            format!("-{}", format_ms(dur_ms - pos_ms))
        } else {
            format_ms(dur_ms)
        };
        let dur_rect = D2D_RECT_F {
            left: bar_right + 8.0,
            top: bar_y - 10.0,
            right: dur_right,
            bottom: bar_y + 10.0,
        };
        dwrite.draw_text(&d2d.context, &rem_str, dur_rect, white_brush, &dwrite.format_regular_right);

        // Controls Row at y = 116
        let ctrl_y = top + 116.0;
        let (bub_x, prev_x, play_x, next_x) = if self.show_lyrics {
            (left + 53.0, left + 91.0, left + 131.0, left + 171.0)
        } else {
            (left + 75.0, left + 162.5, left + 210.0, left + 257.5)
        };

        // 1. Speech-bubble Lyrics Icon (.bub)
        Self::draw_lyrics_bubble(d2d, bub_x, ctrl_y, self.show_lyrics, white_brush, dim_brush);

        // 2. Prev Icon (double left triangle)
        Self::draw_prev_icon(d2d, prev_x, ctrl_y, white_brush);

        // 3. Play/Pause Icon
        let is_playing = if track.title.is_empty() { true } else { track.is_playing };
        Self::draw_play_pause_icon(d2d, play_x, ctrl_y, is_playing, white_brush);

        // 4. Next Icon (double right triangle)
        Self::draw_next_icon(d2d, next_x, ctrl_y, white_brush);

        // 5. Right Side Lyrics Pane when in Lyrics mode
        if self.show_lyrics {
            if let Ok(div_b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.12 }) {
                d2d.fill_rect(
                    D2D_RECT_F {
                        left: left + 252.0,
                        top: top + 22.0,
                        right: left + 253.0,
                        bottom: top + 124.0,
                    },
                    &div_b,
                );
            }

            let status = self.lyrics_status.lock().map(|s| s.clone()).unwrap_or_default();
            let pane_right = bounds.right - 18.0;
            if !status.is_empty() && !track.title.is_empty() {
                let st_rect = D2D_RECT_F {
                    left: left + 264.0,
                    top: top + 54.0,
                    right: pane_right,
                    bottom: top + 84.0,
                };
                dwrite.draw_text(&d2d.context, &status, st_rect, dim_brush, &dwrite.format_lyric);
            } else if let Ok(lines) = self.lyrics.lock() {
                let active_idx = Self::active_index(&lines, track, pos_ms);

                let dim_lyric_b = d2d
                    .create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.30 })
                    .ok();
                let dim_b = dim_lyric_b.as_ref().unwrap_or(dim_brush);

                // Render 4 visible lines around active_idx
                let start_i = active_idx.saturating_sub(1);
                for (row, idx) in (start_i..(start_i + 4).min(lines.len())).enumerate() {
                    let y0 = top + 22.0 + (row as f32) * 26.0;
                    let rect = D2D_RECT_F {
                        left: left + 264.0,
                        top: y0,
                        right: pane_right,
                        bottom: y0 + 24.0,
                    };
                    let b = if idx == active_idx { white_brush } else { dim_b };
                    dwrite.draw_text(&d2d.context, &lines[idx].text, rect, b, &dwrite.format_lyric);
                }
            }
        }
    }

    fn active_index(lines: &[LyricLine], track: &MediaTrackInfo, pos_ms: u64) -> usize {
        if track.title.is_empty() {
            ((std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                / 2600) as usize)
                % lines.len().max(1)
        } else {
            lines.iter().rposition(|l| pos_ms >= l.time_ms).unwrap_or(0)
        }
    }

    /// Width in px needed by the currently active lyric line (or status text), used to size the notch dynamically.
    pub fn lyrics_pane_width(&self, dwrite: &DWriteContext, track: &MediaTrackInfo) -> f32 {
        let status = self.lyrics_status.lock().map(|s| s.clone()).unwrap_or_default();
        if !status.is_empty() && !track.title.is_empty() {
            return dwrite.measure_text_width(&status, &dwrite.format_lyric);
        }
        let pos_ms = if track.title.is_empty() && track.duration_ms == 0 {
            193_000
        } else {
            track.position_ms
        };
        if let Ok(lines) = self.lyrics.lock() {
            if lines.is_empty() {
                return 0.0;
            }
            let idx = Self::active_index(&lines, track, pos_ms);
            return dwrite.measure_text_width(&lines[idx].text, &dwrite.format_lyric);
        }
        0.0
    }

    fn draw_album_art(
        d2d: &D2DContext,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        thumb_bgra: Option<&Vec<u8>>,
    ) {
        let rect = D2D_RECT_F {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        };
        if let Some(bytes) = thumb_bgra {
            if d2d.fill_rounded_rect_bitmap(rect, 10.0, 10.0, bytes, 62, 62) {
                return;
            }
        }
        if let Ok(bg) = d2d.create_linear_gradient(
            (x, y),
            (x + w, y + h),
            &[
                (0.0, D2D1_COLOR_F { r: 238.0 / 255.0, g: 245.0 / 255.0, b: 255.0 / 255.0, a: 1.0 }),
                (1.0, D2D1_COLOR_F { r: 207.0 / 255.0, g: 226.0 / 255.0, b: 251.0 / 255.0, a: 1.0 }),
            ],
        ) {
            d2d.fill_rounded_rect_linear(rect, 10.0, 10.0, &bg);
        }

        // Subtle horizontal scanlines (repeating-linear-gradient)
        if let Ok(line_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 46.0 / 255.0,
            g: 127.0 / 255.0,
            b: 224.0 / 255.0,
            a: 0.16,
        }) {
            let mut ly = y + 4.0;
            while ly < y + h - 4.0 {
                d2d.fill_rect(
                    D2D_RECT_F {
                        left: x + 2.0,
                        top: ly,
                        right: x + w - 2.0,
                        bottom: ly + 1.8,
                    },
                    &line_b,
                );
                ly += 7.0;
            }
        }

        // Blue circle + outer ring at (34%, 58%)
        let cx = x + w * 0.34;
        let cy = y + h * 0.58;
        if let Ok(ring_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 90.0 / 255.0,
            g: 166.0 / 255.0,
            b: 242.0 / 255.0,
            a: 1.0,
        }) {
            d2d.draw_ellipse(cx, cy, w * 0.31, h * 0.31, &ring_b, 1.8);
        }
        if let Ok(disc_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 46.0 / 255.0,
            g: 127.0 / 255.0,
            b: 224.0 / 255.0,
            a: 1.0,
        }) {
            d2d.fill_ellipse(cx, cy, w * 0.22, h * 0.22, &disc_b);
        }

        // Diagonal white highlight streak
        if let Ok(streak_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.72,
        }) {
            d2d.draw_line(x + w * 0.62, y + 2.0, x + w * 0.30, y + h - 2.0, &streak_b, 2.0, false);
        }
    }

    fn draw_lyrics_bubble(
        d2d: &D2DContext,
        cx: f32,
        cy: f32,
        active: bool,
        white_brush: &ID2D1SolidColorBrush,
        dim_brush: &ID2D1SolidColorBrush,
    ) {
        let r = D2D_RECT_F {
            left: cx - 8.2,
            top: cy - 6.8,
            right: cx + 8.2,
            bottom: cy + 4.4,
        };
        if active {
            d2d.fill_rounded_rect(r, 3.2, 3.2, white_brush);
            let tail = [(cx - 4.0, cy + 4.0), (cx - 4.8, cy + 7.2), (cx - 1.0, cy + 4.0)];
            d2d.fill_polygon(&tail, white_brush);
            if let Ok(black_b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) {
                d2d.fill_ellipse(cx - 2.5, cy - 1.4, 1.3, 1.3, &black_b);
                d2d.fill_ellipse(cx + 2.5, cy - 1.4, 1.3, 1.3, &black_b);
            }
        } else {
            d2d.draw_rounded_rect(r, 3.2, 3.2, dim_brush, 1.3);
            let tail = [(cx - 4.0, cy + 4.4), (cx - 4.8, cy + 7.2), (cx - 1.2, cy + 4.4)];
            d2d.draw_polyline(&tail, dim_brush, 1.3, false);
            d2d.fill_ellipse(cx - 2.5, cy - 1.4, 1.3, 1.3, dim_brush);
            d2d.fill_ellipse(cx + 2.5, cy - 1.4, 1.3, 1.3, dim_brush);
        }
    }

    fn draw_prev_icon(d2d: &D2DContext, cx: f32, cy: f32, brush: &ID2D1SolidColorBrush) {
        // viewBox 0 0 24 24 (21x21px): M11 6 3 12l8 6z M21 6l-8 6 8 6z
        let s = 21.0 / 24.0;
        let tx = |x: f32| cx + (x - 12.0) * s;
        let ty = |y: f32| cy + (y - 12.0) * s;
        let t1 = [(tx(11.0), ty(6.0)), (tx(3.0), ty(12.0)), (tx(11.0), ty(18.0))];
        let t2 = [(tx(21.0), ty(6.0)), (tx(13.0), ty(12.0)), (tx(21.0), ty(18.0))];
        d2d.fill_polygon(&t1, brush);
        d2d.fill_polygon(&t2, brush);
    }

    fn draw_next_icon(d2d: &D2DContext, cx: f32, cy: f32, brush: &ID2D1SolidColorBrush) {
        // viewBox 0 0 24 24 (21x21px): M13 6l8 6-8 6z M3 6l8 6-8 6z
        let s = 21.0 / 24.0;
        let tx = |x: f32| cx + (x - 12.0) * s;
        let ty = |y: f32| cy + (y - 12.0) * s;
        let t1 = [(tx(13.0), ty(6.0)), (tx(21.0), ty(12.0)), (tx(13.0), ty(18.0))];
        let t2 = [(tx(3.0), ty(6.0)), (tx(11.0), ty(12.0)), (tx(3.0), ty(18.0))];
        d2d.fill_polygon(&t1, brush);
        d2d.fill_polygon(&t2, brush);
    }

    fn draw_play_pause_icon(
        d2d: &D2DContext,
        cx: f32,
        cy: f32,
        is_playing: bool,
        brush: &ID2D1SolidColorBrush,
    ) {
        let s = 21.0 / 24.0;
        let tx = |x: f32| cx + (x - 12.0) * s;
        let ty = |y: f32| cy + (y - 12.0) * s;
        if is_playing {
            // M7 5h4v14H7z M13 5h4v14h-4z
            let b1 = D2D_RECT_F {
                left: tx(6.8),
                top: ty(4.8),
                right: tx(10.6),
                bottom: ty(19.2),
            };
            let b2 = D2D_RECT_F {
                left: tx(13.4),
                top: ty(4.8),
                right: tx(17.2),
                bottom: ty(19.2),
            };
            d2d.fill_rounded_rect(b1, 1.0, 1.0, brush);
            d2d.fill_rounded_rect(b2, 1.0, 1.0, brush);
        } else {
            let tri = [(tx(7.0), ty(4.5)), (tx(19.0), ty(12.0)), (tx(7.0), ty(19.5))];
            d2d.fill_polygon(&tri, brush);
        }
    }

    pub fn hit_test_controls(
        &mut self,
        bounds: D2D_RECT_F,
        x: f32,
        y: f32,
        media: &MediaService,
    ) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let ctrl_y = top + 116.0;
        let (bub_x, prev_x, play_x, next_x) = if self.show_lyrics {
            (left + 53.0, left + 91.0, left + 131.0, left + 171.0)
        } else {
            (left + 75.0, left + 162.5, left + 210.0, left + 257.5)
        };

        if (y - ctrl_y).abs() <= 14.0 {
            if (x - bub_x).abs() <= 14.0 {
                self.show_lyrics = !self.show_lyrics;
                return true;
            }
            if (x - prev_x).abs() <= 15.0 {
                media.skip_prev();
                return true;
            }
            if (x - play_x).abs() <= 15.0 {
                media.toggle_play_pause();
                return true;
            }
            if (x - next_x).abs() <= 15.0 {
                media.skip_next();
                return true;
            }
        }
        false
    }

    pub fn is_interactive(&self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let ctrl_y = top + 116.0;
        let (bub_x, prev_x, play_x, next_x) = if self.show_lyrics {
            (left + 53.0, left + 91.0, left + 131.0, left + 171.0)
        } else {
            (left + 75.0, left + 162.5, left + 210.0, left + 257.5)
        };
        if (y - ctrl_y).abs() <= 14.0 {
            if (x - bub_x).abs() <= 14.0
                || (x - prev_x).abs() <= 15.0
                || (x - play_x).abs() <= 15.0
                || (x - next_x).abs() <= 15.0
            {
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

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

fn parse_lrc(lrc: &str) -> Vec<LyricLine> {
    let mut out = Vec::new();
    for line in lrc.lines() {
        let line = line.trim();
        if let (Some(open), Some(close)) = (line.find('['), line.find(']')) {
            if open == 0 && close > 3 {
                let tag = &line[1..close];
                let text = line[close + 1..].trim().to_string();
                if text.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = tag.split(':').collect();
                if parts.len() == 2 {
                    if let (Ok(min), Ok(sec)) = (parts[0].parse::<u64>(), parts[1].parse::<f64>()) {
                        let time_ms = min * 60_000 + (sec * 1000.0) as u64;
                        out.push(LyricLine { time_ms, text });
                    }
                }
            }
        }
    }
    out
}
