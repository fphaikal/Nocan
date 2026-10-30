use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::{d2d::D2DContext, dwrite::DWriteContext};
use crate::graphics::icons::{self, Icon};
use crate::integrations::{ai_usage::{AiTab, format_tokens}, media::MediaTrackInfo, system_info::SystemStatus};
use crate::notch::carousel::SectionId;
use crate::notch::todo_editor::{FIELD_LEFT, FIELD_TOP, FIELD_HEIGHT, FIELD_RGB};
use crate::integrations::openmaus_activity::{ActivityKind, ActivityEvent, local_time, local_date, timestamp_ms};
use super::openmaus::{OpenMausTab, PAGE_SIZE as MAUS_PAGE_SIZE};
use super::{music::MusicSection, focus::{FocusSection, PomodoroMode}, water::WaterSection,
    shelf::ShelfSection, todo::{TodoSection, TodoEditing, PAGE_SIZE, page_start}, openmaus::OpenMausSection,
    ai_usage::AiUsageSection, saham::SahamSection};

#[derive(Clone, Copy)]
pub enum Action {
    Lyrics, Previous, PlayPause, Next,
    FocusMode(PomodoroMode), FocusToggle, FocusReset, FocusAddFive, FocusSkip,
    Drink, WaterGoal, WaterReminder,
    ShelfOpen(usize), ShelfClear, ShelfPage(usize),
    TodoToggle(usize), TodoAdd, TodoEdit(usize), TodoDelete(usize), TodoSave, TodoCancel,
    TodoPage(usize), TodoUndoDelete, TodoInputFocus,
    MausTab(OpenMausTab), MausPage(usize), MausUsageGroup(usize), MausNotifications, MausRefresh,
    Provider(AiTab), Stock(usize),
}

#[derive(Clone, Copy)]
pub struct Region { pub rect: D2D_RECT_F, pub action: Action }
impl Region {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.rect.left && x <= self.rect.right && y >= self.rect.top && y <= self.rect.bottom
    }
}

pub enum Page<'a> {
    Music(&'a MusicSection, &'a MediaTrackInfo, f32), Focus(&'a FocusSection, f32), Water(&'a WaterSection),
    Shelf(&'a ShelfSection, usize), Todo(&'a TodoSection, usize, Option<TodoEditing>, Option<&'a str>, bool, bool), OpenMaus(&'a OpenMausSection),
    Ai(&'a AiUsageSection, f32), Stocks(&'a SahamSection),
}

pub fn section_name(section: SectionId) -> &'static str {
    match section {
        SectionId::Music => "Music", SectionId::Focus => "Focus", SectionId::Water => "Water",
        SectionId::Shelf => "Files", SectionId::Todo => "Tasks", SectionId::OpenMaus => "OpenMaus",
        SectionId::AiUsage => "AI Usage", SectionId::Saham => "Stocks",
    }
}


struct Painter<'a> {
    d: &'a D2DContext,
    w: &'a DWriteContext,
    bounds: D2D_RECT_F,
    text: ID2D1SolidColorBrush,
    muted: ID2D1SolidColorBrush,
    accent: ID2D1SolidColorBrush,
    primary: ID2D1SolidColorBrush,
    panel: ID2D1SolidColorBrush,
    edge: ID2D1SolidColorBrush,
    regions: Vec<Region>,
}

impl<'a> Painter<'a> {
    fn new(d: &'a D2DContext, w: &'a DWriteContext, bounds: D2D_RECT_F, accent_mode:u32) -> windows::core::Result<Self> {
        let (r,g,b) = accent_color(accent_mode);
        let brush = |r, g, b, a| d.create_solid_brush(D2D1_COLOR_F { r, g, b, a });
        Ok(Self {
            d, w, bounds,
            text: brush(0.97, 0.98, 1.0, 1.0)?,
            muted: brush(0.64, 0.67, 0.72, 1.0)?,
            accent: brush(r,g,b,1.0)?,
            primary: brush(r*0.65,g*0.65,b*0.65,1.0)?,
            panel: brush(0.60, 0.67, 0.78, 0.13)?,
            edge: brush(0.75, 0.81, 0.90, 0.16)?,
            regions: vec![],
        })
    }

    fn rect(&self, x: f32, y: f32, width: f32, height: f32) -> D2D_RECT_F {
        D2D_RECT_F { left: self.bounds.left + x, top: self.bounds.top + y,
            right: self.bounds.left + x + width, bottom: self.bounds.top + y + height }
    }

    fn label(&self, text: &str, x: f32, y: f32, width: f32, height: f32, style: u8) {
        let font = match style {
            0 => &self.w.format_small, 1 => &self.w.format_sub,
            2 => &self.w.format_title, 3 => &self.w.format_big,
            _ => &self.w.format_regular,
        };
        self.w.draw_text(&self.d.context, text, self.rect(x,y,width,height),
            if style <= 1 { &self.muted } else { &self.text }, font);
    }

    fn center(&self, text: &str, x: f32, y: f32, width: f32, height: f32, small: bool) {
        self.w.draw_text(&self.d.context, text, self.rect(x,y,width,height),
            if small { &self.muted } else { &self.text },
            if small { &self.w.format_sub_center } else { &self.w.format_center });
    }

    fn panel(&self, x: f32, y: f32, width: f32, height: f32) {
        self.d.fill_rounded_rect(self.rect(x,y,width,height), 12.0, 12.0, &self.panel);
    }

    fn symbol(&mut self, icon: Icon, x: f32, y: f32, width: f32, selected: bool, action: Action) {
        let r = self.rect(x,y,width,28.0);
        if selected { self.d.fill_rounded_rect(r,14.0,14.0,&self.panel); }
        icons::draw(self.d, icon, (r.left+r.right)*0.5, (r.top+r.bottom)*0.5,
            18.0, if selected { &self.accent } else { &self.text });
        self.regions.push(Region { rect: r, action });
    }

    fn button(&mut self, text: &str, icon: Option<Icon>, x: f32, y: f32,
        width: f32, selected: bool, action: Action) {
        let r = self.rect(x,y,width,28.0);
        self.d.fill_rounded_rect(r,14.0,14.0,if selected { &self.primary } else { &self.panel });
        let mut caption = r;
        if let Some(icon) = icon {
            icons::draw(self.d,icon,r.left+13.0,(r.top+r.bottom)*0.5,12.0,&self.text);
            caption.left += 24.0;
            caption.right -= 4.0;
        }
        self.w.draw_text(&self.d.context,text,caption,&self.text,&self.w.format_button_center);
        self.regions.push(Region { rect: r, action });
    }

    fn bar(&self, x: f32, y: f32, width: f32, ratio: f32) {
        self.d.fill_rounded_rect(self.rect(x,y,width,3.0),1.5,1.5,&self.edge);
        if ratio > 0.0 {
            self.d.fill_rounded_rect(self.rect(x,y,width*ratio.clamp(0.0,1.0),3.0),1.5,1.5,&self.text);
        }
    }
}

