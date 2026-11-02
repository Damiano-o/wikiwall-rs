//! The handful of Win32 declarations WikiWall needs.
//!
//! Declaring them by hand keeps the build free of binding crates (whose build
//! scripts need `dlltool` on the GNU toolchain) and keeps the binary small.

#![allow(non_snake_case, dead_code)]

pub type Hwnd = isize;
pub type Hinstance = isize;
pub type Hicon = isize;
pub type Hmenu = isize;
pub type Handle = isize;
pub type Hbitmap = isize;
pub type Hdc = isize;
pub type Lresult = isize;
pub type Wndproc = extern "system" fn(Hwnd, u32, usize, isize) -> Lresult;

pub const WS_POPUP: u32 = 0x8000_0000;
pub const WS_EX_LAYERED: u32 = 0x0008_0000;
pub const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
pub const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
pub const WS_EX_TOPMOST: u32 = 0x0000_0008;
pub const LWA_ALPHA: u32 = 0x0000_0002;
pub const HWND_TOPMOST: Hwnd = -1;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const SWP_SHOWWINDOW: u32 = 0x0040;
pub const SWP_NOOWNERZORDER: u32 = 0x0200;
pub const SW_HIDE: i32 = 0;
pub const SW_SHOWNOACTIVATE: i32 = 4;
pub const GWLP_USERDATA: i32 = -21;
pub const IDC_HAND: usize = 32649;
pub const IMAGE_CURSOR: u32 = 2;

pub const WM_DESTROY: u32 = 0x0002;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_SETCURSOR: u32 = 0x0020;
pub const WM_QUIT: u32 = 0x0012;
pub const WM_NULL: u32 = 0x0000;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_LBUTTONDBLCLK: u32 = 0x0203;
pub const WM_CONTEXTMENU: u32 = 0x007B;
pub const WM_APP: u32 = 0x8000;
pub const WM_HOTKEY: u32 = 0x0312;
pub const PM_REMOVE: u32 = 0x0001;
pub const QS_ALLINPUT: u32 = 0x04FF;
pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_NOREPEAT: u32 = 0x4000;
pub const TPM_RETURNCMD: u32 = 0x0100;
pub const TPM_RIGHTBUTTON: u32 = 0x0002;
pub const TPM_NONOTIFY: u32 = 0x0080;
pub const MF_STRING: u32 = 0x0000;
pub const MF_SEPARATOR: u32 = 0x0800;
pub const MF_CHECKED: u32 = 0x0008;
pub const MF_UNCHECKED: u32 = 0x0000;
pub const MF_GRAYED: u32 = 0x0001;
pub const NIF_MESSAGE: u32 = 0x0001;
pub const NIF_ICON: u32 = 0x0002;
pub const NIF_TIP: u32 = 0x0004;
pub const NIM_ADD: u32 = 0x0000;
pub const NIM_MODIFY: u32 = 0x0001;
pub const NIM_DELETE: u32 = 0x0002;
pub const DIB_RGB_COLORS: u32 = 0;
pub const BI_RGB: u32 = 0;
pub const INFINITE: u32 = 0xFFFF_FFFF;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct Msg {
    pub hwnd: Hwnd,
    pub message: u32,
    pub w_param: usize,
    pub l_param: isize,
    pub time: u32,
    pub pt: Point,
}

#[repr(C)]
pub struct WndClassW {
    pub style: u32,
    pub lpfn_wnd_proc: Option<Wndproc>,
    pub cb_cls_extra: i32,
    pub cb_wnd_extra: i32,
    pub h_instance: Hinstance,
    pub h_icon: Hicon,
    pub h_cursor: isize,
    pub hbr_background: isize,
    pub lpsz_menu_name: *const u16,
    pub lpsz_class_name: *const u16,
}

#[repr(C)]
pub struct NotifyIconDataW {
    pub cb_size: u32,
    pub h_wnd: Hwnd,
    pub u_id: u32,
    pub u_flags: u32,
    pub u_callback_message: u32,
    pub h_icon: Hicon,
    pub sz_tip: [u16; 128],
    pub dw_state: u32,
    pub dw_state_mask: u32,
    pub sz_info: [u16; 256],
    pub u_version: u32,
    pub sz_info_title: [u16; 64],
    pub dw_info_flags: u32,
    pub guid_item: [u8; 16],
    pub h_balloon_icon: Hicon,
}

#[repr(C)]
pub struct IconInfo {
    pub f_icon: i32,
    pub x_hotspot: u32,
    pub y_hotspot: u32,
    pub hbm_mask: Hbitmap,
    pub hbm_color: Hbitmap,
}

