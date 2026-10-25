use crate::notch::dots::{DotsIndicator, TOTAL_SECTIONS};
use crate::notch::nav::NavButtons;

pub struct Carousel {
    pub current_section: usize,
    pub page_pos: f32,
    pub page_vel: f32,
    pub dots: DotsIndicator,
    pub nav: NavButtons,
}

impl Carousel {
    pub fn new() -> Self {
        Self {
            current_section: 0,
            page_pos: 0.0,
            page_vel: 0.0,
            dots: DotsIndicator::new(),
            nav: NavButtons::new(),
        }
    }

    pub fn show_prev(&self) -> bool {
        self.current_section > 0
    }

    pub fn show_next(&self) -> bool {
        self.current_section + 1 < TOTAL_SECTIONS
    }

    pub fn prev_alpha(&self) -> f32 {
        self.page_pos.clamp(0.0, 1.0)
    }

    pub fn next_alpha(&self) -> f32 {
        ((TOTAL_SECTIONS - 1) as f32 - self.page_pos).clamp(0.0, 1.0)
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0.001, 0.050);
        let target = self.current_section as f32;
        // Exact spring parameters from nocan.original.exe (0x14002966f): stiffness = 230.0, damping = 19.0
        let force = 230.0 * (target - self.page_pos) - 19.0 * self.page_vel;
        self.page_vel += force * dt;
        self.page_pos += self.page_vel * dt;

        if (self.page_pos - target).abs() < 0.0015 && self.page_vel.abs() < 0.008 {
            self.page_pos = target;
            self.page_vel = 0.0;
        }
    }

    pub fn next(&mut self) {
        if self.current_section + 1 < TOTAL_SECTIONS {
            self.set_section(self.current_section + 1);
        }
    }

    pub fn prev(&mut self) {
        if self.current_section > 0 {
            self.set_section(self.current_section - 1);
        }
    }

    pub fn set_section(&mut self, index: usize) {
        if index < TOTAL_SECTIONS && index != self.current_section {
            self.current_section = index;
            self.dots.set_index(index);
        }
    }

    pub fn snap_to_section(&mut self, index: usize) {
        if index < TOTAL_SECTIONS {
            self.current_section = index;
            self.page_pos = index as f32;
            self.page_vel = 0.0;
            self.dots.set_index(index);
        }
    }

    pub fn on_wheel(&mut self, delta: i16) {
        if delta > 0 {
            self.prev();
        } else if delta < 0 {
            self.next();
        }
    }

    pub fn on_click(&mut self, body_left: f32, body_w: f32, body_h: f32, x: f32, y: f32) -> bool {
        if self.nav.hit_test_prev(body_left, body_w, body_h, x, y, self.show_prev()) {
            self.prev();
            return true;
        }
        if self.nav.hit_test_next(body_left, body_w, body_h, x, y, self.show_next()) {
            self.next();
            return true;
        }
        if let Some(idx) = self.dots.hit_test(body_left, body_w, body_h, x, y) {
            self.set_section(idx);
            return true;
        }
        false
    }

    pub fn on_mouse_move(&mut self, body_left: f32, body_w: f32, body_h: f32, x: f32, y: f32) {
        self.nav.hovered_prev = self.nav.hit_test_prev(body_left, body_w, body_h, x, y, self.show_prev());
        self.nav.hovered_next = self.nav.hit_test_next(body_left, body_w, body_h, x, y, self.show_next());
    }
}
