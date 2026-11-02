# WikiWall (port Rust)

Port in **Rust** di [WikiWall](https://github.com/Damiano-o/wikiwall): una voce di qualità
di Wikipedia in italiano, impaginata come copertina editoriale, usata come sfondo del
desktop Windows.

Questa crate è un port con parità di comportamento rispetto alla versione Python 0.3.0:
stessi comandi, stesso file di configurazione (`%LOCALAPPDATA%\WikiWall\config.toml`),
stessi file in cache, stessi trigger di cambio sfondo, stessa copertina.

## Differenze rispetto alla versione Python

| Aspetto | Python | Rust |
| --- | --- | --- |
| Distribuzione | Python 3.11+ e virtualenv | un solo `.exe`, nessun runtime |
| Rendering | Playwright (scarica Chromium, ~120 MB) | CDP diretto sul Chromium/Chrome/Edge già presente |
| Overlay cliccabile | Tkinter + Pillow (ritaglia il QR) | tre finestre layered native, alpha 1, nessuna immagine da ridisegnare |
| Tray | pystray + Pillow | `Shell_NotifyIconW` + icona 32×32 disegnata a runtime |
| Dipendenze runtime | requests, playwright, pillow, pystray, qrcode | solo il browser per il rendering |

Il QR sullo sfondo non viene più ridisegnato sopra l'overlay: le finestre sono
trasparenti (alpha 1) e lasciano vedere il wallpaper dipinto, quindi il QR resta
identico ma non serve ritagliare PNG a ogni cambio.

## Parità verificata

`scripts/parity.py` rende la stessa voce con le due implementazioni e confronta i PNG:

```text
dimensione: 1920x1080
pixel identici: 100.0000%
pixel con differenza > 8: 0 (0.000000%)
differenza massima: 1
```

Anche i rettangoli cliccabili coincidono al centesimo di pixel
(`qr=(854.40625, 941.40625, 93.59375, 93.59375)`, uguali in entrambe le versioni).
Per ottenerlo il rendering forza `--force-device-scale-factor=1`: senza quel flag
Chrome impagina a 1536×864 CSS px su un display al 125% e la copertina risulterebbe
scalata rispetto alla versione Python.

## Misure su questa macchina

| | Python | Rust |
| --- | --- | --- |
| Binario installato | venv + 5 pacchetti + Chromium 118 MB | un `wikiwall.exe` da 1,7 MB |
| RAM del watcher a riposo | 5,4 MB | 16,2 MB |
| Primo avvio | `pip install -e .` + `wikiwall setup` | nessuna installazione |

Il watcher Rust tiene in memoria i buffer di immagine, quindi a riposo pesa più della
versione Python (che delega il rendering a un processo Chromium avviato solo al cambio).
Il guadagno è nella distribuzione: nessun runtime Python e nessun download di browser.

## Requisiti

- Windows 10 o 11
- Un browser Chromium installato: Google Chrome, Microsoft Edge, Chromium oppure il
  Chromium scaricato da Playwright (se c'è, viene usato per primo)
- Per compilare: Rust (toolchain `stable-x86_64-pc-windows-msvc`)
- Connessione a Internet per scaricare le voci e le immagini

## Compilare

```powershell
cargo build --release
# il binario è in target\release\wikiwall.exe
```

## Usare

```powershell
wikiwall setup            # verifica il browser disponibile
wikiwall next             # sceglie una voce e cambia sfondo
wikiwall watch            # watcher + tray + QR cliccabile + Ctrl+Alt+N
wikiwall install          # avvio automatico al login (WikiWall.vbs)
wikiwall uninstall        # rimuove l'avvio automatico
wikiwall config --open    # apre config.toml in Notepad
wikiwall history          # ultime voci usate
wikiwall theme dark --apply
```

Opzioni di `next`: `--theme paper|dark`, `--width`, `--height`, `--output`,
`--no-set` (scrive solo `cache\preview.png`).
Opzioni di `watch`: `--no-tray`, `--no-overlay`.

Variabili d'ambiente:
- `WIKIWALL_HOME` — cambia la cartella dei dati (utile per le prove)
- `WIKIWALL_OVERLAY_DEBUG=1` — mostra le zone cliccabili anche quando il desktop non è
  in primo piano (serve solo per verificare l'overlay)

## File

Tutto in `%LOCALAPPDATA%\WikiWall\` come nella versione Python: `config.toml`,
`wallpaper.png`, `history.json`, `wikiwall.log`, `cache\quality.json`,
`cache\last_article.json`. La configurazione è quindi **interscambiabile** fra le
due versioni.

## Come funziona

1. `wikipedia.rs` legge `Wikipedia:Vetrina` e `Wikipedia:Voci di qualità` con
   `action=parse&prop=links`, filtra il namespace principale e mette in cache la lista
   per `quality_cache_days` giorni.
2. La scelta evita le ultime `history_size` voci e preferisce la Vetrina (65%).
3. `wikipedia.rs` scarica estratto, descrizione, immagine principale e URL.
4. `template.rs` costruisce l'HTML della copertina (CSS identico alla versione Python,
   font OFL incorporati, QR generato con la crate `qrcode`).
5. `render.rs` avvia il browser headless con `--remote-debugging-port=0`, parla CDP su
   WebSocket, imposta il viewport, inietta l'HTML con `Page.setDocumentContent`, attende
   `document.fonts.ready` e le immagini, legge i `getBoundingClientRect` di QR, link e
   "Salta voce" e salva lo screenshot con `Page.captureScreenshot`.
6. `wallpaper.rs` applica il PNG con `SystemParametersInfoW` e stile "Riempi".
7. `overlay.rs` mette tre finestre `WS_EX_NOACTIVATE | WS_EX_LAYERED` sopra QR, URL e
   pulsante: compaiono quando il desktop è in primo piano e non rubano il focus.
8. `hotkey.rs` registra `Ctrl+Alt+N` in un thread con message loop dedicato.
9. `watcher.rs` decide quando cambiare: intervallo, ritorno dall'inattività, desktop in
   primo piano, richiesta manuale.

## Test

```powershell
cargo test
```

I test coprono configurazione, storico, escape/percent-decoding, split del lead,
troncamento dell'estratto e costruzione della copertina.

## Licenza

MIT, come il progetto originale. Testi e immagini provengono da Wikipedia (CC BY-SA 4.0);
i font Montserrat e Source Serif 4 sono distribuiti con SIL OFL 1.1 (vedi `assets/fonts`).