pub fn section_icon(section: SectionId) -> Icon {
    match section {
        SectionId::Music => Icon::Music, SectionId::Focus => Icon::Clock,
        SectionId::Water => Icon::Drop, SectionId::Shelf => Icon::Folder,
        SectionId::Todo => Icon::Tasks, SectionId::OpenMaus => Icon::Bot,
        SectionId::AiUsage => Icon::Chip, SectionId::Saham => Icon::Chart,
    }
}

pub fn accent_color(mode:u32) -> (f32,f32,f32) {
    match mode { 2=>(0.95,0.43,0.37), 3=>(0.20,0.76,0.51), 4=>(0.15,0.74,0.84), 5=>(0.70,0.48,0.95), _=>(0.32,0.67,1.0) }
}

pub fn shell(d: &D2DContext, w: &DWriteContext, bounds: D2D_RECT_F,
    status: &SystemStatus, section: SectionId, cover_color: (f32, f32, f32), cover_mix: f32, accent_mode:u32, opacity:f32, timer: Option<&FocusSection>) {
    let Ok(p) = Painter::new(d,w,bounds,accent_mode) else { return; };
    let width = bounds.right-bounds.left;
    let accent = accent_color(accent_mode);
    let background = if cover_mix > 0.01 { cover_color } else { accent };
    let mix = if accent_mode == 0 { cover_mix } else { 1.0 };
    crate::graphics::ambient::draw_with_cover(d,bounds,32.0,0.24 * (opacity / 0.9),0.0,background,mix);
    d.draw_rounded_rect(p.rect(0.5,-32.0,width-1.0,bounds.bottom-bounds.top+31.5),
        32.0,32.0,&p.edge,0.6);
    icons::draw(d,section_icon(section),bounds.left+31.0,bounds.top+20.0,14.0,&p.muted);
    p.label(section_name(section),46.0,9.0,140.0,22.0,2);
    if let Some(timer) = timer {
        crate::notch::compact::draw_timer(d, w, p.rect(width - 164.0, 7.0, 118.0, 26.0), timer);
    } else {
        p.w.draw_text(&d.context,&status.date_str,p.rect(width-232.0,10.0,132.0,20.0), &p.muted,&w.format_small_center);
        p.w.draw_text(&d.context,&status.clock_str,p.rect(width-92.0,9.0,44.0,22.0), &p.text,&w.format_regular_semibold_right);
    }
    icons::power(d,status,bounds.left+width-32.0,bounds.top+20.0,16.0,&p.text,&p.muted);
}

pub fn draw(d: &D2DContext, w: &DWriteContext, bounds: D2D_RECT_F, page: Page<'_>, accent_mode:u32) -> Vec<Region> {
    let Ok(mut p) = Painter::new(d,w,bounds,accent_mode) else { return vec![]; };
    match page {
        Page::Music(section,track,morph) => music(&mut p,section,track,morph),
        Page::Focus(section,morph) => focus(&mut p,section,morph),
        Page::Water(section) => water(&mut p,section),
        Page::Shelf(section,offset) => shelf(&mut p,section,offset),
        Page::Todo(section,offset,editing,error,undo,focused) => todo(&mut p,section,offset,editing,error,undo,focused),
        Page::OpenMaus(section) => maus(&mut p,section),
        Page::Ai(section,morph) => ai(&mut p,section,morph),
        Page::Stocks(section) => stocks(&mut p,section),
    }
    p.regions
}

fn music(p: &mut Painter<'_>, s: &MusicSection, t: &MediaTrackInfo, morph: f32) {
    let lyrics = s.show_lyrics;
    let t_morph = morph.clamp(0.0, 1.0);
    let art = 80.0 - 24.0 * t_morph;
    let metadata = 24.0 + art + 16.0;
    let right = 424.0 + (220.0 - 424.0) * t_morph;
    let art_y = 44.0;
    let art_x = 24.0f32;

    p.panel(art_x, art_y, art, art);
    if let Some(bytes) = t.thumbnail_bgra.as_deref() {
        p.d.fill_rounded_rect_bitmap(p.rect(art_x, art_y, art, art), 12.0, 12.0, bytes, 62, 62);
    } else {
        icons::draw(p.d, Icon::Music, p.bounds.left + art_x + art * 0.5, p.bounds.top + art_y + art * 0.5, 28.0, &p.muted);
    }

    let title_y = 44.0;
    let artist_y = 70.0;
    p.label(if t.title.is_empty() { "No music playing" } else { &t.title },
        metadata, title_y, right - metadata, 24.0, 2);
    p.label(if t.artist.is_empty() { "Play a song to start" } else { &t.artist },
        metadata, artist_y, right - metadata, 20.0, 1);

    let bar_y = 96.0 + 10.0 * t_morph;
    let bar_shift = ((t_morph - 0.83) / 0.17).clamp(0.0, 1.0);
    let start = metadata + (24.0 - metadata) * bar_shift;
    p.bar(start, bar_y, right - start,
        if t.duration_ms > 0 { t.position_ms as f32 / t.duration_ms as f32 } else { 0.0 });
    p.label(&time_text(t.position_ms), start, bar_y + 6.0, 54.0, 14.0, 0);
    let duration = time_text(t.duration_ms);
    let duration_w = p.w.measure_text_width(&duration, &p.w.format_small) + 1.0;
    p.label(&duration, right - duration_w, bar_y + 6.0, duration_w, 14.0, 0);

    let center = (120.0 + 424.0) * 0.5 + (136.0 - 272.0) * t_morph;
    let by = 128.0;
    let lyrics_x = 48.0 - 24.0 * t_morph;
    p.symbol(Icon::Lyrics, lyrics_x, by + 4.0, 32.0, lyrics, Action::Lyrics);
    p.symbol(Icon::Previous, center - 64.0, by + 4.0, 32.0, false, Action::Previous);
    let play = p.rect(center - 18.0, by, 36.0, 36.0);
    p.d.fill_rounded_rect(play, 18.0, 18.0, &p.primary);
    icons::draw(p.d, if t.is_playing { Icon::Pause } else { Icon::Play },
        (play.left + play.right) * 0.5, (play.top + play.bottom) * 0.5, 18.0, &p.text);
    p.regions.push(Region { rect: play, action: Action::PlayPause });
    p.symbol(Icon::Next, center + 32.0, by + 4.0, 32.0, false, Action::Next);

    let lyrics_opacity = ((t_morph - 0.90) / 0.10).clamp(0.0, 1.0);
    if lyrics_opacity > 0.01 {
        let width = (p.bounds.right - p.bounds.left - 280.0).max(1.0);
        let pushed = p.d.push_clipped_opacity_layer(p.bounds.left, p.bounds.right - p.bounds.left,
            p.bounds.bottom - p.bounds.top, 32.0, 22.0, lyrics_opacity);
        if pushed {
        p.d.draw_line(p.bounds.left + 240.0, p.bounds.top + 44.0, p.bounds.left + 240.0, p.bounds.top + 164.0, &p.edge, 1.0, false);
        let (lines, status) = s.visible_lyrics(t);
        if lines.is_empty() { p.center(&status, 256.0, 64.0, width, 58.0, true); }
        for (row, (text, active)) in lines.iter().enumerate() {
            p.w.draw_text(&p.d.context, text, p.rect(256.0, 44.0 + row as f32 * 28.0, width, 26.0),
                if *active { &p.text } else { &p.muted },
                if *active { &p.w.format_lyric } else { &p.w.format_regular });
        }
        p.d.pop_layer();
        }
    }
}