#[repr(C)]
pub struct BitmapInfoHeader {
    pub bi_size: u32,
    pub bi_width: i32,
    pub bi_height: i32,
    pub bi_planes: u16,
    pub bi_bit_count: u16,
    pub bi_compression: u32,
    pub bi_size_image: u32,
    pub bi_x_pels_per_meter: i32,
    pub bi_y_pels_per_meter: i32,
    pub bi_clr_used: u32,
    pub bi_clr_important: u32,
}

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassW(class: *const WndClassW) -> u16;
    pub fn CreateWindowExW(
        ex_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        menu: Hmenu,
        instance: Hinstance,
        param: *mut std::ffi::c_void,
    ) -> Hwnd;
    pub fn DefWindowProcW(hwnd: Hwnd, message: u32, w_param: usize, l_param: isize) -> Lresult;
    pub fn DestroyWindow(hwnd: Hwnd) -> i32;
    pub fn ShowWindow(hwnd: Hwnd, command: i32) -> i32;
    pub fn SetWindowPos(
        hwnd: Hwnd,
        insert_after: Hwnd,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    pub fn SetLayeredWindowAttributes(hwnd: Hwnd, color_key: u32, alpha: u8, flags: u32) -> i32;
    pub fn GetMessageW(message: *mut Msg, hwnd: Hwnd, min: u32, max: u32) -> i32;
    pub fn PeekMessageW(message: *mut Msg, hwnd: Hwnd, min: u32, max: u32, remove: u32) -> i32;
    pub fn TranslateMessage(message: *const Msg) -> i32;
    pub fn DispatchMessageW(message: *const Msg) -> Lresult;
    pub fn PostMessageW(hwnd: Hwnd, message: u32, w_param: usize, l_param: isize) -> i32;
    pub fn PostQuitMessage(code: i32);
    pub fn PostThreadMessageW(thread_id: u32, message: u32, w_param: usize, l_param: isize) -> i32;
    pub fn RegisterHotKey(hwnd: Hwnd, id: i32, modifiers: u32, key: u32) -> i32;
    pub fn UnregisterHotKey(hwnd: Hwnd, id: i32) -> i32;
    pub fn GetCursorPos(point: *mut Point) -> i32;
    pub fn SetForegroundWindow(hwnd: Hwnd) -> i32;
    pub fn SetCursor(cursor: isize) -> isize;
    pub fn LoadCursorW(instance: Hinstance, name: usize) -> isize;
    pub fn SetWindowLongPtrW(hwnd: Hwnd, index: i32, value: isize) -> isize;
    pub fn GetWindowLongPtrW(hwnd: Hwnd, index: i32) -> isize;
    pub fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
    pub fn WindowFromPoint(point: Point) -> Hwnd;
    pub fn CreatePopupMenu() -> Hmenu;
    pub fn AppendMenuW(menu: Hmenu, flags: u32, id: usize, text: *const u16) -> i32;
    pub fn DestroyMenu(menu: Hmenu) -> i32;
    pub fn TrackPopupMenu(
        menu: Hmenu,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        hwnd: Hwnd,
        rect: *const Rect,
    ) -> i32;
    pub fn CreateIconIndirect(info: *const IconInfo) -> Hicon;
    pub fn DestroyIcon(icon: Hicon) -> i32;
    pub fn GetSystemMetrics(index: i32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(name: *const u16) -> Hinstance;
    pub fn GetCurrentThreadId() -> u32;
    pub fn CreateEventW(
        attributes: *mut std::ffi::c_void,
        manual_reset: i32,
        initial_state: i32,
        name: *const u16,
    ) -> Handle;
    pub fn SetEvent(handle: Handle) -> i32;
    pub fn CloseHandle(handle: Handle) -> i32;
    pub fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
    pub fn Sleep(milliseconds: u32);
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(dc: Hdc) -> Hdc;
    pub fn DeleteDC(dc: Hdc) -> i32;
    pub fn CreateDIBSection(
        dc: Hdc,
        header: *const BitmapInfoHeader,
        usage: u32,
        bits: *mut *mut std::ffi::c_void,
        section: Handle,
        offset: u32,
    ) -> Hbitmap;
    pub fn CreateBitmap(
        width: i32,
        height: i32,
        planes: u32,
        bits: u32,
        data: *const std::ffi::c_void,
    ) -> Hbitmap;
    pub fn DeleteObject(object: isize) -> i32;
    pub fn SelectObject(dc: Hdc, object: isize) -> isize;
}

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(message: u32, data: *const NotifyIconDataW) -> i32;
}

#[link(name = "shcore")]
extern "system" {
    pub fn SetProcessDpiAwareness(value: u32) -> i32;
}

// Waitable-object + message pump, used by the overlay and the tray thread.
#[link(name = "user32")]
extern "system" {
    pub fn MsgWaitForMultipleObjects(
        count: u32,
        handles: *const Handle,
        wait_all: i32,
        milliseconds: u32,
        wake_mask: u32,
    ) -> u32;
}

pub fn wide(text: &str) -> Vec<u16> {
    let mut buffer: Vec<u16> = text.encode_utf16().collect();
    buffer.push(0);
    buffer
}
