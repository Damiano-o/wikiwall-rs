//! Render the cover to a PNG with headless Chromium, driven over CDP.
//!
//! The Python version used Playwright; here we speak the DevTools protocol
//! directly (Chrome is the same binary Playwright had already downloaded), so
//! the Rust port needs no Playwright install and no bundled browser of its own.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde_json::{json, Value};

use crate::template::build_html;
use crate::wikipedia::Article;

pub type Box = (f64, f64, f64, f64);

#[derive(Debug, Clone)]
pub struct RenderResult {
    pub path: PathBuf,
    pub qr_box: Option<Box>,
    pub link_box: Option<Box>,
    pub skip_box: Option<Box>,
}

#[derive(Debug)]
pub struct RenderError(pub String);

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for RenderError {}

impl From<std::io::Error> for RenderError {
    fn from(err: std::io::Error) -> Self {
        RenderError(err.to_string())
    }
}

/// Chrome builds shipped with Playwright, newest first.
fn playwright_candidates() -> Vec<PathBuf> {
    let Some(local) = std::env::var_os("LOCALAPPDATA") else {
        return Vec::new();
    };
    let base = PathBuf::from(local).join("ms-playwright");
    let Ok(entries) = std::fs::read_dir(&base) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("chromium-") {
            continue;
        }
        for inner in ["chrome-win64/chrome.exe", "chrome-win/chrome.exe"] {
            let candidate = entry.path().join(inner);
            if candidate.is_file() {
                found.push(candidate);
            }
        }
    }
    found.sort();
    found.reverse();
    found
}

fn system_candidates() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for base in [
        std::env::var_os("PROGRAMFILES").map(PathBuf::from),
        std::env::var_os("PROGRAMFILES(X86)").map(PathBuf::from),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
    ]
    .into_iter()
    .flatten()
    {
        for relative in [
            "Google/Chrome/Application/chrome.exe",
            "Microsoft/Edge/Application/msedge.exe",
            "Chromium/Application/chrome.exe",
        ] {
            let candidate = base.join(relative);
            if candidate.is_file() {
                found.push(candidate);
            }
        }
    }
    found
}

pub fn find_browser() -> Vec<PathBuf> {
    let mut all = playwright_candidates();
    all.extend(system_candidates());
    all
}

struct Cdp {
    socket: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    next_id: i64,
}

impl Cdp {
    fn connect(url: &str) -> Result<Self, RenderError> {
        let (socket, _response) =
            tungstenite::connect(url).map_err(|e| RenderError(format!("CDP: {e}")))?;
        Ok(Self { socket, next_id: 1 })
    }

