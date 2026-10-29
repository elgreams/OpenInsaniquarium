//! Finding the user's Insaniquarium Deluxe install (bring your own assets):
//! `--data` → `INSANIQUARIUM_DATA` → the config file → a few usual install
//! locations (remembered in the config file when found). The engine never
//! talks to Steam; it only looks for the game's files.

use std::path::{Path, PathBuf};

use sexy_core::data_dir::{DATA_ENV_VAR, DataDir};

/// `<config dir>/openinsaniquarium/data-path.txt`: one line, the install
/// folder.
pub fn config_file() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    base.map(|b| b.join("openinsaniquarium").join("data-path.txt"))
}

fn usual_locations() -> Vec<PathBuf> {
    let mut v = Vec::new();
    let game = "steamapps/common/Insaniquarium Deluxe";
    if cfg!(windows) {
        for pf in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(p) = std::env::var_os(pf) {
                let p = PathBuf::from(p);
                v.push(p.join("Steam").join(game));
                v.push(p.join("PopCap Games").join("Insaniquarium Deluxe"));
            }
        }
    } else if let Some(h) = std::env::var_os("HOME") {
        let h = PathBuf::from(h);
        for steam in [
            ".steam/steam",
            ".steam/debian-installation",
            ".local/share/Steam",
        ] {
            v.push(h.join(steam).join(game));
        }
    }
    // Next to the executable.
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        v.push(dir.join("Insaniquarium Deluxe"));
    }
    v
}

/// Resolve the install, or a message telling the user how to point us at it.
pub fn find_data(cli: Option<PathBuf>) -> Result<DataDir, String> {
    if cli.is_some() || std::env::var_os(DATA_ENV_VAR).is_some() {
        return DataDir::resolve(cli.as_deref()).map_err(|e| e.to_string());
    }
    let cfg = config_file();
    if let Some(p) = cfg.as_ref().and_then(|c| std::fs::read_to_string(c).ok()) {
        let p = PathBuf::from(p.trim());
        return DataDir::open(&p).map_err(|e| {
            format!(
                "{e}\n\nThe folder comes from {}. Edit or delete that file.",
                cfg.as_ref().unwrap().display()
            )
        });
    }
    if let Some(d) = usual_locations()
        .into_iter()
        .find_map(|p| DataDir::open(p).ok())
    {
        if let Some(c) = &cfg {
            let _ = std::fs::create_dir_all(c.parent().unwrap());
            let _ = std::fs::write(c, d.root().display().to_string());
        }
        return Ok(d);
    }
    Err(format!(
        "OpenInsaniquarium needs your own copy of Insaniquarium Deluxe.\n\n\
         Tell it where the game is installed (the folder with \"images\", \"sounds\" \
         and \"properties\") in one of these ways:\n\n\
         - start it with:  openinsaniquarium --data \"<game folder>\"\n\
         - or set the {DATA_ENV_VAR} environment variable\n\
         - or write the folder path into:\n  {}\n",
        cfg.map_or("(no config folder found)".into(), |c| c
            .display()
            .to_string())
    ))
}

/// Show `msg` to the user: on the terminal, and in a message box on
/// Windows (where a double-clicked game has no console).
pub fn report(msg: &str) {
    eprintln!("{msg}");
    #[cfg(windows)]
    {
        // A PowerShell message box: no extra dependency.
        let script = format!(
            "Add-Type -AssemblyName PresentationFramework; [System.Windows.MessageBox]::Show(@'\n{}\n'@, 'OpenInsaniquarium')",
            msg
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
            .status();
    }
}
