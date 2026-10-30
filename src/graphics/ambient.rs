use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};

use super::d2d::D2DContext;

// The cyan, blue, and violet stops used by the project's original mascot renderer.
pub const STOPS: [(f32, D2D1_COLOR_F); 3] = [
    (0.00, D2D1_COLOR_F { r: 0.10, g: 0.74, b: 1.00, a: 1.0 }),
    (0.50, D2D1_COLOR_F { r: 0.16, g: 0.44, b: 1.00, a: 1.0 }),
    (1.00, D2D1_COLOR_F { r: 0.46, g: 0.30, b: 1.00, a: 1.0 }),
];

pub fn draw(d: &D2DContext, bounds: D2D_RECT_F, corner: f32, opacity: f32, shift: f32) {
    draw_with_cover(d, bounds, corner, opacity, shift, (0.0, 0.0, 0.0), 0.0);
}

pub fn draw_with_cover(d: &D2DContext, bounds: D2D_RECT_F, corner: f32, opacity: f32,
    shift: f32, cover: (f32, f32, f32), mix: f32) {
    if let Ok(black) = d.create_solid_brush(D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) {
        d.fill_rect(bounds, &black);
    }
    let mix = mix.clamp(0.0, 1.0);
    let stops = STOPS.map(|(position, base)| {
        let shade = 0.78 + 0.44 * position;
        let blend = |original: f32, channel: f32| original * (1.0 - mix) + (channel * shade).min(1.0) * mix;
        (position, D2D1_COLOR_F { r: blend(base.r, cover.0), g: blend(base.g, cover.1),
            b: blend(base.b, cover.2), a: 1.0 })
    });
    d.draw_ambient_glow(bounds.left, bounds.right - bounds.left,
        bounds.bottom - bounds.top, corner, 0.90, opacity, shift, &stops);
}
