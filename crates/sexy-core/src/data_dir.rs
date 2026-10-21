//! Locating and validating the user's game install (bring-your-own-assets).

use std::path::{Path, PathBuf};

pub const DATA_ENV_VAR: &str = "INSANIQUARIUM_DATA";

/// Files that must exist for a directory to count as a game install.
const REQUIRED: &[&str] = &["properties/resources.xml", "images", "sounds", "data"];

#[derive(Debug, thiserror::Error)]
pub enum DataDirError {
    #[error(
        "no game data directory given; pass --data <path> or set {DATA_ENV_VAR} to your Insaniquarium Deluxe install"
    )]
    NotConfigured,
    #[error("{path} is not an Insaniquarium Deluxe install (missing {missing})")]
    Invalid {
        path: PathBuf,
        missing: &'static str,
    },
}

/// A validated game install root.
#[derive(Debug, Clone)]
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    /// Resolve from an explicit CLI value, falling back to `INSANIQUARIUM_DATA`.
    pub fn resolve(cli: Option<&Path>) -> Result<Self, DataDirError> {
        let path = match cli {
            Some(p) => p.to_path_buf(),
            None => std::env::var_os(DATA_ENV_VAR)
                .map(PathBuf::from)
                .ok_or(DataDirError::NotConfigured)?,
        };
        Self::open(path)
    }

    pub fn open(root: impl Into<PathBuf>) -> Result<Self, DataDirError> {
        let root = root.into();
        if let Some(missing) = REQUIRED.iter().find(|r| !root.join(r).exists()) {
            return Err(DataDirError::Invalid {
                path: root,
                missing,
            });
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Join a resource-relative path. Accepts the original's `\` separators.
    pub fn join(&self, rel: &str) -> PathBuf {
        self.root.join(rel.replace('\\', "/"))
    }

    /// Case-insensitively resolve an existing file path. The original ran on
    /// case-insensitive Windows filesystems, so resource paths often differ
    /// in case from the files on disk.
    pub fn find(&self, rel: &str) -> Option<PathBuf> {
        let rel = rel.replace('\\', "/");
        let mut cur = self.root.clone();
        for part in rel.split('/').filter(|p| !p.is_empty()) {
            let exact = cur.join(part);
            cur = if exact.exists() {
                exact
            } else {
                std::fs::read_dir(&cur)
                    .ok()?
                    .flatten()
                    .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?
                    .path()
            };
        }
        Some(cur)
    }

    /// Like [`find`](Self::find) for a path with no extension, trying `exts`
    /// in order (e.g. `["png", "gif", "jpg"]`).
    pub fn find_with_ext(&self, rel: &str, exts: &[&str]) -> Option<PathBuf> {
        exts.iter()
            .find_map(|ext| self.find(&format!("{rel}.{ext}")))
    }
}

/// The reference install used by tests, if available. Tests that need real
/// assets must skip when this returns `None`.
pub fn test_data_dir() -> Option<DataDir> {
    let from_env = std::env::var_os(DATA_ENV_VAR).map(PathBuf::from);
    let default = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../original Game");
    from_env
        .into_iter()
        .chain(std::iter::once(default))
        .find_map(|p| DataDir::open(p).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_install() {
        let err = DataDir::open(env!("CARGO_MANIFEST_DIR")).unwrap_err();
        assert!(matches!(err, DataDirError::Invalid { .. }));
    }

    #[test]
    fn finds_files_case_insensitively() {
        let Some(d) = test_data_dir() else { return };
        // On disk: images/Aquarium5.jpg; resources.xml refers to "aquarium5".
        let p = d.find_with_ext("images/aquarium5", &["png", "gif", "jpg"]);
        assert!(p.is_some_and(|p| p.ends_with("Aquarium5.jpg")));
    }
}
