//! Global Ctrl+Alt+N shortcut, mirroring the Python RegisterHotKey thread.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::win32::*;

const HOTKEY_ID: i32 = 1;
const VK_N: u32 = 0x4E;

pub struct Hotkey {
    thread: Option<JoinHandle<()>>,
    thread_id: Arc<AtomicU32>,
    running: Arc<AtomicU32>,
}

impl Hotkey {
    /// Start listening on a dedicated message-loop thread.
    pub fn start<F>(callback: F) -> Self
    where
        F: Fn() + Send + 'static,
    {
        let thread_id = Arc::new(AtomicU32::new(0));
        let running = Arc::new(AtomicU32::new(0));
        let thread = {
            let thread_id = Arc::clone(&thread_id);
            let running = Arc::clone(&running);
            std::thread::Builder::new()
                .name("wikiwall-hotkey".into())
                .spawn(move || {
                    unsafe {
                        thread_id.store(GetCurrentThreadId(), Ordering::SeqCst);
                        let modifiers = MOD_CONTROL | MOD_ALT | MOD_NOREPEAT;
                        if RegisterHotKey(0, HOTKEY_ID, modifiers, VK_N) == 0 {
                            crate::log_warn!(
                                "Ctrl+Alt+N non disponibile: un'altra applicazione lo sta usando"
                            );
                            thread_id.store(0, Ordering::SeqCst);
                            return;
                        }
                        running.store(1, Ordering::SeqCst);
                        crate::log_info!("Scorciatoia Ctrl+Alt+N attiva");

                        let mut message: Msg = std::mem::zeroed();
                        while GetMessageW(&mut message, 0, 0, 0) > 0 {
                            if message.message == WM_HOTKEY && message.w_param as i32 == HOTKEY_ID {
                                callback();
                            }
                        }
                        UnregisterHotKey(0, HOTKEY_ID);
                        running.store(0, Ordering::SeqCst);
                        thread_id.store(0, Ordering::SeqCst);
                    }
                })
                .ok()
        };

        Self {
            thread,
            thread_id,
            running,
        }
    }

    pub fn stop(&mut self) {
        let id = self.thread_id.swap(0, Ordering::SeqCst);
        if id != 0 {
            unsafe {
                PostThreadMessageW(id, WM_QUIT, 0, 0);
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.running.store(0, Ordering::SeqCst);
    }
}

impl Drop for Hotkey {
    fn drop(&mut self) {
        self.stop();
    }
}
