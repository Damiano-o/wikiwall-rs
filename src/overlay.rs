//! Clickable QR, link and "Salta voce" zones above the wallpaper.
//!
//! Three frameless, non-activating layered windows sit exactly on top of the
//! painted elements. They are almost fully transparent (alpha 1) so the
//! wallpaper keeps showing through, but they still receive the clicks: the
//! user never has to "hit" the desktop itself.

use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

use crate::render::Box;
use crate::watcher::Watcher;
use crate::win32::*;

const CLASS_NAME: &str = "WikiWallHotspot";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Qr = 1,
    Link = 2,
    Skip = 3,
}

impl Kind {
    fn from_id(id: isize) -> Kind {
        match id {
            2 => Kind::Link,
            3 => Kind::Skip,
            _ => Kind::Qr,
        }
    }
}

static WATCHER: OnceLock<Mutex<Option<Arc<Watcher>>>> = OnceLock::new();

fn watcher_slot() -> &'static Mutex<Option<Arc<Watcher>>> {
    WATCHER.get_or_init(|| Mutex::new(None))
}

extern "system" fn hotspot_proc(hwnd: Hwnd, message: u32, w_param: usize, l_param: isize) -> Lresult {
    unsafe {
        match message {
            WM_LBUTTONUP => {
                let kind = Kind::from_id(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
                let watcher = watcher_slot().lock().ok().and_then(|slot| slot.clone());
                if let Some(watcher) = watcher {
                    match kind {
                        Kind::Skip => watcher.skip_async(),
                        _ => watcher.open_current(),
                    }
                }
                0
            }
            WM_SETCURSOR => {
                SetCursor(LoadCursorW(0, IDC_HAND));
                1
            }
            _ => DefWindowProcW(hwnd, message, w_param, l_param),
        }
    }
}

struct Hotspot {
    hwnd: Hwnd,
    kind: Kind,
    visible: bool,
    geometry: Option<(i32, i32, i32, i32)>,
}

impl Hotspot {
    fn create(kind: Kind, class: &[u16], instance: Hinstance) -> Option<Self> {
        let title = wide("WikiWall");
        let hwnd = unsafe {
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                class.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                0,
                0,
                10,
                10,
                0,
                0,
                instance,
                std::ptr::null_mut(),
            );
            if hwnd == 0 {
                return None;
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, kind as isize);
            // Alpha 1 keeps the window clickable while the wallpaper shows through.
            SetLayeredWindowAttributes(hwnd, 0, 1, LWA_ALPHA);
            hwnd
        };
        Some(Self {
            hwnd,
            kind,
            visible: false,
            geometry: None,
        })
    }

    fn show(&mut self, area: Box) {
        let x = area.0.round() as i32;
        let y = area.1.round() as i32;
        let width = area.2.round().max(1.0) as i32;
        let height = area.3.round().max(1.0) as i32;
        let geometry = (x, y, width, height);
        unsafe {
            if self.geometry != Some(geometry) {
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    x,
                    y,
                    width,
                    height,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                self.geometry = Some(geometry);
            } else if !self.visible {
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    x,
                    y,
                    width,
                    height,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
            if !self.visible {
                ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                self.visible = true;
            }
        }
    }

    fn hide(&mut self) {
        if !self.visible {
            return;
        }
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
        self.visible = false;
    }

    fn destroy(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

pub struct Overlay {
    thread: Option<JoinHandle<()>>,
    stop_event: Handle,
}

unsafe impl Send for Overlay {}

impl Overlay {
    pub fn start(watcher: Arc<Watcher>) -> Self {
        if let Ok(mut slot) = watcher_slot().lock() {
            *slot = Some(watcher);
        }
        let stop_event = unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null()) };
        let thread = {
            let stop_event = stop_event;
            std::thread::Builder::new()
                .name("wikiwall-overlay".into())
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

impl Drop for Overlay {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(stop_event: Handle) {
    unsafe {
        let _ = SetProcessDpiAwareness(2);
        let class_name = wide(CLASS_NAME);
        let class = WndClassW {
            style: 0,
            lpfn_wnd_proc: Some(hotspot_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: GetModuleHandleW(std::ptr::null()),
            h_icon: 0,
            h_cursor: LoadCursorW(0, IDC_HAND),
            hbr_background: 0,
            lpsz_menu_name: std::ptr::null(),
            lpsz_class_name: class_name.as_ptr(),
        };
        RegisterClassW(&class);
        let instance = GetModuleHandleW(std::ptr::null());

        let mut spots: Vec<Hotspot> = [Kind::Qr, Kind::Link, Kind::Skip]
            .into_iter()
            .filter_map(|kind| Hotspot::create(kind, &class_name, instance))
            .collect();

        crate::log_info!("Overlay cliccabile avviato");

        loop {
            let mut message: Msg = std::mem::zeroed();
            while PeekMessageW(&mut message, 0, 0, 0, PM_REMOVE) != 0 {
                if message.message == WM_QUIT {
                    break;
                }
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }

            tick(&mut spots);

            let handles = [stop_event];
            let index = MsgWaitForMultipleObjects(1, handles.as_ptr(), 0, 120, QS_ALLINPUT);
            if index == 0 {
                break;
            }
        }

        for spot in spots.iter_mut() {
            spot.destroy();
        }
        crate::log_info!("Overlay cliccabile fermato");
    }
}

fn tick(spots: &mut [Hotspot]) {
    let watcher = watcher_slot().lock().ok().and_then(|slot| slot.clone());
    let Some(watcher) = watcher else {
        hide_all(spots);
        return;
    };

    let snapshot = watcher.snapshot();
    let wanted = watcher.clickable_qr_flag()
        && snapshot.is_some()
        && (crate::wallpaper::is_desktop_foreground()
            || std::env::var_os("WIKIWALL_OVERLAY_DEBUG").is_some());
    if !wanted {
        hide_all(spots);
        return;
    }
    let Some(state) = snapshot else {
        return;
    };

    for spot in spots.iter_mut() {
        let area = match spot.kind {
            Kind::Qr => state.qr_box,
            Kind::Link => state.link_box,
            Kind::Skip => state.skip_box,
        };
        match area {
            Some(area) if area.2 > 1.0 && area.3 > 1.0 => spot.show(area),
            _ => spot.hide(),
        }
    }
}

fn hide_all(spots: &mut [Hotspot]) {
    for spot in spots.iter_mut() {
        spot.hide();
    }
}
