//! Minimal logger: stderr plus a rotating file in `%LOCALAPPDATA%\WikiWall`.

use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

use chrono::Local;

const MAX_BYTES: u64 = 1_000_000;
const BACKUPS: usize = 2;

static LEVEL: AtomicU8 = AtomicU8::new(1);
static FILE: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();

pub const ERROR: u8 = 0;
pub const INFO: u8 = 1;
pub const DEBUG: u8 = 2;

pub fn init(verbose: bool) {
    LEVEL.store(if verbose { DEBUG } else { INFO }, Ordering::Relaxed);
    FILE.get_or_init(|| Mutex::new(open_log()));
}

fn open_log() -> Option<std::fs::File> {
    let path = crate::paths::log_path();
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > MAX_BYTES {
            for index in (1..=BACKUPS).rev() {
                let from = if index == 1 {
                    path.clone()
                } else {
                    with_suffix(&path, index - 1)
                };
                let _ = std::fs::rename(&from, with_suffix(&path, index));
            }
        }
    }
    OpenOptions::new().create(true).append(true).open(path).ok()
}

fn with_suffix(path: &std::path::Path, index: usize) -> std::path::PathBuf {
    let mut name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("log")
        .to_string();
    name.push_str(&format!(".{index}"));
    path.with_file_name(name)
}

pub fn enabled(level: u8) -> bool {
    level <= LEVEL.load(Ordering::Relaxed)
}

fn write_line(level: u8, module: &str, message: fmt::Arguments<'_>) {
    if !enabled(level) {
        return;
    }
    let label = match level {
        ERROR => "ERROR",
        DEBUG => "DEBUG",
        _ => "INFO",
    };
    let line = format!(
        "{} {} wikiwall::{}: {}\n",
        Local::now().format("%Y-%m-%d %H:%M:%S,%3f"),
        label,
        module,
        message
    );
    if enabled(INFO) {
        eprint!("{line}");
    }
    if let Some(mutex) = FILE.get() {
        if let Ok(mut guard) = mutex.lock() {
            if guard.is_none() {
                *guard = open_log();
            }
            if let Some(file) = guard.as_mut() {
                let _ = file.write_all(line.as_bytes());
                let _ = file.flush();
            }
        }
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logging::write_info(module_path!(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logging::write_warn(module_path!(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logging::write_debug(module_path!(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logging::write_error(module_path!(), format_args!($($arg)*))
    };
}

pub fn write_info(module: &str, args: fmt::Arguments<'_>) {
    write_line(INFO, short(module), args);
}

pub fn write_warn(module: &str, args: fmt::Arguments<'_>) {
    write_line(INFO, short(module), args);
}

pub fn write_debug(module: &str, args: fmt::Arguments<'_>) {
    write_line(DEBUG, short(module), args);
}

pub fn write_error(module: &str, args: fmt::Arguments<'_>) {
    write_line(ERROR, short(module), args);
}

fn short(module: &str) -> &str {
    module.rsplit("::").next().unwrap_or(module)
}