fn focus(p: &mut Painter<'_>, s: &FocusSection, morph: f32) {
    // -------------------------------------------------------------
    // TIER 1: Mode Segmented Pills & Cycle Status (y: 38.0 - 58.0)
    // -------------------------------------------------------------
    let modes = [
        ("Focus", Icon::Target, PomodoroMode::Focus, 74.0f32),
        ("Break", Icon::Coffee, PomodoroMode::ShortBreak, 72.0f32),
        ("Long", Icon::Moon, PomodoroMode::LongBreak, 82.0f32),
    ];

    let positions = [(24.0f32, 74.0f32), (104.0, 72.0), (182.0, 82.0)];
    let pos = morph.clamp(0.0, 2.0);
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(2);
    let t = pos - lo as f32;
    let x = positions[lo].0 + (positions[hi].0 - positions[lo].0) * t;
    let width = positions[lo].1 + (positions[hi].1 - positions[lo].1) * t;
    let highlight = p.rect(x, 38.0, width, 22.0);
    p.d.fill_rounded_rect(highlight, 11.0, 11.0, &p.panel);
    p.d.draw_rounded_rect(highlight, 11.0, 11.0, &p.edge, 0.8);
    p.d.fill_ellipse(highlight.left + 11.0, (highlight.top + highlight.bottom) * 0.5, 2.2, 2.2, &p.accent);
    let mut mode_x = 24.0f32;
    for (label, icon, mode, width) in modes {
        let r = p.rect(mode_x, 38.0, width, 22.0);
        let is_active = s.mode == mode;

        if is_active {
            icons::draw(p.d, icon, r.left + 23.0, (r.top + r.bottom) * 0.5, 11.0, &p.accent);
            let text_rect = D2D_RECT_F {
                left: r.left + 32.0,
                top: r.top,
                right: r.right - 4.0,
                bottom: r.bottom,
            };
            p.w.draw_text(&p.d.context, label, text_rect, &p.text, &p.w.format_button);
        } else {
            icons::draw(p.d, icon, r.left + 14.0, (r.top + r.bottom) * 0.5, 10.0, &p.muted);
            let text_rect = D2D_RECT_F {
                left: r.left + 24.0,
                top: r.top,
                right: r.right - 4.0,
                bottom: r.bottom,
            };
            p.w.draw_text(&p.d.context, label, text_rect, &p.muted, &p.w.format_button);
        }

        p.regions.push(Region { rect: r, action: Action::FocusMode(mode) });
        mode_x += width + 6.0;
    }

    // Cycle Tracker on top right: Sesi X/4 with 4 micro-pills
    let cycle = s.sessions_completed % 4;
    p.w.draw_text(
        &p.d.context,
        &format!("Session {}/4", cycle + 1),
        p.rect(296.0, 39.0, 60.0, 20.0),
        &p.muted,
        &p.w.format_small_center,
    );
    for idx in 0..4 {
        let dot_x = 362.0 + idx as f32 * 14.0;
        let dot = p.rect(dot_x, 47.0, 10.0, 3.5);
        if idx < cycle {
            p.d.fill_rounded_rect(dot, 1.75, 1.75, &p.accent);
        } else if idx == cycle && s.is_running {
            p.d.fill_rounded_rect(dot, 1.75, 1.75, &p.accent);
        } else {
            p.d.fill_rounded_rect(dot, 1.75, 1.75, &p.edge);
        }
    }

    // -------------------------------------------------------------
    // TIER 2: Luminous Hero Ring + Big Digits + Modern Island HUD
    // -------------------------------------------------------------
    let ring_cx = p.bounds.left + 62.0;
    let ring_cy = p.bounds.top + 107.0;
    let ring_r = 28.0;

    // Ambient backlight glow when running
    if s.is_running {
        let glow_color = match s.mode {
            PomodoroMode::Focus => D2D1_COLOR_F { r: 0.23, g: 0.51, b: 0.96, a: 0.24 },
            PomodoroMode::ShortBreak => D2D1_COLOR_F { r: 0.18, g: 0.78, b: 0.55, a: 0.24 },
            PomodoroMode::LongBreak => D2D1_COLOR_F { r: 0.65, g: 0.35, b: 0.95, a: 0.24 },
        };
        if let Ok(glow) = p.d.create_radial_gradient(
            (ring_cx, ring_cy),
            38.0,
            38.0,
            &[
                (0.0, glow_color),
                (0.7, D2D1_COLOR_F { a: 0.05, ..glow_color }),
                (1.0, D2D1_COLOR_F { a: 0.0, ..glow_color }),
            ],
        ) {
            p.d.fill_ellipse_radial(ring_cx, ring_cy, 38.0, 38.0, &glow);
        }
    }

    let remaining_ratio = s.remaining_secs as f32 / s.total_secs.max(1) as f32;
    let elapsed_ratio = 1.0 - remaining_ratio.clamp(0.0, 1.0);

    // Luminous progress ring
    p.d.draw_progress_ring(ring_cx, ring_cy, ring_r, 3.2, remaining_ratio, &p.edge, &p.accent);

    // Center icon inside the ring
    let center_icon = match s.mode {
        PomodoroMode::Focus => Icon::Target,
        PomodoroMode::ShortBreak => Icon::Coffee,
        PomodoroMode::LongBreak => Icon::Moon,
    };
    icons::draw(p.d, center_icon, ring_cx, ring_cy, 15.0, if s.is_running { &p.text } else { &p.muted });

    // Center zone: Countdown digits (Plus Jakarta Sans)
    let time_str = format!("{:02}:{:02}", s.remaining_secs / 60, s.remaining_secs % 60);
    p.w.draw_text(
        &p.d.context,
        &time_str,
        p.rect(106.0, 67.0, 160.0, 36.0),
        &p.text,
        &p.w.format_big,
    );

    // Status / ambient subtitle
    let break_status = format!("Ready for a {} min break", s.break_mins);
    let status_str = if s.is_running {
        match s.mode {
            PomodoroMode::Focus => "Stay focused • Session active",
            PomodoroMode::ShortBreak => "Short break • Relax",
            PomodoroMode::LongBreak => "Long break • Recharge",
        }
    } else if s.remaining_secs < s.total_secs {
        "Paused • Tap to resume"
    } else {
        match s.mode {
            PomodoroMode::Focus => "Ready to focus",
            PomodoroMode::ShortBreak => break_status.as_str(),
            PomodoroMode::LongBreak => "Ready for a long break",
        }
    };
    p.w.draw_text(
        &p.d.context,
        status_str,
        p.rect(106.0, 103.0, 170.0, 16.0),
        &p.muted,
        &p.w.format_sub,
    );

    // Linear progress track
    let bar_w = 160.0f32;
    p.d.fill_rounded_rect(p.rect(106.0, 126.0, bar_w, 3.0), 1.5, 1.5, &p.edge);
    if elapsed_ratio > 0.0 {
        p.d.fill_rounded_rect(
            p.rect(106.0, 126.0, (bar_w * elapsed_ratio).clamp(3.0, bar_w), 3.0),
            1.5,
            1.5,
            &p.accent,
        );
    }
    p.w.draw_text(
        &p.d.context,
        &format!("{:.0}%", elapsed_ratio * 100.0),
        p.rect(106.0 + bar_w - 30.0, 131.0, 30.0, 14.0),
        &p.muted,
        &p.w.format_small,
    );

    // Right cluster: Tactical action controls
    let primary_rect = p.rect(288.0, 68.0, 136.0, 36.0);
    if s.is_running {
        p.d.fill_rounded_rect(primary_rect, 18.0, 18.0, &p.panel);
        p.d.draw_rounded_rect(primary_rect, 18.0, 18.0, &p.accent, 1.2);
        icons::draw(p.d, Icon::Pause, primary_rect.left + 22.0, (primary_rect.top + primary_rect.bottom) * 0.5, 13.0, &p.text);
        p.w.draw_text(
            &p.d.context,
            "Pause",
            D2D_RECT_F {
                left: primary_rect.left + 36.0,
                top: primary_rect.top,
                right: primary_rect.right - 8.0,
                bottom: primary_rect.bottom,
            },
            &p.text,
            &p.w.format_button_center,
        );
    } else {
        p.d.fill_rounded_rect(primary_rect, 18.0, 18.0, &p.primary);
        icons::draw(p.d, Icon::Play, primary_rect.left + 22.0, (primary_rect.top + primary_rect.bottom) * 0.5, 13.0, &p.text);
        let btn_label = if s.remaining_secs == s.total_secs { "Start" } else { "Resume" };
        p.w.draw_text(
            &p.d.context,
            btn_label,
            D2D_RECT_F {
                left: primary_rect.left + 36.0,
                top: primary_rect.top,
                right: primary_rect.right - 8.0,
                bottom: primary_rect.bottom,
            },
            &p.text,
            &p.w.format_button_center,
        );
    }
    p.regions.push(Region { rect: primary_rect, action: Action::FocusToggle });

    // Secondary row: Reset (↺), +5m, Skip (⏭)
    let btn_y = 114.0f32;
    let btn_h = 26.0f32;

    // Reset button
    let reset_rect = p.rect(288.0, btn_y, 38.0, btn_h);
    p.d.fill_rounded_rect(reset_rect, 13.0, 13.0, &p.panel);
    icons::draw(p.d, Icon::Reset, (reset_rect.left + reset_rect.right) * 0.5, (reset_rect.top + reset_rect.bottom) * 0.5, 12.0, &p.muted);
    p.regions.push(Region { rect: reset_rect, action: Action::FocusReset });

    // +5m button
    let add_rect = p.rect(332.0, btn_y, 48.0, btn_h);
    p.d.fill_rounded_rect(add_rect, 13.0, 13.0, &p.panel);
    p.w.draw_text(&p.d.context, "+5m", add_rect, &p.text, &p.w.format_small_center);
    p.regions.push(Region { rect: add_rect, action: Action::FocusAddFive });

    // Skip button
    let skip_rect = p.rect(386.0, btn_y, 38.0, btn_h);
    p.d.fill_rounded_rect(skip_rect, 13.0, 13.0, &p.panel);
    icons::draw(p.d, Icon::Next, (skip_rect.left + skip_rect.right) * 0.5, (skip_rect.top + skip_rect.bottom) * 0.5, 12.0, &p.muted);
    p.regions.push(Region { rect: skip_rect, action: Action::FocusSkip });
}

