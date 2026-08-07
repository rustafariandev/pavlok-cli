//! Persistent configuration: the stored Pavlok API token.
//!
//! Token resolution order (highest priority first):
//!   1. `PAVLOK_TOKEN` environment variable
//!   2. `~/.config/pavlok-cli/config.toml`

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
    }
    let cfg = Config {
        token: Some(token.to_string()),
    };
    let text = toml::to_string_pretty(&cfg).context("serializing config")?;
    fs::write(&path, text).with_context(|| format!("writing config to {}", path.display()))?;

    // The token is a credential; restrict to owner read/write on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("setting permissions on {}", path.display()))?;
    }

    Ok(path)
}
