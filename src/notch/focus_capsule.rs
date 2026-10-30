use windows::core::{w, Result};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_ALPHA_MODE_IGNORE, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_U};
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{BeginPaint, CreateRoundRectRgn, EndPaint, InvalidateRect, ScreenToClient, SetWindowRgn, PAINTSTRUCT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use crate::graphics::dwrite::DWriteContext;
use crate::sections::focus::{FocusSection, PomodoroMode};

pub const WM_TIMER_ACTION: u32 = WM_USER + 120;
const WIDTH: i32 = 164;
const HEIGHT: i32 = 36;

pub fn should_detach(active: bool, lyric: Option<&str>) -> bool {
    active && lyric.map_or(false, |line| !line.trim().is_empty())
}

pub fn timer_color(focus: &FocusSection) -> D2D1_COLOR_F {
    let (r, g, b) = background(Snapshot { remaining: focus.remaining_secs, total: focus.total_secs,
        running: focus.is_running, mode: focus.mode, complete: focus.just_completed() });
    D2D1_COLOR_F { r, g, b, a: 1.0 }
}

fn chip_scale(compact: (f32, f32)) -> f32 {
    (compact.0 / 188.0).min(compact.1 / 40.0).clamp(26.0 / HEIGHT as f32, 1.0)
}

fn scaled_size(scale: f32) -> (i32, i32) {
    ((WIDTH as f32 * scale).round() as i32, (HEIGHT as f32 * scale).round() as i32)
}

#[derive(Debug, PartialEq)]
enum Phase { Plenty, Low, Urgent, Paused, Complete }

fn phase(remaining: u32, total: u32, running: bool, complete: bool) -> Phase {
    if complete { Phase::Complete }
    else if !running { Phase::Paused }
    else if remaining <= 60 { Phase::Urgent }
    else if remaining as f32 / total.max(1) as f32 <= 0.25 { Phase::Low }
    else { Phase::Plenty }
}

fn background(s: Snapshot) -> (f32, f32, f32) {
    let mix = |a: (f32, f32, f32), b: (f32, f32, f32), t: f32| {
        let t = t.clamp(0.0, 1.0);
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
    };
    let ratio = s.remaining as f32 / s.total.max(1) as f32;
    match phase(s.remaining, s.total, s.running, s.complete) {
        Phase::Plenty => mix((0.06, 0.30, 0.20), (0.26, 0.30, 0.10), (1.0 - ratio) / 0.75),
        Phase::Low => mix((0.39, 0.29, 0.06), (0.48, 0.20, 0.05), 1.0 - ratio / 0.25),
        Phase::Urgent => mix((0.45, 0.10, 0.13), (0.62, 0.07, 0.12), 1.0 - s.remaining as f32 / 60.0),
        Phase::Paused => (0.19, 0.21, 0.25),
        Phase::Complete => (0.08, 0.29, 0.56),
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Snapshot { remaining: u32, total: u32, running: bool, mode: PomodoroMode, complete: bool }

pub struct FloatingTimer { hwnd: HWND }
struct State {
    owner: HWND,
    target: ID2D1HwndRenderTarget,
    brush: ID2D1SolidColorBrush,
    font: DWriteContext,
    snapshot: Snapshot,
    hover: Option<usize>,
    visible: bool,
    moved: bool,
    position: (i32, i32),
    notch_size: (i32, i32),
    size: (i32, i32),
    scale: f32,
    last_update: std::time::Instant,
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> D2D_RECT_F {
    D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h }
}

fn control(index: usize) -> D2D_RECT_F {
    debug_assert_eq!(index, 0);
    rect(134.0, 6.0, 24.0, 24.0)
}

fn hit(x: f32, y: f32) -> Option<usize> {
    (0..1).find(|&i| { let r = control(i); x >= r.left && x <= r.right && y >= r.top && y <= r.bottom })
}

fn hit_scaled(x: f32, y: f32, size: (i32, i32)) -> Option<usize> {
    hit(x * WIDTH as f32 / size.0 as f32, y * HEIGHT as f32 / size.1 as f32)
}

fn aligned_position(monitor: (i32, i32, i32), notch: (f32, f32), chip: (i32, i32)) -> (i32, i32) {
    let (sx, sy, sw) = monitor;
    let wanted = sx + sw / 2 + (notch.0 * 0.5).round() as i32 + 12;
    let x = wanted.min(sx + sw - chip.0 - 12).max(sx + 12);
    // Keep the chip on the notch's compact row while the panel expands below it.
    let row_height = notch.1.min(40.0);
    let y = sy + ((row_height - chip.1 as f32) * 0.5).round().max(2.0) as i32;
    (x, y)
}

impl FloatingTimer {
    pub fn create(owner: HWND) -> Result<Self> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = w!("NocanFloatingTimer");
            let _ = RegisterClassExW(&WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(Self::wndproc), hInstance: instance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW)?, lpszClassName: class,
                ..Default::default()
            });
            let preview = std::env::args().any(|arg| arg == "--preview-focus");
            let visibility = if preview { WS_EX_APPWINDOW } else { WS_EX_TOOLWINDOW };
            let hwnd = CreateWindowExW(visibility | WS_EX_TOPMOST, class, w!("Nocan Focus Timer"),
                WS_POPUP, 0, 2, WIDTH, HEIGHT, None, None, instance, None)?;
            let factory: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let target = match factory.CreateHwndRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_IGNORE },
                dpiX: 96.0, dpiY: 96.0, ..Default::default()
            }, &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd, pixelSize: D2D_SIZE_U { width: WIDTH as u32, height: HEIGHT as u32 },
                ..Default::default()
            }) {
                Ok(target) => target,
                Err(error) => { let _ = DestroyWindow(hwnd); return Err(error); }
            };
            let brush = target.CreateSolidColorBrush(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, None)?;
            let state = Box::new(State {
                owner, target, brush, font: DWriteContext::new()?,
                snapshot: Snapshot { remaining: 1500, total: 1500, running: false, mode: PomodoroMode::Focus, complete: false },
                hover: None, visible: false, moved: false, position: (0, 0), notch_size: (0, 0),
                size: (WIDTH, HEIGHT), scale: 1.0, last_update: std::time::Instant::now(),
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
            let region = CreateRoundRectRgn(0, 0, WIDTH + 1, HEIGHT + 1, HEIGHT, HEIGHT);
            if !region.is_invalid() { let _ = SetWindowRgn(hwnd, region, false); }
            Ok(Self { hwnd })
        }
    }

    pub fn update(&self, focus: &FocusSection, monitor: (i32, i32, i32), notch: (f32, f32), compact: (f32, f32), detached: bool) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut State;
            if ptr.is_null() { return; }
            let state = &mut *ptr;
            let now = std::time::Instant::now();
            let dt = now.duration_since(state.last_update).as_secs_f32().min(0.05);
            state.last_update = now;
            let target = chip_scale(compact);
            state.scale += (target - state.scale) * (1.0 - (-14.0 * dt).exp());
            if (target - state.scale).abs() < 0.002 { state.scale = target; }
            let new_size = scaled_size(state.scale);
            let resized = state.size != new_size && state.target.Resize(&D2D_SIZE_U {
                width: new_size.0 as u32, height: new_size.1 as u32,
            }).is_ok();
            if resized {
                state.size = new_size;
                let region = CreateRoundRectRgn(0, 0, new_size.0 + 1, new_size.1 + 1, new_size.1, new_size.1);
                if !region.is_invalid() { let _ = SetWindowRgn(self.hwnd, region, false); }
                let _ = InvalidateRect(self.hwnd, None, false);
            }
            let next = Snapshot { remaining: focus.remaining_secs, total: focus.total_secs, running: focus.is_running, mode: focus.mode, complete: focus.just_completed() };
            if state.snapshot != next { state.snapshot = next; let _ = InvalidateRect(self.hwnd, None, false); }
            if !state.moved {
                let (x, y) = aligned_position(monitor, notch, state.size);
                if state.position != (x, y) || resized {
                    state.position = (x, y);
                    let _ = SetWindowPos(self.hwnd, HWND_TOPMOST, x, y, state.size.0, state.size.1, SWP_NOACTIVATE);
                }
            } else if resized {
                let _ = SetWindowPos(self.hwnd, HWND_TOPMOST, 0, 0, state.size.0, state.size.1, SWP_NOMOVE | SWP_NOACTIVATE);
            }
            let size = (notch.0.round() as i32, notch.1.round() as i32);
            if state.notch_size != size {
                state.notch_size = size;
                let _ = SetWindowPos(self.hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
            let visible = focus.session_active && detached;
            if state.visible != visible {
                if !visible {
                    state.moved = false;
                    state.position = (i32::MIN, i32::MIN);
                }
                state.visible = visible;
                let _ = ShowWindow(self.hwnd, if state.visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
            }
        }
    }

    pub fn contains_screen_point(&self, point: POINT) -> bool {
        unsafe {
            let mut r = RECT::default();
            IsWindowVisible(self.hwnd).as_bool() && GetWindowRect(self.hwnd, &mut r).is_ok()
                && point.x >= r.left && point.x < r.right && point.y >= r.top && point.y < r.bottom
        }
    }

    pub fn is_window(&self, window: HWND) -> bool { self.hwnd == window }

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
        if ptr.is_null() { return DefWindowProcW(hwnd, msg, wp, lp); }
        let s = &mut *ptr;
        match msg {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let _ = BeginPaint(hwnd, &mut ps);
                s.draw();
                let _ = EndPaint(hwnd, &ps);
                return LRESULT(0);
            }
            WM_ERASEBKGND => return LRESULT(1),
            WM_MOUSEMOVE => {
                let next = hit_scaled((lp.0 & 0xffff) as i16 as f32, ((lp.0 >> 16) & 0xffff) as i16 as f32, s.size);
                if next != s.hover { s.hover = next; let _ = InvalidateRect(hwnd, None, false); }
                let mut tracking = windows::Win32::UI::Input::KeyboardAndMouse::TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<windows::Win32::UI::Input::KeyboardAndMouse::TRACKMOUSEEVENT>() as u32,
                    dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0,
                };
                let _ = windows::Win32::UI::Input::KeyboardAndMouse::TrackMouseEvent(&mut tracking);
            }
            WM_SETCURSOR if s.hover.is_some() => {
                if let Ok(cursor) = LoadCursorW(None, IDC_HAND) { SetCursor(cursor); }
                return LRESULT(1);
            }
            0x02a3 => { s.hover = None; let _ = InvalidateRect(hwnd, None, false); }
            WM_LBUTTONDOWN => {
                if let Some(index) = hit_scaled((lp.0 & 0xffff) as i16 as f32, ((lp.0 >> 16) & 0xffff) as i16 as f32, s.size) {
                    let _ = PostMessageW(s.owner, WM_TIMER_ACTION, WPARAM(index), LPARAM(0));
                }
                return LRESULT(0);
            }
            WM_KEYDOWN => {
                let action = match wp.0 { 0x20 | 0x0d => Some(0), 0x41 => Some(1), 0x52 => Some(2), 0x4e => Some(3), _ => None };
                if let Some(index) = action { let _ = PostMessageW(s.owner, WM_TIMER_ACTION, WPARAM(index), LPARAM(0)); return LRESULT(0); }
            }
            WM_NCHITTEST => {
                let mut p = POINT { x: (lp.0 & 0xffff) as i16 as i32, y: ((lp.0 >> 16) & 0xffff) as i16 as i32 };
                let _ = ScreenToClient(hwnd, &mut p);
                return LRESULT(if hit_scaled(p.x as f32, p.y as f32, s.size).is_none() { HTCAPTION } else { HTCLIENT } as isize);
            }
            WM_RBUTTONUP | WM_NCRBUTTONUP => {
                if let Ok(menu) = CreatePopupMenu() {
                    let _ = AppendMenuW(menu, MF_STRING, 1, w!("Add 5 minutes\tA"));
                    let _ = AppendMenuW(menu, MF_STRING, 3, w!("Skip session\tN"));
                    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
                    let _ = AppendMenuW(menu, MF_STRING, 2, w!("Reset timer\tR"));
                    let mut p = POINT::default();
                    let _ = GetCursorPos(&mut p);
                    let _ = SetForegroundWindow(hwnd);
                    let choice = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON, p.x, p.y, 0, hwnd, None).0;
                    if choice > 0 { let _ = PostMessageW(s.owner, WM_TIMER_ACTION, WPARAM(choice as usize), LPARAM(0)); }
                    let _ = DestroyMenu(menu);
                }
                return LRESULT(0);
            }
            WM_ENTERSIZEMOVE | WM_EXITSIZEMOVE => { s.moved = true; }
            WM_CLOSE => { let _ = PostMessageW(s.owner, WM_TIMER_ACTION, WPARAM(2), LPARAM(0)); return LRESULT(0); }
            WM_DESTROY => { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0); drop(Box::from_raw(ptr)); return LRESULT(0); }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}