fn water(p: &mut Painter<'_>, s: &WaterSection) {
    p.label(&format!("{:.1} / {:.1} L",s.intake_ml as f32/1000.0,s.goal_ml as f32/1000.0),
        38.0,41.0,194.0,36.0,3);
    let percent = s.intake_ml as f32/s.goal_ml.max(1) as f32*100.0;
    p.label(&format!("Today · {:.0}% of goal",percent),38.0,81.0,194.0,19.0,1);
    p.button("+250 ml",Some(Icon::Drop),38.0,116.0,92.0,true,Action::Drink);
    p.button("Goal",Some(Icon::Sliders),138.0,116.0,82.0,false,Action::WaterGoal);
    p.label("Hourly",244.0,36.0,95.0,20.0,1);
    let switch = p.rect(374.0,35.0,36.0,20.0);
    p.d.fill_rounded_rect(switch,10.0,10.0,if s.reminder_on { &p.primary } else { &p.edge });
    let knob = switch.left+if s.reminder_on { 26.0 } else { 10.0 };
    p.d.fill_ellipse(knob,switch.top+10.0,8.0,8.0,&p.text);
    p.regions.push(Region { rect:switch, action:Action::WaterReminder });
    let max = s.hourly_ml.iter().copied().max().unwrap_or(0).max(400) as f32;
    for (i,amount) in s.hourly_ml.iter().enumerate() {
        let x = 245.0+i as f32*20.5;
        p.d.fill_rounded_rect(p.rect(x,65.0,10.0,57.0),5.0,5.0,&p.panel);
        let h = 57.0*(*amount as f32/max).clamp(0.0,1.0);
        if h>0.0 { p.d.fill_rounded_rect(p.rect(x,122.0-h,10.0,h),5.0,5.0,&p.accent); }
        p.center(&format!("{:02}",i+9),x-5.0,127.0,20.0,17.0,true);
    }
    if let Some(error)=&s.storage_error {
        if let Ok(brush)=p.d.create_solid_brush(D2D1_COLOR_F{r:1.0,g:0.60,b:0.57,a:1.0}) {
            p.w.draw_text(&p.d.context,error,p.rect(38.0,148.0,372.0,18.0),&brush,&p.w.format_small);
        }
    }
}

fn shelf(p: &mut Painter<'_>, s: &ShelfSection, offset: usize) {
    if s.items.is_empty() {
        let r = p.rect(38.0,42.0,372.0,100.0);
        p.d.draw_dashed_rounded_rect(r,16.0,16.0,&p.edge,1.2);
        icons::draw(p.d,Icon::Folder,p.bounds.left+224.0,68.0,27.0,&p.muted);
        p.center("Drop files here",48.0,87.0,352.0,23.0,false);
        p.center("Drag to the notch · Click to open",48.0,111.0,352.0,20.0,true);
        return;
    }
    p.label(&format!("{} files",s.items.len()),38.0,34.0,112.0,23.0,1);
    if offset>0 { p.symbol(Icon::Previous,183.0,32.0,28.0,false,Action::ShelfPage(offset.saturating_sub(3))); }
    if offset+3<s.items.len() { p.symbol(Icon::Next,218.0,32.0,28.0,false,Action::ShelfPage(offset+3)); }
    p.button("Clear",None,320.0,32.0,90.0,false,Action::ShelfClear);
    for (i,item) in s.items.iter().enumerate().skip(offset).take(3) {
        let x = 38.0+(i-offset) as f32*128.0;
        icons::draw(p.d,if item.is_dir { Icon::Folder } else if item.is_image { Icon::Image } else { Icon::File },
            p.bounds.left+x+58.0,87.0,34.0,&p.accent);
        p.center(&item.name,x,109.0,116.0,19.0,false);
        p.center(&item.size_label,x,130.0,116.0,16.0,true);
        p.regions.push(Region { rect:p.rect(x,64.0,116.0,84.0), action:Action::ShelfOpen(i) });
    }
}

