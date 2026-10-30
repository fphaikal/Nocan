use windows::core::{w, PCWSTR, Result};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, FillRect, GetDC, GetTextMetricsW, ReleaseDC, SelectObject, SetBkMode, TextOutW, CreateFontW, CreateSolidBrush, DeleteObject, SetBkColor, SetTextColor, HBRUSH, HDC, HFONT, PAINTSTRUCT, TEXTMETRICW, TRANSPARENT, DEFAULT_CHARSET, DEFAULT_PITCH, CLIP_DEFAULT_PRECIS, OUT_DEFAULT_PRECIS, CLEARTYPE_QUALITY};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, GetFocus, SetFocus, VK_CONTROL, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_TODO_EDITOR: u32 = WM_USER + 121;
const EM_SETSEL: u32 = 0x00b1;
const EM_SETLIMITTEXT: u32 = 0x00c5;
const EM_SETCUEBANNER: u32 = 0x1501;
pub const FIELD_LEFT: f32 = 54.0;
pub const FIELD_TOP: f32 = 108.0;
pub const FIELD_HEIGHT: f32 = 34.0;
pub const FIELD_PADDING: f32 = 12.0;
pub const FIELD_RGB: (u8,u8,u8) = (35,35,38);
const FIELD_COLOR: COLORREF = COLORREF(0x00262323);
struct InputState { owner: HWND, original: WNDPROC, font: HFONT, brush: HBRUSH }

pub struct TodoInput { pub hwnd: HWND, font: HFONT, family: String, pub line_height:i32, brush: HBRUSH, visible: bool, bounds: (i32,i32,i32,i32) }