    fn call(
        &mut self,
        method: &str,
        params: Value,
        session: Option<&str>,
    ) -> Result<Value, RenderError> {
        let id = self.next_id;
        self.next_id += 1;
        let mut message = serde_json::Map::new();
        message.insert("id".into(), json!(id));
        message.insert("method".into(), json!(method));
        message.insert("params".into(), params);
        if let Some(session) = session {
            message.insert("sessionId".into(), json!(session));
        }
        let payload = serde_json::Value::Object(message).to_string();
        self.socket
            .send(tungstenite::Message::text(payload))
            .map_err(|e| RenderError(format!("CDP send: {e}")))?;

        let deadline = Instant::now() + Duration::from_secs(45);
        while Instant::now() < deadline {
            let message = self
                .socket
                .read()
                .map_err(|e| RenderError(format!("CDP read: {e}")))?;
            let Ok(text) = message.to_text() else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            if value.get("id").and_then(|v| v.as_i64()) != Some(id) {
                continue;
            }
            if let Some(error) = value.get("error") {
                return Err(RenderError(format!("CDP {method}: {error}")));
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
        Err(RenderError(format!("CDP {method}: nessuna risposta")))
    }

    fn evaluate(&mut self, session: &str, expression: &str, await_promise: bool) -> Result<Value, RenderError> {
        let result = self.call(
            "Runtime.evaluate",
            json!({
                "expression": expression,
                "awaitPromise": await_promise,
                "returnByValue": true,
            }),
            Some(session),
        )?;
        if let Some(exception) = result.get("exceptionDetails") {
            return Err(RenderError(format!("JS: {exception}")));
        }
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }
}

fn devtools_port(profile: &Path) -> Result<u16, RenderError> {
    let file = profile.join("DevToolsActivePort");
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if let Ok(text) = std::fs::read_to_string(&file) {
            if let Some(first) = text.lines().next() {
                if let Ok(port) = first.trim().parse::<u16>() {
                    return Ok(port);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(120));
    }
    Err(RenderError(
        "Chromium non ha aperto la porta di debug".into(),
    ))
}

struct BrowserChild(Child);

impl Drop for BrowserChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_for_websocket(port: u16) -> Result<String, RenderError> {
    let url = format!("http://127.0.0.1:{port}/json/version");
    let client = reqwest::blocking::Client::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last = String::from("nessuna risposta");
    while Instant::now() < deadline {
        match client.get(&url).send() {
            Ok(response) => match response.json::<Value>() {
                Ok(value) => {
                    if let Some(ws) = value
                        .get("webSocketDebuggerUrl")
                        .and_then(|v| v.as_str())
                    {
                        return Ok(ws.to_string());
                    }
                    last = "webSocketDebuggerUrl assente".into();
                }
                Err(err) => last = err.to_string(),
            },
            Err(err) => last = err.to_string(),
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    Err(RenderError(format!("DevTools non raggiungibile: {last}")))
}

fn launch(browser: &Path, width: u32, height: u32) -> Result<(BrowserChild, PathBuf), RenderError> {
    let profile = std::env::temp_dir().join(format!("wikiwall-chrome-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&profile);
    std::fs::create_dir_all(&profile)?;

    let child = Command::new(browser)
        .args([
            "--headless=new",
            "--disable-gpu",
            // The cover is laid out in CSS pixels: keep the device scale at 1:1
            // even on a display with 125% / 150% scaling.
            "--force-device-scale-factor=1",
            "--high-dpi-support=1",
            "--disable-extensions",
            "--disable-background-networking",
            "--hide-scrollbars",
            "--no-first-run",
            "--no-default-browser-check",
            "--no-sandbox",
            "--mute-audio",
            "--remote-debugging-port=0",
            "--allow-file-access-from-files",
        ])
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg(format!("--window-size={width},{height}"))
        .arg("about:blank")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| RenderError(format!("Chromium non avviato: {e}")))?;

    Ok((BrowserChild(child), profile))
}

fn box_of(value: &Value) -> Option<Box> {
    let object = value.as_object()?;
    let number = |key: &str| object.get(key).and_then(|v| v.as_f64());
    Some((
        number("x")?,
        number("y")?,
        number("width")?,
        number("height")?,
    ))
}

/// Render `article` to `output` at exactly `width` x `height` pixels.
pub fn render_article(
    article: &Article,
    output: &Path,
    width: u32,
    height: u32,
    theme: &str,
) -> Result<RenderResult, RenderError> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let markup = build_html(article, width, height, theme);

    let browsers = find_browser();
    if browsers.is_empty() {
        return Err(RenderError(
            "Chromium non è disponibile. Esegui: wikiwall setup".into(),
        ));
    }

    let mut last_error = String::from("nessun browser avviato");
    for browser in &browsers {
        match render_with(browser, &markup, output, width, height) {
            Ok(result) => return Ok(result),
            Err(err) => {
                crate::log_debug!("{browser:?} non ha funzionato: {err}");
                last_error = err.0;
            }
        }
    }
    Err(RenderError(last_error))
}

fn render_with(
    browser: &Path,
    markup: &str,
    output: &Path,
    width: u32,
    height: u32,
) -> Result<RenderResult, RenderError> {
    let (child, profile) = launch(browser, width, height)?;
    let port = devtools_port(&profile)?;
    let ws_url = wait_for_websocket(port)?;

    let mut cdp = Cdp::connect(&ws_url)?;
    let target = cdp.call("Target.createTarget", json!({"url": "about:blank"}), None)?;
    let target_id = target
        .get("targetId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RenderError("target non creato".into()))?;
    let attached = cdp.call(
        "Target.attachToTarget",
        json!({"targetId": target_id, "flatten": true}),
        None,
    )?;
    let session = attached
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RenderError("sessione non creata".into()))?
        .to_string();

    cdp.call("Page.enable", json!({}), Some(&session))?;
    cdp.call("Runtime.enable", json!({}), Some(&session))?;
    cdp.call(
        "Emulation.setDeviceMetricsOverride",
        json!({
            "width": width,
            "height": height,
            "deviceScaleFactor": 1,
            "mobile": false,
        }),
        Some(&session),
    )?;

    let tree = cdp.call("Page.getFrameTree", json!({}), Some(&session))?;
    let frame_id = tree
        .get("frameTree")
        .and_then(|f| f.get("frame"))
        .and_then(|f| f.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| RenderError("frame non trovato".into()))?
        .to_string();

    cdp.call(
        "Page.setDocumentContent",
        json!({"frameId": frame_id, "html": markup}),
        Some(&session),
    )?;
    cdp.evaluate(&session, "document.fonts.ready", true)?;
    cdp.evaluate(
        &session,
        "Promise.all(Array.from(document.images).map((img) => {
            if (img.complete) return Promise.resolve();
            return new Promise((resolve) => {
                img.addEventListener('load', resolve, { once: true });
                img.addEventListener('error', resolve, { once: true });
                setTimeout(resolve, 10000);
            });
        }))",
        true,
    )?;
    std::thread::sleep(Duration::from_millis(250));

    let boxes = cdp.evaluate(
        &session,
        r#"(() => {
            const read = (id) => {
                const el = document.getElementById(id);
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return { x: r.x, y: r.y, width: r.width, height: r.height };
            };
            return { qr: read('qr-target'), link: read('article-link'), skip: read('skip-target') };
        })()"#,
        false,
    )?;

