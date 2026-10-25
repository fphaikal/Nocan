use std::time::Instant;
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PomodoroMode {
    Focus,
    ShortBreak,
    LongBreak,
}

pub struct FocusSection {
    pub mode: PomodoroMode,
    pub is_running: bool,
    pub remaining_secs: u32,
    pub total_secs: u32,
    pub sessions_completed: u32,
    last_tick: Option<Instant>,
}

impl FocusSection {
    pub fn new() -> Self {
        Self {
            mode: PomodoroMode::Focus,
            is_running: false,
            remaining_secs: 25 * 60,
            total_secs: 25 * 60,
            sessions_completed: 0,
            last_tick: None,
        }
    }

    pub fn update(&mut self) {
        if self.is_running {
            let now = Instant::now();
            if let Some(last) = self.last_tick {
                let elapsed = now.duration_since(last).as_secs() as u32;
                if elapsed >= 1 {
                    self.remaining_secs = self.remaining_secs.saturating_sub(elapsed);
                    self.last_tick = Some(now);

                    if self.remaining_secs == 0 {
                        match self.mode {
                            PomodoroMode::Focus => {
                                self.sessions_completed += 1;
                                if self.sessions_completed % 4 == 0 {
                                    self.mode = PomodoroMode::LongBreak;
                                    self.total_secs = 15 * 60;
                                } else {
                                    self.mode = PomodoroMode::ShortBreak;
                                    self.total_secs = 5 * 60;
                                }
                            }
                            PomodoroMode::ShortBreak | PomodoroMode::LongBreak => {
                                self.mode = PomodoroMode::Focus;
                                self.total_secs = 25 * 60;
                            }
                        }
                        self.remaining_secs = self.total_secs;
                        self.is_running = false;
                        self.last_tick = None;
                    }
                }
            } else {
                self.last_tick = Some(now);
            }
        } else {
            self.last_tick = None;
        }
    }

    pub fn toggle_start_pause(&mut self) {
        self.is_running = !self.is_running;
        if self.is_running {
            self.last_tick = Some(Instant::now());
        } else {
            self.last_tick = None;
        }
    }

    pub fn reset(&mut self) {
        self.is_running = false;
        self.last_tick = None;
        self.remaining_secs = self.total_secs;
    }

