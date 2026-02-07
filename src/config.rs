use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub last_deploy_dir: Option<PathBuf>,
    pub last_domain: Option<String>,
}

pub fn resolve_config_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .context("unable to resolve config directory (XDG_CONFIG_HOME or HOME)")?;

    Ok(base.join("mvre-hub").join("config.json"))
}

pub fn load() -> Result<AppConfig> {
    let path = resolve_config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let raw = fs::read_to_string(&path).with_context(|| format!("failed to read config at {}", path.display()))?;
    let cfg = serde_json::from_str(&raw).with_context(|| format!("failed to parse config at {}", path.display()))?;
    Ok(cfg)
}

pub fn save(path: &Path, config: &AppConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("failed to create config dir {}", parent.display()))?;
    }

    let serialized = serde_json::to_string_pretty(config).context("failed to serialize config")?;
    crate::util::atomic_write(path, serialized.as_bytes())
        .with_context(|| format!("failed to write config to {}", path.display()))?;
    Ok(())
}
