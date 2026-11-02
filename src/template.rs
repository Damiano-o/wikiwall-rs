//! The editorial cover: HTML + CSS for one article, plus the QR code.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{Datelike, Local};
use qrcode::{EcLevel, QrCode};

use crate::util::{hex_color, html_escape, percent_decode};
use crate::wikipedia::Article;

#[path = "template_css.rs"]
mod template_css;
pub use template_css::CSS;

pub const ITALIAN_MONTHS: [&str; 12] = [
    "gennaio",
    "febbraio",
    "marzo",
    "aprile",
    "maggio",
    "giugno",
    "luglio",
    "agosto",
    "settembre",
    "ottobre",
    "novembre",
    "dicembre",
];

const MONTSERRAT: &[u8] = include_bytes!("../assets/fonts/montserrat-variable.woff2");
const SOURCE_SERIF: &[u8] = include_bytes!("../assets/fonts/source-serif-4-variable.woff2");

#[derive(Clone, Copy)]
pub struct Palette {
    pub paper: &'static str,
    pub ink: &'static str,
    pub body: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub rule: &'static str,
    pub texture: &'static str,
    pub qr_bg: &'static str,
    pub qr_ink: &'static str,
}

pub const PAPER: Palette = Palette {
    paper: "#f3ede2",
    ink: "#191714",
    body: "#2b2721",
    muted: "#5f574d",
    accent: "#970302",
    rule: "rgba(25, 23, 20, 0.16)",
    texture: "rgba(25, 23, 20, 0.05)",
    qr_bg: "#fbf7ef",
    qr_ink: "#191714",
};

pub const DARK: Palette = Palette {
    paper: "#0e0f11",
    ink: "#f4efe6",
    body: "#d9d3c8",
    muted: "#a49c92",
    accent: "#e2b45f",
    rule: "rgba(244, 239, 230, 0.18)",
    texture: "rgba(255, 255, 255, 0.045)",
    // Il QR resta scuro su carta chiara: così lo leggono tutti gli scanner.
    qr_bg: "#f4efe6",
    qr_ink: "#14161a",
};

pub fn palette(theme: &str) -> &'static Palette {
    if theme == "dark" {
        &DARK
    } else {
        &PAPER
    }
}

fn font_face_css() -> String {
    let montserrat = BASE64.encode(MONTSERRAT);
    let source_serif = BASE64.encode(SOURCE_SERIF);
    format!(
        r#"
@font-face {{
  font-family: "Montserrat";
  src: url(data:font/woff2;base64,{montserrat}) format("woff2");
  font-weight: 100 900;
  font-style: normal;
  font-display: block;
}}
@font-face {{
  font-family: "Source Serif 4";
  src: url(data:font/woff2;base64,{source_serif}) format("woff2");
  font-weight: 200 900;
  font-style: normal;
  font-display: block;
}}
"#
    )
}

/// Drop heading-only lines and collapse whitespace.
pub fn clean_extract(text: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("==") && trimmed.ends_with("==") {
            continue;
        }
        if !trimmed.is_empty() {
            parts.push(trimmed);
        }
    }
    parts
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Trim an extract at a sentence or word boundary, preserving whole words.
pub fn truncate_extract(text: &str, limit: usize) -> String {
    let clean = clean_extract(text);
    let chars: Vec<char> = clean.chars().collect();
    if chars.len() <= limit {
        return clean;
    }

    // `limit` counts characters; slicing needs the matching byte offset.
    let limit_byte: usize = chars[..limit].iter().map(|c| c.len_utf8()).sum();
    let window = &clean[..limit_byte];

    let mut cut: Option<usize> = None;
    for marker in [". ", "! ", "? "] {
        if let Some(position) = window.rfind(marker) {
            let position = position + 1;
            if cut.map(|best| position > best).unwrap_or(true) {
                cut = Some(position);
            }
        }
    }

    if let Some(cut) = cut {
        let kept_chars = clean[..cut].chars().count();
        if kept_chars as f64 >= limit as f64 * 0.55 {
            return clean[..cut].trim_end().to_string();
        }
    }

    match window.rfind(' ') {
        Some(space) if space > 0 => {
            let kept = clean[..space].trim_end_matches([' ', ',', ';', ':']);
            format!("{kept}…")
        }
        _ => format!("{}…", window.trim_end()),
    }
}

