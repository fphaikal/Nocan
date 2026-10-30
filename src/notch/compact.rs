use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};

use crate::graphics::{d2d::D2DContext, dwrite::DWriteContext, icons};
use crate::integrations::{media::MediaTrackInfo, system_info::SystemStatus};
use crate::sections::focus::FocusSection;

pub fn timer_pause_rect(bounds: D2D_RECT_F) -> D2D_RECT_F {
    let size = ((bounds.bottom - bounds.top) - 8.0).clamp(18.0, 26.0);
    D2D_RECT_F { left: bounds.right - size - 6.0, top: (bounds.top + bounds.bottom - size) * 0.5,
        right: bounds.right - 6.0, bottom: (bounds.top + bounds.bottom + size) * 0.5 }
}

pub fn draw_timer(d2d: &D2DContext, dwrite: &DWriteContext, bounds: D2D_RECT_F, focus: &FocusSection) {
    let color = crate::notch::focus_capsule::timer_color(focus);
    let Ok(bg) = d2d.create_solid_brush(color) else { return; };
    let Ok(text) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }) else { return; };
    let Ok(dim) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.22 }) else { return; };
    let height = bounds.bottom - bounds.top;
    let cy = (bounds.top + bounds.bottom) * 0.5;
    d2d.fill_rounded_rect(bounds, height * 0.5, height * 0.5, &bg);
    d2d.draw_progress_ring(bounds.left + 18.0, cy, 7.0, 1.6,
        if focus.just_completed() { 1.0 } else { focus.elapsed_ratio() }, &dim, &text);
    let button = timer_pause_rect(bounds);
    d2d.fill_rounded_rect(button, 13.0, 13.0, &dim);
    icons::draw(d2d, if focus.is_running { icons::Icon::Pause } else { icons::Icon::Play },
        (button.left + button.right) * 0.5, cy, 9.0, &text);
    dwrite.draw_text(&d2d.context, &if focus.just_completed() { "Done".to_string() } else { focus.time_text() },
        D2D_RECT_F { left: bounds.left + 30.0, top: bounds.top, right: button.left - 3.0, bottom: bounds.bottom },
        &text, if bounds.right - bounds.left < 150.0 { &dwrite.format_button_center } else { &dwrite.format_idle_time });
}

pub fn draw(
    d2d: &D2DContext,
    dwrite: &DWriteContext,
    bounds: D2D_RECT_F,
    status: &SystemStatus,
    track: &MediaTrackInfo,
    time: f32,
    focus: &FocusSection,
    show_timer: bool,
) {
    if show_timer && (focus.session_active || focus.is_running) {
        draw_timer(d2d, dwrite, bounds, focus);
        return;
    }
    let left = bounds.left;
    let right = bounds.right;
    let cy = (bounds.top + bounds.bottom) * 0.5;
    let Ok(text) = d2d.create_solid_brush(D2D1_COLOR_F { r: 0.97, g: 0.98, b: 1.0, a: 1.0 }) else { return; };
    let Ok(muted) = d2d.create_solid_brush(D2D1_COLOR_F { r: 0.64, g: 0.68, b: 0.75, a: 1.0 }) else { return; };
    crate::graphics::ambient::draw(d2d,bounds,14.0,0.22,0.0);
    icons::power(d2d,status,left+27.0,cy-1.0,18.0,&text,&muted);
    dwrite.draw_text(&d2d.context,&status.clock_str,D2D_RECT_F {
        left:left+49.0,top:bounds.top,right:right-49.0,bottom:bounds.bottom,
    },&text,&dwrite.format_idle_time);
    if track.is_playing {
        for index in 0..4 {
            let h = 3.0 + ((time*4.0+index as f32*1.4).sin()*0.5+0.5)*9.0;
            d2d.draw_line(right-35.0+index as f32*4.0,cy-h*0.5,
                right-35.0+index as f32*4.0,cy+h*0.5,&text,1.8,true);
        }
    } else {
        icons::draw(d2d,icons::Icon::Music,right-28.0,cy,15.0,&muted);
    }
    if track.is_playing && track.duration_ms>0 {
        let progress=(track.position_ms as f64/track.duration_ms as f64).clamp(0.0,1.0) as f32;
        let y=bounds.bottom-1.5;
        d2d.draw_line(left+59.0,y,right-59.0,y,&muted,0.7,true);
        d2d.draw_line(left+59.0,y,left+59.0+(right-left-118.0)*progress,y,&text,0.9,true);
    }
}
