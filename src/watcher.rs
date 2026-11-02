//! Background loop: refresh the wallpaper at the right moments.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::core::{ChangeRequest, ChangeResult, WikiWall};
use crate::hotkey::Hotkey;
use crate::overlay::Overlay;
use crate::tray::Tray;
use crate::wallpaper::{get_idle_seconds, is_desktop_foreground};

struct LoopState {
    last_change: Instant,
    last_idle: f64,
    desktop_since: Option<Instant>,
    desktop_was_foreground: bool,
    desktop_triggered: bool,
}

pub struct Watcher {
    app: Mutex<WikiWall>,
    stop: AtomicBool,
    change_requested: AtomicBool,
    paused: AtomicBool,
    clickable_qr: AtomicBool,
    current: Mutex<Option<ChangeResult>>,
    last_error: Mutex<Option<String>>,
    state: Mutex<LoopState>,

    interval_seconds: f64,
    idle_threshold_seconds: f64,
    desktop_cooldown_seconds: f64,
    tick_seconds: f64,
    desktop_hold_seconds: f64,
    min_gap_seconds: f64,
}

impl Watcher {
    pub fn new(app: WikiWall) -> Arc<Self> {
        let config = app.config.clone();
        let now = Instant::now();
        Arc::new(Self {
            app: Mutex::new(app),
            stop: AtomicBool::new(false),
            change_requested: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            clickable_qr: AtomicBool::new(config.clickable_qr),
            current: Mutex::new(None),
            last_error: Mutex::new(None),
            state: Mutex::new(LoopState {
                last_change: now,
                last_idle: 0.0,
                desktop_since: None,
                desktop_was_foreground: false,
                desktop_triggered: false,
            }),
            interval_seconds: f64::from(config.interval_minutes) * 60.0,
            idle_threshold_seconds: f64::from(config.idle_minutes) * 60.0,
            desktop_cooldown_seconds: f64::from(config.desktop_cooldown_minutes) * 60.0,
            tick_seconds: 2.0,
            desktop_hold_seconds: 3.0,
            min_gap_seconds: 60.0,
        })
    }

    // ---------------------------------------------------------------- public API

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn clickable_qr_flag(&self) -> bool {
        self.clickable_qr.load(Ordering::Relaxed)
    }

    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> Option<ChangeResult> {
        self.current.lock().ok().and_then(|guard| guard.clone())
    }

    pub fn current_url(&self) -> String {
        self.snapshot()
            .map(|state| state.article.url)
            .unwrap_or_else(|| "https://it.wikipedia.org".to_string())
    }

    pub fn open_current(&self) {
        let url = self.current_url();
        crate::log_info!("Apertura voce: {url}");
        if let Err(err) = open_url(&url) {
            crate::log_error!("Impossibile aprire la voce nel browser: {err}");
        }
    }

    /// Ask the loop to change the wallpaper at the next tick.
    pub fn request_change(&self) {
        self.change_requested.store(true, Ordering::SeqCst);
    }

    /// Change the wallpaper immediately, ignoring the normal cooldowns.
    pub fn skip_current(&self) {
        self.change("salta");
    }

    /// Skip in a worker thread so tray and hotkey callbacks stay responsive.
    pub fn skip_async(self: &Arc<Self>) {
        let watcher = Arc::clone(self);
        std::thread::Builder::new()
            .name("wikiwall-skip".into())
            .spawn(move || watcher.skip_current())
            .ok();
    }

    pub fn toggle_pause(&self) {
        let paused = !self.paused.load(Ordering::SeqCst);
        self.paused.store(paused, Ordering::SeqCst);
        if !paused {
            if let Ok(mut state) = self.state.lock() {
                state.last_idle = get_idle_seconds();
            }
        }
        crate::log_info!("Watcher {}", if paused { "in pausa" } else { "riattivato" });
    }