    let shot = cdp.call(
        "Page.captureScreenshot",
        json!({
            "format": "png",
            "clip": {"x": 0, "y": 0, "width": width, "height": height, "scale": 1},
            "captureBeyondViewport": false,
        }),
        Some(&session),
    )?;
    let data = shot
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RenderError("screenshot vuoto".into()))?;
    let bytes = BASE64
        .decode(data)
        .map_err(|e| RenderError(format!("PNG non decodificato: {e}")))?;
    // Atomic write: the wallpaper may be read by the overlay while we replace it.
    let tmp = output.with_extension("png.tmp");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, output)?;

    crate::log_debug!(
        "box qr={:?} link={:?} skip={:?}",
        boxes.get("qr").and_then(box_of),
        boxes.get("link").and_then(box_of),
        boxes.get("skip").and_then(box_of)
    );

    let _ = cdp.call("Target.closeTarget", json!({"targetId": target_id}), None);
    drop(cdp);
    drop(child);
    let _ = std::fs::remove_dir_all(&profile);

    Ok(RenderResult {
        path: output.to_path_buf(),
        qr_box: boxes.get("qr").and_then(box_of),
        link_box: boxes.get("link").and_then(box_of),
        skip_box: boxes.get("skip").and_then(box_of),
    })
}

/// Print the first line of a browser version, used by `wikiwall setup`.
pub fn browser_version(browser: &Path) -> Option<String> {
    let output = Command::new(browser).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// Read the PNG size, used by tests and diagnostics.
pub fn png_size(path: &Path) -> Option<(u32, u32)> {
    image::image_dimensions(path).ok()
}