const ABBREVIATIONS: [&str; 49] = [
    "a.c", "a.m", "art", "avv", "ca", "cap", "cfr", "cin", "cit", "col", "d.c", "dep", "dott",
    "ebr", "ecc", "ed", "es", "etc", "fig", "fr", "gen", "giapp", "gr", "ibid", "ing", "ital",
    "lat", "lett", "max", "min", "n", "n.b", "on", "op", "p.m", "p.s", "pag", "port", "pres",
    "prof", "rus", "sans", "sec", "sen", "sig", "sp", "ted", "ten", "trad",
];

/// Return the index just after the first real sentence terminator.
fn sentence_boundary(text: &str) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    let mut depth: i32 = 0;
    let mut quote: Option<char> = None;
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else if ".!?".contains(ch) && i + 1 < chars.len() && chars[i + 1] == active {
                return Some(i + 2);
            }
            i += 1;
            continue;
        }
        match ch {
            '"' => {
                quote = Some('"');
                i += 1;
                continue;
            }
            '“' => {
                quote = Some('”');
                i += 1;
                continue;
            }
            '«' => {
                quote = Some('»');
                i += 1;
                continue;
            }
            '(' | '[' | '{' => {
                depth += 1;
                i += 1;
                continue;
            }
            ')' | ']' | '}' => {
                depth = (depth - 1).max(0);
                i += 1;
                continue;
            }
            _ => {}
        }
        if ".!?".contains(ch) && depth == 0 {
            if ch == '.' && chars[i..].starts_with(&['.', '.', '.']) {
                i += 3;
                continue;
            }
            if ch == '.' {
                let mut j = i as i64 - 1;
                while j >= 0 && (chars[j as usize].is_alphabetic() || chars[j as usize] == '.') {
                    j -= 1;
                }
                let word: String = chars[(j + 1) as usize..i].iter().collect();
                let word = word.to_lowercase();
                let word = word.trim_matches('.');
                if ABBREVIATIONS.contains(&word) {
                    i += 1;
                    continue;
                }
                if i > 0
                    && chars[i - 1].is_ascii_digit()
                    && i + 1 < chars.len()
                    && chars[i + 1].is_ascii_digit()
                {
                    i += 1;
                    continue;
                }
            }
            let mut end = i + 1;
            if end < chars.len() && "”»\"".contains(chars[end]) {
                end += 1;
            }
            if end >= chars.len() || chars[end].is_whitespace() {
                return Some(end);
            }
        }
        i += 1;
    }
    None
}

/// Split the opening sentence from the rest of an extract.
pub fn split_lead(text: &str, max_lead: usize) -> (String, String) {
    let clean = clean_extract(text);
    if clean.is_empty() {
        return (String::new(), String::new());
    }
    let chars: Vec<char> = clean.chars().collect();
    if let Some(boundary) = sentence_boundary(&clean) {
        let lead: String = chars[..boundary].iter().collect();
        let rest: String = chars[boundary..].iter().collect();
        return (lead.trim().to_string(), rest.trim().to_string());
    }
    if chars.len() <= max_lead {
        return (clean, String::new());
    }
    (String::new(), clean)
}

fn title_size(title: &str) -> u32 {
    match title.chars().count() {
        0..=22 => 100,
        23..=34 => 88,
        35..=50 => 74,
        _ => 62,
    }
}

fn extract_limit(title: &str, scale: f64) -> usize {
    let length = title.chars().count();
    let base = if length <= 34 {
        560
    } else if length <= 50 {
        500
    } else {
        430
    };
    ((base as f64 * scale.min(1.0)) as usize).max(280)
}

fn hero_colors(title: &str) -> (String, String) {
    let hue: u32 = title.chars().map(|c| c as u32).sum::<u32>() % 360;
    (
        format!("hsl({hue} 32% 30%)"),
        format!("hsl({} 38% 13%)", (hue + 42) % 360),
    )
}