impl Drop for FloatingTimer {
    fn drop(&mut self) { unsafe { let _ = DestroyWindow(self.hwnd); } }
}

impl State {
    fn fill(&self, r: D2D_RECT_F, radius: f32, rgb: (f32, f32, f32), a: f32) {
        unsafe {
            self.brush.SetColor(&D2D1_COLOR_F { r: rgb.0, g: rgb.1, b: rgb.2, a });
            self.target.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: r, radiusX: radius, radiusY: radius }, &self.brush);
        }
    }

    fn text(&self, text: &str, r: D2D_RECT_F, format: &windows::Win32::Graphics::DirectWrite::IDWriteTextFormat, alpha: f32) {
        unsafe {
            self.brush.SetColor(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: alpha });
            self.target.DrawText(&text.encode_utf16().collect::<Vec<_>>(), format, &r, &self.brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP, windows::Win32::Graphics::DirectWrite::DWRITE_MEASURING_MODE_NATURAL);
        }
    }

    fn draw(&self) {
        unsafe {
            let bg = background(self.snapshot);
            self.target.BeginDraw();
            self.target.SetTransform(&windows::Foundation::Numerics::Matrix3x2 {
                M11: self.size.0 as f32 / WIDTH as f32, M12: 0.0,
                M21: 0.0, M22: self.size.1 as f32 / HEIGHT as f32,
                M31: 0.0, M32: 0.0,
            });
            self.target.Clear(Some(&D2D1_COLOR_F { r: bg.0, g: bg.1, b: bg.2, a: 1.0 }));
            let progress = if self.snapshot.complete { 1.0 } else {
                (1.0 - self.snapshot.remaining as f32 / self.snapshot.total.max(1) as f32).clamp(0.0, 1.0)
            };
            for segment in 0..24 {
                let angle = segment as f32 / 24.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                self.fill(rect(17.0 + angle.cos() * 8.0, 17.0 + angle.sin() * 8.0, 2.0, 2.0), 1.0,
                    (1.0, 1.0, 1.0), if (segment as f32 / 24.0) < progress { 0.95 } else { 0.28 });
            }
            if self.snapshot.complete { self.text("✓", rect(10.0, 7.0, 16.0, 22.0), &self.font.format_control_text, 1.0); }
            let label = if self.snapshot.complete { "Done".to_string() }
                else { format!("{:02}:{:02}", self.snapshot.remaining / 60, self.snapshot.remaining % 60) };
            self.text(&label, rect(33.0, 0.0, 94.0, 36.0), &self.font.format_idle_time, 1.0);
            let button = control(0);
            self.fill(button, 12.0, (1.0, 1.0, 1.0), if self.hover.is_some() { 0.25 } else { 0.12 });
            self.text(if self.snapshot.running { "Ⅱ" } else { "▶" }, button, &self.font.format_control_text, 1.0);
            let _ = self.target.EndDraw(None, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_timer_detaches_only_for_nonempty_lyrics_and_reattaches_afterwards() {
        assert!(!should_detach(false, Some("A lyric")));
        assert!(!should_detach(true, None));
        assert!(should_detach(true, Some("A lyric")));
        assert!(!should_detach(true, Some("   ")));
        assert!(!should_detach(true, None));
        let mut focus = FocusSection::new();
        focus.toggle_start_pause();
        focus.toggle_start_pause();
        assert!(should_detach(focus.session_active, Some("A lyric")));
        focus.reset();
        assert!(!should_detach(focus.session_active, Some("A lyric")));
    }
    #[test]
    fn chip_stays_on_the_notch_row_when_expanding_or_clamped_to_screen() {
        let idle = aligned_position((0, 0, 1920), (188.0, 40.0), (WIDTH, HEIGHT));
        assert_eq!(idle, (1066, 2));
        assert_eq!(idle.1 as f32 + HEIGHT as f32 / 2.0, 40.0 / 2.0);
        assert_eq!(aligned_position((0, 0, 1920), (448.0, 196.0), (WIDTH, HEIGHT)), (1196, 2));
        assert_eq!(aligned_position((-800, -100, 800), (700.0, 196.0), (WIDTH, HEIGHT)), (-176, -98));
    }

    #[test]
    fn chip_scales_to_the_compact_notch_with_a_readable_minimum() {
        assert_eq!(scaled_size(chip_scale((44.0, 4.0))), (118, 26));
        assert_eq!(scaled_size(chip_scale((188.0, 32.0))), (131, 29));
        assert_eq!(scaled_size(chip_scale((188.0, 40.0))), (164, 36));
        assert_eq!(scaled_size(chip_scale((700.0, 196.0))), (164, 36));
        assert!(chip_scale((120.0, 25.0)) <= chip_scale((160.0, 35.0)));
    }

    #[test]
    fn scaled_pause_button_tracks_its_visible_position() {
        for size in [(118, 26), (131, 29), (164, 36)] {
            let x = 146.0 * size.0 as f32 / WIDTH as f32;
            let y = 18.0 * size.1 as f32 / HEIGHT as f32;
            assert_eq!(hit_scaled(x, y, size), Some(0));
            assert_eq!(hit_scaled(size.0 as f32 * 0.4, y, size), None);
            assert_eq!(hit_scaled(x, size.1 as f32 + 1.0, size), None);
        }
    }

    #[test]
    fn chip_body_can_be_dragged_without_triggering_pause() {
        assert_eq!(hit(146.0, 18.0), Some(0));
        assert_eq!(hit(80.0, 18.0), None);
        assert_eq!(hit(146.0, 40.0), None);
    }

    #[test]
    fn countdown_colors_cover_boundaries_and_completion_overrides_next_session() {
        assert_eq!(phase(1500, 1500, true, false), Phase::Plenty);
        assert_eq!(phase(376, 1500, true, false), Phase::Plenty);
        assert_eq!(phase(375, 1500, true, false), Phase::Low);
        assert_eq!(phase(61, 1500, true, false), Phase::Low);
        assert_eq!(phase(60, 1500, true, false), Phase::Urgent);
        assert_eq!(phase(1, 1500, true, false), Phase::Urgent);
        assert_eq!(phase(300, 300, false, true), Phase::Complete);
        assert_eq!(phase(300, 300, false, false), Phase::Paused);
        assert_eq!(phase(0, 0, true, false), Phase::Urgent);
    }
}
