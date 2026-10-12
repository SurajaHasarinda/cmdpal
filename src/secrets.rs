use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const SERVICE: &str = "cmdpal";
const ACCOUNT: &str = "ai_api_key";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Keyring,
    File,
}

pub fn file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("cmdpal").join("credentials"))
}

/// Where a newly stored key would land.
pub fn backend() -> Backend {
    if entry().is_ok() {
        Backend::Keyring
    } else {
        Backend::File
    }
}

pub fn store(key: &str) -> Result<Backend> {
    if let Ok(entry) = entry() {
        if entry.set_password(key).is_ok() {
            return Ok(Backend::Keyring);
        }
    }
    let path = file_path().context("no OS credential store and no config directory")?;
    store_file(&path, key).with_context(|| format!("could not write {}", path.display()))?;
    Ok(Backend::File)
}

pub fn load() -> Option<Zeroizing<String>> {
    load_keyring().or_else(|| file_path().and_then(|path| load_file(&path)))
}

pub fn available() -> bool {
    entry().is_ok() || file_path().is_some()
}

fn load_keyring() -> Option<Zeroizing<String>> {
    entry().ok()?.get_password().ok().map(Zeroizing::new)
}

fn load_file(path: &Path) -> Option<Zeroizing<String>> {
    let key = std::fs::read_to_string(path).ok()?;
    let key = key.trim();
    (!key.is_empty()).then(|| Zeroizing::new(key.to_string()))
}

fn store_file(path: &Path, key: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_private(path, key)
}

/// Create the file already restricted, so the key is never briefly readable by
/// other users between write and chmod.
#[cfg(unix)]
fn write_private(path: &Path, key: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(key.as_bytes())
}

#[cfg(not(unix))]
fn write_private(path: &Path, key: &str) -> std::io::Result<()> {
    std::fs::write(path, key)
}

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT).context("no OS credential store available")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_backend_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("credentials");
        store_file(&path, "AIza-secret").unwrap();
        assert_eq!(
            load_file(&path).map(|key| key.to_string()),
            Some("AIza-secret".to_string())
        );
    }

    #[test]
    fn a_missing_file_yields_no_key() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_file(&dir.path().join("credentials")).is_none());
    }

    #[test]
    fn an_empty_file_yields_no_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials");
        std::fs::write(&path, "  \n").unwrap();
        assert!(load_file(&path).is_none());
    }

    #[test]
    fn a_trailing_newline_is_tolerated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials");
        std::fs::write(&path, "AIza-secret\n").unwrap();
        assert_eq!(
            load_file(&path).map(|key| key.to_string()),
            Some("AIza-secret".to_string())
        );
    }

    #[test]
    fn storing_twice_replaces_the_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials");
        store_file(&path, "AIza-old").unwrap();
        store_file(&path, "AIza-new").unwrap();
        assert_eq!(
            load_file(&path).map(|key| key.to_string()),
            Some("AIza-new".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_readable_only_by_its_owner() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials");
        store_file(&path, "AIza-secret").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