/// Render the QR as a PNG data URI, coloured with the theme palette.
pub fn qr_data_uri(url: &str, theme: &str) -> String {
    let chosen = palette(theme);
    let ink = hex_color(if chosen.qr_ink.is_empty() {
        chosen.ink
    } else {
        chosen.qr_ink
    });
    let back = hex_color(chosen.qr_bg);

    let Ok(code) = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::M) else {
        return String::new();
    };
    let colors = code.to_colors();
    let modules = code.width();
    let box_size = 10u32;
    let size = modules as u32 * box_size;

    let mut buffer = image::RgbImage::from_pixel(size, size, image::Rgb([back.0, back.1, back.2]));
    for (index, color) in colors.iter().enumerate() {
        if *color != qrcode::Color::Dark {
            continue;
        }
        let x = (index % modules) as u32;
        let y = (index / modules) as u32;
        for dy in 0..box_size {
            for dx in 0..box_size {
                buffer.put_pixel(
                    x * box_size + dx,
                    y * box_size + dy,
                    image::Rgb([ink.0, ink.1, ink.2]),
                );
            }
        }
    }

    let mut png: Vec<u8> = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png);
    if image::ImageEncoder::write_image(
        encoder,
        buffer.as_raw(),
        size,
        size,
        image::ExtendedColorType::Rgb8,
    )
    .is_err()
    {
        return String::new();
    }
    format!("data:image/png;base64,{}", BASE64.encode(&png))
}

fn date_text(now: chrono::DateTime<Local>) -> String {
    format!(
        "{} {} {}",
        now.day(),
        ITALIAN_MONTHS[(now.month() - 1) as usize],
        now.year()
    )
    .to_uppercase()
}