fn todo(p: &mut Painter<'_>, s: &TodoSection, offset:usize, editing:Option<TodoEditing>, error:Option<&str>, undo:bool, focused:bool) {
    let width=p.bounds.right-p.bounds.left;
    let done = s.items.iter().filter(|i|i.completed).count();
    p.label(&format!("{} / {} done",done,s.items.len()),38.0,43.0,120.0,24.0,1);
    if let Some(editing)=editing {
        let brush=|r:u8,g:u8,b:u8,a:f32|p.d.create_solid_brush(D2D1_COLOR_F{r:r as f32/255.0,g:g as f32/255.0,b:b as f32/255.0,a});
        if let (Ok(card),Ok(edge),Ok(field),Ok(focus),Ok(invalid),Ok(button),Ok(primary))=(
            brush(28,28,30,1.0),brush(255,255,255,0.14),brush(FIELD_RGB.0,FIELD_RGB.1,FIELD_RGB.2,1.0),
            brush(10,132,255,1.0),brush(255,105,97,1.0),brush(58,58,61,1.0),brush(0,113,227,1.0)) {
            let card_rect=p.rect(38.0,76.0,width-76.0,112.0);
            p.d.fill_rounded_rect(card_rect,12.0,12.0,&card);
            p.d.draw_rounded_rect(card_rect,12.0,12.0,&edge,0.6);
            p.label(if editing==TodoEditing::New { "New task" } else { "Edit task" },54.0,82.0,width-108.0,20.0,2);
            let field_rect=p.rect(FIELD_LEFT,FIELD_TOP,width-2.0*FIELD_LEFT,FIELD_HEIGHT);
            p.d.fill_rounded_rect(field_rect,7.0,7.0,&field);
            p.d.draw_rounded_rect(field_rect,7.0,7.0,&edge,1.0);
            if focused || error.is_some() {
                let ring=D2D_RECT_F{left:field_rect.left-1.0,top:field_rect.top-1.0,right:field_rect.right+1.0,bottom:field_rect.bottom+1.0};
                p.d.draw_rounded_rect(ring,8.0,8.0,if error.is_some(){&invalid}else{&focus},2.0);
            }
            p.regions.push(Region{rect:field_rect,action:Action::TodoInputFocus});
            for (text,x,w,fill,action) in [("Cancel",width-210.0,72.0,&button,Action::TodoCancel),("Save",width-130.0,76.0,&primary,Action::TodoSave)] {
                let rect=p.rect(x,152.0,w,26.0);
                p.d.fill_rounded_rect(rect,6.0,6.0,fill);
                p.d.draw_rounded_rect(rect,6.0,6.0,&edge,0.6);
                p.w.draw_text(&p.d.context,text,rect,&p.text,&p.w.format_button_center);
                p.regions.push(Region{rect,action});
            }
            p.w.draw_text(&p.d.context,error.unwrap_or("Enter to save · Esc to cancel"),p.rect(54.0,190.0,width-108.0,18.0),
                if error.is_some(){&invalid}else{&p.muted},&p.w.format_small);
        }
        return;
    }
    p.button("New task",Some(Icon::Plus),width-142.0,42.0,104.0,true,Action::TodoAdd);
    if undo { p.button("Undo",Some(Icon::Reset),164.0,42.0,96.0,false,Action::TodoUndoDelete); }
    let offset=page_start(offset,s.items.len());
    if s.items.is_empty() {
        icons::draw(p.d,Icon::Tasks,p.bounds.left+width*0.5,101.0,24.0,&p.accent);
        p.center("No tasks yet",38.0,120.0,width-76.0,25.0,false);
        p.center("Add your first task",38.0,146.0,width-76.0,20.0,true);
    }
    for (i,item) in s.items.iter().skip(offset).take(PAGE_SIZE).enumerate() {
        let y = 76.0+i as f32*34.0;
        p.panel(38.0,y,width-76.0,30.0);
        let cx = p.bounds.left+48.0;
        p.d.draw_ellipse(cx,y+15.0,7.0,7.0,if item.completed { &p.accent } else { &p.muted },1.3);
        if item.completed {
            p.d.fill_ellipse(cx,y+15.0,7.0,7.0,&p.accent);
            p.d.draw_polyline(&[(cx-4.0,y+15.0),(cx-1.0,y+18.0),(cx+4.0,y+12.0)],&p.text,1.5,true);
        }
        p.label(&item.title,67.0,y+2.0,width-183.0,26.0,if item.completed { 1 } else { 4 });
        p.regions.push(Region { rect:p.rect(38.0,y,width-140.0,30.0), action:Action::TodoToggle(item.id) });
        p.symbol(Icon::Pencil,width-100.0,y+1.0,28.0,false,Action::TodoEdit(item.id));
        p.symbol(Icon::Trash,width-70.0,y+1.0,28.0,false,Action::TodoDelete(item.id));
    }
    if let Some(error)=error { p.label(error,38.0,182.0,width-76.0,20.0,0); }
    else if s.items.len()>PAGE_SIZE {
        if offset>0 { p.symbol(Icon::Previous,width*0.5-65.0,174.0,28.0,false,Action::TodoPage(offset-PAGE_SIZE)); }
        p.center(&format!("{} / {}",offset/PAGE_SIZE+1,s.items.len().div_ceil(PAGE_SIZE)),width*0.5-31.0,176.0,62.0,24.0,true);
        if offset+PAGE_SIZE<s.items.len() { p.symbol(Icon::Next,width*0.5+37.0,174.0,28.0,false,Action::TodoPage(offset+PAGE_SIZE)); }
    }
}

