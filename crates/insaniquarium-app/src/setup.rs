//! Finding the user's Insaniquarium Deluxe install (bring your own assets).
//! In order, the first that is a valid install wins:
//!
//! 1. `--data <folder>`;
//! 2. `INSANIQUARIUM_DATA`;
//! 3. `data-path.txt` next to the program (shipped with a comment only);
//! 4. the `gameData` folder next to the program (the game files dragged in);
//! 5. `<config dir>/openinsaniquarium/data-path.txt`;
//! 6. the usual install folders, incl. every Steam library (remembered in
//!    the config file when found).
//!
//! The engine never talks to Steam; it only looks for the game's files.

use std::path::{Path, PathBuf};

use sexy_core::data_dir::{DATA_ENV_VAR, DataDir};

pub const PATH_FILE: &str = "data-path.txt";
pub const GAME_DATA_DIR: &str = "gameData";

/// The comment a fresh `data-path.txt` holds (also shipped in the archive).
pub const PATH_FILE_TEMPLATE: &str = "\
# OpenInsaniquarium: where is your Insaniquarium Deluxe?
# Put the path of the game folder (the one with \"images\", \"sounds\" and
# \"properties\" in it) on a line below, for example:
#   C:\\Program Files (x86)\\Steam\\steamapps\\common\\Insaniquarium Deluxe
# Or leave this file alone and copy the game's files into the \"gameData\"
# folder next to the program. Lines starting with # are ignored.
";

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
}

/// `<config dir>/openinsaniquarium/data-path.txt`.
pub fn config_file() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    base.map(|b| b.join("openinsaniquarium").join(PATH_FILE))
}

/// The folder a path file names: its first line that isn't blank or a
/// `#` comment.
pub fn parse_path_file(text: &str) -> Option<PathBuf> {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| PathBuf::from(l.trim_matches('"')))
}

fn read_path_file(file: &Path) -> Option<PathBuf> {
    parse_path_file(&std::fs::read_to_string(file).ok()?)
}

/// Library folders listed in Steam's `libraryfolders.vdf` (`"path" "…"`).
pub fn parse_library_folders(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|l| {
            let mut parts = l.split('"').filter(|p| !p.trim().is_empty());
            if parts.next()? != "path" {
                return None;
            }
            Some(PathBuf::from(parts.next()?.replace("\\\\", "\\")))
        })
        .collect()
}

fn usual_locations() -> Vec<PathBuf> {
    let mut steam_roots = Vec::new();
    let mut v = Vec::new();
    if cfg!(windows) {
        for pf in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(p) = std::env::var_os(pf) {
                let p = PathBuf::from(p);
                steam_roots.push(p.join("Steam"));
                v.push(p.join("PopCap Games").join("Insaniquarium Deluxe"));
            }
        }
    } else if let Some(h) = std::env::var_os("HOME") {
        let h = PathBuf::from(h);
        for steam in [
            ".steam/steam",
            ".steam/debian-installation",
            ".local/share/Steam",
            ".var/app/com.valvesoftware.Steam/.local/share/Steam",
        ] {
            steam_roots.push(h.join(steam));
        }
    }
    let mut libraries = steam_roots.clone();
    for root in &steam_roots {
        if let Ok(vdf) = std::fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            libraries.extend(parse_library_folders(&vdf));
        }
    }
    for lib in libraries {
        v.push(lib.join("steamapps/common/Insaniquarium Deluxe"));
    }
    v
}

/// Resolve the install, or a message telling the user how to point us at it.
pub fn find_data(cli: Option<PathBuf>) -> Result<DataDir, String> {
    if cli.is_some() || std::env::var_os(DATA_ENV_VAR).is_some() {
        return DataDir::resolve(cli.as_deref()).map_err(|e| e.to_string());
    }
    let exe = exe_dir();
    let local_file = exe.as_ref().map(|d| d.join(PATH_FILE));
    if let Some(p) = local_file.as_deref().and_then(read_path_file) {
        return DataDir::open(&p).map_err(|e| {
            format!(
                "{e}\n\nThe folder comes from {}. Fix the path there.",
                local_file.as_ref().unwrap().display()
            )
        });
    }
    let game_data = exe.as_ref().map(|d| d.join(GAME_DATA_DIR));
    if let Some(d) = game_data.as_ref().and_then(|p| DataDir::open(p).ok()) {
        return Ok(d);
    }
    let cfg = config_file();
    if let Some(p) = cfg.as_deref().and_then(read_path_file) {
        return DataDir::open(&p).map_err(|e| {
            format!(
                "{e}\n\nThe folder comes from {}. Fix the path there.",
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
            let _ = std::fs::write(c, format!("{PATH_FILE_TEMPLATE}{}\n", d.root().display()));
        }
        return Ok(d);
    }
    // Nothing found: leave a path file to fill in where there's none.
    let file = local_file
        .filter(|f| f.exists() || std::fs::write(f, PATH_FILE_TEMPLATE).is_ok())
        .or_else(|| {
            let c = cfg?;
            std::fs::create_dir_all(c.parent()?).ok()?;
            (c.exists() || std::fs::write(&c, PATH_FILE_TEMPLATE).is_ok()).then_some(c)
        });
    let shown =
        |p: Option<PathBuf>| p.map_or("(not available)".into(), |p| p.display().to_string());
    Err(format!(
        "OpenInsaniquarium needs your own copy of Insaniquarium Deluxe.\n\n\
         Do one of these, then start it again:\n\n\
         1. Copy the game's files (the folders \"images\", \"sounds\", \"properties\", \
         \"data\" and the rest) into:\n   {}\n\n\
         2. Or put the game folder's path into:\n   {}\n\n\
         3. Or start it with:  openinsaniquarium --data \"<game folder>\"\n",
        shown(game_data),
        shown(file),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_file_skips_comments_and_quotes() {
        assert_eq!(parse_path_file(PATH_FILE_TEMPLATE), None);
        let t = format!("{PATH_FILE_TEMPLATE}\n  \"/games/Insaniquarium Deluxe\"  \n");
        assert_eq!(
            parse_path_file(&t),
            Some(PathBuf::from("/games/Insaniquarium Deluxe"))
        );
    }

    #[test]
    fn steam_library_paths() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t\t\"label\"\t\t\"\"\n\t}\n}\n";
        assert_eq!(
            parse_library_folders(vdf),
            vec![
                PathBuf::from("C:\\Program Files (x86)\\Steam"),
                PathBuf::from("D:\\SteamLibrary")
            ]
        );
    }
}
