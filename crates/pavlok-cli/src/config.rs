//! Persistent configuration: the stored Pavlok API token.
//!
//! Token resolution order (highest priority first):
//!   1. `PAVLOK_TOKEN` environment variable
//!   2. the config file, whose location is platform-dependent (see
//!      [`config_path`])

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub token: Option<String>,
}

/// Absolute path to the config file, creating no directories.
///
/// Resolved via `directories::ProjectDirs`, so it differs per platform:
///
/// | Platform | Path |
/// |---|---|
/// | Linux   | `~/.config/pavlok-cli/config.toml` |
/// | macOS   | `~/Library/Application Support/com.pavlok.pavlok-cli/config.toml` |
/// | Windows | `%APPDATA%\pavlok\pavlok-cli\config\config.toml` |
pub fn config_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "pavlok", "pavlok-cli")
        .context("could not determine a config directory for this platform")?;
    Ok(dirs.config_dir().join("config.toml"))
}

impl Config {
    /// Load the config file, returning defaults if it does not exist.
    pub fn load() -> Result<Config> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Config::default());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&path) {
                let mode = meta.permissions().mode() & 0o777;
                if mode != 0o600 {
                    eprintln!(
                        "warning: config at {} has permissions {:03o}, expected 600",
                        path.display(),
                        mode
                    );
                }
            }
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading config at {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing config at {}", path.display()))
    }
}

/// Resolve a token from the environment, then the config file.
pub fn resolve_token() -> Result<Option<String>> {
    if let Ok(token) = std::env::var("PAVLOK_TOKEN")
        && !token.trim().is_empty()
    {
        return Ok(Some(token));
    }
    Ok(Config::load()?.token)
}

/// Resolve a token or fail with an actionable message.
pub fn require_token() -> Result<String> {
    resolve_token()?.context(
        "no Pavlok token found — run `pavlok-cli login` or set the PAVLOK_TOKEN environment variable",
    )
}

/// Persist a token to the config file, returning the path written.
pub fn save_token(token: &str) -> Result<PathBuf> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating config directory {}", parent.display()))?;
        // The directory holds a credential; keep it out of other users' reach.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                .with_context(|| format!("setting permissions on {}", parent.display()))?;
        }
    }
    let cfg = Config {
        token: Some(token.to_string()),
    };
    let text = toml::to_string_pretty(&cfg).context("serializing config")?;
    write_private(&path, &text).with_context(|| format!("writing config to {}", path.display()))?;

    Ok(path)
}

/// Write `text` to `path`, never leaving the file readable by anyone else.
///
/// The mode is set as the file is created rather than afterwards: a plain
/// write-then-chmod leaves the token on disk at the umask default (commonly
/// 0644) for as long as the write takes. The explicit `set_permissions` after
/// the fact is still needed because `create` does not apply `mode` to a file
/// that already exists — e.g. one left at 0644 by an older version.
#[cfg(unix)]
fn write_private(path: &std::path::Path, text: &str) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private(path: &std::path::Path, text: &str) -> Result<()> {
    fs::write(path, text)?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pavlok-cli-test-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn mode_of(path: &std::path::Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn write_private_creates_the_file_owner_only() {
        let dir = scratch("create");
        let path = dir.join("config.toml");
        write_private(&path, "token = \"abc\"\n").unwrap();

        assert_eq!(mode_of(&path), 0o600);
        assert_eq!(fs::read_to_string(&path).unwrap(), "token = \"abc\"\n");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_private_tightens_an_existing_world_readable_file() {
        // A config left at 0644 by an older version must not stay that way:
        // OpenOptions::mode only applies when the file is actually created.
        let dir = scratch("tighten");
        let path = dir.join("config.toml");
        fs::write(&path, "token = \"old\"\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        write_private(&path, "token = \"new\"\n").unwrap();

        assert_eq!(mode_of(&path), 0o600);
        assert_eq!(fs::read_to_string(&path).unwrap(), "token = \"new\"\n");
        fs::remove_dir_all(&dir).unwrap();
    }
}
