//! `config.toml`, read and written with the same keys as the Python version.

use std::path::{Path, PathBuf};

use crate::paths::atomic_write_text;

pub const THEMES: [&str; 2] = ["paper", "dark"];

pub const DEFAULT_TOML: &str = r#"[wikiwall]
language = "it"
theme = "paper"
interval_minutes = 10
idle_minutes = 5
desktop_cooldown_minutes = 10
history_size = 50
quality_cache_days = 7
clickable_qr = true
# width = 1920
# height = 1200
"#;

#[derive(Debug, Clone)]
pub struct Config {
    pub language: String,
    pub theme: String,
    pub interval_minutes: u32,
    pub idle_minutes: u32,
    pub desktop_cooldown_minutes: u32,
    pub history_size: usize,
    pub quality_cache_days: u32,
    pub clickable_qr: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: "it".into(),
            theme: "paper".into(),
            interval_minutes: 10,
            idle_minutes: 5,
            desktop_cooldown_minutes: 10,
            history_size: 50,
            quality_cache_days: 7,
            clickable_qr: true,
            width: None,
            height: None,
        }
    }
}

fn positive(value: Option<&toml::Value>, default: u32) -> u32 {
    value
        .and_then(|v| v.as_integer())
        .filter(|n| *n > 0)
        .map(|n| n as u32)
        .unwrap_or(default)
}

fn optional_positive(value: Option<&toml::Value>) -> Option<u32> {
    value
        .and_then(|v| v.as_integer())
        .filter(|n| *n > 0)
        .map(|n| n as u32)
}

fn boolean(value: Option<&toml::Value>, default: bool) -> bool {
    match value {
        Some(toml::Value::Boolean(flag)) => *flag,
        Some(toml::Value::String(text)) => match text.trim().to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" | "si" | "sì" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        },
        _ => default,
    }
}

impl Config {
    fn from_table(table: &toml::value::Table) -> Self {
        let defaults = Config::default();
        let theme = table
            .get("theme")
            .and_then(|v| v.as_str())
            .unwrap_or(&defaults.theme)
            .trim()
            .to_lowercase();
        let theme = if THEMES.contains(&theme.as_str()) {
            theme
        } else {
            defaults.theme
        };
        let language = table
            .get("language")
            .and_then(|v| v.as_str())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or(defaults.language);

        Self {
            language,
            theme,
            interval_minutes: positive(table.get("interval_minutes"), defaults.interval_minutes),
            idle_minutes: positive(table.get("idle_minutes"), defaults.idle_minutes),
            desktop_cooldown_minutes: positive(
                table.get("desktop_cooldown_minutes"),
                defaults.desktop_cooldown_minutes,
            ),
            history_size: positive(table.get("history_size"), defaults.history_size as u32)
                as usize,
            quality_cache_days: positive(
                table.get("quality_cache_days"),
                defaults.quality_cache_days,
            ),
            clickable_qr: boolean(table.get("clickable_qr"), defaults.clickable_qr),
            width: optional_positive(table.get("width")),
            height: optional_positive(table.get("height")),
        }
    }
}

pub fn save_default_config(path: &Path) -> std::io::Result<PathBuf> {
    if !path.exists() {
        atomic_write_text(path, DEFAULT_TOML)?;
    }
    Ok(path.to_path_buf())
}

pub fn load_config(path: Option<&Path>) -> Config {
    let target = path.map(Path::to_path_buf).unwrap_or_else(crate::paths::config_path);
    if !target.exists() {
        let _ = save_default_config(&target);
    }
    let Ok(text) = std::fs::read_to_string(&target) else {
        return Config::default();
    };
    match text.parse::<toml::Value>() {
        Ok(toml::Value::Table(root)) => {
            let table = match root.get("wikiwall") {
                Some(toml::Value::Table(section)) => section.clone(),
                _ => root,
            };
            Config::from_table(&table)
        }
        _ => Config::default(),
    }
}

/// Store `theme` in the config file, keeping comments and other keys.
pub fn save_theme(theme: &str, path: Option<&Path>) -> Result<PathBuf, String> {
    let name = theme.trim().to_lowercase();
    if !THEMES.contains(&name.as_str()) {
        return Err(format!("Tema sconosciuto: {theme}"));
    }
    let target = path.map(Path::to_path_buf).unwrap_or_else(crate::paths::config_path);
    if !target.exists() {
        save_default_config(&target).map_err(|e| e.to_string())?;
    }
    let mut lines: Vec<String> = std::fs::read_to_string(&target)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();

    let section = lines.iter().position(|line| line.trim() == "[wikiwall]");
    let (start, end) = match section {
        Some(index) => {
            let end = lines
                .iter()
                .enumerate()
                .skip(index + 1)
                .find(|(_, line)| line.trim_start().starts_with('['))
                .map(|(i, _)| i)
                .unwrap_or(lines.len());
            (index + 1, end)
        }
        None => {
            let top_end = lines
                .iter()
                .position(|line| line.trim_start().starts_with('['))
                .unwrap_or(lines.len());
            (0, top_end)
        }
    };

    let existing = (start..end).find(|i| lines[*i].trim_start().starts_with("theme"));
    match existing {
        Some(index) => {
            let prefix = lines[index].split('=').next().unwrap_or("theme").trim_end();
            lines[index] = format!("{prefix} = \"{name}\"");
        }
        None if section.is_some() => lines.insert(start, format!("theme = \"{name}\"")),
        None => {
            if lines.last().map(|l| !l.trim().is_empty()).unwrap_or(false) {
                lines.push(String::new());
            }
            lines.push("[wikiwall]".into());
            lines.push(format!("theme = \"{name}\""));
        }
    }

    let mut text = lines.join("\n").trim_end().to_string();
    text.push('\n');
    atomic_write_text(&target, &text).map_err(|e| e.to_string())?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_section_and_defaults() {
        let dir = std::env::temp_dir().join("wikiwall-config-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        atomic_write_text(&path, "[wikiwall]\ntheme = \"dark\"\ninterval_minutes = 3\n").unwrap();
        let config = load_config(Some(&path));
        assert_eq!(config.theme, "dark");
        assert_eq!(config.interval_minutes, 3);
        assert_eq!(config.idle_minutes, 5);
        assert!(config.clickable_qr);
    }

    #[test]
    fn keeps_comments_when_saving_theme() {
        let dir = std::env::temp_dir().join("wikiwall-theme-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        atomic_write_text(&path, DEFAULT_TOML).unwrap();
        save_theme("dark", Some(&path)).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("theme = \"dark\""));
        assert!(text.contains("# width = 1920"));
    }
}
