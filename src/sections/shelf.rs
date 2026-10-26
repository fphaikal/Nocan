use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;

pub struct ShelfItem {
    pub name: String,
    pub size_label: String,
    pub is_dir: bool,
    pub is_image: bool,
    pub path: PathBuf,
}

pub struct ShelfSection {
    pub shelf_dir: PathBuf,
    pub items: Vec<ShelfItem>,
}

impl ShelfSection {
    pub fn new() -> Self {
        let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());
        let mut p = PathBuf::from(&local_appdata);
        p.push("Nocan");
        p.push("Shelf");
        let _ = std::fs::create_dir_all(&p);

        let mut section = Self {
            shelf_dir: p,
            items: Vec::new(),
        };
        section.refresh();
        section
    }

    pub fn refresh(&mut self) {
        self.items.clear();
        if let Ok(entries) = std::fs::read_dir(&self.shelf_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                    let meta = std::fs::metadata(&path).ok();
                    let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    let size_label = if is_dir {
                        "Folder".to_string()
                    } else if let Some(m) = meta {
                        format_size(m.len())
                    } else {
                        "File".to_string()
                    };
                    let lower = file_name.to_lowercase();
                    let is_image = lower.ends_with(".png")
                        || lower.ends_with(".jpg")
                        || lower.ends_with(".jpeg")
                        || lower.ends_with(".webp")
                        || lower.ends_with(".gif");

                    self.items.push(ShelfItem {
                        name: file_name.to_string(),
                        size_label,
                        is_dir,
                        is_image,
                        path,
                    });
                }
            }
        }
    }

    pub fn add_file(&mut self, source_path: &std::path::Path) {
        if let Some(file_name) = source_path.file_name() {
            let target_path = self.shelf_dir.join(file_name);
            let _ = std::fs::copy(source_path, target_path);
            self.refresh();
        }
    }

    pub fn clear_all(&mut self) {
        if let Ok(entries) = std::fs::read_dir(&self.shelf_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let _ = std::fs::remove_dir_all(p);
                } else {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        self.refresh();
    }

    pub fn open_file(&self, path: &std::path::Path) {
        let path_str = path.to_string_lossy();
        let utf16: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
        let op: Vec<u16> = "open\0".encode_utf16().collect();
        unsafe {
            ShellExecuteW(
                None,
                PCWSTR(op.as_ptr()),
                PCWSTR(utf16.as_ptr()),
                None,
                None,
                SW_SHOW,
            );
        }
    }

    pub fn draw(
        &self,
        d2d: &D2DContext,
        dwrite: &DWriteContext,
        bounds: D2D_RECT_F,
        white_brush: &ID2D1SolidColorBrush,
        dim_brush: &ID2D1SolidColorBrush,
        pill_bg_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;

        // 1. Section Title: "Shelf" at left: 15px, top: 19px (vc)
        let title_rect = D2D_RECT_F {
            left: left + 15.0,
            top: top + 9.0,
            right: left + 65.0,
            bottom: top + 29.0,
        };
        dwrite.draw_text(&d2d.context, "Shelf", title_rect, white_brush, &dwrite.format_title);

        if self.items.is_empty() {
            // 2. Empty state: Dashed Drop Zone (.zone) at left: 44px, top: 33px, width: 336px, height: 90px, rx: 16px
            let drop_box = D2D_RECT_F {
                left: left + 44.0,
                top: top + 33.0,
                right: left + 380.0,
                bottom: top + 123.0,
            };
            if let Ok(zone_bg) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.03 }) {
                d2d.fill_rounded_rect(drop_box, 16.0, 16.0, &zone_bg);
            }
            if let Ok(zone_border) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.30 }) {
                d2d.draw_dashed_rounded_rect(drop_box, 16.0, 16.0, &zone_border, 1.4);
            }

            // Tray + Down Arrow Icon centered at (left + 212, top + 61)
            let icx = left + 212.0;
            let icy = top + 66.0;
            if let Ok(tray_back) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.28 }) {
                let back_pts = [
                    (icx - 10.2, icy - 8.0),
                    (icx + 10.2, icy - 8.0),
                    (icx + 13.4, icy - 0.2),
                    (icx - 13.4, icy - 0.2),
                ];
                d2d.fill_polygon(&back_pts, &tray_back);
            }
            if let Ok(arrow_b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.55 }) {
                let arrow_pts = [
                    (icx - 1.7, icy - 20.0),
                    (icx + 1.7, icy - 20.0),
                    (icx + 1.7, icy - 13.5),
                    (icx + 5.2, icy - 13.5),
                    (icx, icy - 7.5),
                    (icx - 5.2, icy - 13.5),
                    (icx - 1.7, icy - 13.5),
                ];
                d2d.fill_polygon(&arrow_pts, &arrow_b);
            }
            if let Ok(tray_front) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.85 }) {
                let front_pts = [
                    (icx - 14.0, icy),
                    (icx - 5.2, icy),
                    (icx - 3.6, icy + 3.2),
                    (icx + 3.6, icy + 3.2),
                    (icx + 5.2, icy),
                    (icx + 14.0, icy),
                    (icx + 14.0, icy + 8.0),
                    (icx - 14.0, icy + 8.0),
                ];
                d2d.fill_polygon(&front_pts, &tray_front);
            }

            // "Drop files here" centered at y = top + 91 (33 + 58)
            let msg1_rect = D2D_RECT_F {
                left: drop_box.left,
                top: top + 81.0,
                right: drop_box.right,
                bottom: top + 100.0,
            };
            dwrite.draw_text(&d2d.context, "Drop files here", msg1_rect, white_brush, &dwrite.format_center);

            // "Drag out anytime · click to open" centered at y = top + 106 (33 + 73)
            let msg2_rect = D2D_RECT_F {
                left: drop_box.left,
                top: top + 98.0,
                right: drop_box.right,
                bottom: top + 114.0,
            };
            dwrite.draw_text(
                &d2d.context,
                "Drag out anytime · click to open",
                msg2_rect,
                dim_brush,
                &dwrite.format_small_center,
            );
        } else {
            // Clear pill button at left: 62px, top: 19px (vc), height: 16px
            let clr_rect = D2D_RECT_F {
                left: left + 62.0,
                top: top + 11.0,
                right: left + 100.0,
                bottom: top + 27.0,
            };
            d2d.fill_rounded_rect(clr_rect, 8.0, 8.0, pill_bg_brush);
            dwrite.draw_text(&d2d.context, "Clear", clr_rect, white_brush, &dwrite.format_small_center);

            // File Cards (.sh-card) at left: 44, 112, 180, 248, 316
            for (idx, item) in self.items.iter().take(5).enumerate() {
                let card_x = left + 44.0 + (idx as f32) * 68.0;
                let thumb_rect = D2D_RECT_F {
                    left: card_x + 9.0,
                    top: top + 37.0,
                    right: card_x + 53.0,
                    bottom: top + 81.0,
                };

                if item.is_dir {
                    // Folder icon (#e8a720 back + #ffc94d front)
                    if let (Ok(f_back), Ok(f_front)) = (
                        d2d.create_solid_brush(D2D1_COLOR_F { r: 232.0 / 255.0, g: 167.0 / 255.0, b: 32.0 / 255.0, a: 1.0 }),
                        d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 201.0 / 255.0, b: 77.0 / 255.0, a: 1.0 }),
                    ) {
                        let tab = D2D_RECT_F {
                            left: thumb_rect.left + 3.0,
                            top: thumb_rect.top + 7.0,
                            right: thumb_rect.left + 20.0,
                            bottom: thumb_rect.top + 16.0,
                        };
                        d2d.fill_rounded_rect(tab, 3.0, 3.0, &f_back);
                        let back = D2D_RECT_F {
                            left: thumb_rect.left + 3.0,
                            top: thumb_rect.top + 11.0,
                            right: thumb_rect.right - 3.0,
                            bottom: thumb_rect.bottom - 5.0,
                        };
                        d2d.fill_rounded_rect(back, 3.0, 3.0, &f_back);
                        let front = D2D_RECT_F {
                            left: thumb_rect.left + 3.0,
                            top: thumb_rect.top + 15.0,
                            right: thumb_rect.right - 3.0,
                            bottom: thumb_rect.bottom - 5.0,
                        };
                        d2d.fill_rounded_rect(front, 3.0, 3.0, &f_front);
                    }
                } else if item.is_image {
                    // Beach/sky thumbnail (.t-img)
                    if let Ok(sky) = d2d.create_linear_gradient(
                        (thumb_rect.left, thumb_rect.top),
                        (thumb_rect.left, thumb_rect.bottom),
                        &[
                            (0.0, D2D1_COLOR_F { r: 86.0 / 255.0, g: 194.0 / 255.0, b: 1.0, a: 1.0 }),
                            (0.58, D2D1_COLOR_F { r: 243.0 / 255.0, g: 211.0 / 255.0, b: 143.0 / 255.0, a: 1.0 }),
                            (1.0, D2D1_COLOR_F { r: 46.0 / 255.0, g: 155.0 / 255.0, b: 214.0 / 255.0, a: 1.0 }),
                        ],
                    ) {
                        d2d.fill_rounded_rect_linear(thumb_rect, 9.0, 9.0, &sky);
                    }
                    if let Ok(sun) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 243.0 / 255.0, b: 196.0 / 255.0, a: 1.0 }) {
                        d2d.fill_ellipse(thumb_rect.left + 31.0, thumb_rect.top + 13.0, 5.0, 5.0, &sun);
                    }
                } else {
                    // Document card thumbnail
                    d2d.fill_rounded_rect(thumb_rect, 9.0, 9.0, pill_bg_brush);
                    let doc = D2D_RECT_F {
                        left: thumb_rect.left + 12.0,
                        top: thumb_rect.top + 9.0,
                        right: thumb_rect.right - 12.0,
                        bottom: thumb_rect.bottom - 9.0,
                    };
                    d2d.draw_rounded_rect(doc, 2.5, 2.5, white_brush, 1.4);
                    d2d.draw_line(doc.left + 4.0, doc.top + 8.0, doc.right - 4.0, doc.top + 8.0, dim_brush, 1.2, true);
                    d2d.draw_line(doc.left + 4.0, doc.top + 13.0, doc.right - 4.0, doc.top + 13.0, dim_brush, 1.2, true);
                }

                // File Name (10.5px SemiBold)
                let name_rect = D2D_RECT_F {
                    left: card_x - 2.0,
                    top: top + 85.0,
                    right: card_x + 64.0,
                    bottom: top + 101.0,
                };
                dwrite.draw_text(&d2d.context, &item.name, name_rect, white_brush, &dwrite.format_card_name);

                // Size / Folder label (9.5px dim)
                let size_rect = D2D_RECT_F {
                    left: card_x - 2.0,
                    top: top + 101.0,
                    right: card_x + 64.0,
                    bottom: top + 115.0,
                };
                dwrite.draw_text(&d2d.context, &item.size_label, size_rect, dim_brush, &dwrite.format_small_center);
            }
        }
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;

        if !self.items.is_empty() {
            // Clear button
            if x >= left + 60.0 && x <= left + 102.0 && y >= top + 9.0 && y <= top + 29.0 {
                self.clear_all();
                return true;
            }

            for (idx, item) in self.items.iter().take(5).enumerate() {
                let card_x = left + 44.0 + (idx as f32) * 68.0;
                if x >= card_x && x <= card_x + 62.0 && y >= top + 34.0 && y <= top + 116.0 {
                    self.open_file(&item.path);
                    return true;
                }
            }
        }
        false
    }

    pub fn is_interactive(&self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        if !self.items.is_empty() {
            if x >= left + 60.0 && x <= left + 102.0 && y >= top + 9.0 && y <= top + 29.0 {
                return true;
            }
            for idx in 0..self.items.len().min(5) {
                let card_x = left + 44.0 + (idx as f32) * 68.0;
                if x >= card_x && x <= card_x + 62.0 && y >= top + 34.0 && y <= top + 116.0 {
                    return true;
                }
            }
        }
        false
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{} KB", bytes / 1024)
    } else {
        format!("{} B", bytes)
    }
}
