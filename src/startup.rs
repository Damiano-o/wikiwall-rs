//! `WikiWall.vbs` in the Startup folder: start the watcher at login.

use std::path::{Path, PathBuf};

pub const STARTUP_FILENAME: &str = "WikiWall.vbs";

pub fn startup_dir() -> Result<PathBuf, String> {
    let appdata = std::env::var_os("APPDATA")
        .ok_or_else(|| "La variabile APPDATA non è impostata.".to_string())?;
    Ok(PathBuf::from(appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup"))
}

pub fn startup_path() -> Result<PathBuf, String> {
    Ok(startup_dir()?.join(STARTUP_FILENAME))
}

fn vbs_content(executable: &Path, working_dir: &Path) -> String {
    format!(
        "Set shell = CreateObject(\"WScript.Shell\")\n\
         shell.CurrentDirectory = \"{}\"\n\
         shell.Run Chr(34) & \"{}\" & Chr(34) & \" watch\", 0, False\n",
        working_dir.display(),
        executable.display()
    )
}

/// Write the launcher as UTF-16 (with BOM), which Windows Script Host reads reliably.
pub fn install_startup(
    executable: Option<&Path>,
    working_dir: Option<&Path>,
) -> Result<PathBuf, String> {
    let destination = startup_path()?;
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let executable = executable
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_exe().ok())
        // Prefer the GUI subsystem binary when it exists, so no console flashes.
        .map(|path| {
            let windowed = path.with_file_name("wikiwallw.exe");
            if windowed.is_file() {
                windowed
            } else {
                path
            }
        })
        .ok_or_else(|| "Percorso dell'eseguibile non disponibile".to_string())?;
    let working_dir = working_dir
        .map(Path::to_path_buf)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| PathBuf::from("."));

    let mut bytes: Vec<u8> = vec![0xFF, 0xFE];
    for unit in vbs_content(&executable, &working_dir).encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(&destination, bytes).map_err(|e| e.to_string())?;
    Ok(destination)
}

pub fn uninstall_startup() -> Result<bool, String> {
    let destination = startup_path()?;
    match std::fs::remove_file(&destination) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_quoted_launcher() {
        let text = vbs_content(
            Path::new(r"C:\tools\wikiwall.exe"),
            Path::new(r"C:\Users\me"),
        );
        assert!(
            text.contains("shell.Run Chr(34) & \"C:\\tools\\wikiwall.exe\" & Chr(34) & \" watch\"")
        );
        assert!(text.contains("shell.CurrentDirectory = \"C:\\Users\\me\""));
    }
}
