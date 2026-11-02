//! Where WikiWall keeps its configuration, cache and logs.

use std::path::{Path, PathBuf};

pub const APP_NAME: &str = "WikiWall";

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key).filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn base_dir() -> PathBuf {
    if let Some(override_dir) = env_path("WIKIWALL_HOME") {
        return override_dir;
    }
    if let Some(local) = env_path("LOCALAPPDATA") {
        return local.join(APP_NAME);
    }
    env_path("USERPROFILE")
        .or_else(|| env_path("HOME"))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".wikiwall")
}

pub fn app_dir() -> PathBuf {
    let path = base_dir();
    let _ = std::fs::create_dir_all(&path);
    path
}

pub fn cache_dir() -> PathBuf {
    let path = app_dir().join("cache");
    let _ = std::fs::create_dir_all(&path);
    path
}

pub fn config_path() -> PathBuf {
    app_dir().join("config.toml")
}

pub fn history_path() -> PathBuf {
    app_dir().join("history.json")
}

pub fn wallpaper_path() -> PathBuf {
    app_dir().join("wallpaper.png")
}

pub fn log_path() -> PathBuf {
    app_dir().join("wikiwall.log")
}

pub fn last_article_path() -> PathBuf {
    cache_dir().join("last_article.json")
}

/// Write `text` to `path` atomically (temp file in the same folder, then rename).
pub fn atomic_write_text(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut tmp = path.to_path_buf();
    let suffix = std::process::id();
    tmp.set_file_name(format!(
        "{}.{suffix}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("wikiwall")
    ));
    std::fs::write(&tmp, text.as_bytes())?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = std::fs::remove_file(&tmp);
            Err(err)
        }
    }
}
