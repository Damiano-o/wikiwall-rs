//! WikiWall — una voce di qualità di Wikipedia italiano sul tuo desktop Windows.

mod config;
mod core;
mod history;
mod hotkey;
mod logging;
mod overlay;
mod paths;
mod render;
mod single_instance;
mod startup;
mod template;
mod tray;
mod util;
mod wallpaper;
mod watcher;
mod wikipedia;
mod win32;

use std::io::Write;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::{load_config, save_default_config, save_theme, THEMES};
use crate::core::{ChangeRequest, WikiWall};
use crate::history::History;
use crate::paths::{config_path, history_path};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(
    name = "wikiwall",
    version = VERSION,
    about = "Una voce di qualità di Wikipedia italiano sul tuo desktop Windows.",
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Cli {
    /// Log dettagliato
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Cambia subito lo sfondo
    Next {
        /// paper oppure dark
        #[arg(long)]
        theme: Option<String>,
        /// Larghezza in pixel
        #[arg(long)]
        width: Option<u32>,
        /// Altezza in pixel
        #[arg(long)]
        height: Option<u32>,
        /// Percorso del PNG
        #[arg(long)]
        output: Option<PathBuf>,
        /// Genera l'immagine senza applicarla al desktop
        #[arg(long)]
        no_set: bool,
    },
    /// Aggiorna lo sfondo in background
    Watch {
        /// Avvia senza icona nella tray
        #[arg(long)]
        no_tray: bool,
        /// Avvia senza QR e link cliccabili
        #[arg(long)]
        no_overlay: bool,
    },
    /// Avvia WikiWall automaticamente al login
    Install,
    /// Rimuove l'avvio automatico
    Uninstall,
    /// Mostra il file di configurazione
    Config {
        /// Apri il file in Notepad
        #[arg(long)]
        open: bool,
    },
    /// Mostra le ultime voci usate
    History {
        /// Svuota lo storico
        #[arg(long)]
        clear: bool,
    },
    /// Mostra o cambia il tema
    Theme {
        /// paper oppure dark
        name: Option<String>,
        /// Applica subito il nuovo tema allo sfondo
        #[arg(long)]
        apply: bool,
    },
    /// Verifica che un browser Chromium sia disponibile
    Setup,
}

fn main() {
    let cli = Cli::parse();
    logging::init(cli.verbose);

    let code = match run(cli) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("Errore: {message}");
            1
        }
    };
    std::process::exit(code);
}

fn run(cli: Cli) -> Result<i32, String> {
    match cli.command {
        Commands::Next {
            theme,
            width,
            height,
            output,
            no_set,
        } => cmd_next(theme, width, height, output, no_set),
        Commands::Watch {
            no_tray,
            no_overlay,
        } => cmd_watch(no_tray, no_overlay),
        Commands::Install => cmd_install(),
        Commands::Uninstall => cmd_uninstall(),
        Commands::Config { open } => cmd_config(open),
        Commands::History { clear } => cmd_history(clear),
        Commands::Theme { name, apply } => cmd_theme(name, apply),
        Commands::Setup => cmd_setup(),
    }
}

fn validate_theme(theme: &str) -> Result<String, String> {
    let name = theme.trim().to_lowercase();
    if THEMES.contains(&name.as_str()) {
        Ok(name)
    } else {
        Err(format!(
            "Tema sconosciuto: {theme}. Temi disponibili: {}",
            THEMES.join(", ")
        ))
    }
}

fn cmd_next(
    theme: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    output: Option<PathBuf>,
    no_set: bool,
) -> Result<i32, String> {
    let theme = theme.map(|value| validate_theme(&value)).transpose()?;
    let mut app = WikiWall::from_disk();
    let result = app.change(ChangeRequest {
        theme: theme.as_deref(),
        width,
        height,
        output: output.as_deref(),
        apply: !no_set,
    })?;

    let label = if result.article.quality == "vetrina" {
        "in vetrina"
    } else {
        "di qualità"
    };
    println!("Voce {label}: {}", result.article.title);
    println!("Immagine: {}", result.image_path.display());
    if logging::enabled(logging::DEBUG) {
        if let Some((width, height)) = render::png_size(&result.image_path) {
            println!("Dimensioni: {width}x{height}");
        }
    }
    if no_set {
        println!("Sfondo non modificato (--no-set).");
    } else {
        println!("Sfondo aggiornato.");
    }
    let _ = std::io::stdout().flush();
    Ok(0)
}

