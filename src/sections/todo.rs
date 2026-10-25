use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use windows::core::PCWSTR;
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
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
                    if !parsed.is_empty() {
                        self.items = parsed;
                        return;
                    }
                }
            }
        }

        self.items = vec![
            TodoItem {
                id: 1,
                title: "Revise the report".to_string(),
                completed: true,
            },
            TodoItem {
                id: 2,
                title: "Reply to Dina".to_string(),
                completed: true,
            },
            TodoItem {
                id: 3,
                title: "Post weekly update".to_string(),
                completed: true,
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

    pub fn add_quick_task(&mut self) {
        let next_id = self.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        self.items.insert(
            0,
            TodoItem {
                id: next_id,
                title: format!("New task #{}", next_id),
                completed: false,
            },
        );
        self.save();
    }

    pub fn open_todo_file(&self) {
        let path_str = self.file_path.to_string_lossy();
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

        // 1. Section Title: "Todo" (#fff) + "List" (#38d36f) at left: 15px, top: 19px (vc)
        let title_w_rect = D2D_RECT_F {
            left: left + 15.0,
            top: top + 9.0,
            right: left + 55.0,
            bottom: top + 29.0,
        };
        dwrite.draw_text(&d2d.context, "Todo", title_w_rect, white_brush, &dwrite.format_title);

        if let Ok(green_title_b) = d2d.create_solid_brush(D2D1_COLOR_F {
            r: 56.0 / 255.0,
            g: 211.0 / 255.0,
            b: 111.0 / 255.0,
            a: 1.0,
        }) {
            let title_g_rect = D2D_RECT_F {
                left: left + 51.0,
                top: top + 9.0,
                right: left + 110.0,
                bottom: top + 29.0,
            };
            dwrite.draw_text(&d2d.context, "List", title_g_rect, &green_title_b, &dwrite.format_title);
        }

        // 2. Left Column: Counter "4/5" (30px bold) at left: 44px, top: 48px (vc)
        let completed_count = self.items.iter().filter(|i| i.completed).count();
        let total_count = self.items.len().max(1);
        let count_str = format!("{}/{}", completed_count, total_count);
        let count_rect = D2D_RECT_F {
            left: left + 44.0,
            top: top + 30.0,
            right: left + 150.0,
            bottom: top + 64.0,
        };
        dwrite.draw_text(&d2d.context, &count_str, count_rect, white_brush, &dwrite.format_big);

        // Subtitle: "Task Done Today" at left: 44px, top: 76px (vc)
        let sub_rect = D2D_RECT_F {
            left: left + 44.0,
            top: top + 67.0,
            right: left + 180.0,
            bottom: top + 85.0,
        };
        dwrite.draw_text(&d2d.context, "Task Done Today", sub_rect, dim_brush, &dwrite.format_sub);

        // 3. Add Task & Edit Task Pill Buttons at top: 93px, height: 28px, rx: 14px
        let btn_top = top + 93.0;
        let btn_bot = btn_top + 28.0;
        let btn_cy = btn_top + 14.0;

        // Add Task pill: left: 41px, width: 69px
        let add_rect = D2D_RECT_F {
            left: left + 41.0,
            top: btn_top,
            right: left + 110.0,
            bottom: btn_bot,
        };
        d2d.fill_rounded_rect(add_rect, 14.0, 14.0, pill_bg_brush);
        let plus_cx = left + 51.5;
        d2d.fill_ellipse(plus_cx, btn_cy, 5.2, 5.2, white_brush);
        if let Ok(black_b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) {
            d2d.draw_line(plus_cx - 2.4, btn_cy, plus_cx + 2.4, btn_cy, &black_b, 1.3, true);
            d2d.draw_line(plus_cx, btn_cy - 2.4, plus_cx, btn_cy + 2.4, &black_b, 1.3, true);
        }
        let add_txt_rect = D2D_RECT_F {
            left: left + 59.5,
            top: btn_top,
            right: left + 109.0,
            bottom: btn_bot,
        };
        dwrite.draw_text(&d2d.context, "Add Task", add_txt_rect, white_brush, &dwrite.format_sub);

        // Edit Task pill: left: 116px, width: 68px
        let edit_rect = D2D_RECT_F {
            left: left + 116.0,
            top: btn_top,
            right: left + 184.0,
            bottom: btn_bot,
        };
        d2d.fill_rounded_rect(edit_rect, 14.0, 14.0, pill_bg_brush);
        let pen_cx = left + 126.5;
        let pen_pts = [
            (pen_cx - 3.5, btn_cy + 3.5),
            (pen_cx - 3.5, btn_cy + 1.0),
            (pen_cx + 1.5, btn_cy - 4.0),
            (pen_cx + 4.0, btn_cy - 1.5),
            (pen_cx - 1.0, btn_cy + 3.5),
        ];
        d2d.draw_polyline(&pen_pts, white_brush, 1.2, true);
        let edit_txt_rect = D2D_RECT_F {
            left: left + 134.5,
            top: btn_top,
            right: left + 183.0,
            bottom: btn_bot,
        };
        dwrite.draw_text(&d2d.context, "Edit Task", edit_txt_rect, white_brush, &dwrite.format_sub);

        // 4. Right Column: 3 Task Rows (.trow) at left: 202px, width: 176px, height: 28px, rx: 9px
        let done_row_bg = d2d
            .create_solid_brush(D2D1_COLOR_F {
                r: 40.0 / 255.0,
                g: 120.0 / 255.0,
                b: 60.0 / 255.0,
                a: 0.38,
            })
            .ok();
        let chk_green_b = d2d
            .create_solid_brush(D2D1_COLOR_F {
                r: 47.0 / 255.0,
                g: 208.0 / 255.0,
                b: 107.0 / 255.0,
                a: 1.0,
            })
            .ok();
        let done_txt_b = d2d
            .create_solid_brush(D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.75,
            })
            .ok();

        let row_tops = [top + 34.0, top + 65.0, top + 96.0];
        for (idx, item) in self.items.iter().take(3).enumerate() {
            let ry = row_tops[idx];
            let r_rect = D2D_RECT_F {
                left: left + 198.0,
                top: ry,
                right: left + 373.0,
                bottom: ry + 28.0,
            };

            if item.completed {
                if let Some(bg) = &done_row_bg {
                    d2d.fill_rounded_rect(r_rect, 9.0, 9.0, bg);
                }
            } else {
                d2d.fill_rounded_rect(r_rect, 9.0, 9.0, pill_bg_brush);
            }

            let chk_cx = left + 213.0;
            let chk_cy = ry + 14.0;
            if item.completed {
                if let Some(gb) = &chk_green_b {
                    d2d.fill_ellipse(chk_cx, chk_cy, 8.0, 8.0, gb);
                }
                let chk_pts = [
                    (chk_cx - 3.0, chk_cy + 0.2),
                    (chk_cx - 1.0, chk_cy + 2.2),
                    (chk_cx + 3.2, chk_cy - 2.2),
                ];
                d2d.draw_polyline(&chk_pts, white_brush, 2.0, false);
            } else {
                d2d.draw_ellipse(chk_cx, chk_cy, 7.5, 7.5, dim_brush, 1.4);
            }

            let txt_left = left + 228.0;
            let txt_rect = D2D_RECT_F {
                left: txt_left,
                top: ry,
                right: left + 367.0,
                bottom: ry + 28.0,
            };
            let tb = if item.completed {
                done_txt_b.as_ref().unwrap_or(dim_brush)
            } else {
                white_brush
            };
            dwrite.draw_text(&d2d.context, &item.title, txt_rect, tb, &dwrite.format_button);

            if item.completed {
                let approx_w = ((item.title.chars().count() as f32) * 6.1).min(134.0);
                d2d.draw_line(
                    txt_left,
                    ry + 14.5,
                    txt_left + approx_w,
                    ry + 14.5,
                    tb,
                    1.2,
                    false,
                );
            }
        }

        // 5. Scrollbar indicator at left: 377px, top: 36px, width: 3px, height: 34px, rx: 1.5px
        if let Ok(sb_b) = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.40 }) {
            let sb_rect = D2D_RECT_F {
                left: left + 377.0,
                top: top + 36.0,
                right: left + 380.0,
                bottom: top + 70.0,
            };
            d2d.fill_rounded_rect(sb_rect, 1.5, 1.5, &sb_b);
        }
    }

    pub fn on_click(&mut self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;

        // Add Task / Edit Task buttons
        if y >= top + 91.0 && y <= top + 123.0 {
            if x >= left + 41.0 && x <= left + 110.0 {
                self.add_quick_task();
                return true;
            }
            if x >= left + 116.0 && x <= left + 184.0 {
                self.open_todo_file();
                return true;
            }
        }

        // Task rows at left + 202..378
        let row_tops = [top + 34.0, top + 65.0, top + 96.0];
        for (idx, item) in self.items.iter().take(3).enumerate() {
            let ry = row_tops[idx];
            if x >= left + 202.0 && x <= left + 378.0 && y >= ry && y <= ry + 28.0 {
                let id = item.id;
                self.toggle_item(id);
                return true;
            }
        }
        false
    }

    pub fn is_interactive(&self, bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
        let left = bounds.left;
        let top = bounds.top;
        if y >= top + 91.0 && y <= top + 123.0 {
            if (x >= left + 41.0 && x <= left + 110.0) || (x >= left + 116.0 && x <= left + 184.0) {
                return true;
            }
        }
        let row_tops = [top + 34.0, top + 65.0, top + 96.0];
        for idx in 0..self.items.len().min(3) {
            let ry = row_tops[idx];
            if x >= left + 202.0 && x <= left + 378.0 && y >= ry && y <= ry + 28.0 {
                return true;
            }
        }
        false
    }
}
