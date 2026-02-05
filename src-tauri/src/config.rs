use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub shortcut: String,
    pub active_model: String,
    pub preferred_model: String,
    pub language: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            shortcut: "Ctrl+Alt+Space".to_string(),
            active_model: "base".to_string(),
            preferred_model: "base".to_string(),
            language: "en".to_string(),
        }
    }
}

pub fn config_path() -> Result<PathBuf> {
    let dirs = BaseDirs::new().context("missing base dirs")?;
    let dir = dirs.config_dir().join("ECO");
    fs::create_dir_all(&dir).context("create config dir")?;
    Ok(dir.join("config.json"))
}

pub fn load_config() -> Result<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let data = fs::read_to_string(&path).context("read config")?;
    let config = serde_json::from_str(&data).context("parse config")?;
    Ok(config)
}

pub fn save_config(config: &AppConfig) -> Result<()> {
    let path = config_path()?;
    let data = serde_json::to_string_pretty(config).context("serialize config")?;
    fs::write(path, data).context("write config")?;
    Ok(())
}
