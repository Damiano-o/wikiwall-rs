//! Tray icon with the same menu as the Python version, drawn at runtime.

use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

use crate::watcher::Watcher;
use crate::win32::*;

const CLASS_NAME: &str = "WikiWallTray";
const CALLBACK_MESSAGE: u32 = WM_APP + 1;

const CMD_CHANGE: usize = 1;
const CMD_SKIP: usize = 2;
const CMD_OPEN: usize = 3;
const CMD_TOGGLE_QR: usize = 4;
const CMD_PAUSE: usize = 5;
const CMD_EXIT: usize = 6;

static WATCHER: OnceLock<Mutex<Option<Arc<Watcher>>>> = OnceLock::new();

fn watcher_slot() -> &'static Mutex<Option<Arc<Watcher>>> {
    WATCHER.get_or_init(|| Mutex::new(None))
}

fn current_watcher() -> Option<Arc<Watcher>> {
    watcher_slot().lock().ok().and_then(|slot| slot.clone())
}

extern "system" fn tray_proc(hwnd: Hwnd, message: u32, w_param: usize, l_param: isize) -> Lresult {
    if message == CALLBACK_MESSAGE {
        let mouse = (l_param as u32) & 0xFFFF;
        if mouse == WM_LBUTTONUP || mouse == WM_RBUTTONUP || mouse == WM_LBUTTONDBLCLK {
            show_menu(hwnd);
        }
        return 0;
    }
    unsafe { DefWindowProcW(hwnd, message, w_param, l_param) }
}

fn menu_item(menu: Hmenu, flags: u32, id: usize, text: &str) {
    let text = wide(text);
    unsafe {
        AppendMenuW(menu, flags | MF_STRING, id, text.as_ptr());
    }
}

fn show_menu(hwnd: Hwnd) {
    let Some(watcher) = current_watcher() else {
        return;
    };
    unsafe {
        let menu = CreatePopupMenu();
        if menu == 0 {
            return;
        }
        menu_item(menu, MF_STRING, CMD_CHANGE, "Cambia ora");
        menu_item(menu, MF_STRING, CMD_SKIP, "Salta voce");
        menu_item(menu, MF_STRING, CMD_OPEN, "Apri voce");
        menu_item(
            menu,
            MF_STRING
                | if watcher.clickable_qr_flag() {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                },
            CMD_TOGGLE_QR,
            "QR e link cliccabili",
        );
        menu_item(
            menu,
            MF_STRING
                | if watcher.paused() {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                },
            CMD_PAUSE,
            "Pausa",
        );
        {
            let text = wide("");
            AppendMenuW(menu, MF_SEPARATOR, 0, text.as_ptr());
        }
        menu_item(menu, MF_STRING, CMD_EXIT, "Esci");

        let mut point = Point::default();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        // Windows keeps the popup on screen until the next message arrives.
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);

        match command as usize {
            CMD_CHANGE => watcher.request_change(),
            CMD_SKIP => watcher.skip_async(),
            CMD_OPEN => watcher.open_current(),
            CMD_TOGGLE_QR => watcher.toggle_clickable_qr(),
            CMD_PAUSE => watcher.toggle_pause(),
            CMD_EXIT => watcher.stop(),
            _ => {}
        }
    }
}

pub struct Tray {
    thread: Option<JoinHandle<()>>,
    stop_event: Handle,
}

unsafe impl Send for Tray {}

