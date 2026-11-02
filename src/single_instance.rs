//! Keep a single watcher alive, so two overlays never fight over the desktop.

use std::ffi::c_void;

use crate::win32::CloseHandle;

type Handle = *mut c_void;

const ERROR_ALREADY_EXISTS: u32 = 183;

#[link(name = "kernel32")]
extern "system" {
    fn CreateMutexW(attributes: *mut c_void, initial_owner: i32, name: *const u16) -> Handle;
    fn GetLastError() -> u32;
}

/// A named kernel mutex: Windows releases it when the process dies, so a
/// crashed watcher never leaves a stale lock behind.
pub struct InstanceLock {
    handle: Option<Handle>,
}

impl InstanceLock {
    pub fn new(name: &str) -> Self {
        let mut wide: Vec<u16> = name.encode_utf16().collect();
        wide.push(0);
        let name = wide.as_ptr();
        let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 0, name) };
        if handle.is_null() {
            return Self { handle: None };
        }
        let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        if already {
            unsafe {
                CloseHandle(handle as isize);
            }
            return Self { handle: None };
        }
        Self {
            handle: Some(handle),
        }
    }

    pub fn acquire(&mut self) -> bool {
        self.handle.is_some()
    }

    pub fn release(&mut self) {
        if let Some(handle) = self.handle.take() {
            unsafe {
                CloseHandle(handle as isize);
            }
        }
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        self.release();
    }
}