impl TodoInput {
    pub fn new(owner: HWND, family: &str) -> Result<Self> {
        unsafe {
            let hwnd = CreateWindowExW(WS_EX_LAYERED, w!("EDIT"), w!(""),
                WS_CHILD | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32), 0,0,1,1,owner,None,None,None)?;
            SetLayeredWindowAttributes(hwnd,COLORREF(0),255,LWA_ALPHA)?;
            SendMessageW(hwnd,EM_SETLIMITTEXT,WPARAM(320),LPARAM(0));
            SendMessageW(hwnd,EM_SETCUEBANNER,WPARAM(1),LPARAM(w!("What needs to be done?").as_ptr() as isize));
            let brush=CreateSolidBrush(FIELD_COLOR);
            let original = SetWindowLongPtrW(hwnd,GWLP_WNDPROC,input_proc as *const () as isize);
            let state = Box::new(InputState { owner, original: std::mem::transmute::<isize,WNDPROC>(original),font:HFONT::default(),brush });
            SetWindowLongPtrW(hwnd,GWLP_USERDATA,Box::into_raw(state) as isize);
            let mut input=Self { hwnd,font:HFONT::default(),family:String::new(),line_height:20,brush,visible:false,bounds:(0,0,1,1) };
            input.set_font(family);
            Ok(input)
        }
    }
    pub fn set_font(&mut self, family:&str) {
        if self.family==family { return; }
        let name:Vec<u16>=family.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let font=CreateFontW(-13,0,0,0,400,0,0,0,DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,CLIP_DEFAULT_PRECIS.0 as u32,CLEARTYPE_QUALITY.0 as u32,
                DEFAULT_PITCH.0 as u32,PCWSTR(name.as_ptr()));
            if font.is_invalid() { return; }
            let ptr=GetWindowLongPtrW(self.hwnd,GWLP_USERDATA) as *mut InputState;
            (*ptr).font=font;
            SendMessageW(self.hwnd,WM_SETFONT,WPARAM(font.0 as usize),LPARAM(1));
            let dc=GetDC(self.hwnd);
            let previous=SelectObject(dc,font);
            let mut metrics=TEXTMETRICW::default();
            if GetTextMetricsW(dc,&mut metrics).as_bool() { self.line_height=metrics.tmHeight.clamp(16,26); }
            SelectObject(dc,previous); ReleaseDC(self.hwnd,dc);
            let _=DeleteObject(self.font);
            self.font=font;self.family=family.into();
        }
    }
    pub fn is_focused(&self)->bool { unsafe { GetFocus()==self.hwnd } }
    pub fn focus(&self) { unsafe { let _=SetFocus(self.hwnd); } }
    pub fn set_text(&self, text: &str) {
        let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        unsafe { let _=SetWindowTextW(self.hwnd,PCWSTR(text.as_ptr())); SendMessageW(self.hwnd,EM_SETSEL,WPARAM(0),LPARAM(-1)); }
    }
    pub fn text(&self) -> String {
        unsafe { let mut buf=vec![0u16;GetWindowTextLengthW(self.hwnd) as usize+1];
            let n=GetWindowTextW(self.hwnd,&mut buf); String::from_utf16_lossy(&buf[..n as usize]) }
    }
    pub fn sync(&mut self, visible: bool, bounds: (i32,i32,i32,i32)) {
        unsafe {
            if visible && self.bounds != bounds {
                let _=SetWindowPos(self.hwnd,None,bounds.0,bounds.1,bounds.2,bounds.3,SWP_NOZORDER|SWP_NOACTIVATE);
                self.bounds=bounds;
            }
            if visible != self.visible {
                let _=ShowWindow(self.hwnd,if visible { SW_SHOW } else { SW_HIDE });
                if visible { let _=SetFocus(self.hwnd); }
                else if GetFocus()==self.hwnd { let _=SetFocus(GetParent(self.hwnd).unwrap_or_default()); }
                self.visible=visible;
            }
        }
    }
    pub unsafe fn color(&self, dc: HDC) -> LRESULT {
        SetBkColor(dc,FIELD_COLOR); SetTextColor(dc,COLORREF(0x00f5f5f5));
        LRESULT(self.brush.0 as isize)
    }
}
impl Drop for TodoInput {
    fn drop(&mut self) { unsafe { let _=DestroyWindow(self.hwnd); let _=DeleteObject(self.font); let _=DeleteObject(self.brush); } }
}
unsafe extern "system" fn input_proc(hwnd: HWND,msg:u32,wp:WPARAM,lp:LPARAM)->LRESULT {
    let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *mut InputState;
    if ptr.is_null() { return DefWindowProcW(hwnd,msg,wp,lp); }
    let original=(*ptr).original;
    if msg==WM_PAINT && GetWindowTextLengthW(hwnd)==0 {
        let mut paint=PAINTSTRUCT::default();
        let dc=BeginPaint(hwnd,&mut paint);
        FillRect(dc,&paint.rcPaint,(*ptr).brush);
        let previous=SelectObject(dc,(*ptr).font);
        SetBkMode(dc,TRANSPARENT); SetTextColor(dc,COLORREF(0x00a8a8ad));
        let hint:Vec<u16>="What needs to be done?".encode_utf16().collect();
        let _=TextOutW(dc,0,0,&hint);
        SelectObject(dc,previous); let _=EndPaint(hwnd,&paint);
        return LRESULT(0);
    }
    if msg==WM_KEYDOWN {
        let action=match wp.0 { 13=>Some(1),27=>Some(2),9=>Some(if GetAsyncKeyState(VK_SHIFT.0 as i32)<0 {4} else {3}),_=>None };
        if let Some(action)=action { let _=PostMessageW((*ptr).owner,WM_TODO_EDITOR,WPARAM(action),LPARAM(0)); return LRESULT(0); }
        if wp.0==65 && GetAsyncKeyState(VK_CONTROL.0 as i32)<0 {
            SendMessageW(hwnd,EM_SETSEL,WPARAM(0),LPARAM(-1)); return LRESULT(0);
        }
    }
    if msg==WM_CHAR && matches!(wp.0,9|13|27) { return LRESULT(0); }
    let result=CallWindowProcW(original,hwnd,msg,wp,lp);
    if msg==WM_NCDESTROY { SetWindowLongPtrW(hwnd,GWLP_USERDATA,0); drop(Box::from_raw(ptr)); }
    result
}
