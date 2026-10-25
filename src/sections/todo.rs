use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;
use crate::graphics::dwrite::DWriteContext;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TodoItem {
    pub id: usize,
    pub title: String,
    pub completed: bool,
}

pub struct TodoSection {
    pub items: Vec<TodoItem>,
    pub file_path: PathBuf,
}

impl TodoSection {
    pub fn new() -> Self {
        let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());
        let mut p = PathBuf::from(local_appdata);
        p.push("Nocan");
        let _ = std::fs::create_dir_all(&p);
        p.push("todo.json");

        let mut section = Self {
            items: Vec::new(),
            file_path: p,
        };
        section.load();
        section
    }

    pub fn load(&mut self) {
        if self.file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.file_path) {
                if let Ok(parsed) = serde_json::from_str::<Vec<TodoItem>>(&content) {
                    self.items = parsed;
                    return;
                }
            }
        }

        // Default tasks if empty
        self.items = vec![
            TodoItem {
                id: 1,
                title: "Build Nocan Living Notch in Rust".to_string(),
                completed: true,
            },
            TodoItem {
                id: 2,
                title: "Integrate OpenMausBot Usage Section".to_string(),
                completed: true,
            },
            TodoItem {
                id: 3,
                title: "Keep shelf & media controls intact".to_string(),
                completed: false,
            },
        ];
        self.save();
    }

    pub fn save(&self) {
        if let Ok(serialized) = serde_json::to_string_pretty(&self.items) {
            let _ = std::fs::write(&self.file_path, serialized);
        }
    }

    pub fn toggle_item(&mut self, id: usize) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.completed = !item.completed;
            self.save();
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
        accent_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;
        let right = bounds.right;

        // Header on top left
        let header_rect = D2D_RECT_F {
            left: left + 24.0,
            top: top + 10.0,
            right: left + 150.0,
            bottom: top + 24.0,
        };
        dwrite.draw_text(&d2d.context, "✅ Daily Tasks", header_rect, dim_brush, &dwrite.format_small);

        // Completion count on top right
        let completed_count = self.items.iter().filter(|i| i.completed).count();
        let count_str = format!("{}/{} done", completed_count, self.items.len());
        let count_rect = D2D_RECT_F {
            left: right - 120.0,
            top: top + 10.0,
            right: right - 24.0,
            bottom: top + 24.0,
        };
        dwrite.draw_text(&d2d.context, &count_str, count_rect, dim_brush, &dwrite.format_small);

        // Render up to 3 task rows
        let row_start_y = top + 26.0;
        let row_h = 20.0;

        for (idx, item) in self.items.iter().take(3).enumerate() {
            let cur_y = row_start_y + (idx as f32 * (row_h + 3.0));

            // Checkbox box
            let box_rect = D2D_RECT_F {
                left: left + 24.0,
                top: cur_y + 2.0,
                right: left + 38.0,
                bottom: cur_y + 16.0,
            };
            d2d.fill_rounded_rect(box_rect, 3.0, 3.0, card_brush);

            if item.completed {
                d2d.fill_rounded_rect(box_rect, 3.0, 3.0, accent_brush);
                let check_rect = D2D_RECT_F {
                    left: box_rect.left,
                    top: box_rect.top - 1.0,
                    right: box_rect.right,
                    bottom: box_rect.bottom,
                };
                dwrite.draw_text(&d2d.context, "✓", check_rect, white_brush, &dwrite.format_center);
            }

            // Task Title
            let text_rect = D2D_RECT_F {
                left: box_rect.right + 10.0,
                top: cur_y,
                right: right - 24.0,
                bottom: cur_y + row_h,
            };
            let brush = if item.completed { dim_brush } else { white_brush };
            dwrite.draw_text(&d2d.context, &item.title, text_rect, brush, &dwrite.format_regular);
        }
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let row_start_y = top + 26.0;
        let row_h = 20.0;

        for (idx, item) in self.items.iter().take(3).enumerate() {
            let cur_y = row_start_y + (idx as f32 * (row_h + 3.0));
            if y >= cur_y && y <= cur_y + row_h && x >= left + 20.0 && x <= left + 45.0 {
                let id = item.id;
                self.toggle_item(id);
                return true;
            }
        }
        false
    }
}
