#![allow(dead_code)] // Phase 5 crash report persistence

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CrashReport {
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    pub timestamp: String,
    pub runtime: String,
    #[serde(rename = "error")]
    pub reason: String,
    #[serde(rename = "stack")]
    pub stack_trace: String,
    #[serde(rename = "isolateState")]
    pub isolate_state: String,
    #[serde(rename = "crashCount")]
    pub crash_count: u32,
}

impl CrashReport {
    /// 保存 crash report 到 logs/crash/<plugin-id>-<timestamp>.json
    pub fn save(&self) -> Result<PathBuf> {
        let crash_dir = get_crash_dir()?;
        fs::create_dir_all(&crash_dir).context("failed to create crash report directory")?;

        let sanitized_id = self.plugin_id.replace('@', "").replace('/', "-");
        let filename = format!("{}-{}.json", sanitized_id, self.timestamp);
        let path = crash_dir.join(filename);

        let json = serde_json::to_string_pretty(self)
            .context("failed to serialize crash report")?;
        fs::write(&path, json).context("failed to write crash report")?;

        tracing::info!("Crash report saved: {}", path.display());
        Ok(path)
    }
}

pub fn get_crash_dir() -> Result<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Ok(PathBuf::from(appdata).join("BetterCPT").join("logs").join("crash"));
    }
    Ok(PathBuf::from("logs").join("crash"))
}
