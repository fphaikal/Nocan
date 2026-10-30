use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    pub work_mins: u32,
    pub break_mins: u32,
    pub session_active: bool,
    last_tick: Option<Instant>,
    completed_at: Option<Instant>,
}

impl FocusSection {
    pub fn new() -> Self {
        Self {
            mode: PomodoroMode::Focus,
            is_running: false,
            remaining_secs: 25 * 60,
            total_secs: 25 * 60,
            sessions_completed: 0,
            work_mins: 25,
            break_mins: 5,
            session_active: false,
            last_tick: None,
            completed_at: None,
        }
    }

    pub fn update(&mut self) {
        self.update_at(Instant::now());
    }

    fn update_at(&mut self, now: Instant) {
        if self.is_running {
            if let Some(last) = self.last_tick {
                let elapsed = now.duration_since(last).as_secs() as u32;
                if elapsed >= 1 {
                    self.remaining_secs = self.remaining_secs.saturating_sub(elapsed);
                    self.last_tick = Some(last + std::time::Duration::from_secs(elapsed as u64));

                    if self.remaining_secs == 0 {
                        self.skip_next();
                        self.completed_at = Some(now);
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
        self.completed_at = None;
        self.session_active = true;
        self.is_running = !self.is_running;
        if self.is_running {
            self.last_tick = Some(Instant::now());
        } else {
            self.last_tick = None;
        }
    }

    pub fn reset(&mut self) {
        self.completed_at = None;
        self.session_active = false;
        self.is_running = false;
        self.last_tick = None;
        self.remaining_secs = self.total_secs;
    }

    pub fn set_mode(&mut self, mode: PomodoroMode) {
        self.completed_at = None;
        self.session_active = false;
        self.mode = mode;
        self.is_running = false;
        self.last_tick = None;
        self.total_secs = match mode {
            PomodoroMode::Focus => self.work_mins * 60,
            PomodoroMode::ShortBreak => self.break_mins * 60,
            PomodoroMode::LongBreak => 15 * 60,
        };
        self.remaining_secs = self.total_secs;
    }

    pub fn add_seconds(&mut self, secs: u32) {
        self.completed_at = None;
        self.remaining_secs = self.remaining_secs.saturating_add(secs);
        self.total_secs = self.total_secs.saturating_add(secs);
    }

    pub fn skip_next(&mut self) {
        self.completed_at = None;
        match self.mode {
            PomodoroMode::Focus => {
                self.sessions_completed += 1;
                if self.sessions_completed % 4 == 0 {
                    self.mode = PomodoroMode::LongBreak;
                    self.total_secs = 15 * 60;
                } else {
                    self.mode = PomodoroMode::ShortBreak;
                    self.total_secs = self.break_mins * 60;
                }
            }
            PomodoroMode::ShortBreak | PomodoroMode::LongBreak => {
                self.mode = PomodoroMode::Focus;
                self.total_secs = self.work_mins * 60;
            }
        }
        self.remaining_secs = self.total_secs;
        self.is_running = false;
        self.last_tick = None;
    }

    pub fn progress_ratio(&self) -> f32 {
        if self.total_secs == 0 {
            1.0
        } else {
            (self.remaining_secs as f32 / self.total_secs as f32).clamp(0.0, 1.0)
        }
    }

    pub fn just_completed(&self) -> bool {
        self.completed_at.map_or(false, |at| Instant::now().saturating_duration_since(at).as_secs() < 5)
    }

    pub fn elapsed_ratio(&self) -> f32 {
        1.0 - self.progress_ratio()
    }

    pub fn time_text(&self) -> String {
        format!("{:02}:{:02}", self.remaining_secs / 60, self.remaining_secs % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn countdown_keeps_fractional_elapsed_time() {
        let mut timer = FocusSection::new();
        timer.toggle_start_pause();
        let start = timer.last_tick.unwrap();
        timer.update_at(start + Duration::from_millis(1900));
        timer.update_at(start + Duration::from_millis(2100));
        assert_eq!(timer.remaining_secs, 25 * 60 - 2);
    }

    #[test]
    fn extending_a_session_preserves_time_already_spent() {
        let mut timer = FocusSection::new();
        timer.remaining_secs -= 41;
        timer.add_seconds(300);
        assert_eq!(timer.total_secs, 30 * 60);
        assert_eq!(timer.total_secs - timer.remaining_secs, 41);
        assert!(!timer.is_running);
    }

    #[test]
    fn paused_and_finished_sessions_stay_available_until_reset() {
        let mut timer = FocusSection::new();
        timer.toggle_start_pause();
        timer.toggle_start_pause();
        assert!(timer.session_active);
        timer.toggle_start_pause();
        timer.remaining_secs = 1;
        timer.update_at(timer.last_tick.unwrap() + Duration::from_secs(1));
        assert_eq!(timer.mode, PomodoroMode::ShortBreak);
        assert!(!timer.is_running);
        assert!(timer.session_active);
        assert_eq!(timer.sessions_completed, 1);
        assert!(timer.just_completed());
        timer.completed_at = Some(Instant::now() - Duration::from_secs(6));
        assert!(!timer.just_completed());
        timer.reset();
        assert!(!timer.session_active);
        assert!(!timer.just_completed());
    }

    #[test]
    fn skipping_a_session_does_not_report_a_completed_countdown() {
        let mut timer = FocusSection::new();
        timer.toggle_start_pause();
        timer.skip_next();
        assert!(!timer.just_completed());
    }
}
