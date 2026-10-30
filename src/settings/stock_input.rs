use windows::core::{w, PCWSTR, Result};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, GetFocus, VK_CONTROL, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const FIELD:D2D_RECT_F=D2D_RECT_F{left:322.0,top:191.0,right:678.0,bottom:227.0};
pub const MARKET:D2D_RECT_F=D2D_RECT_F{left:688.0,top:191.0,right:814.0,bottom:227.0};
pub const ADD:D2D_RECT_F=D2D_RECT_F{left:824.0,top:191.0,right:914.0,bottom:227.0};
pub const PLACEHOLDER:&str="Ticker, e.g. BBCA.JK";
pub const GLOBAL_PLACEHOLDER:&str="Ticker, e.g. AAPL";
pub const WM_STOCK_INPUT:u32=WM_USER+124;
const EM_SETSEL:u32=0x00b1;
struct State {owner:HWND,original:WNDPROC,font:HFONT,brush:HBRUSH,dark:bool,idx:bool}

pub struct StockInput {pub hwnd:HWND,font:HFONT,brush:HBRUSH,family:String,dark:bool}

impl StockInput {
    pub fn new(owner:HWND,family:&str)->Result<Self> {
        unsafe {
            let hwnd=CreateWindowExW(WINDOW_EX_STYLE(0),w!("EDIT"),w!(""),WS_CHILD|WS_TABSTOP|WINDOW_STYLE(ES_AUTOHSCROLL as u32),0,0,1,1,owner,None,None,None)?;
            SendMessageW(hwnd,0x00c5,WPARAM(255),LPARAM(0));
            let cue:Vec<u16>=PLACEHOLDER.encode_utf16().chain(Some(0)).collect();
            SendMessageW(hwnd,0x1501,WPARAM(1),LPARAM(cue.as_ptr() as isize));
            let brush=CreateSolidBrush(Self::background(false));
            let original=SetWindowLongPtrW(hwnd,GWLP_WNDPROC,input_proc as *const () as isize);
            let state=Box::new(State{owner,original:std::mem::transmute::<isize,WNDPROC>(original),font:HFONT::default(),brush,dark:false,idx:true});
            SetWindowLongPtrW(hwnd,GWLP_USERDATA,Box::into_raw(state) as isize);
            let mut input=Self{hwnd,font:HFONT::default(),brush,family:String::new(),dark:false};
            input.update(false,family);
            Ok(input)
        }
    }
    fn background(dark:bool)->COLORREF {COLORREF(if dark{0x00262323}else{0x00ffffff})}
    pub fn focused(&self)->bool {unsafe{GetFocus()==self.hwnd}}
    pub fn set_market(&self,idx:bool) {
        unsafe {
            let ptr=GetWindowLongPtrW(self.hwnd,GWLP_USERDATA) as *mut State;
            (*ptr).idx=idx;
            let hint=if idx{PLACEHOLDER}else{GLOBAL_PLACEHOLDER};
            let cue:Vec<u16>=hint.encode_utf16().chain(Some(0)).collect();
            SendMessageW(self.hwnd,0x1501,WPARAM(1),LPARAM(cue.as_ptr() as isize));
            let _=InvalidateRect(self.hwnd,None,false);
        }
    }
    pub fn update(&mut self,dark:bool,family:&str) {
        unsafe {
            let ptr=GetWindowLongPtrW(self.hwnd,GWLP_USERDATA) as *mut State;
            if self.dark!=dark {
                let brush=CreateSolidBrush(Self::background(dark));
                if !brush.is_invalid() {(*ptr).brush=brush;let _=DeleteObject(self.brush);self.brush=brush;}
                self.dark=dark;(*ptr).dark=dark;
            }
            if self.family!=family {
                let name:Vec<u16>=family.encode_utf16().chain(Some(0)).collect();
                let font=CreateFontW(-13,0,0,0,400,0,0,0,DEFAULT_CHARSET.0 as u32,OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,CLEARTYPE_QUALITY.0 as u32,DEFAULT_PITCH.0 as u32,PCWSTR(name.as_ptr()));
                if !font.is_invalid() {
                    (*ptr).font=font;SendMessageW(self.hwnd,WM_SETFONT,WPARAM(font.0 as usize),LPARAM(1));
                    let _=DeleteObject(self.font);self.font=font;self.family=family.into();
                }
            }
            let dc=GetDC(self.hwnd);let previous=SelectObject(dc,self.font);
            let mut metrics=TEXTMETRICW::default();let _=GetTextMetricsW(dc,&mut metrics);
            SelectObject(dc,previous);ReleaseDC(self.hwnd,dc);
            let height=metrics.tmHeight.clamp(16,26);
            let _=SetWindowPos(self.hwnd,None,FIELD.left as i32+12,FIELD.top as i32+(36-height)/2,
                (FIELD.right-FIELD.left) as i32-24,height,SWP_NOZORDER|SWP_NOACTIVATE);
            let _=InvalidateRect(self.hwnd,None,true);
        }
    }
    pub unsafe fn color(&self,dc:HDC)->LRESULT {
        SetBkColor(dc,Self::background(self.dark));
        SetTextColor(dc,COLORREF(if self.dark{0x00f5f5f5}else{0x002c2c2e}));
        LRESULT(self.brush.0 as isize)
    }
}
impl Drop for StockInput {
    fn drop(&mut self){unsafe{let _=DestroyWindow(self.hwnd);let _=DeleteObject(self.font);let _=DeleteObject(self.brush);}}
}
unsafe extern "system" fn input_proc(hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM)->LRESULT {
    let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *mut State;
    if ptr.is_null(){return DefWindowProcW(hwnd,msg,wp,lp);}
    let original=(*ptr).original;
    if msg==WM_PAINT && GetWindowTextLengthW(hwnd)==0 {
        let mut paint=PAINTSTRUCT::default();let dc=BeginPaint(hwnd,&mut paint);
        FillRect(dc,&paint.rcPaint,(*ptr).brush);let previous=SelectObject(dc,(*ptr).font);
        SetBkMode(dc,TRANSPARENT);SetTextColor(dc,COLORREF(if (*ptr).dark{0x00aaa6a6}else{0x00706c6c}));
        let hint=if (*ptr).idx{PLACEHOLDER}else{GLOBAL_PLACEHOLDER};
        let text:Vec<u16>=hint.encode_utf16().collect();let _=TextOutW(dc,0,0,&text);
        if GetFocus()==hwnd {let _=SetCaretPos(0,0);}
        SelectObject(dc,previous);let _=EndPaint(hwnd,&paint);return LRESULT(0);
    }
    if msg==WM_KEYDOWN {
        let action=match wp.0 {13=>Some(1),9=>Some(if GetAsyncKeyState(VK_SHIFT.0 as i32)<0{3}else{2}),27=>Some(5),_=>None};
        if let Some(action)=action {let _=PostMessageW((*ptr).owner,WM_STOCK_INPUT,WPARAM(action),LPARAM(0));return LRESULT(0);}
        if wp.0==65 && GetAsyncKeyState(VK_CONTROL.0 as i32)<0 {SendMessageW(hwnd,EM_SETSEL,WPARAM(0),LPARAM(-1));return LRESULT(0);}
    }
    if msg==WM_CHAR && matches!(wp.0,9|13|27){return LRESULT(0);}
    if msg==WM_SETFOCUS || msg==WM_KILLFOCUS {let _=PostMessageW((*ptr).owner,WM_STOCK_INPUT,WPARAM(4),LPARAM(0));}
    let result=CallWindowProcW(original,hwnd,msg,wp,lp);
    if msg==WM_NCDESTROY {SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);drop(Box::from_raw(ptr));}
    result
}
