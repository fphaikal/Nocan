use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;

pub struct ShelfItem {
    pub name: String,
    pub path: PathBuf,
}

pub struct ShelfSection {
    pub shelf_dir: PathBuf,
    pub items: Vec<ShelfItem>,
}

impl ShelfSection {
    pub fn new() -> Self {
        let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());
        let mut p = PathBuf::from(local_appdata);
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
                    self.items.push(ShelfItem {
                        name: file_name.to_string(),
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
        card_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;
        let right = bounds.right;
        let _center_x = (left + right) / 2.0;

        // Header on top left
        let header_rect = D2D_RECT_F {
            left: left + 24.0,
            top: top + 10.0,
            right: left + 150.0,
            bottom: top + 24.0,
        };
        dwrite.draw_text(&d2d.context, "📁 File Shelf", header_rect, dim_brush, &dwrite.format_small);

        // Item count on top right
        let count_rect = D2D_RECT_F {
            left: right - 120.0,
            top: top + 10.0,
            right: right - 24.0,
            bottom: top + 24.0,
        };
        let count_str = format!("{} items", self.items.len());
        dwrite.draw_text(&d2d.context, &count_str, count_rect, dim_brush, &dwrite.format_small);

        if self.items.is_empty() {
            // Drop zone placeholder box
            let drop_box = D2D_RECT_F {
                left: left + 24.0,
                top: top + 28.0,
                right: right - 24.0,
                bottom: top + 78.0,
            };
            d2d.fill_rounded_rect(drop_box, 6.0, 6.0, card_brush);

            let msg1_rect = D2D_RECT_F {
                left: drop_box.left,
                top: drop_box.top + 10.0,
                right: drop_box.right,
                bottom: drop_box.top + 26.0,
            };
            dwrite.draw_text(&d2d.context, "Drop files here", msg1_rect, white_brush, &dwrite.format_center);

            let msg2_rect = D2D_RECT_F {
                left: drop_box.left,
                top: drop_box.top + 28.0,
                right: drop_box.right,
                bottom: drop_box.top + 42.0,
            };
            dwrite.draw_text(&d2d.context, "Drag out anytime • Click to open", msg2_rect, dim_brush, &dwrite.format_center);
        } else {
            // Render file chips
            let chip_start_y = top + 28.0;
            for (idx, item) in self.items.iter().take(2).enumerate() {
                let cur_y = chip_start_y + (idx as f32 * 26.0);
                let chip_rect = D2D_RECT_F {
                    left: left + 24.0,
                    top: cur_y,
                    right: right - 24.0,
                    bottom: cur_y + 22.0,
                };
                d2d.fill_rounded_rect(chip_rect, 4.0, 4.0, card_brush);

                let icon_rect = D2D_RECT_F {
                    left: chip_rect.left + 8.0,
                    top: chip_rect.top + 2.0,
                    right: chip_rect.left + 24.0,
                    bottom: chip_rect.bottom,
                };
                dwrite.draw_text(&d2d.context, "📄", icon_rect, dim_brush, &dwrite.format_small);

                let name_rect = D2D_RECT_F {
                    left: chip_rect.left + 28.0,
                    top: chip_rect.top + 2.0,
                    right: chip_rect.right - 8.0,
                    bottom: chip_rect.bottom,
                };
                dwrite.draw_text(&d2d.context, &item.name, name_rect, white_brush, &dwrite.format_regular);
            }
        }
    }

    pub fn on_click(&self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let right = bounds.right;

        if !self.items.is_empty() {
            let chip_start_y = top + 28.0;
            for (idx, item) in self.items.iter().take(2).enumerate() {
                let cur_y = chip_start_y + (idx as f32 * 26.0);
                if y >= cur_y && y <= cur_y + 22.0 && x >= left + 24.0 && x <= right - 24.0 {
                    self.open_file(&item.path);
                    return true;
                }
            }
        }
        false
    }
}
