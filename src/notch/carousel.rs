use crate::notch::dots::{DotsIndicator, TOTAL_SECTIONS};
use crate::notch::nav::NavButtons;

pub struct Carousel {
    pub current_section: usize,
    pub target_section: usize,
    pub transition_offset: f32,
    pub dots: DotsIndicator,
    pub nav: NavButtons,
}

impl Carousel {
    pub fn new() -> Self {
        Self {
            current_section: 0,
            target_section: 0,
            transition_offset: 0.0,
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

    pub fn next(&mut self) {
        let next_idx = (self.current_section + 1) % TOTAL_SECTIONS;
        self.set_section(next_idx);
    }

    pub fn prev(&mut self) {
        let prev_idx = if self.current_section == 0 {
            TOTAL_SECTIONS - 1
        } else {
            self.current_section - 1
        };
        self.set_section(prev_idx);
    }

    pub fn set_section(&mut self, index: usize) {
        if index < TOTAL_SECTIONS && index != self.current_section {
            self.current_section = index;
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