    pub fn set_mode(&mut self, mode: PomodoroMode) {
        self.mode = mode;
        self.is_running = false;
        self.last_tick = None;
        self.total_secs = match mode {
            PomodoroMode::Focus => 25 * 60,
            PomodoroMode::ShortBreak => 5 * 60,
            PomodoroMode::LongBreak => 15 * 60,
        };
        self.remaining_secs = self.total_secs;
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

        // 1. Section Title: "Pomodoro" at left: 15px, top: 19px (vc)
        let title_rect = D2D_RECT_F {
            left: left + 15.0,
            top: top + 9.0,
            right: left + 135.0,
            bottom: top + 29.0,
        };
        dwrite.draw_text(&d2d.context, "Pomodoro", title_rect, white_brush, &dwrite.format_title);

        // 2. Left Circular Ring + Glow + Stopwatch Icon at (left + 80, top + 74)
        let ring_cx = left + 80.0;
        let ring_cy = top + 74.0;

        if let Ok(glow_b) = d2d.create_radial_gradient(
            (ring_cx, ring_cy),
            44.0,
            44.0,
            &[
                (0.0, D2D1_COLOR_F { r: 1.0, g: 60.0 / 255.0, b: 60.0 / 255.0, a: 0.34 }),
                (0.65, D2D1_COLOR_F { r: 1.0, g: 60.0 / 255.0, b: 60.0 / 255.0, a: 0.10 }),
                (1.0, D2D1_COLOR_F { r: 1.0, g: 60.0 / 255.0, b: 60.0 / 255.0, a: 0.0 }),
            ],
        ) {
            d2d.fill_ellipse_radial(ring_cx, ring_cy, 44.0, 44.0, &glow_b);
        }

        let progress = if self.total_secs > 0 {
            (self.remaining_secs as f32 / self.total_secs as f32).clamp(0.0, 1.0)
        } else {
            1.0
        };

        if let (Ok(track_b), Ok(arc_b)) = (
            d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 90.0 / 255.0, b: 110.0 / 255.0, a: 0.25 }),
            d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 90.0 / 255.0, b: 102.0 / 255.0, a: 1.0 }),
        ) {
            d2d.draw_progress_ring(ring_cx, ring_cy, 28.8, 6.4, progress, &track_b, &arc_b);
        }

        // Stopwatch Icon inside ring at (left + 80, top + 75)
        let sw_cx = left + 80.0;
        let sw_cy = top + 78.0;
        d2d.draw_ellipse(sw_cx, sw_cy, 13.5, 13.5, white_brush, 2.8);
        d2d.draw_line(sw_cx, sw_cy, sw_cx, sw_cy - 7.5, white_brush, 2.8, true);
        d2d.draw_line(sw_cx, sw_cy - 13.5, sw_cx, sw_cy - 17.8, white_brush, 2.8, true);
        d2d.draw_line(sw_cx - 4.2, sw_cy - 17.8, sw_cx + 4.2, sw_cy - 17.8, white_brush, 2.8, true);
        d2d.draw_line(sw_cx + 9.8, sw_cy - 9.8, sw_cx + 12.8, sw_cy - 12.8, white_brush, 2.6, true);

        // 3. Countdown Time: "25:00" centered at (left + 265, top + 50)
        let min = self.remaining_secs / 60;
        let sec = self.remaining_secs % 60;
        let time_str = format!("{:02}:{:02}", min, sec);
        let time_rect = D2D_RECT_F {
            left: left + 205.0,
            top: top + 32.0,
            right: left + 325.0,
            bottom: top + 66.0,
        };
        dwrite.draw_text(&d2d.context, &time_str, time_rect, white_brush, &dwrite.format_big_center);

        // 4. Circular Reset Button at (left + 330, top + 50), 24x24px
        let rst_cx = left + 330.0;
        let rst_cy = top + 50.0;
        d2d.fill_ellipse(rst_cx, rst_cy, 12.0, 12.0, pill_bg_brush);
        // Circular arrow icon inside reset button
        if let Ok(dim_ring) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.0 }) {
            d2d.draw_progress_ring(rst_cx, rst_cy, 4.6, 1.7, 0.80, &dim_ring, white_brush);
        }
        let arrow_pts = [
            (rst_cx + 1.8, rst_cy - 5.2),
            (rst_cx + 4.8, rst_cy - 5.2),
            (rst_cx + 4.8, rst_cy - 2.2),
        ];
        d2d.draw_polyline(&arrow_pts, white_brush, 1.7, false);

        // 5. Subtitle: "Focus time" centered at (left + 265, top + 76)
        let sub_text = match self.mode {
            PomodoroMode::Focus => "Focus time",
            PomodoroMode::ShortBreak => "Short break",
            PomodoroMode::LongBreak => "Long break",
        };
        let sub_rect = D2D_RECT_F {
            left: left + 205.0,
            top: top + 67.0,
            right: left + 325.0,
            bottom: top + 85.0,
        };
        dwrite.draw_text(&d2d.context, sub_text, sub_rect, dim_brush, &dwrite.format_sub_center);

        // 6. Three Pill Buttons at top: 92px, height: 28px, rx: 14px
        let btn_top = top + 92.0;
        let btn_bot = btn_top + 28.0;
        let btn_cy = btn_top + 14.0;

        // Pill 1: Short Break (left: 144, width: 86)
        let sb_rect = D2D_RECT_F {
            left: left + 144.0,
            top: btn_top,
            right: left + 230.0,
            bottom: btn_bot,
        };
        d2d.fill_rounded_rect(sb_rect, 14.0, 14.0, pill_bg_brush);
        // Coffee icon at left + 155.5, btn_cy
        let cup_rect = D2D_RECT_F {
            left: left + 151.0,
            top: btn_cy - 2.5,
            right: left + 158.0,
            bottom: btn_cy + 4.0,
        };
        d2d.draw_rounded_rect(cup_rect, 1.8, 1.8, white_brush, 1.3);
        d2d.draw_line(left + 158.0, btn_cy - 1.2, left + 160.0, btn_cy - 1.2, white_brush, 1.2, true);
        d2d.draw_line(left + 158.0, btn_cy + 1.8, left + 160.0, btn_cy + 1.8, white_brush, 1.2, true);
        d2d.draw_line(left + 153.2, btn_cy - 5.2, left + 153.2, btn_cy - 3.8, white_brush, 1.2, true);
        d2d.draw_line(left + 155.8, btn_cy - 5.2, left + 155.8, btn_cy - 3.8, white_brush, 1.2, true);

        let sb_text_rect = D2D_RECT_F {
            left: left + 163.5,
            top: btn_top,
            right: left + 228.0,
            bottom: btn_bot,
        };
        dwrite.draw_text(&d2d.context, "Short Break", sb_text_rect, white_brush, &dwrite.format_button);

        // Pill 2: Play/Pause Red Pill (left: 238, width: 55, #ff4b55)
        let play_rect = D2D_RECT_F {
            left: left + 238.0,
            top: btn_top,
            right: left + 293.0,
            bottom: btn_bot,
        };
        if let Ok(red_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 1.0,
            g: 75.0 / 255.0,
            b: 85.0 / 255.0,
            a: 1.0,
        }) {
            d2d.fill_rounded_rect(play_rect, 14.0, 14.0, &red_b);
        }
        let play_cx = left + 265.5;
        if self.is_running {
            let b1 = D2D_RECT_F {
                left: play_cx - 4.0,
                top: btn_cy - 4.5,
                right: play_cx - 1.4,
                bottom: btn_cy + 4.5,
            };
            let b2 = D2D_RECT_F {
                left: play_cx + 1.4,
                top: btn_cy - 4.5,
                right: play_cx + 4.0,
                bottom: btn_cy + 4.5,
            };
            d2d.fill_rounded_rect(b1, 0.8, 0.8, white_brush);
            d2d.fill_rounded_rect(b2, 0.8, 0.8, white_brush);
        } else {
            let tri = [
                (play_cx - 3.2, btn_cy - 4.6),
                (play_cx + 4.5, btn_cy),
                (play_cx - 3.2, btn_cy + 4.6),
            ];
            d2d.fill_polygon(&tri, white_brush);
        }

        // Pill 3: Focus Time (left: 301, width: 85)
        let ft_rect = D2D_RECT_F {
            left: left + 301.0,
            top: btn_top,
            right: left + 386.0,
            bottom: btn_bot,
        };
        d2d.fill_rounded_rect(ft_rect, 14.0, 14.0, pill_bg_brush);
        // Target icon at left + 312.5, btn_cy
        let t_cx = left + 312.5;
        d2d.draw_ellipse(t_cx, btn_cy, 4.6, 4.6, white_brush, 1.3);
        d2d.draw_ellipse(t_cx, btn_cy, 2.0, 2.0, white_brush, 1.2);
        d2d.draw_line(t_cx, btn_cy, t_cx + 4.0, btn_cy - 4.0, white_brush, 1.3, true);

        let ft_text_rect = D2D_RECT_F {
            left: left + 320.5,
            top: btn_top,
            right: left + 384.0,
            bottom: btn_bot,
        };
        dwrite.draw_text(&d2d.context, "Focus Time", ft_text_rect, white_brush, &dwrite.format_button);
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;

        // Reset button at (left + 330, top + 50, r = 12)
        let dx = x - (left + 330.0);
        let dy = y - (top + 50.0);
        if dx * dx + dy * dy <= 14.0 * 14.0 {
            self.reset();
            return true;
        }

        let btn_top = top + 92.0;
        let btn_bot = btn_top + 28.0;
        if y >= btn_top - 2.0 && y <= btn_bot + 2.0 {
            if x >= left + 144.0 && x <= left + 230.0 {
                self.set_mode(PomodoroMode::ShortBreak);
                return true;
            }
            if x >= left + 238.0 && x <= left + 293.0 {
                self.toggle_start_pause();
                return true;
            }
            if x >= left + 301.0 && x <= left + 386.0 {
                self.set_mode(PomodoroMode::Focus);
                return true;
            }
        }
        false
    }

    pub fn is_interactive(&self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let dx = x - (left + 330.0);
        let dy = y - (top + 50.0);
        if dx * dx + dy * dy <= 14.0 * 14.0 {
            return true;
        }
        let btn_top = top + 92.0;
        let btn_bot = btn_top + 28.0;
        if y >= btn_top - 2.0 && y <= btn_bot + 2.0 {
            if (x >= left + 144.0 && x <= left + 230.0)
                || (x >= left + 238.0 && x <= left + 293.0)
                || (x >= left + 301.0 && x <= left + 386.0)
            {
                return true;
            }
        }
        false
    }
}
