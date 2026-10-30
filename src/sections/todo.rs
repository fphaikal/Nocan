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
    pub storage_error: Option<String>,
    read_failed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TodoEditing { New, Existing(usize) }

pub const PAGE_SIZE: usize = 3;
pub const EXPANDED_SIZE: (f32, f32) = (520.0, 232.0);

pub fn page_start(offset: usize, count: usize) -> usize {
    (offset / PAGE_SIZE).min(count.saturating_sub(1) / PAGE_SIZE) * PAGE_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SERIAL: AtomicUsize = AtomicUsize::new(0);
    fn fixture() -> TodoSection {
        let dir=std::env::temp_dir().join(format!("nocan-todo-{}-{}",std::process::id(),SERIAL.fetch_add(1,Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        TodoSection::from_file(dir.join("todo.json"))
    }
    fn clean(s:TodoSection) { std::fs::remove_dir_all(s.file_path.parent().unwrap()).unwrap(); }

    #[test]
    fn create_edit_complete_delete_and_restore_survive_reload() {
        let mut s=fixture();
        let first=s.add_task("  Pelajari 日本語 💧  ").unwrap();
        let second=s.add_task("Tugas kedua").unwrap();
        s.toggle_task(first).unwrap();
        s.edit_task(first,"Judul diperbarui").unwrap();
        assert!(s.items[1].completed);
        let removed=s.delete_task(first).unwrap();
        assert_eq!(s.items[0].id,second);
        s.restore_task(removed.0,removed.1).unwrap();
        let s=TodoSection::from_file(s.file_path.clone());
        assert_eq!(s.items.len(),2);
        assert_eq!(s.items[1].id,first);
        assert_eq!(s.items[1].title,"Judul diperbarui");
        assert!(s.items[1].completed);
        clean(s);
    }
    #[test]
    fn validation_and_failed_storage_keep_previous_items() {
        let mut s=fixture();
        assert!(s.add_task(" \n ").is_err());
        assert!(s.add_task(&"水".repeat(161)).is_err());
        let id=s.add_task(&"水".repeat(160)).unwrap();
        assert!(s.edit_task(id," ").is_err());
        assert_eq!(s.items[0].title.chars().count(),160);
        let real_path=s.file_path.clone();
        let blocker=s.file_path.parent().unwrap().join("blocker");
        std::fs::write(&blocker,b"keep").unwrap();
        s.file_path=blocker.join("todo.json");
        assert!(s.add_task("Gagal").is_err());
        assert!(s.delete_task(id).is_err());
        assert!(s.toggle_task(id).is_err());
        assert!(s.edit_task(id,"Gagal").is_err());
        assert_eq!(s.items.len(),1); assert!(!s.items[0].completed);
        assert_eq!(s.items[0].title.chars().count(),160);
        s.file_path=real_path;clean(s);
    }

    #[test]
    fn missing_todo_file_is_not_created_or_saved_as_empty_during_load() {
        let mut s=fixture();
        assert!(!s.file_path.exists());
        assert!(s.storage_error.is_none());
        s.items.push(TodoItem { id:42,title:"In memory".into(),completed:false });
        s.load();
        assert_eq!(s.items[0].id,42);
        assert!(!s.file_path.exists());
        clean(s);
    }

    #[test]
    fn corrupt_todo_without_backup_preserves_bytes_and_in_memory_tasks() {
        let mut s=fixture();
        let id=s.add_task("Tetap tersimpan").unwrap();
        let corrupt=b"[{\"id\":1,\"title\":\"unfinished";
        std::fs::write(&s.file_path,corrupt).unwrap();
        s.load();
        assert_eq!(s.items[0].title,"Tetap tersimpan");
        assert!(s.storage_error.is_some());
        let loaded=TodoSection::from_file(s.file_path.clone());
        assert!(loaded.items.is_empty());
        assert!(loaded.storage_error.is_some());
        assert!(s.add_task("Tidak boleh menimpa").is_err());
        assert!(s.edit_task(id,"Tidak boleh menimpa").is_err());
        assert!(s.delete_task(id).is_err());
        s.toggle_item(id);s.add_quick_task();s.save();
        assert_eq!(s.items.len(),1);
        assert!(!s.items[0].completed);
        assert_eq!(std::fs::read(&s.file_path).unwrap(),corrupt);
        clean(s);
    }

    #[test]
    fn valid_backup_recovers_tasks_after_update_and_allows_safe_new_changes() {
        let mut s=fixture();
        let id=s.add_task("Catatan 💧").unwrap();
        s.toggle_task(id).unwrap();
        s.edit_task(id,"Judul versi terbaru").unwrap();
        let backup=crate::storage::backup_path(&s.file_path);
        let backup_bytes=std::fs::read(&backup).unwrap();
        std::fs::write(&s.file_path,b"broken").unwrap();
        let mut reopened=TodoSection::from_file(s.file_path.clone());
        assert_eq!(reopened.items[0].title,"Catatan 💧");
        assert!(reopened.items[0].completed);
        assert_eq!(std::fs::read(&s.file_path).unwrap(),b"broken");
        reopened.add_task("Sesudah pemulihan").unwrap();
        assert_eq!(std::fs::read(&backup).unwrap(),backup_bytes);
        let reopened=TodoSection::from_file(s.file_path.clone());
        assert_eq!(reopened.items.len(),2);
        assert!(reopened.items[1].completed);
        clean(s);
    }
    #[test]
    fn repairing_a_file_cannot_be_overwritten_by_an_unloaded_empty_list() {
        let s=fixture();let path=s.file_path.clone();
        std::fs::write(&path,"{broken").unwrap();
        let mut unloaded=TodoSection::from_file(path.clone());
        let repaired=br#"[{"id":7,"title":"Recovered note","completed":true}]"#;
        std::fs::write(&path,repaired).unwrap();
        assert!(unloaded.add_task("New note").is_err());unloaded.save();
        assert_eq!(std::fs::read(&path).unwrap(),repaired);
        unloaded.load();assert_eq!(unloaded.items.len(),1);
        assert!(unloaded.add_task("New note").is_ok());assert_eq!(unloaded.items.len(),2);
        clean(s);
    }

    #[test]
    fn failed_backup_write_does_not_change_tasks_or_existing_file() {
        let mut s=fixture();let id=s.add_task("Tetap utuh").unwrap();
        let original=std::fs::read(&s.file_path).unwrap();
        std::fs::create_dir(crate::storage::backup_path(&s.file_path)).unwrap();
        assert!(s.delete_task(id).is_err());
        assert!(s.storage_error.is_some());
        s.toggle_item(id);s.add_quick_task();
        assert_eq!(s.items.len(),1);assert!(!s.items[0].completed);
        assert_eq!(std::fs::read(&s.file_path).unwrap(),original);
        clean(s);
    }
    #[test]
    fn last_page_clamps_after_deletion() {
        assert_eq!(page_start(99,0),0);
        assert_eq!(page_start(3,4),3);
        assert_eq!(page_start(3,3),0);
        assert_eq!(page_start(9,7),6);
    }
}

impl TodoSection {
    pub fn new() -> Self {
        Self::from_file(crate::storage::data_dir().join("todo.json"))
    }

    pub fn from_file(file_path: PathBuf) -> Self {
        let mut section = Self {
            items: Vec::new(),
            file_path,
            storage_error: None,
            read_failed: false,
        };
        section.load();
        section
    }

    pub fn load(&mut self) {
        match crate::storage::load_json::<Vec<TodoItem>>(&self.file_path) {
            Ok(Some(items)) => { self.items = items; self.storage_error = None; self.read_failed=false; }
            Ok(None) => { self.storage_error = None; self.read_failed=false; }
            Err(error) => { self.storage_error = Some(error); self.read_failed=true; }
        }
    }

    pub fn save(&self) {
        if self.read_failed {return;}
        if let Err(error) = crate::storage::save_json(&self.file_path, &self.items) {
            eprintln!("{}", error);
        }
    }

    fn commit(&mut self, items: Vec<TodoItem>) -> Result<(), String> {
        if self.read_failed {return Err("Notes could not be loaded. Reopen the app after restoring the file.".into());}
        if let Err(error) = crate::storage::save_json(&self.file_path, &items) {
            self.storage_error = Some(error.clone());
            return Err(error);
        }
        self.items = items;
        self.storage_error = None;
        Ok(())
    }

    fn checked_title(title: &str) -> Result<String, String> {
        let title = title.trim();
        if title.is_empty() { return Err("Task title cannot be empty.".into()); }
        if title.chars().count() > 160 { return Err("Task title must be 160 characters or fewer.".into()); }
        Ok(title.to_string())
    }

    pub fn add_task(&mut self, title: &str) -> Result<usize, String> {
        let title = Self::checked_title(title)?;
        let id = self.items.iter().map(|i| i.id).max().unwrap_or(0).checked_add(1)
            .ok_or_else(|| "Could not create a new task.".to_string())?;
        let mut items = self.items.clone();
        items.insert(0, TodoItem { id, title, completed: false });
        self.commit(items)?;
        Ok(id)
    }

    pub fn edit_task(&mut self, id: usize, title: &str) -> Result<(), String> {
        let title = Self::checked_title(title)?;
        let mut items = self.items.clone();
        let item = items.iter_mut().find(|i| i.id == id).ok_or_else(|| "Task not found.".to_string())?;
        item.title = title;
        self.commit(items)
    }

    pub fn delete_task(&mut self, id: usize) -> Result<(TodoItem, usize), String> {
        let mut items = self.items.clone();
        let index = items.iter().position(|i| i.id == id).ok_or_else(|| "Task not found.".to_string())?;
        let removed = items.remove(index);
        self.commit(items)?;
        Ok((removed, index))
    }

    pub fn restore_task(&mut self, item: TodoItem, index: usize) -> Result<(), String> {
        if self.items.iter().any(|i| i.id == item.id) { return Err("Task already exists.".into()); }
        let mut items = self.items.clone();
        items.insert(index.min(items.len()), item);
        self.commit(items)
    }

    pub fn toggle_task(&mut self, id: usize) -> Result<(), String> {
        let mut items = self.items.clone();
        let item = items.iter_mut().find(|i| i.id == id).ok_or_else(|| "Task not found.".to_string())?;
        item.completed = !item.completed;
        self.commit(items)
    }

    pub fn toggle_item(&mut self, id: usize) {
        let _ = self.toggle_task(id);
    }

    pub fn add_quick_task(&mut self) {
        if let Some(next_id) = self.items.iter().map(|i| i.id).max().unwrap_or(0).checked_add(1) {
            let _ = self.add_task(&format!("New task #{}", next_id));
        }
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