pub fn build_html(article: &Article, width: u32, height: u32, theme: &str) -> String {
    let theme = if theme == "dark" { "dark" } else { "paper" };
    let chosen = palette(theme);
    let scale = (width as f64 / 1920.0).min(height as f64 / 1200.0);

    let size = title_size(&article.title);
    let title_style = format!("--title-size: clamp(44px, calc({size}px * var(--s)), {size}px)");

    let (mut lead, mut rest) = split_lead(
        &article.extract,
        ((220.0 * scale.min(1.0)) as usize).max(150),
    );
    let lead_limit = ((360.0 * scale.min(1.0)) as usize).max(300);
    let lead_chars: Vec<char> = lead.chars().collect();
    if lead_chars.len() > lead_limit {
        let limit_byte: usize = lead_chars[..lead_limit].iter().map(|c| c.len_utf8()).sum();
        let mut cut = lead[..limit_byte].rfind(' ').unwrap_or(limit_byte);
        if cut < limit_byte / 2 {
            cut = limit_byte;
        }
        let overflow = lead[cut..].trim_matches([' ', ',', ';', ':']).to_string();
        let kept = lead[..cut]
            .trim_end_matches([' ', ',', ';', ':'])
            .to_string();
        lead = format!("{kept}…");
        rest = format!("{overflow} {rest}").trim().to_string();
    }

    let body_text = truncate_extract(&rest, extract_limit(&article.title, scale));
    let kicker = if article.quality == "vetrina" {
        "VOCE IN VETRINA"
    } else {
        "VOCE DI QUALITÀ"
    };
    let (hero_a, hero_b) = hero_colors(&article.title);

    let body_style = [
        format!("--s: {scale:.4}"),
        format!("--paper: {}", chosen.paper),
        format!("--ink: {}", chosen.ink),
        format!("--body: {}", chosen.body),
        format!("--muted: {}", chosen.muted),
        format!("--accent: {}", chosen.accent),
        format!("--rule: {}", chosen.rule),
        format!("--texture: {}", chosen.texture),
        format!("--qr-bg: {}", chosen.qr_bg),
        format!("--hero-a: {hero_a}"),
        format!("--hero-b: {hero_b}"),
    ]
    .join("; ");

    let (hero_media, hero_class, hero_caption) = match &article.image_url {
        Some(url) if !url.is_empty() => (
            format!(
                r#"<img class="hero-img" src="{}" alt="" onerror="this.remove()">"#,
                html_escape(url)
            ),
            "hero",
            r#"<div class="hero-caption">Immagine dalla voce · Wikipedia</div>"#.to_string(),
        ),
        _ => (String::new(), "hero no-image", String::new()),
    };

    let initial = article
        .title
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    let hero_letter = format!(
        r#"<div class="hero-letter">{}</div>"#,
        html_escape(&initial)
    );

    let description_html = if article.description.trim().is_empty() {
        String::new()
    } else {
        format!(
            r#"<p class="description">{}</p>"#,
            html_escape(&article.description)
        )
    };

    let first_word = lead.split_whitespace().next().unwrap_or("");
    let mut lead_class = String::from("lead");
    if first_word.contains('\'') || first_word.contains('’') {
        lead_class.push_str(" no-dropcap");
    }
    let lead_html = if lead.is_empty() {
        String::new()
    } else {
        format!(r#"<p class="{lead_class}">{}</p>"#, html_escape(&lead))
    };
    let extract_html = if body_text.is_empty() {
        String::new()
    } else {
        format!(r#"<p class="extract">{}</p>"#, html_escape(&body_text))
    };

    let display_url = percent_decode(&article.url)
        .replace("https://", "")
        .replace("http://", "");

    format!(
        r#"<!doctype html>
<html lang="it">
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>{fonts}{CSS}</style>
</head>
<body style="{body_style}">
  <div class="canvas">
    <section class="panel">
      <div class="watermark">W</div>
      <header class="masthead">
        <div class="brand">
          <span class="mark">W</span>
          <span class="brand-name">WIKIPEDIA</span>
        </div>
        <div class="meta">
          <span class="kicker">{kicker}</span>
          <span class="date">{date}</span>
          <span class="skip" id="skip-target">Salta voce <span class="arrow">→</span></span>
        </div>
      </header>
      <div class="rule"></div>
      <main class="content">
        <h1 class="title" style="{title_style}">{title}</h1>
        {description_html}
        {lead_html}
        {extract_html}
      </main>
      <footer class="footer">
        <div class="source">
          <div class="url" id="article-link">
            <span class="url-text">{display_url}</span>
            <span class="arrow">↗</span>
          </div>
          <div class="license">Testo e immagini: Wikipedia · CC BY-SA 4.0</div>
        </div>
        <div class="qr">
          <img id="qr-target" src="{qr}" alt="QR code">
          <span>Clicca il QR<br>o il link</span>
        </div>
      </footer>
    </section>
    <aside class="{hero_class}">
      {hero_media}
      {hero_letter}
      <div class="hero-badge">IT · WIKIPEDIA</div>
      {hero_caption}
    </aside>
  </div>
</body>
</html>"#,
        title = html_escape(&article.title),
        fonts = font_face_css(),
        date = date_text(Local::now()),
        qr = qr_data_uri(&article.url, theme),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn article(title: &str, extract: &str) -> Article {
        Article {
            title: title.into(),
            description: "descrizione breve".into(),
            extract: extract.into(),
            url: "https://it.wikipedia.org/wiki/Prova".into(),
            image_url: Some("https://upload.wikimedia.org/x.jpg".into()),
            quality: "vetrina".into(),
        }
    }

    #[test]
    fn drops_heading_lines() {
        assert_eq!(clean_extract("== Storia ==\nTesto  qui\n"), "Testo qui");
    }

    #[test]
    fn truncates_at_sentence_boundary() {
        let text = "Prima frase. Seconda frase molto più lunga che continua ancora.";
        assert_eq!(truncate_extract(text, 20), "Prima frase.");
    }

    #[test]
    fn truncates_accented_text_without_splitting_characters() {
        let text = "La città è molto bella e antica. Secondo periodo che continua con altre parole                     utili a superare il limite impostato nel test.";
        let out = truncate_extract(text, 40);
        assert_eq!(out, "La città è molto bella e antica.");
    }

    #[test]
    fn keeps_whole_words_when_no_sentence_fits() {
        let text = "àèìòù àèìòù àèìòù àèìòù àèìòù àèìòù àèìòù àèìòù";
        let out = truncate_extract(text, 20);
        assert!(out.ends_with('…'));
        assert!(!out.contains("àèìòù…à"));
    }

    #[test]
    fn splits_the_lead_sentence() {
        let (lead, rest) = split_lead("Una frase. Il resto.", 240);
        assert_eq!(lead, "Una frase.");
        assert_eq!(rest, "Il resto.");
    }

    #[test]
    fn ignores_abbreviations_in_sentence_boundary() {
        let (lead, _) = split_lead("Il dott. Rossi è qui. Poi altro.", 240);
        assert_eq!(lead, "Il dott. Rossi è qui.");
    }

    #[test]
    fn builds_a_cover_with_qr_and_link() {
        let html = build_html(
            &article("Prova", "Testo della voce. Altro testo."),
            1920,
            1080,
            "dark",
        );
        assert!(html.contains("data:image/png;base64,"));
        assert!(html.contains("it.wikipedia.org/wiki/Prova"));
        assert!(html.contains("VOCE IN VETRINA"));
        assert!(html.contains("--hero-a: hsl("));
    }
}