fn maus(p: &mut Painter<'_>, s: &OpenMausSection) {
    let width=p.bounds.right-p.bounds.left;
    for (i,(tab,label)) in [(OpenMausTab::Activity,"Activity"),(OpenMausTab::Schedule,"Schedule"),(OpenMausTab::Usage,"Usage")].iter().enumerate() {
        let x=38.0+i as f32*100.0;
        let rect=p.rect(x,38.0,96.0,26.0);
        if s.tab==*tab {p.d.fill_rounded_rect(rect,7.0,7.0,&p.panel);p.d.draw_rounded_rect(rect,7.0,7.0,&p.edge,0.6);}
        let caption=if *tab==OpenMausTab::Activity && s.unread>0 {format!("Activity · {}",s.unread.min(99))}else{label.to_string()};
        p.center(&caption,x,38.0,96.0,26.0,s.tab!=*tab);
        p.regions.push(Region{rect,action:Action::MausTab(*tab)});
    }
    let state=if s.activity.updated_at_ms==0 {"Loading"} else if s.activity.sync_error.is_some(){"Check data"} else if s.activity.server_online {"Connected"} else {"Offline"};
    let color=if s.activity.sync_error.is_some(){(1.0,0.70,0.32)}else if s.activity.server_online{(0.32,0.83,0.60)}else{(0.60,0.63,0.69)};
    if let Ok(dot)=p.d.create_solid_brush(D2D1_COLOR_F{r:color.0,g:color.1,b:color.2,a:1.0}) {
        p.d.fill_ellipse(p.bounds.left+width-123.0,51.0,3.0,3.0,&dot);
    }
    p.label(state,width-114.0,41.0,80.0,20.0,1);
    match s.tab {
        OpenMausTab::Activity=>{
            let title=if s.activity.updated_at_ms==0 {"Loading activity…".into()}
                else if !s.activity.source_found {"OpenMaus not found".into()}
                else if !s.activity.server_online {"OpenMaus server is offline".into()}
                else if s.activity.running_agents+s.activity.running_automations==0 {"No work in progress".into()}
                else {format!("{} agents · {} automations running",s.activity.running_agents,s.activity.running_automations)};
            p.label(&title,38.0,72.0,width-144.0,20.0,2);
            p.label(s.activity.sync_error.as_deref().unwrap_or(if s.activity.source_found {"Latest activity from OpenMaus Bot"}else{"Open OpenMaus Bot to monitor activity."}),38.0,93.0,width-144.0,15.0,0);
            p.symbol(Icon::Reset,width-102.0,74.0,28.0,false,Action::MausRefresh);
            p.symbol(Icon::Bell,width-68.0,74.0,28.0,s.notifications_enabled,Action::MausNotifications);
            if !s.notifications_enabled {
                p.d.draw_line(p.bounds.left+width-61.0,95.0,p.bounds.left+width-47.0,81.0,&p.muted,1.5,true);
            }
            for (i,event) in s.activity.events.iter().skip(s.offset).take(MAUS_PAGE_SIZE).enumerate() {
                let y=112.0+i as f32*34.0;
                activity_symbol(p,event,51.0,y+14.0,14.0);
                p.label(&event.title,69.0,y,width-202.0,18.0,4);
                p.label(&event.detail,69.0,y+17.0,width-140.0,15.0,0);
                let time=event_time(event.at_ms);
                p.w.draw_text(&p.d.context,&time,p.rect(width-122.0,y,84.0,18.0),&p.muted,&p.w.format_regular_right);
            }
            if s.activity.events.is_empty() && s.activity.source_found && s.activity.updated_at_ms!=0 {
                p.center("No activity yet",38.0,126.0,width-76.0,20.0,false);
                p.center("Agent replies and automations will appear here.",38.0,148.0,width-76.0,18.0,true);
            }
        },
        OpenMausTab::Schedule=>{
            let enabled=s.activity.routines.iter().filter(|r|r.enabled).count();
            p.label(&format!("{} active automations",enabled),38.0,72.0,width-76.0,20.0,2);
            p.label("Schedules saved in OpenMaus Bot",38.0,93.0,width-76.0,15.0,0);
            for (i,routine) in s.activity.routines.iter().skip(s.offset).take(MAUS_PAGE_SIZE).enumerate() {
                let y=112.0+i as f32*34.0;
                icons::draw(p.d,Icon::Clock,p.bounds.left+51.0,y+14.0,14.0,if routine.enabled{&p.accent}else{&p.muted});
                p.label(&routine.name,69.0,y,width-246.0,18.0,4);
                p.label(&format!("{} · {}",routine.bot_name,routine.schedule),69.0,y+17.0,width-140.0,15.0,0);
                let next=if !routine.enabled {"Disabled".into()}else if let Some(at)=routine.next_run_at {
                    if at<timestamp_ms(){"Overdue".into()}else{event_time(at)}
                }else{"Not scheduled".into()};
                p.w.draw_text(&p.d.context,&next,p.rect(width-166.0,y,128.0,18.0),&p.muted,&p.w.format_regular_right);
            }
            if s.activity.routines.is_empty() {
                p.center("No automation schedules yet",38.0,126.0,width-76.0,22.0,false);
                p.center("Create a schedule in OpenMaus Bot to see it here.",38.0,150.0,width-76.0,18.0,true);
            }
        },
        OpenMausTab::Usage=>{
            maus_usage(p,s);
        },
    }
    let count=s.row_count();
    if count>MAUS_PAGE_SIZE && s.tab!=OpenMausTab::Usage {
        if s.offset>0 {p.symbol(Icon::Previous,width*0.5-65.0,174.0,28.0,false,Action::MausPage(s.offset-MAUS_PAGE_SIZE));}
        p.center(&format!("{} / {}",s.offset/MAUS_PAGE_SIZE+1,count.div_ceil(MAUS_PAGE_SIZE)),width*0.5-31.0,176.0,62.0,24.0,true);
        if s.offset+MAUS_PAGE_SIZE<count {p.symbol(Icon::Next,width*0.5+37.0,174.0,28.0,false,Action::MausPage(s.offset+MAUS_PAGE_SIZE));}
    }
}

