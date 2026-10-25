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
                        // Switch mode
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
        card_brush: &ID2D1SolidColorBrush,
        accent_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;
        let right = bounds.right;
        let center_x = (left + right) / 2.0;

        // Header / Label on top left
        let header_rect = D2D_RECT_F {
            left: left + 24.0,
            top: top + 10.0,
            right: left + 150.0,
            bottom: top + 24.0,
        };
        let mode_label = match self.mode {
            PomodoroMode::Focus => "Pomodoro Focus",
            PomodoroMode::ShortBreak => "Short Break",
            PomodoroMode::LongBreak => "Long Break",
        };
        dwrite.draw_text(&d2d.context, mode_label, header_rect, dim_brush, &dwrite.format_small);

        // Session counter on top right
        let session_rect = D2D_RECT_F {
            left: right - 120.0,
            top: top + 10.0,
            right: right - 24.0,
            bottom: top + 24.0,
        };
        let session_str = format!("Cycle: {} / 4", (self.sessions_completed % 4) + 1);
        dwrite.draw_text(&d2d.context, &session_str, session_rect, dim_brush, &dwrite.format_small);

        // Big Countdown Display in center
        let min = self.remaining_secs / 60;
        let sec = self.remaining_secs % 60;
        let time_str = format!("{:02}:{:02}", min, sec);

        let time_rect = D2D_RECT_F {
            left: center_x - 100.0,
            top: top + 22.0,
            right: center_x + 100.0,
            bottom: top + 52.0,
        };
        dwrite.draw_text(&d2d.context, &time_str, time_rect, white_brush, &dwrite.format_big);

        // Status text below time
        let sub_rect = D2D_RECT_F {
            left: center_x - 80.0,
            top: top + 52.0,
            right: center_x + 80.0,
            bottom: top + 66.0,
        };
        let sub_text = if self.is_running { "Focus time" } else { "Paused" };
        dwrite.draw_text(&d2d.context, sub_text, sub_rect, dim_brush, &dwrite.format_center);

        // Interactive Buttons
        let btn_y = top + 68.0;
        let start_btn_rect = D2D_RECT_F {
            left: center_x - 55.0,
            top: btn_y,
            right: center_x - 5.0,
            bottom: btn_y + 22.0,
        };
        d2d.fill_rounded_rect(start_btn_rect, 4.0, 4.0, if self.is_running { card_brush } else { accent_brush });

        let start_label_rect = D2D_RECT_F {
            left: start_btn_rect.left,
            top: start_btn_rect.top + 2.0,
            right: start_btn_rect.right,
            bottom: start_btn_rect.bottom,
        };
        let start_btn_text = if self.is_running { "Pause" } else { "Start" };
        dwrite.draw_text(&d2d.context, start_btn_text, start_label_rect, white_brush, &dwrite.format_center);

        let reset_btn_rect = D2D_RECT_F {
            left: center_x + 5.0,
            top: btn_y,
            right: center_x + 55.0,
            bottom: btn_y + 22.0,
        };
        d2d.fill_rounded_rect(reset_btn_rect, 4.0, 4.0, card_brush);

        let reset_label_rect = D2D_RECT_F {
            left: reset_btn_rect.left,
            top: reset_btn_rect.top + 2.0,
            right: reset_btn_rect.right,
            bottom: reset_btn_rect.bottom,
        };
        dwrite.draw_text(&d2d.context, "Reset", reset_label_rect, white_brush, &dwrite.format_center);
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let right = bounds.right;
        let center_x = (left + right) / 2.0;

        let btn_y = top + 68.0;
        if y >= btn_y && y <= btn_y + 22.0 {
            if x >= center_x - 55.0 && x <= center_x - 5.0 {
                self.toggle_start_pause();
                return true;
            }
            if x >= center_x + 5.0 && x <= center_x + 55.0 {
                self.reset();
                return true;
            }
        }
        false
    }
}