impl Tray {
    pub fn start(watcher: Arc<Watcher>) -> Self {
        if let Ok(mut slot) = watcher_slot().lock() {
            *slot = Some(watcher);
        }
        let stop_event = unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null()) };
        let thread = {
            let stop_event = stop_event;
            std::thread::Builder::new()
                .name("wikiwall-tray".into())
                .spawn(move || run(stop_event))
                .ok()
        };
        Self { thread, stop_event }
    }

    pub fn stop(&mut self) {
        unsafe {
            SetEvent(self.stop_event);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        unsafe {
            CloseHandle(self.stop_event);
        }
        if let Ok(mut slot) = watcher_slot().lock() {
            *slot = None;
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(stop_event: Handle) {
    unsafe {
        let class_name = wide(CLASS_NAME);
        let class = WndClassW {
            style: 0,
            lpfn_wnd_proc: Some(tray_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: GetModuleHandleW(std::ptr::null()),
            h_icon: 0,
            h_cursor: 0,
            hbr_background: 0,
            lpsz_menu_name: std::ptr::null(),
            lpsz_class_name: class_name.as_ptr(),
        };
        RegisterClassW(&class);

        let instance = GetModuleHandleW(std::ptr::null());
        let title = wide("WikiWall");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            0,
            0,
            instance,
            std::ptr::null_mut(),
        );
        if hwnd == 0 {
            crate::log_error!("Tray non disponibile: finestra non creata");
            return;
        }

        let icon = make_icon();
        let mut data: NotifyIconDataW = std::mem::zeroed();
        data.cb_size = std::mem::size_of::<NotifyIconDataW>() as u32;
        data.h_wnd = hwnd;
        data.u_id = 1;
        data.u_flags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.u_callback_message = CALLBACK_MESSAGE;
        data.h_icon = icon;
        for (slot, ch) in data.sz_tip.iter_mut().zip("WikiWall".encode_utf16()) {
            *slot = ch;
        }
        if Shell_NotifyIconW(NIM_ADD, &data) == 0 {
            crate::log_error!("Tray non disponibile: icona non aggiunta");
        } else {
            crate::log_info!("Icona nella tray avviata");
        }

        loop {
            let mut message: Msg = std::mem::zeroed();
            while PeekMessageW(&mut message, 0, 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            let handles = [stop_event];
            let index = MsgWaitForMultipleObjects(1, handles.as_ptr(), 0, 150, QS_ALLINPUT);
            if index == 0 || current_watcher().map(|w| w.stopped()).unwrap_or(true) {
                break;
            }
        }

        Shell_NotifyIconW(NIM_DELETE, &data);
        DestroyIcon(icon);
        DestroyWindow(hwnd);
        crate::log_info!("Icona nella tray rimossa");
    }
}

/// A 32x32 icon: paper card, red spine, hand-drawn "W".
fn make_icon() -> Hicon {
    const SIZE: usize = 32;
    let mut pixels = vec![0u8; SIZE * SIZE * 4];

    let paper = (0xf3u8, 0xedu8, 0xe2u8);
    let ink = (0x25u8, 0x23u8, 0x20u8);
    let accent = (0x97u8, 0x03u8, 0x02u8);

    let radius = 6.0f32;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let inside = inside_rounded(
                x as f32,
                y as f32,
                1.0,
                1.0,
                SIZE as f32 - 2.0,
                SIZE as f32 - 2.0,
                radius,
            );
            if !inside {
                continue;
            }
            let mut color = paper;
            let border = !inside_rounded(
                x as f32,
                y as f32,
                2.6,
                2.6,
                SIZE as f32 - 4.2,
                SIZE as f32 - 4.2,
                radius - 1.5,
            );
            if border {
                color = ink;
            } else if (2..8).contains(&x) {
                color = accent;
            }
            let index = (y * SIZE + x) * 4;
            pixels[index] = color.2;
            pixels[index + 1] = color.1;
            pixels[index + 2] = color.0;
            pixels[index + 3] = 255;
        }
    }

    // The "W": four strokes, drawn by hand so no font is needed.
    let strokes = [
        (11.0, 11.0, 14.5, 22.0),
        (14.5, 22.0, 17.5, 15.0),
        (17.5, 15.0, 20.5, 22.0),
        (20.5, 22.0, 24.0, 11.0),
    ];
    for (x0, y0, x1, y1) in strokes {
        draw_line(&mut pixels, SIZE, x0, y0, x1, y1, ink, 2);
    }

    unsafe {
        let header = BitmapInfoHeader {
            bi_size: std::mem::size_of::<BitmapInfoHeader>() as u32,
            bi_width: SIZE as i32,
            bi_height: -(SIZE as i32),
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: BI_RGB,
            bi_size_image: 0,
            bi_x_pels_per_meter: 0,
            bi_y_pels_per_meter: 0,
            bi_clr_used: 0,
            bi_clr_important: 0,
        };
        let dc = CreateCompatibleDC(0);
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let color = CreateDIBSection(dc, &header, DIB_RGB_COLORS, &mut bits, 0, 0);
        if color == 0 || bits.is_null() {
            DeleteDC(dc);
            return 0;
        }
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u8, pixels.len());
        let mask = CreateBitmap(SIZE as i32, SIZE as i32, 1, 1, std::ptr::null());
        let info = IconInfo {
            f_icon: 1,
            x_hotspot: 0,
            y_hotspot: 0,
            hbm_mask: mask,
            hbm_color: color,
        };
        let icon = CreateIconIndirect(&info);
        DeleteObject(color);
        if mask != 0 {
            DeleteObject(mask);
        }
        DeleteDC(dc);
        icon
    }
}

fn inside_rounded(
    x: f32,
    y: f32,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    radius: f32,
) -> bool {
    let right = left + width;
    let bottom = top + height;
    if x < left || x > right || y < top || y > bottom {
        return false;
    }
    let cx = x.clamp(left + radius, right - radius);
    let cy = y.clamp(top + radius, bottom - radius);
    let dx = x - cx;
    let dy = y - cy;
    dx * dx + dy * dy <= radius * radius
}

#[allow(clippy::too_many_arguments)]
fn draw_line(
    pixels: &mut [u8],
    size: usize,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: (u8, u8, u8),
    thickness: i32,
) {
    let steps = ((x1 - x0).abs().max((y1 - y0).abs()) * 2.0) as i32 + 1;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let x = x0 + (x1 - x0) * t;
        let y = y0 + (y1 - y0) * t;
        for dy in -thickness / 2..=thickness / 2 {
            for dx in -thickness / 2..=thickness / 2 {
                let px = x.round() as i32 + dx;
                let py = y.round() as i32 + dy;
                if px < 0 || py < 0 || px >= size as i32 || py >= size as i32 {
                    continue;
                }
                let index = (py as usize * size + px as usize) * 4;
                pixels[index] = color.2;
                pixels[index + 1] = color.1;
                pixels[index + 2] = color.0;
                pixels[index + 3] = 255;
            }
        }
    }
}
