use crate::notch::dots::DotsIndicator;
use crate::notch::nav::NavButtons;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SectionId {
    Music,
    Focus,
    Water,
    Shelf,
    Todo,
    OpenMaus,
    AiUsage,
    Saham,
}

pub struct Carousel {
    pub active_sections: Vec<SectionId>,
    pub current_section: usize,
    pub page_pos: f32,
    pub page_vel: f32,
    pub dots: DotsIndicator,
    pub nav: NavButtons,
}

impl Carousel {
    pub fn new() -> Self {
        let active = vec![
            SectionId::Music,
            SectionId::Focus,
            SectionId::Water,
            SectionId::Shelf,
            SectionId::Todo,
            SectionId::OpenMaus,
            SectionId::AiUsage,
            SectionId::Saham,
        ];
        let mut dots = DotsIndicator::new();
        dots.set_total_sections(active.len());
        Self {
            active_sections: active,
            current_section: 0,
            page_pos: 0.0,
            page_vel: 0.0,
            dots,
            nav: NavButtons::new(),
        }
    }

    pub fn set_active_sections(&mut self, sections: Vec<SectionId>) {
        if self.active_sections == sections { return; }
        if sections.is_empty() {
            self.active_sections = vec![SectionId::Music];
        } else {
            let current_id = self.current_section_id();
            self.active_sections = sections;
            if let Some(pos) = self.active_sections.iter().position(|&s| s == current_id) {
                self.current_section = pos;
                self.page_pos = pos as f32;
                self.page_vel = 0.0;
            } else if self.current_section >= self.active_sections.len() {
                self.current_section = self.active_sections.len() - 1;
                self.page_pos = self.current_section as f32;
                self.page_vel = 0.0;
            }
        }
        self.dots.set_total_sections(self.active_sections.len());
        self.dots.set_index(self.current_section);
    }

    pub fn current_section_id(&self) -> SectionId {
        self.active_sections
            .get(self.current_section)
            .copied()
            .unwrap_or(SectionId::Music)
    }

    pub fn total_sections(&self) -> usize {
        self.active_sections.len()
    }

    pub fn show_prev(&self) -> bool {
        self.current_section > 0
    }

    pub fn show_next(&self) -> bool {
        self.current_section + 1 < self.total_sections()
    }

    pub fn prev_alpha(&self) -> f32 {
        self.page_pos.clamp(0.0, 1.0)
    }

    pub fn next_alpha(&self) -> f32 {
        let total = self.total_sections();
        if total <= 1 {
            0.0
        } else {
            ((total - 1) as f32 - self.page_pos).clamp(0.0, 1.0)
        }
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0.001, 0.050);
        let target = self.current_section as f32;
        // Exact spring parameters: stiffness = 230.0, damping = 19.0
        let force = 230.0 * (target - self.page_pos) - 19.0 * self.page_vel;
        self.page_vel += force * dt;
        self.page_pos += self.page_vel * dt;

        if (self.page_pos - target).abs() < 0.0015 && self.page_vel.abs() < 0.008 {
            self.page_pos = target;
            self.page_vel = 0.0;
        }
    }

    pub fn next(&mut self) {
        if self.total_sections() > 0 {
            let next_idx = (self.current_section + 1) % self.total_sections();
            self.set_section(next_idx);
        }
    }

    pub fn prev(&mut self) {
        if self.total_sections() > 0 {
            let prev_idx = if self.current_section == 0 {
                self.total_sections() - 1
            } else {
                self.current_section - 1
            };
            self.set_section(prev_idx);
        }
    }

    pub fn set_section(&mut self, index: usize) {
        if index < self.total_sections() && index != self.current_section {
            self.current_section = index;
            self.dots.set_index(index);
        }
    }

    pub fn snap_to_section(&mut self, index: usize) {
        if index < self.total_sections() {
            self.current_section = index;
            self.page_pos = index as f32;
            self.page_vel = 0.0;
            self.dots.set_index(index);
        }
    }

    pub fn snap_to_section_id(&mut self, sec_id: SectionId) {
        if let Some(pos) = self.active_sections.iter().position(|&s| s == sec_id) {
            self.snap_to_section(pos);
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