fn maus_usage(p:&mut Painter<'_>,s:&OpenMausSection) {
    let width=p.bounds.right-p.bounds.left;
    let r=&s.report;
    let list_x=208.0;
    let list_w=(width-list_x-38.0).max(180.0);
    p.label("Today's tokens",38.0,75.0,142.0,17.0,1);
    let tokens=if r.total_turns>0 {format_tokens(r.today_tokens)}else{"—".into()};
    p.label(&tokens,36.0,91.0,148.0,39.0,3);
    p.label(&format!("{} turns today",r.today_turns),38.0,130.0,146.0,18.0,1);
    p.d.draw_line(p.bounds.left+38.0,p.bounds.top+157.0,p.bounds.left+172.0,p.bounds.top+157.0,&p.edge,0.7,false);
    p.label("Recorded cost · total",38.0,164.0,148.0,16.0,0);
    let cost=if r.cost_reported {format!("${:.3}",r.total_cost_usd)}else{"—".into()};
    p.label(&cost,38.0,180.0,146.0,23.0,2);
    p.d.draw_line(p.bounds.left+190.0,p.bounds.top+77.0,p.bounds.left+190.0,p.bounds.top+199.0,&p.edge,0.7,false);

    let brush=|r:f32,g:f32,b:f32,a:f32|p.d.create_solid_brush(D2D1_COLOR_F{r,g,b,a});
    let (Ok(track),Ok(selected),Ok(group),Ok(icon_bg))=(brush(0.16,0.17,0.19,1.0),brush(0.31,0.32,0.35,1.0),brush(0.11,0.12,0.14,0.95),brush(0.25,0.29,0.35,0.65))else{return;};
    let selector=p.rect(list_x,76.0,list_w,26.0);
    p.d.fill_rounded_rect(selector,7.0,7.0,&track);
    p.d.draw_rounded_rect(selector,7.0,7.0,&p.edge,0.6);
    let segment=(list_w-4.0)/3.0;
    for (i,label) in ["Bot","Model","Provider"].iter().enumerate() {
        let x=list_x+2.0+i as f32*segment;
        let rect=p.rect(x,78.0,segment,22.0);
        if s.usage_group==i {
            p.d.fill_rounded_rect(rect,5.0,5.0,&selected);
            p.d.draw_rounded_rect(rect,5.0,5.0,&p.edge,0.6);
        }
        p.w.draw_text(&p.d.context,label,rect,if s.usage_group==i{&p.text}else{&p.muted},&p.w.format_button_center);
        p.regions.push(Region{rect,action:Action::MausUsageGroup(i)});
    }
    p.label("Total tokens",list_x,106.0,list_w-92.0,18.0,0);
    let items=s.usage_items();
    if items.len()>MAUS_PAGE_SIZE {
        p.center(&format!("{} / {}",s.offset/MAUS_PAGE_SIZE+1,items.len().div_ceil(MAUS_PAGE_SIZE)),width-92.0,104.0,30.0,22.0,true);
        for (x,previous,available,offset) in [(width-116.0,true,s.offset>0,s.offset.saturating_sub(MAUS_PAGE_SIZE)),(width-62.0,false,s.offset+MAUS_PAGE_SIZE<items.len(),s.offset+MAUS_PAGE_SIZE)] {
            if available {
                let rect=p.rect(x,104.0,24.0,22.0);
                let cx=(rect.left+rect.right)*0.5;let cy=(rect.top+rect.bottom)*0.5;
                let sign=if previous{-1.0}else{1.0};
                p.d.draw_polyline(&[(cx-2.0*sign,cy-3.5),(cx+2.0*sign,cy),(cx-2.0*sign,cy+3.5)],&p.text,1.3,true);
                p.regions.push(Region{rect,action:Action::MausPage(offset)});
            }
        }
    }
    let visible=items.len().saturating_sub(s.offset).min(MAUS_PAGE_SIZE);
    let group_rect=p.rect(list_x,128.0,list_w,visible.max(1) as f32*38.0);
    p.d.fill_rounded_rect(group_rect,9.0,9.0,&group);
    p.d.draw_rounded_rect(group_rect,9.0,9.0,&p.edge,0.6);
    if items.is_empty() {
        p.center("No usage recorded yet",list_x+10.0,132.0,list_w-20.0,28.0,true);
    }
    for (i,item) in items.iter().skip(s.offset).take(MAUS_PAGE_SIZE).enumerate() {
        let y=128.0+i as f32*38.0;
        let icon=match s.usage_group {1=>Icon::Chip,2=>Icon::Plug,_=>Icon::Bot};
        p.d.fill_rounded_rect(p.rect(list_x+10.0,y+10.0,20.0,20.0),5.0,5.0,&icon_bg);
        icons::draw(p.d,icon,p.bounds.left+list_x+20.0,p.bounds.top+y+20.0,12.0,&p.text);
        let name=if s.usage_group==1 {clean_model_name(&item.name)}else{item.name.clone()};
        p.label(&name,list_x+40.0,y+3.0,list_w-122.0,18.0,4);
        p.w.draw_text(&p.d.context,&format_tokens(item.total_tokens),p.rect(width-112.0,y+3.0,62.0,18.0),&p.text,&p.w.format_regular_semibold_right);
        let cost=if item.cost_reported {format!("${:.3}",item.cost_usd)}else{"—".into()};
        p.label(&format!("{} turns · cost {}",item.turns,cost),list_x+40.0,y+20.0,list_w-52.0,15.0,0);
        if i+1<visible {p.d.draw_line(p.bounds.left+list_x+40.0,p.bounds.top+y+38.0,p.bounds.left+width-50.0,p.bounds.top+y+38.0,&p.edge,0.6,false);}
    }
}

fn event_time(at:u64)->String {
    let time=local_time(at);
    let date=local_date(at);
    if date==local_date(timestamp_ms()) {time}else{format!("{} {}",date.get(5..).unwrap_or(&date),time)}
}

pub fn activity_color(kind:&ActivityKind)->(f32,f32,f32) {
    match kind {
        ActivityKind::AgentCompleted|ActivityKind::AutomationCompleted=>(0.32,0.83,0.60),
        ActivityKind::AgentFailed|ActivityKind::AutomationFailed=>(1.0,0.42,0.40),
        ActivityKind::NeedsAttention=>(1.0,0.72,0.32),
        _=>(0.32,0.67,1.0),
    }
}

fn activity_symbol(p:&Painter<'_>,event:&ActivityEvent,x:f32,y:f32,size:f32) {
    let (r,g,b)=activity_color(&event.kind);
    if let Ok(brush)=p.d.create_solid_brush(D2D1_COLOR_F{r,g,b,a:1.0}) {
        let icon=match event.kind {ActivityKind::AutomationStarted|ActivityKind::AutomationCompleted|ActivityKind::AutomationFailed=>Icon::Clock,ActivityKind::NeedsAttention=>Icon::Bell,_=>Icon::Bot};
        icons::draw(p.d,icon,p.bounds.left+x,p.bounds.top+y,size,&brush);
    }
}

