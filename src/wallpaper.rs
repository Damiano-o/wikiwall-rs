//! Windows desktop integration: wallpaper, idle time, foreground window.
//!
//! Raw `extern "system"` declarations keep the crate free of heavy Win32
//! bindings, so the same code builds with both the GNU and the MSVC toolchain.

use std::ffi::c_void;
use std::path::Path;

use crate::log_warn;

type Bool = i32;
type Hwnd = isize;

const SPI_SETDESKWALLPAPER: u32 = 20;
const SPIF_UPDATEINIFILE: u32 = 0x01;
const SPIF_SENDWININICHANGE: u32 = 0x02;
const SM_CXSCREEN: i32 = 0;
const SM_CYSCREEN: i32 = 1;
const PROCESS_PER_MONITOR_DPI_AWARE: u32 = 2;

#[repr(C)]
struct LastInputInfo {
    cb_size: u32,
    dw_time: u32,
}

#[link(name = "user32")]
extern "system" {
    fn SystemParametersInfoW(action: u32, param: u32, pv: *mut c_void, win_ini: u32) -> Bool;
    fn GetSystemMetrics(index: i32) -> i32;
    fn GetLastInputInfo(info: *mut LastInputInfo) -> Bool;
    fn GetForegroundWindow() -> Hwnd;
    fn GetClassNameW(hwnd: Hwnd, buffer: *mut u16, max: i32) -> i32;
    fn SetProcessDpiAwarenessContext(context: isize) -> Bool;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetTickCount() -> u32;
}

#[link(name = "shcore")]
extern "system" {
    fn SetProcessDpiAwareness(value: u32) -> i32;
}

/// Make `GetSystemMetrics` report physical pixels instead of scaled ones.
pub fn enable_dpi_awareness() {
    // -4 == PER_MONITOR_AWARE_V2; the shcore fallback covers Windows 8.1.
    let per_monitor_v2: isize = -4;
    unsafe {
        if SetProcessDpiAwarenessContext(per_monitor_v2) == 0 {
            let _ = SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
        }
    }
}

pub fn get_screen_size() -> (u32, u32) {
    enable_dpi_awareness();
    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);
        if width > 0 && height > 0 {
            (width as u32, height as u32)
        } else {
            (1920, 1080)
        }
    }
}

fn set_wallpaper_fill() {
    if let Err(message) = set_registry_string(
        r"Control Panel\Desktop",
        "WallpaperStyle",
        "10",
    ) {
        log_warn!("Impossibile impostare lo stile dello sfondo: {message}");
    }
    let _ = set_registry_string(r"Control Panel\Desktop", "TileWallpaper", "0");
}

const HKEY_CURRENT_USER: isize = 0x8000_0001u32 as i32 as isize;
const KEY_SET_VALUE: u32 = 0x0002;
const REG_SZ: u32 = 1;

#[link(name = "advapi32")]
extern "system" {
    fn RegOpenKeyExW(
        key: isize,
        sub_key: *const u16,
        options: u32,
        access: u32,
        result: *mut isize,
    ) -> i32;
    fn RegSetValueExW(
        key: isize,
        value_name: *const u16,
        reserved: u32,
        kind: u32,
        data: *const u8,
        length: u32,
    ) -> i32;
    fn RegCloseKey(key: isize) -> i32;
}

fn wide(text: &str) -> Vec<u16> {
    let mut buffer: Vec<u16> = text.encode_utf16().collect();
    buffer.push(0);
    buffer
}

fn set_registry_string(path: &str, name: &str, value: &str) -> Result<(), String> {
    let sub_key = wide(path);
    let mut handle: isize = 0;
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            sub_key.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut handle,
        )
    };
    if opened != 0 {
        return Err(format!("RegOpenKeyExW: codice {opened}"));
    }
    let name = wide(name);
    let data: Vec<u8> = value.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let written = unsafe {
        RegSetValueExW(
            handle,
            name.as_ptr(),
            0,
            REG_SZ,
            data.as_ptr(),
            data.len() as u32,
        )
    };
    unsafe {
        RegCloseKey(handle);
    }
    if written != 0 {
        return Err(format!("RegSetValueExW: codice {written}"));
    }
    Ok(())
}

/// Apply an image as the Windows desktop wallpaper.
pub fn set_wallpaper(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("Immagine non trovata: {}", path.display()));
    }
    set_wallpaper_fill();

    let mut wide: Vec<u16> = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .encode_utf16()
        .collect();
    wide.push(0);

    let ok = unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            wide.as_mut_ptr() as *mut c_void,
            SPIF_UPDATEINIFILE | SPIF_SENDWININICHANGE,
        )
    };
    if ok == 0 {
        return Err("Windows non ha accettato il nuovo sfondo.".into());
    }
    Ok(())
}

/// Seconds since the last keyboard or mouse input.
pub fn get_idle_seconds() -> f64 {
    unsafe {
        let mut info = LastInputInfo {
            cb_size: std::mem::size_of::<LastInputInfo>() as u32,
            dw_time: 0,
        };
        if GetLastInputInfo(&mut info) == 0 {
            return 0.0;
        }
        let now = GetTickCount();
        let elapsed = now.wrapping_sub(info.dw_time);
        f64::from(elapsed) / 1000.0
    }
}

fn window_class(hwnd: Hwnd) -> Option<String> {
    if hwnd == 0 {
        return None;
    }
    let mut buffer = [0u16; 256];
    let length = unsafe { GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if length <= 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..length as usize]))
}

/// True when the Windows desktop itself owns the foreground.
pub fn is_desktop_foreground() -> bool {
    let hwnd = unsafe { GetForegroundWindow() };
    matches!(
        window_class(hwnd).as_deref(),
        Some("Progman") | Some("WorkerW")
    )
}
