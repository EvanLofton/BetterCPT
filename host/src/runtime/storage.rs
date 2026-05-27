#![allow(dead_code)] // Phase 5 plugin KV storage

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const STORAGE_QUOTA_BYTES: u64 = 1024 * 1024; // 1MB

#[derive(Debug, Serialize, Deserialize)]
struct StorageFile {
    data: HashMap<String, Value>,
}

/// Plugin KV Storage，Host 管理路径和配额
pub struct PluginStorage {
    plugin_id: String,
    data: HashMap<String, Value>,
    current_size: u64,
    storage_path: PathBuf,
}

impl PluginStorage {
    pub fn new(plugin_id: &str) -> Result<Self> {
        let storage_path = get_storage_path(plugin_id);
        let mut storage = Self {
            plugin_id: plugin_id.into(),
            data: HashMap::new(),
            current_size: 0,
            storage_path,
        };

        // 尝试加载已有数据
        let _ = storage.load();
        Ok(storage)
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn set(&mut self, key: &str, value: Value) -> Result<()> {
        let entry_json = serde_json::to_string(&value)
            .context("failed to serialize storage value")?;

        let new_entry_size = key.len() as u64 + entry_json.len() as u64;

        // 如果是更新，先减去旧值大小
        let old_size = if let Some(old) = self.data.get(key) {
            let old_json = serde_json::to_string(old)
                .context("failed to serialize old value for size calculation")?;
            key.len() as u64 + old_json.len() as u64
        } else {
            0
        };

        let projected_size = self.current_size - old_size + new_entry_size;
        self.check_quota(projected_size)?;

        self.data.insert(key.into(), value);
        self.current_size = projected_size;
        self.persist()?;

        tracing::debug!(
            "Storage '{}' set key '{}' ({} bytes total)",
            self.plugin_id,
            key,
            self.current_size
        );
        Ok(())
    }

    pub fn delete(&mut self, key: &str) -> Result<()> {
        if let Some(old) = self.data.remove(key) {
            let old_json = serde_json::to_string(&old)
                .context("failed to serialize old value for size calculation")?;
            let old_size = key.len() as u64 + old_json.len() as u64;
            self.current_size = self.current_size.saturating_sub(old_size);
            self.persist()?;
        }
        Ok(())
    }

    pub fn check_quota(&self, projected_bytes: u64) -> Result<()> {
        if projected_bytes > STORAGE_QUOTA_BYTES {
            anyhow::bail!(
                "STORAGE_QUOTA_EXCEEDED: {} bytes exceeds 1MB limit for plugin '{}'",
                projected_bytes,
                self.plugin_id
            );
        }
        Ok(())
    }

    pub fn current_size(&self) -> u64 {
        self.current_size
    }

    pub fn persist(&self) -> Result<()> {
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent).context("failed to create storage directory")?;
        }
        let file = StorageFile {
            data: self.data.clone(),
        };
        let json = serde_json::to_string_pretty(&file).context("failed to serialize storage")?;
        fs::write(&self.storage_path, json).context("failed to write storage file")?;
        Ok(())
    }

    pub fn load(&mut self) -> Result<()> {
        if !self.storage_path.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&self.storage_path)
            .with_context(|| format!("failed to read storage: {}", self.storage_path.display()))?;
        let file: StorageFile = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse storage: {}", self.storage_path.display()))?;
        self.data = file.data;
        // 重新计算大小
        let mut size: u64 = 0;
        for (k, v) in &self.data {
            let v_json = serde_json::to_string(v)
                .with_context(|| format!("failed to serialize value for key '{}' during load", k))?;
            size += k.len() as u64 + v_json.len() as u64;
        }
        self.current_size = size;
        Ok(())
    }
}

fn get_storage_path(plugin_id: &str) -> PathBuf {
    let base = if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("BetterCPT").join("plugins")
    } else {
        PathBuf::from("plugins")
    };
    let dir_name = plugin_id.strip_prefix('@').unwrap_or(plugin_id).replace('/', "-");
    base.join(dir_name).join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_storage(plugin_id: &str) -> PluginStorage {
        PluginStorage {
            plugin_id: plugin_id.into(),
            data: HashMap::new(),
            current_size: 0,
            storage_path: std::env::temp_dir().join(format!("bettercpt_test_{}_config.json", plugin_id)),
        }
    }

    #[test]
    fn set_and_get() {
        let mut s = test_storage("test");
        s.set("key1", json!("value1")).unwrap();
        assert_eq!(s.get("key1"), Some(&json!("value1")));
    }

    #[test]
    fn delete_key() {
        let mut s = test_storage("test");
        s.set("temp", json!(42)).unwrap();
        s.delete("temp").unwrap();
        assert_eq!(s.get("temp"), None);
    }

    #[test]
    fn quota_exceeded() {
        let mut s = test_storage("test");
        // 写入接近 1MB 的单个值
        let big = "x".repeat(800_000);
        assert!(s.set("big", json!(big)).is_ok());
        // 再写一个大的，总和超过 1MB
        let more = "y".repeat(300_000);
        assert!(s.set("more", json!(more)).is_err());
    }

    #[test]
    fn check_quota_rejects_over_limit() {
        let s = test_storage("test");
        assert!(s.check_quota(STORAGE_QUOTA_BYTES + 1).is_err());
        assert!(s.check_quota(STORAGE_QUOTA_BYTES).is_ok());
    }
}