fn ai(p: &mut Painter<'_>, s: &AiUsageSection, morph: f32) {
    let tabs = [
        (AiTab::Antigravity, "Gemini", include_bytes!("../../assets/logos/gemini.bgra") as &[u8]),
        (AiTab::Claude, "Claude", include_bytes!("../../assets/logos/claude.bgra") as &[u8]),
        (AiTab::Codex, "Codex", include_bytes!("../../assets/logos/codex.bgra") as &[u8]),
        (AiTab::OpenCode, "OpenCode", include_bytes!("../../assets/logos/opencode.bgra") as &[u8]),
    ];
    if !s.enabled.iter().any(|enabled| *enabled) {
        p.center("Enable a provider in Settings",24.0,72.0,400.0,60.0,true);
        return;
    }
    let highlight_x = 24.0 + morph.clamp(0.0, 3.0) * 102.0;
    let highlight = p.rect(highlight_x, 36.0, 94.0, 26.0);
    p.d.fill_rounded_rect(highlight, 8.0, 8.0, &p.panel);
    p.d.draw_rounded_rect(highlight, 8.0, 8.0, &p.edge, 1.0);
    p.d.fill_rounded_rect(p.rect(highlight_x + 10.0, 60.0, 74.0, 2.0), 1.0, 1.0, &p.accent);
    for (i, (tab, name, logo)) in tabs.iter().enumerate() {
        if !s.enabled[i] { continue; }
        let x = 24.0 + i as f32 * 102.0;
        let rect = p.rect(x, 36.0, 94.0, 26.0);
        let active = s.active_tab == *tab;
        p.d.fill_rounded_rect_bitmap(p.rect(x + 9.0, 42.0, 14.0, 14.0), 2.0, 2.0, logo, 48, 48);
        p.w.draw_text(&p.d.context, name, p.rect(x + 29.0, 36.0, 62.0, 26.0),
            if active { &p.text } else { &p.muted }, &p.w.format_sub);
        p.regions.push(Region { rect, action: Action::Provider(*tab) });
    }

    let data = match s.active_tab {
        AiTab::Antigravity => &s.summary.antigravity,
        AiTab::Claude => &s.summary.claude,
        AiTab::Codex => &s.summary.codex,
        AiTab::OpenCode => &s.summary.opencode,
    };
    p.label("Today's tokens", 28.0, 72.0, 160.0, 17.0, 1);
    p.w.draw_text(&p.d.context, &format_tokens(data.today_tokens),
        p.rect(27.0, 91.0, 166.0, 36.0), &p.text, &p.w.format_big);

    let status = if data.status_text.trim().is_empty() { "No activity yet" } else { &data.status_text };
    let status_color = if data.is_active {
        D2D1_COLOR_F { r: 0.25, g: 0.82, b: 0.58, a: 1.0 }
    } else {
        D2D1_COLOR_F { r: 0.55, g: 0.59, b: 0.65, a: 1.0 }
    };
    if let Ok(dot) = p.d.create_solid_brush(status_color) {
        p.d.fill_ellipse(p.bounds.left + 31.0, p.bounds.top + 145.0, 2.5, 2.5, &dot);
    }
    let status_text = if data.today_turns > 0 {
        format!("{} · {} req", status, data.today_turns)
    } else { status.to_string() };
    p.w.draw_text(&p.d.context, &status_text, p.rect(40.0, 136.0, 156.0, 18.0),
        &p.muted, &p.w.format_sub);

    p.d.draw_line(p.bounds.left + 210.0, p.bounds.top + 74.0,
        p.bounds.left + 210.0, p.bounds.top + 154.0, &p.edge, 1.0, false);
    p.label("Model", 228.0, 72.0, 190.0, 16.0, 1);
    let model = if data.active_model.trim().is_empty() { "No model yet".to_string() }
        else { clean_model_name(&data.active_model) };
    p.w.draw_text(&p.d.context, &model, p.rect(228.0, 88.0, 196.0, 20.0),
        &p.text, &p.w.format_regular_bold);

    let detail = if data.input_tokens_today > 0 || data.output_tokens_today > 0 {
        format!("Input {}  ·  Output {}", format_tokens(data.input_tokens_today), format_tokens(data.output_tokens_today))
    } else if data.cached_tokens_today > 0 {
        format!("Cached {} tokens", format_tokens(data.cached_tokens_today))
    } else {
        format!("{} turns total", data.total_turns)
    };
    p.label(&detail, 228.0, 113.0, 196.0, 18.0, 1);
    let total = if let Some(percent) = data.quota_percent {
        format!("Quota used {}%", percent)
    } else {
        format!("Total {} tokens", format_tokens(data.total_tokens))
    };
    p.label(&total, 228.0, 135.0, 196.0, 18.0, 1);
    if s.show_proportional_bar {
        let values = [s.summary.antigravity.today_tokens, s.summary.claude.today_tokens, s.summary.codex.today_tokens, s.summary.opencode.today_tokens];
        let sum: u64 = values.iter().enumerate().filter(|(i,_)|s.enabled[*i]).map(|(_,v)|*v).sum();
        if sum > 0 {
            let colors = [(0.26,0.52,0.96),(0.85,0.47,0.34),(0.06,0.64,0.50),(0.02,0.71,0.83)];
            let mut x = 28.0;
            for (i,value) in values.iter().enumerate() {
                if s.enabled[i] && *value > 0 {
                    let width = 396.0 * *value as f32 / sum as f32;
                    if let Ok(brush) = p.d.create_solid_brush(D2D1_COLOR_F { r:colors[i].0,g:colors[i].1,b:colors[i].2,a:0.8 }) {
                        p.d.fill_rounded_rect(p.rect(x,158.0,width,3.0),1.5,1.5,&brush);
                    }
                    x += width;
                }
            }
        }
    }
}

fn clean_model_name(m: &str) -> String {
    let s = m.trim();
    if s.is_empty() {
        return "Default".to_string();
    }
    let after_slash = if let Some(pos) = s.rfind('/') {
        &s[pos + 1..]
    } else {
        s
    };
    if after_slash.starts_with("tokenrouter-") {
        return after_slash.replace("tokenrouter-", "");
    }
    after_slash.to_string()
}

fn stocks(p: &mut Painter<'_>, s: &SahamSection) {
    if s.stocks.is_empty() {
        p.label("Your watchlist is empty",38.0,53.0,340.0,25.0,2);
        p.label("Add tickers in Settings > Stocks.",38.0,86.0,340.0,20.0,1);
        return;
    }
    let slot = 382.0/s.stocks.len() as f32;
    p.d.fill_rounded_rect(p.rect(34.0+s.selection_position*slot,33.0,slot-4.0,25.0),7.0,7.0,&p.panel);
    for (i,stock) in s.stocks.iter().enumerate() {
        let r = p.rect(34.0+i as f32*slot,33.0,slot-4.0,25.0);
        let label = stock.symbol.strip_suffix(".JK").unwrap_or(&stock.symbol);
        p.w.draw_text(&p.d.context,label,r,
            if s.selected_index==i { &p.text } else { &p.muted },&p.w.format_card_name);
        p.regions.push(Region { rect:r, action:Action::Stock(i) });
    }
    let Some(stock) = s.stocks.get(s.selected_index) else { return; };
    let Some(quote) = &stock.quote else {
        p.label(if stock.error { "Data unavailable" } else { "Fetching market data..." },38.0,76.0,330.0,24.0,2);
        p.label(if stock.error { "Check the ticker or your internet connection." } else { "Yahoo Finance · data may be delayed" },38.0,108.0,350.0,18.0,1);
        return;
    };
    p.label(&quote.name,38.0,62.0,220.0,18.0,1);
    p.label(&format!("{} {:.2}",quote.currency,quote.price),38.0,82.0,215.0,34.0,3);
    let change = quote.change_pct.map(|v| format!("{:+.2}%",v)).unwrap_or_else(|| "Change —".into());
    let (r,g,b) = if quote.change_pct.unwrap_or(0.0) < 0.0 { (0.98,0.39,0.39) } else { (0.35,0.83,0.64) };
    if let Ok(brush) = p.d.create_solid_brush(D2D1_COLOR_F { r,g,b,a:1.0 }) {
        p.w.draw_text(&p.d.context,&change,p.rect(38.0,116.0,150.0,20.0),&brush,&p.w.format_regular);
        if quote.history.len() >= 2 {
            let min=quote.history.iter().copied().fold(f64::INFINITY,f64::min);
            let max=quote.history.iter().copied().fold(f64::NEG_INFINITY,f64::max);
            let spread=(max-min).max(0.0001);
            p.label("5 DAYS",275.0,61.0,129.0,15.0,0);
            for y in [80.0, 101.0, 122.0] {
                p.d.draw_line(p.bounds.left+275.0,p.bounds.top+y,p.bounds.left+404.0,p.bounds.top+y,&p.edge,0.7,false);
            }
            let pts: Vec<_> = quote.history.iter().enumerate().map(|(i,v)|
                (p.bounds.left+275.0+i as f32/(quote.history.len()-1) as f32*129.0,
                 p.bounds.top+122.0-((*v-min)/spread) as f32*42.0)).collect();
            p.d.draw_polyline(&pts,&brush,1.8,false);
        }
    }
    p.label(if stock.error { "Update failed · previous data" } else { "Yahoo Finance · delayed data" },38.0,139.0,225.0,15.0,0);
    p.label(&crate::sections::saham::quote_age(quote.timestamp),265.0,139.0,145.0,15.0,0);
}

fn time_text(ms: u64) -> String { format!("{:02}:{:02}",ms/60_000,(ms/1000)%60) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn action_regions_accept_their_center_and_reject_outside() {
        let r=Region{rect:D2D_RECT_F{left:20.0,top:40.0,right:100.0,bottom:68.0},action:Action::PlayPause};
        assert!(r.contains(60.0,54.0)); assert!(!r.contains(101.0,54.0));
    }
}
