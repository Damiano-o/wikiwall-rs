//! Wikipedia API: the quality article lists, summaries and the offline fallback.

use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::paths::{atomic_write_text, cache_dir, last_article_path};
use crate::util::{percent_decode, percent_encode_path};

pub const USER_AGENT: &str = concat!(
    "WikiWall/0.1.0-rs (https://github.com/Damiano-o/wikiwall; ",
    "contact: Damiano-o@users.noreply.github.com)"
);
const MAX_CANDIDATE_ATTEMPTS: usize = 12;
const VETRINA_SHARE: f64 = 0.65;
const MIN_QUALITY_TITLES: usize = 100;

pub const QUALITY_SOURCES: [(&str, &str); 2] = [
    ("vetrina", "Wikipedia:Vetrina"),
    ("qualità", "Wikipedia:Voci di qualità"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Article {
    pub title: String,
    pub description: String,
    pub extract: String,
    pub url: String,
    pub image_url: Option<String>,
    pub quality: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArticleRecord {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    extract: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    image_url: Option<String>,
    #[serde(default)]
    quality: String,
}

impl Article {
    fn to_record(&self) -> ArticleRecord {
        ArticleRecord {
            title: self.title.clone(),
            description: self.description.clone(),
            extract: self.extract.clone(),
            url: self.url.clone(),
            image_url: self.image_url.clone(),
            quality: self.quality.clone(),
        }
    }
}

#[derive(Debug)]
pub enum WikipediaError {
    Network(String),
    NotFound(String),
}

impl std::fmt::Display for WikipediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WikipediaError::Network(message) => write!(f, "{message}"),
            WikipediaError::NotFound(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for WikipediaError {}

impl From<reqwest::Error> for WikipediaError {
    fn from(err: reqwest::Error) -> Self {
        WikipediaError::Network(err.to_string())
    }
}

pub struct WikipediaClient {
    pub language: String,
    pub cache_days: u32,
    pub api_url: String,
    pub cache_path: PathBuf,
    client: reqwest::blocking::Client,
}

impl WikipediaClient {
    pub fn new(language: &str, cache_days: u32) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self {
            language: language.to_string(),
            cache_days: cache_days.max(1),
            api_url: format!("https://{language}.wikipedia.org/w/api.php"),
            cache_path: cache_dir().join("quality.json"),
            client,
        }
    }

    // ---------------------------------------------------------------- quality list

    pub fn quality_titles(&self, refresh: bool) -> Result<Vec<(String, String)>, WikipediaError> {
        let cached = self.load_cache();
        if let Some((fetched, items)) = &cached {
            if !refresh && self.is_fresh(*fetched) {
                return Ok(items.clone());
            }
        }

        match self.fetch_quality_index() {
            Ok(items) => {
                self.write_cache(&items);
                Ok(items)
            }
            Err(err) => match cached {
                Some((_, items)) => {
                    crate::log_warn!("Lista di qualità non aggiornata, uso la cache: {err}");
                    Ok(items)
                }
                None => Err(WikipediaError::Network(format!(
                    "Impossibile scaricare la lista delle voci di qualità: {err}"
                ))),
            },
        }
    }

    fn fetch_quality_index(&self) -> Result<Vec<(String, String)>, WikipediaError> {
        let mut titles: Vec<(String, String)> = Vec::new();
        for (label, page) in QUALITY_SOURCES {
            let data = self.api_get(&[
                ("action", "parse"),
                ("page", page),
                ("prop", "links"),
                ("format", "json"),
                ("formatversion", "2"),
                ("maxlag", "5"),
            ])?;
            if let Some(links) = data
                .get("parse")
                .and_then(|parse| parse.get("links"))
                .and_then(|links| links.as_array())
            {
                for link in links {
                    let ns = link.get("ns").and_then(|ns| ns.as_i64()).unwrap_or(-1);
                    let title = link
                        .get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .trim();
                    if ns != 0 || title.is_empty() {
                        continue;
                    }
                    if !titles.iter().any(|(existing, _)| existing == title) {
                        titles.push((title.to_string(), label.to_string()));
                    }
                }
            }
        }

        if titles.len() < MIN_QUALITY_TITLES {
            return Err(WikipediaError::Network(format!(
                "Lista di qualità troppo corta ({} voci)",
                titles.len()
            )));
        }
        titles.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(titles)
    }

    fn api_get(&self, params: &[(&str, &str)]) -> Result<serde_json::Value, WikipediaError> {
        let mut last_error = String::from("errore sconosciuto");
        for attempt in 0..3 {
            let response = self
                .client
                .get(&self.api_url)
                .query(params)
                .send();

            let response = match response {
                Ok(response) => response,
                Err(err) => {
                    last_error = err.to_string();
                    std::thread::sleep(Duration::from_secs(1 + attempt));
                    continue;
                }
            };

            let status = response.status();
            if status.as_u16() == 429 {
                last_error = "HTTP 429 (troppe richieste)".into();
                let wait = retry_after(&response).min(30.0);
                std::thread::sleep(Duration::from_secs_f64(wait));
                continue;
            }
            if status.is_server_error() {
                last_error = format!("HTTP {}", status.as_u16());
                std::thread::sleep(Duration::from_secs(1 + attempt));
                continue;
            }
            if !status.is_success() {
                last_error = format!("HTTP {}", status.as_u16());
                break;
            }
            match response.json::<serde_json::Value>() {
                Ok(value) => return Ok(value),
                Err(err) => {
                    last_error = err.to_string();
                    break;
                }
            }
        }
        Err(WikipediaError::Network(format!(
            "Richiesta a Wikipedia fallita: {last_error}"
        )))
    }

    fn load_cache(&self) -> Option<(DateTime<Utc>, Vec<(String, String)>)> {
        let raw = std::fs::read_to_string(&self.cache_path).ok()?;
        let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let fetched_at = value.get("fetched_at")?.as_str()?;
        let fetched = DateTime::parse_from_rfc3339(fetched_at)
            .ok()
            .map(|d| d.with_timezone(&Utc))?;
        let items: Vec<(String, String)> = value
            .get("items")?
            .as_array()?
            .iter()
            .filter_map(|item| {
                let pair = item.as_array()?;
                if pair.len() != 2 {
                    return None;
                }
                Some((
                    pair[0].as_str()?.to_string(),
                    pair[1].as_str()?.to_string(),
                ))
            })
            .collect();
        if items.is_empty() {
            return None;
        }
        Some((fetched, items))
    }

    fn is_fresh(&self, fetched: DateTime<Utc>) -> bool {
        let age = Utc::now().signed_duration_since(fetched);
        age.num_days() < self.cache_days as i64
    }

    fn write_cache(&self, items: &[(String, String)]) {
        let payload = serde_json::json!({
            "fetched_at": Utc.timestamp_opt(Utc::now().timestamp(), 0).unwrap().to_rfc3339(),
            "items": items
                .iter()
                .map(|(title, label)| serde_json::json!([title, label]))
                .collect::<Vec<_>>(),
        });
        let text = serde_json::to_string_pretty(&payload).unwrap_or_default();
        if let Err(err) = atomic_write_text(&self.cache_path, &text) {
            crate::log_warn!("cache della lista non salvata: {err}");
        }
    }

    // ---------------------------------------------------------------- article

    pub fn random_article(&self, recent: &[String]) -> Result<Article, WikipediaError> {
        let items = self.quality_titles(false)?;
        if items.is_empty() {
            return Err(WikipediaError::Network(
                "Nessuna voce di qualità disponibile".into(),
            ));
        }

        let mut candidates: Vec<(String, String)> = items
            .iter()
            .filter(|(title, _)| !recent.iter().any(|r| r == title))
            .cloned()
            .collect();
        if candidates.is_empty() {
            candidates = items.clone();
        }

        let vetrina: Vec<String> = candidates
            .iter()
            .filter(|(_, label)| label == "vetrina")
            .map(|(title, _)| title.clone())
            .collect();
        let qualita: Vec<String> = candidates
            .iter()
            .filter(|(_, label)| label != "vetrina")
            .map(|(title, _)| title.clone())
            .collect();

        let mut rng = rand::thread_rng();
        let pool = match (vetrina.is_empty(), qualita.is_empty()) {
            (false, false) => {
                if rng.gen::<f64>() < VETRINA_SHARE {
                    vetrina.clone()
                } else {
                    qualita.clone()
                }
            }
            (true, false) => qualita.clone(),
            _ => vetrina.clone(),
        };

        if let Some(article) = self.first_usable(&pool, &candidates, &mut rng) {
            return Ok(article);
        }
        if candidates.len() < items.len() {
            let all: Vec<String> = items.iter().map(|(title, _)| title.clone()).collect();
            if let Some(article) = self.first_usable(&all, &items, &mut rng) {
                return Ok(article);
            }
        }
        Err(WikipediaError::Network(
            "Nessuna voce recuperabile: estrazione non disponibile".into(),
        ))
    }

    fn first_usable(
        &self,
        pool: &[String],
        labels: &[(String, String)],
        rng: &mut impl Rng,
    ) -> Option<Article> {
        let attempts = MAX_CANDIDATE_ATTEMPTS.min(pool.len());
        let picked: Vec<&String> = pool.choose_multiple(rng, attempts).collect();
        for title in picked {
            let quality = labels
                .iter()
                .find(|(name, _)| name == title)
                .map(|(_, label)| label.clone())
                .unwrap_or_default();
            match self.summary(title, &quality) {
                Ok(article) if !article.extract.trim().is_empty() => return Some(article),
                Ok(_) => continue,
                Err(err) => {
                    crate::log_warn!("Voce {title} non disponibile: {err}");
                    continue;
                }
            }
        }
        None
    }

    pub fn summary(&self, title: &str, quality: &str) -> Result<Article, WikipediaError> {
        let data = self.api_get(&[
            ("action", "query"),
            ("prop", "extracts|pageimages|description|info"),
            ("exchars", "1400"),
            ("explaintext", "1"),
            ("piprop", "original|thumbnail"),
            ("inprop", "url"),
            ("redirects", "1"),
            ("titles", title),
            ("format", "json"),
            ("formatversion", "2"),
            ("maxlag", "5"),
        ])?;

        let pages = data
            .get("query")
            .and_then(|query| query.get("pages"))
            .and_then(|pages| pages.as_array());
        let page = match pages.and_then(|pages| pages.first()) {
            Some(page) => page,
            None => return Err(WikipediaError::NotFound(format!("Voce non trovata: {title}"))),
        };
        if page.get("missing").is_some() || page.get("invalid").is_some() {
            return Err(WikipediaError::NotFound(format!("Voce non trovata: {title}")));
        }

        let extract = page
            .get("extract")
            .and_then(|e| e.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if extract.is_empty() {
            return Err(WikipediaError::NotFound(format!("Nessun estratto per {title}")));
        }

        let image_url = page
            .get("original")
            .and_then(|o| o.get("source"))
            .and_then(|s| s.as_str())
            .or_else(|| {
                page.get("thumbnail")
                    .and_then(|t| t.get("source"))
                    .and_then(|s| s.as_str())
            })
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let page_url = page
            .get("fullurl")
            .and_then(|u| u.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| {
                format!(
                    "https://{}.wikipedia.org/wiki/{}",
                    self.language,
                    percent_encode_path(&title.replace(' ', "_"))
                )
            });

        Ok(Article {
            title: page
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or(title)
                .to_string(),
            description: page
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .trim()
                .to_string(),
            extract,
            url: percent_decode(&page_url),
            image_url,
            quality: quality.to_string(),
        })
    }

    // ---------------------------------------------------------------- offline fallback

    pub fn save_last(&self, article: &Article) {
        let record = article.to_record();
        match serde_json::to_string_pretty(&record) {
            Ok(text) => {
                if let Err(err) = atomic_write_text(&last_article_path(), &text) {
                    crate::log_warn!("ultima voce non salvata: {err}");
                }
            }
            Err(err) => crate::log_warn!("ultima voce non serializzata: {err}"),
        }
    }

    pub fn load_last(&self) -> Option<Article> {
        let raw = std::fs::read_to_string(last_article_path()).ok()?;
        let record: ArticleRecord = serde_json::from_str(&raw).ok()?;
        let article = Article {
            title: record.title,
            description: record.description,
            extract: record.extract,
            url: record.url,
            image_url: record.image_url.filter(|url| !url.is_empty()),
            quality: record.quality,
        };
        if article.title.is_empty() || article.extract.is_empty() {
            return None;
        }
        Some(article)
    }
}

fn retry_after(response: &reqwest::blocking::Response) -> f64 {
    response
        .headers()
        .get("Retry-After")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<f64>().ok())
        .unwrap_or(5.0)
        .max(0.0)
}
