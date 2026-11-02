//! A small, crash-safe list of recently shown article titles.

use std::path::PathBuf;

use crate::paths::atomic_write_text;

pub struct History {
    pub path: PathBuf,
    pub limit: usize,
}

impl History {
    pub fn new(path: PathBuf, limit: usize) -> Self {
        Self {
            path,
            limit: limit.max(1),
        }
    }

    fn load(&self) -> Vec<String> {
        let Ok(raw) = std::fs::read_to_string(&self.path) else {
            return Vec::new();
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
            return Vec::new();
        };
        let Some(items) = value.as_array() else {
            return Vec::new();
        };
        items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect()
    }

    fn save(&self, items: &[String]) {
        let payload = serde_json::to_string_pretty(items).unwrap_or_else(|_| "[]".into());
        if let Err(err) = atomic_write_text(&self.path, &payload) {
            crate::log_warn!("storico non salvato: {err}");
        }
    }

    pub fn recent(&self) -> Vec<String> {
        let items = self.load();
        let start = items.len().saturating_sub(self.limit);
        items[start..].to_vec()
    }

    pub fn add(&self, title: &str) {
        let mut items: Vec<String> = self
            .load()
            .into_iter()
            .filter(|item| item != title)
            .collect();
        items.push(title.to_string());
        let start = items.len().saturating_sub(self.limit);
        let items = items[start..].to_vec();
        self.save(&items);
    }

    pub fn clear(&self) {
        let _ = std::fs::remove_file(&self.path);
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_tail_and_deduplicates() {
        let dir = std::env::temp_dir().join("wikiwall-history-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.json");
        let _ = std::fs::remove_file(&path);
        let history = History::new(path, 3);
        for title in ["A", "B", "C", "A"] {
            history.add(title);
        }
        assert_eq!(history.recent(), vec!["B", "C", "A"]);
        history.clear();
        assert!(history.recent().is_empty());
    }
}
