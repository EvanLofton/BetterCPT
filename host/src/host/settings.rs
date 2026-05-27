use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    // 常规
    pub auto_start: bool,
    pub minimize_to_tray: bool,
    pub dev_mode: bool,
    // 外观
    pub theme: String, // "light" | "dark"
    // 插件
    pub auto_enable_new: bool,
    pub auto_restart_crash: bool,
    pub max_crash_restart: u32,
    pub storage_quota: String,
    // 日志
    pub log_level: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_start: true,
            minimize_to_tray: false,
            dev_mode: false,
            theme: "light".into(),
            auto_enable_new: true,
            auto_restart_crash: true,
            max_crash_restart: 5,
            storage_quota: "1 MB".into(),
            log_level: "Info".into(),
        }
    }
}

fn settings_path() -> Result<PathBuf> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    Ok(PathBuf::from(appdata).join("BetterCPT").join("settings.json"))
}

pub fn load_settings() -> Result<AppSettings> {
    let path = settings_path()?;
    if path.exists() {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read settings from {}", path.display()))?;
        let settings: AppSettings = serde_json::from_str(&raw)
            .with_context(|| "Failed to parse settings.json")?;
        tracing::info!("Settings loaded from {}", path.display());
        Ok(settings)
    } else {
        let defaults = AppSettings::default();
        // 首次运行：写入默认设置文件
        save_settings(&defaults)?;
        tracing::info!("Default settings written to {}", path.display());
        Ok(defaults)
    }
}

pub fn save_settings(settings: &AppSettings) -> Result<()> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(settings)
        .with_context(|| "Failed to serialize settings")?;
    fs::write(&path, &raw)
        .with_context(|| format!("Failed to write settings to {}", path.display()))?;
    tracing::info!("Settings saved to {}", path.display());
    Ok(())
}
