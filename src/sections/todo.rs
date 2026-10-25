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

        self.items = vec![
            TodoItem {
                id: 1,
                title: "Complete Nocan Living Notch in Rust".to_string(),
                completed: true,
            },
            TodoItem {
                id: 2,
                title: "Integrate OpenMausBot Usage Section".to_string(),
                completed: true,
            },
            TodoItem {
                id: 3,
                title: "Check Shelf & Media controls".to_string(),
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
        accent_brush: &ID2D1SolidColorBrush,
        check_bg_brush: &ID2D1SolidColorBrush,
    ) {
        let left = bounds.left;
        let top = bounds.top;

        // Section Title: "TodoList"
        let title_rect = D2D_RECT_F {
            left: left + 24.0,
            top: top + 10.0,
            right: left + 140.0,
            bottom: top + 26.0,
        };
        dwrite.draw_text(&d2d.context, "TodoList", title_rect, white_brush, &dwrite.format_title);

        // Completion Counter on Right
        let completed_count = self.items.iter().filter(|i| i.completed).count();
        let count_str = format!("Done Today: {}/{}", completed_count, self.items.len());
        let count_rect = D2D_RECT_F {
            left: left + 250.0,
            top: top + 10.0,
            right: left + 395.0,
            bottom: top + 26.0,
        };
        dwrite.draw_text(&d2d.context, &count_str, count_rect, dim_brush, &dwrite.format_small);

        // Render task rows
        let row_start_y = top + 34.0;
        let row_h = 24.0;

        for (idx, item) in self.items.iter().take(3).enumerate() {
            let cur_y = row_start_y + (idx as f32 * (row_h + 4.0));

            // Circular Checkbox
            let center_x = left + 50.0;
            let center_y = cur_y + 10.0;

            if item.completed {
                d2d.fill_ellipse(center_x, center_y, 8.0, 8.0, accent_brush);
                let check_rect = D2D_RECT_F {
                    left: center_x - 8.0,
                    top: center_y - 8.0,
                    right: center_x + 8.0,
                    bottom: center_y + 8.0,
                };
                dwrite.draw_text(&d2d.context, "✓", check_rect, white_brush, &dwrite.format_center);
            } else {
                d2d.fill_ellipse(center_x, center_y, 8.0, 8.0, check_bg_brush);
            }

            // Task Title
            let text_rect = D2D_RECT_F {
                left: center_x + 14.0,
                top: cur_y,
                right: left + 380.0,
                bottom: cur_y + row_h,
            };
            let brush = if item.completed { dim_brush } else { white_brush };
            dwrite.draw_text(&d2d.context, &item.title, text_rect, brush, &dwrite.format_regular);
        }
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        let row_start_y = top + 34.0;
        let row_h = 24.0;

        for (idx, item) in self.items.iter().take(3).enumerate() {
            let cur_y = row_start_y + (idx as f32 * (row_h + 4.0));
            if y >= cur_y - 2.0 && y <= cur_y + row_h && x >= left + 35.0 && x <= left + 75.0 {
                let id = item.id;
                self.toggle_item(id);
                return true;
            }
        }
        false
    }
}