    pub fn toggle_clickable_qr(&self) {
        let enabled = !self.clickable_qr.load(Ordering::SeqCst);
        self.clickable_qr.store(enabled, Ordering::SeqCst);
        crate::log_info!(
            "QR e link cliccabili: {}",
            if enabled { "sì" } else { "no" }
        );
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    // ---------------------------------------------------------------- decision logic

    /// Return a trigger name when the wallpaper should change now.
    fn should_change(&self, now: Instant, idle: f64, desktop: bool) -> Option<&'static str> {
        let mut state = self.state.lock().ok()?;
        if self.paused() {
            state.last_idle = idle;
            return None;
        }

        let since_change = now.duration_since(state.last_change).as_secs_f64();
        let mut reason: Option<&'static str> = None;
        if since_change >= self.interval_seconds {
            reason = Some("intervallo");
        } else if state.last_idle >= self.idle_threshold_seconds
            && idle <= 3.0
            && since_change >= self.min_gap_seconds
        {
            reason = Some("ritorno");
        }

        if desktop {
            if !state.desktop_was_foreground {
                state.desktop_was_foreground = true;
                state.desktop_since = Some(now);
                state.desktop_triggered = false;
            } else if !state.desktop_triggered
                && state
                    .desktop_since
                    .map(|since| {
                        now.duration_since(since).as_secs_f64() >= self.desktop_hold_seconds
                    })
                    .unwrap_or(false)
                && since_change >= self.desktop_cooldown_seconds
            {
                reason = reason.or(Some("desktop"));
                state.desktop_triggered = true;
            }
        } else {
            state.desktop_was_foreground = false;
            state.desktop_since = None;
            state.desktop_triggered = false;
        }

        state.last_idle = idle;
        reason
    }

    fn change(&self, reason: &str) -> bool {
        let result = {
            let mut app = match self.app.lock() {
                Ok(app) => app,
                Err(_) => return false,
            };
            app.change(ChangeRequest::default())
        };

        match result {
            Ok(result) => {
                crate::log_info!("Sfondo cambiato ({reason}): {}", result.article.title);
                if let Ok(mut current) = self.current.lock() {
                    *current = Some(result);
                }
                if let Ok(mut state) = self.state.lock() {
                    state.last_change = Instant::now();
                    state.last_idle = 0.0;
                    state.desktop_triggered = true;
                }
                if let Ok(mut error) = self.last_error.lock() {
                    *error = None;
                }
                true
            }
            Err(message) => {
                crate::log_error!("Cambio dello sfondo fallito ({reason}): {message}");
                if let Ok(mut error) = self.last_error.lock() {
                    *error = Some(message);
                }
                false
            }
        }
    }

    // ---------------------------------------------------------------- loop

    pub fn run(self: &Arc<Self>, with_tray: bool, with_overlay: bool) {
        let mut tray: Option<Tray> = None;
        if with_tray {
            tray = Some(Tray::start(Arc::clone(self)));
        }

        let mut overlay: Option<Overlay> = None;
        if with_overlay {
            overlay = Some(Overlay::start(Arc::clone(self)));
        }

        let mut hotkey = Some(Hotkey::start({
            let watcher = Arc::clone(self);
            move || watcher.skip_async()
        }));

        if let Ok(mut state) = self.state.lock() {
            state.last_change =
                Instant::now() - Duration::from_secs_f64(self.interval_seconds + 1.0);
            state.last_idle = get_idle_seconds();
        }
        crate::log_info!(
            "Watcher avviato: intervallo {:.0} min, soglia inattività {:.0} min",
            self.interval_seconds / 60.0,
            self.idle_threshold_seconds / 60.0
        );

        while !self.stopped() {
            let now = Instant::now();
            let idle = get_idle_seconds();
            let desktop = is_desktop_foreground();

            let reason = if self.change_requested.swap(false, Ordering::SeqCst) {
                if let Ok(mut state) = self.state.lock() {
                    state.last_idle = idle;
                }
                Some("manuale")
            } else {
                self.should_change(now, idle, desktop)
            };

            if let Some(reason) = reason {
                self.change(reason);
            }

            // Sleep in slices so `stop()` is honoured quickly.
            let deadline = Instant::now() + Duration::from_secs_f64(self.tick_seconds);
            while Instant::now() < deadline && !self.stopped() {
                std::thread::sleep(Duration::from_millis(100));
            }
        }

        if let Some(mut hotkey) = hotkey.take() {
            hotkey.stop();
        }
        if let Some(mut overlay) = overlay.take() {
            overlay.stop();
        }
        if let Some(mut tray) = tray.take() {
            tray.stop();
        }
        crate::log_info!("Watcher fermato");
    }
}

fn open_url(url: &str) -> Result<(), String> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
