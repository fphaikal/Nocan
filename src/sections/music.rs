use std::sync::atomic::{AtomicBool, Ordering};
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
    fetch_in_progress: Arc<AtomicBool>,
    fetch_failed: Arc<AtomicBool>,
    last_fetch_instant: std::time::Instant,
    lyrics: Arc<Mutex<Vec<LyricLine>>>,
    lyrics_status: Arc<Mutex<String>>,
    lyrics_loaded: Arc<AtomicBool>,
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
            fetch_in_progress: Arc::new(AtomicBool::new(false)),
            fetch_failed: Arc::new(AtomicBool::new(false)),
            last_fetch_instant: std::time::Instant::now(),
            lyrics: Arc::new(Mutex::new(default_lyrics)),
            lyrics_status: Arc::new(Mutex::new(String::new())),
            lyrics_loaded: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn update_lyrics(&mut self, track: &MediaTrackInfo) {
        if track.title.is_empty() {
            return;
        }
        let key = format!("{}|{}|{}", track.title, track.artist, track.album);

        if self.fetch_in_progress.load(Ordering::SeqCst) {
            return;
        }

        // Retry condition: if the previous attempt failed, lyrics are not loaded, track is still playing,
        // and at least 6 seconds have passed since the previous attempt:
        let is_retry = key == self.last_fetched_key
            && self.fetch_failed.load(Ordering::SeqCst)
            && !self.lyrics_loaded.load(Ordering::SeqCst)
            && track.is_playing
            && self.last_fetch_instant.elapsed().as_secs() >= 6;

        if key == self.last_fetched_key && !is_retry {
            return;
        }

        self.last_fetched_key = key;
        self.last_fetch_instant = std::time::Instant::now();
        self.fetch_failed.store(false, Ordering::SeqCst);
        self.fetch_in_progress.store(true, Ordering::SeqCst);

        let title = track.title.clone();
        let artist = track.artist.clone();
        let album = track.album.clone();
        let duration_ms = track.duration_ms;

        let lyrics_arc = Arc::clone(&self.lyrics);
        let status_arc = Arc::clone(&self.lyrics_status);
        let loaded_arc = Arc::clone(&self.lyrics_loaded);
        let in_progress_arc = Arc::clone(&self.fetch_in_progress);
        let failed_arc = Arc::clone(&self.fetch_failed);

        if !is_retry {
            loaded_arc.store(false, Ordering::SeqCst);
            if let Ok(mut l) = lyrics_arc.lock() {
                l.clear();
            }
            if let Ok(mut st) = status_arc.lock() {
                *st = "Loading lyrics...".to_string();
            }
        }

        std::thread::spawn(move || {
            let result = fetch_lyrics_multitier(&title, &artist, &album, duration_ms);
            in_progress_arc.store(false, Ordering::SeqCst);

            match result {
                Some(lines) => {
                    if let Ok(mut l) = lyrics_arc.lock() {
                        *l = lines;
                    }
                    loaded_arc.store(true, Ordering::SeqCst);
                    failed_arc.store(false, Ordering::SeqCst);
                    if let Ok(mut st) = status_arc.lock() {
                        st.clear();
                    }
                }
                None => {
                    failed_arc.store(true, Ordering::SeqCst);
                    if let Ok(mut st) = status_arc.lock() {
                        *st = "Lyrics not available".to_string();
                    }
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
        ambient_color: (f32, f32, f32),
    ) {
        let left = bounds.left;
        let top = bounds.top;

        // If in lyrics mode, draw bottom ambient glow from album cover & right lyrics pane
        if self.show_lyrics {
            let body_w = bounds.right - bounds.left;
            let body_h = bounds.bottom - bounds.top;
            let (cr, cg, cb) = ambient_color;
            let sh = |v: f32, m: f32| (v * m).min(1.0);
            d2d.draw_ambient_glow(
                left,
                body_w,
                body_h,
                32.0,
                0.85,
                0.90,
                0.0,
                &[
                    (0.0, D2D1_COLOR_F { r: sh(cr, 1.15), g: sh(cg, 1.15), b: sh(cb, 1.15), a: 1.0 }),
                    (0.5, D2D1_COLOR_F { r: cr, g: cg, b: cb, a: 1.0 }),
                    (1.0, D2D1_COLOR_F { r: sh(cr, 0.80), g: sh(cg, 0.80), b: sh(cb, 0.80), a: 1.0 }),
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

    /// Active synced lyric sentence for the notch bar (only while a track is playing and its lyrics are loaded).
    pub fn current_lyric_line(&self, track: &MediaTrackInfo) -> Option<String> {
        if track.title.is_empty() || !track.is_playing || !self.lyrics_loaded.load(Ordering::SeqCst) {
            return None;
        }
        let lines = self.lyrics.lock().ok()?;
        if lines.is_empty() {
            return None;
        }
        let idx = lines.iter().rposition(|l| track.position_ms >= l.time_ms).unwrap_or(0);
        Some(lines[idx].text.clone())
    }

    /// Width in px needed by the visible lyric lines (or status text), used to size the notch dynamically.
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
            let start_i = idx.saturating_sub(1);
            let mut max_w = 0.0f32;
            for i in start_i..(start_i + 4).min(lines.len()) {
                let w = dwrite.measure_text_width(&lines[i].text, &dwrite.format_lyric);
                if w > max_w {
                    max_w = w;
                }
            }
            return max_w;
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
        if line.is_empty() {
            continue;
        }
        let mut rest = line;
        let mut timestamps = Vec::new();
        while let Some(open) = rest.find('[') {
            if let Some(close) = rest[open..].find(']') {
                let close = open + close;
                let tag = &rest[open + 1..close];
                if let Some(time_ms) = parse_lrc_timestamp(tag) {
                    timestamps.push(time_ms);
                }
                rest = &rest[close + 1..];
            } else {
                break;
            }
        }
        let text = rest.trim().to_string();
        if text.is_empty() {
            continue;
        }
        for time_ms in timestamps {
            out.push(LyricLine {
                time_ms,
                text: text.clone(),
            });
        }
    }
    out.sort_by_key(|l| l.time_ms);
    out
}

fn parse_lrc_timestamp(tag: &str) -> Option<u64> {
    let parts: Vec<&str> = tag.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let min = parts[0].trim().parse::<u64>().ok()?;
    let sec_parts: Vec<&str> = parts[1].trim().split('.').collect();
    let sec = sec_parts[0].trim().parse::<u64>().ok()?;
    let ms = if sec_parts.len() > 1 {
        let mut ms_str = sec_parts[1].trim().to_string();
        if ms_str.len() == 1 {
            ms_str.push_str("00");
        } else if ms_str.len() == 2 {
            ms_str.push('0');
        } else if ms_str.len() > 3 {
            ms_str.truncate(3);
        }
        ms_str.parse::<u64>().unwrap_or(0)
    } else {
        0
    };
    Some(min * 60_000 + sec * 1_000 + ms)
}

fn parse_plain_lyrics(plain: &str, duration_ms: u64) -> Vec<LyricLine> {
    let raw_lines: Vec<&str> = plain
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    if raw_lines.is_empty() {
        return Vec::new();
    }
    let count = raw_lines.len();
    let step_ms = if duration_ms > 0 && count > 0 {
        duration_ms / count as u64
    } else {
        4000
    };
    raw_lines
        .into_iter()
        .enumerate()
        .map(|(i, text)| LyricLine {
            time_ms: i as u64 * step_ms,
            text: text.to_string(),
        })
        .collect()
}

fn strip_video_noise(s: &str) -> String {
    let mut out = s.trim().to_string();

    let brackets: &[(&str, &str)] = &[("(", ")"), ("[", "]")];
    for &(open, close) in brackets {
        let mut changed = true;
        while changed {
            changed = false;
            if let Some(start) = out.rfind(open) {
                if let Some(rel_end) = out[start..].find(close) {
                    let end = start + rel_end;
                    let inner = out[start + open.len()..end].to_ascii_lowercase();
                    let is_noise = inner.contains("video")
                        || inner.contains("audio")
                        || inner.contains("official")
                        || inner.contains("lyric")
                        || inner.contains("lyrics")
                        || inner.contains("visualizer")
                        || inner.contains("mv")
                        || inner.contains("hd")
                        || inner.contains("4k")
                        || inner.contains("remaster")
                        || inner.contains("live")
                        || inner.contains("clip")
                        || inner.contains("prod")
                        || inner.contains("feat")
                        || inner.contains("ft.");
                    if is_noise {
                        out.replace_range(start..end + close.len(), "");
                        changed = true;
                    }
                }
            }
        }
    }

    if let Some(pos) = out.to_ascii_lowercase().rfind(" - remastered") {
        out.truncate(pos);
    }

    out.trim().to_string()
}

fn fetch_lyrics_multitier(
    raw_title: &str,
    raw_artist: &str,
    album: &str,
    duration_ms: u64,
) -> Option<Vec<LyricLine>> {
    let clean_full_title = strip_video_noise(raw_title);
    let duration_sec = if duration_ms > 0 { duration_ms / 1000 } else { 0 };

    let mut candidates: Vec<(String, String)> = Vec::new();

    // If title has " - ", split into (Artist, Title) - standard YouTube music format
    if clean_full_title.contains(" - ") {
        let parts: Vec<&str> = clean_full_title.splitn(2, " - ").collect();
        let part_artist = parts[0].trim().to_string();
        let part_title = parts[1].trim().to_string();
        if !part_artist.is_empty() && !part_title.is_empty() {
            candidates.push((part_artist, part_title));
        }
    }

    // Also include (raw_artist, clean_full_title) if artist is not empty
    if !raw_artist.is_empty() && !clean_full_title.is_empty() {
        let clean_art = strip_video_noise(raw_artist);
        if !candidates.iter().any(|(a, t)| a == &clean_art && t == &clean_full_title) {
            candidates.push((clean_art, clean_full_title.clone()));
        }
    }

    // Fallback if no split was possible
    if candidates.is_empty() {
        candidates.push((raw_artist.trim().to_string(), clean_full_title.clone()));
    }

    let call_api = |url: &str| -> Option<serde_json::Value> {
        for attempt in 0..2 {
            let resp = ureq::get(url)
                .set("User-Agent", "Nocan/1.1.0 (https://fph.my.id)")
                .timeout(std::time::Duration::from_secs(3))
                .call();

            match resp {
                Ok(r) => {
                    return r.into_json::<serde_json::Value>().ok();
                }
                Err(ureq::Error::Status(503, _)) | Err(ureq::Error::Status(429, _)) => {
                    if attempt == 0 {
                        // LRCLIB server overloaded: respect retry-after and pause 1s
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    }
                }
                Err(_) => break,
            }
        }
        None
    };

    let extract_lyrics = |json: &serde_json::Value| -> Option<Vec<LyricLine>> {
        if let Some(arr) = json.as_array() {
            for item in arr {
                if let Some(synced) = item.get("syncedLyrics").and_then(|v| v.as_str()) {
                    let lines = parse_lrc(synced);
                    if !lines.is_empty() {
                        return Some(lines);
                    }
                }
            }
            for item in arr {
                if let Some(plain) = item.get("plainLyrics").and_then(|v| v.as_str()) {
                    let lines = parse_plain_lyrics(plain, duration_ms);
                    if !lines.is_empty() {
                        return Some(lines);
                    }
                }
            }
        } else if json.is_object() {
            if let Some(synced) = json.get("syncedLyrics").and_then(|v| v.as_str()) {
                let lines = parse_lrc(synced);
                if !lines.is_empty() {
                    return Some(lines);
                }
            }
            if let Some(plain) = json.get("plainLyrics").and_then(|v| v.as_str()) {
                let lines = parse_plain_lyrics(plain, duration_ms);
                if !lines.is_empty() {
                    return Some(lines);
                }
            }
        }
        None
    };

    // Strategy 1: Exact /api/get for each candidate (matches fph-portfolio)
    for (artist, title) in &candidates {
        if !album.is_empty() {
            let mut url = format!(
                "https://lrclib.net/api/get?artist_name={}&track_name={}&album_name={}",
                url_encode(artist),
                url_encode(title),
                url_encode(album)
            );
            if duration_sec > 0 {
                url.push_str(&format!("&duration={}", duration_sec));
            }
            if let Some(json) = call_api(&url) {
                if let Some(lines) = extract_lyrics(&json) {
                    return Some(lines);
                }
            }
        }

        let mut url = format!(
            "https://lrclib.net/api/get?artist_name={}&track_name={}",
            url_encode(artist),
            url_encode(title)
        );
        if duration_sec > 0 {
            url.push_str(&format!("&duration={}", duration_sec));
        }
        if let Some(json) = call_api(&url) {
            if let Some(lines) = extract_lyrics(&json) {
                return Some(lines);
            }
        }
    }

    // Strategy 2: Search /api/search with artist_name & track_name
    for (artist, title) in &candidates {
        let url = format!(
            "https://lrclib.net/api/search?track_name={}&artist_name={}",
            url_encode(title),
            url_encode(artist)
        );
        if let Some(json) = call_api(&url) {
            if let Some(lines) = extract_lyrics(&json) {
                return Some(lines);
            }
        }
    }

    // Strategy 3: General query search /api/search?q={title}+{artist}
    for (artist, title) in &candidates {
        let q = format!("{} {}", title, artist).trim().to_string();
        let url = format!("https://lrclib.net/api/search?q={}", url_encode(&q));
        if let Some(json) = call_api(&url) {
            if let Some(lines) = extract_lyrics(&json) {
                return Some(lines);
            }
        }
    }

    // Strategy 4: Fallback - Search by title alone (crucial for songs like 'Beautiful in White' where album/artist varies)
    for (_, title) in &candidates {
        let url = format!("https://lrclib.net/api/search?track_name={}", url_encode(title));
        if let Some(json) = call_api(&url) {
            if let Some(lines) = extract_lyrics(&json) {
                return Some(lines);
            }
        }
        let url = format!("https://lrclib.net/api/search?q={}", url_encode(title));
        if let Some(json) = call_api(&url) {
            if let Some(lines) = extract_lyrics(&json) {
                return Some(lines);
            }
        }
    }

    None
}
