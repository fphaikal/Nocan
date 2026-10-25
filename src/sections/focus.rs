use std::time::Instant;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
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

    pub fn draw(
        &self,
        d2d: &D2DContext,
        dwrite: &DWriteContext,
        bounds: D2D_RECT_F,
        white_brush: &ID2D1SolidColorBrush,
        dim_brush: &ID2D1SolidColorBrush,
        tomato_brush: &ID2D1SolidColorBrush,
        btn_dark_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;

        // Section Title: "Pomodoro" at x=24, y=12
        let title_rect = D2D_RECT_F {
            left: left + 24.0,
            top: top + 10.0,
            right: left + 140.0,
            bottom: top + 26.0,
        };
        dwrite.draw_text(&d2d.context, "Pomodoro", title_rect, white_brush, &dwrite.format_title);

        // Circular Tomato / Timer Graphic on Left at x=44, y=32, size 68x68
        let tomato_center_x = left + 44.0 + 34.0;
        let tomato_center_y = top + 32.0 + 34.0;
        d2d.fill_ellipse(tomato_center_x, tomato_center_y, 32.0, 32.0, tomato_brush);

        // Tomato leaf / icon inside
        let tomato_icon_rect = D2D_RECT_F {
            left: tomato_center_x - 16.0,
            top: tomato_center_y - 14.0,
            right: tomato_center_x + 16.0,
            bottom: tomato_center_y + 14.0,
        };
        dwrite.draw_text(&d2d.context, "🍅", tomato_icon_rect, white_brush, &dwrite.format_center);

        // Right side: Countdown and controls (x=210..380)
        let min = self.remaining_secs / 60;
        let sec = self.remaining_secs % 60;
        let time_str = format!("{:02}:{:02}", min, sec);

        let time_rect = D2D_RECT_F {
            left: left + 215.0,
            top: top + 30.0,
            right: left + 360.0,
            bottom: top + 56.0,
        };
        dwrite.draw_text(&d2d.context, &time_str, time_rect, white_brush, &dwrite.format_big);

        // Subtitle "Focus time" or "Break"
        let sub_rect = D2D_RECT_F {
            left: left + 217.0,
            top: top + 58.0,
            right: left + 360.0,
            bottom: top + 74.0,
        };
        let sub_text = match self.mode {
            PomodoroMode::Focus => {
                if self.is_running { "Focus time" } else { "Focus Time • Paused" }
            }
            PomodoroMode::ShortBreak => "Short Break",
            PomodoroMode::LongBreak => "Long Break",
        };
        dwrite.draw_text(&d2d.context, sub_text, sub_rect, dim_brush, &dwrite.format_regular);

        // Action Buttons at y=86
        let btn_y = top + 84.0;

        // Start / Pause red pill button
        let start_btn_rect = D2D_RECT_F {
            left: left + 215.0,
            top: btn_y,
            right: left + 280.0,
            bottom: btn_y + 26.0,
        };
        d2d.fill_rounded_rect(start_btn_rect, 13.0, 13.0, tomato_brush);

        let start_label = if self.is_running { "Pause" } else { "Start" };
        let start_text_rect = D2D_RECT_F {
            left: start_btn_rect.left,
            top: start_btn_rect.top + 3.0,
            right: start_btn_rect.right,
            bottom: start_btn_rect.bottom,
        };
        dwrite.draw_text(&d2d.context, start_label, start_text_rect, white_brush, &dwrite.format_button);

        // Reset dark pill button
        let reset_btn_rect = D2D_RECT_F {
            left: left + 288.0,
            top: btn_y,
            right: left + 362.0,
            bottom: btn_y + 26.0,
        };
        d2d.fill_rounded_rect(reset_btn_rect, 13.0, 13.0, btn_dark_brush);

        let reset_text_rect = D2D_RECT_F {
            left: reset_btn_rect.left,
            top: reset_btn_rect.top + 3.0,
            right: reset_btn_rect.right,
            bottom: reset_btn_rect.bottom,
        };
        dwrite.draw_text(&d2d.context, "Reset", reset_text_rect, white_brush, &dwrite.format_button);
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let btn_y = top + 84.0;

        if y >= btn_y - 2.0 && y <= btn_y + 28.0 {
            if x >= left + 215.0 && x <= left + 280.0 {
                self.toggle_start_pause();
                return true;
            }
            if x >= left + 288.0 && x <= left + 362.0 {
                self.reset();
                return true;
            }
        }
        false
    }
}