fn cmd_install() -> Result<i32, String> {
    let destination = startup::install_startup(None, None)?;
    println!("Avvio automatico installato: {}", destination.display());
    println!("WikiWall partirà al prossimo accesso. Per avviarlo ora: wikiwall watch");
    Ok(0)
}

fn cmd_uninstall() -> Result<i32, String> {
    let removed = startup::uninstall_startup()?;
    println!(
        "{}",
        if removed {
            "Avvio automatico rimosso."
        } else {
            "Avvio automatico non presente."
        }
    );
    Ok(0)
}

fn cmd_config(open: bool) -> Result<i32, String> {
    let path = config_path();
    if !path.exists() {
        save_default_config(&path).map_err(|e| e.to_string())?;
    }
    if open {
        std::process::Command::new("notepad")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Impossibile aprire Notepad: {e}"))?;
        return Ok(0);
    }
    println!("File di configurazione: {}\n", path.display());
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    print!("{text}");
    Ok(0)
}

fn cmd_history(clear: bool) -> Result<i32, String> {
    let config = load_config(None);
    let history = History::new(history_path(), config.history_size);
    if clear {
        history.clear();
        println!("Storico svuotato.");
        return Ok(0);
    }
    let mut titles = history.recent();
    titles.reverse();
    if titles.is_empty() {
        println!("Lo storico è vuoto.");
        return Ok(0);
    }
    for (index, title) in titles.iter().enumerate() {
        println!("{:2}. {title}", index + 1);
    }
    Ok(0)
}

fn cmd_theme(name: Option<String>, apply: bool) -> Result<i32, String> {
    let config = load_config(None);
    let Some(name) = name else {
        println!("Tema attuale: {}", config.theme);
        println!("Temi disponibili: {}", THEMES.join(", "));
        return Ok(0);
    };
    let name = validate_theme(&name)?;
    let path = save_theme(&name, None)?;
    println!("Tema impostato: {name} ({})", path.display());
    if !apply {
        println!("Verrà usato al prossimo cambio. Per applicarlo subito: --apply");
        return Ok(0);
    }
    let mut app = WikiWall::from_disk();
    let result = app.change(ChangeRequest {
        theme: Some(&name),
        ..Default::default()
    })?;
    println!("Sfondo aggiornato: {}", result.article.title);
    Ok(0)
}

fn cmd_setup() -> Result<i32, String> {
    let browsers = render::find_browser();
    if browsers.is_empty() {
        eprintln!("Nessun browser Chromium trovato.");
        eprintln!("Installa Google Chrome, Microsoft Edge o Chromium: WikiWall usa quello");
        eprintln!("per fotografare l'HTML della copertina (nessun browser viene incluso).");
        return Ok(1);
    }
    println!("Browser disponibile:");
    for browser in browsers.iter().take(4) {
        match render::browser_version(browser) {
            Some(version) => println!("  {} — {version}", browser.display()),
            None => println!("  {}", browser.display()),
        }
    }
    println!("Pronto. Esegui: wikiwall next");
    Ok(0)
}

fn cmd_watch(no_tray: bool, no_overlay: bool) -> Result<i32, String> {
    let mut lock = single_instance::InstanceLock::new(r"Local\WikiWall-Watcher");
    if !lock.acquire() {
        println!("WikiWall è già in esecuzione: nessun secondo watcher avviato.");
        return Ok(0);
    }

    let watcher = watcher::Watcher::new(WikiWall::from_disk());
    watcher.run(!no_tray, !no_overlay);
    lock.release();
    Ok(0)
}
