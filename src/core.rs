//! Selection → rendering → wallpaper, with history and the offline fallback.

use std::path::PathBuf;

use crate::config::{load_config, Config};
use crate::history::History;
use crate::paths::{cache_dir, history_path, wallpaper_path};
use crate::render::{render_article, Box, RenderResult};
use crate::wallpaper::{get_screen_size, set_wallpaper};
use crate::wikipedia::{Article, WikipediaClient};

#[derive(Debug, Clone)]
#[allow(dead_code)] // width/height/theme sono diagnostica, non servono all'overlay
pub struct ChangeResult {
    pub article: Article,
    pub image_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub theme: String,
    pub qr_box: Option<Box>,
    pub link_box: Option<Box>,
    pub skip_box: Option<Box>,
}

impl ChangeResult {
    fn from_render(
        article: Article,
        rendered: RenderResult,
        width: u32,
        height: u32,
        theme: &str,
    ) -> Self {
        Self {
            article,
            image_path: rendered.path,
            width,
            height,
            theme: theme.to_string(),
            qr_box: rendered.qr_box,
            link_box: rendered.link_box,
            skip_box: rendered.skip_box,
        }
    }
}

pub struct ChangeRequest<'a> {
    pub theme: Option<&'a str>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub output: Option<&'a std::path::Path>,
    pub apply: bool,
}

impl Default for ChangeRequest<'_> {
    fn default() -> Self {
        Self {
            theme: None,
            width: None,
            height: None,
            output: None,
            apply: true,
        }
    }
}

pub struct WikiWall {
    pub config: Config,
    client: WikipediaClient,
    history: History,
}

impl WikiWall {
    pub fn new(config: Config) -> Self {
        let client = WikipediaClient::new(&config.language, config.quality_cache_days);
        let history = History::new(history_path(), config.history_size);
        Self {
            config,
            client,
            history,
        }
    }

    pub fn from_disk() -> Self {
        Self::new(load_config(None))
    }

    /// Pick a new article, render it and (optionally) set it as wallpaper.
    pub fn change(&mut self, request: ChangeRequest<'_>) -> Result<ChangeResult, String> {
        let theme = request.theme.unwrap_or(&self.config.theme).to_string();
        let (width, height) = match (request.width, request.height) {
            (Some(w), Some(h)) => (w, h),
            (w, h) => {
                let (screen_w, screen_h) = get_screen_size();
                (
                    w.or(self.config.width).unwrap_or(screen_w),
                    h.or(self.config.height).unwrap_or(screen_h),
                )
            }
        };

        let article = match self.client.random_article(&self.history.recent()) {
            Ok(article) => article,
            Err(err) => match self.client.load_last() {
                Some(article) => {
                    crate::log_warn!("Wikipedia non raggiungibile, riuso l'ultima voce: {err}");
                    article
                }
                None => return Err(err.to_string()),
            },
        };

        self.client.save_last(&article);
        let destination = match request.output {
            Some(path) => path.to_path_buf(),
            None if request.apply => wallpaper_path(),
            // Anteprima: non sovrascrivere il PNG che sta sullo sfondo.
            None => cache_dir().join("preview.png"),
        };

        let rendered = render_article(&article, &destination, width, height, &theme)
            .map_err(|err| err.to_string())?;

        if request.apply {
            set_wallpaper(&rendered.path)?;
            self.history.add(&article.title);
        }

        Ok(ChangeResult::from_render(
            article, rendered, width, height, &theme,
        ))
    }
}
