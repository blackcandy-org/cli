use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    pub server: Option<String>,
    pub email: Option<String>,
    pub api_token: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        let directory = path
            .parent()
            .context("config path unexpectedly has no parent directory")?;
        fs::create_dir_all(directory)
            .with_context(|| format!("failed to create {}", directory.display()))?;

        let raw = toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&path)?.permissions();
            permissions.set_mode(0o600);
            fs::set_permissions(&path, permissions)?;
        }

        Ok(())
    }

    pub fn require_server(&self) -> Result<&str> {
        self.server
            .as_deref()
            .filter(|value| !value.is_empty())
            .context("server is not configured; run `blackcandy login <server>` first")
    }

    pub fn require_token(&self) -> Result<String> {
        self.api_token
            .clone()
            .filter(|value| !value.is_empty())
            .context("API token is not configured; run `blackcandy login <server>` first")
    }
}

pub fn config_path() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("BLACKCANDY_CONFIG") {
        return Ok(PathBuf::from(path));
    }

    let Some(mut directory) = dirs::config_dir() else {
        bail!("could not find a user config directory");
    };
    directory.push("blackcandy-cli");
    directory.push("config.toml");
    Ok(directory)
}
