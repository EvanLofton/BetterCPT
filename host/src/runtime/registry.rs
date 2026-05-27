use std::collections::HashMap;

use anyhow::Result;
use serde::Serialize;

/// Runtime 条目
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEntry {
    pub id: String,
    pub runtime_type: String,
    pub status: String,
}

/// 快照条目：包含运行时监控数据
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotEntry {
    pub id: String,
    pub runtime_type: String,
    pub status: String,
    #[serde(default)]
    pub memory_bytes: u64,
    #[serde(default)]
    pub crash_count: u32,
    #[serde(default)]
    pub last_heartbeat: Option<String>,
}

/// RuntimeRegistry：管理所有 Runtime 的注册/注销/查询
pub struct RuntimeRegistry {
    entries: HashMap<String, RuntimeEntry>,
    crash_counters: HashMap<String, u32>,
    memory_samples: HashMap<String, u64>,
    last_heartbeats: HashMap<String, String>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            crash_counters: HashMap::new(),
            memory_samples: HashMap::new(),
            last_heartbeats: HashMap::new(),
        }
    }

    pub fn register(&mut self, entry: RuntimeEntry) -> Result<()> {
        let id = entry.id.clone();
        if self.entries.contains_key(&id) {
            anyhow::bail!("Runtime '{}' already registered", id);
        }
        tracing::info!(
            "Runtime registered: id={}, type={}, status={}",
            entry.id,
            entry.runtime_type,
            entry.status
        );
        self.entries.insert(id, entry);
        Ok(())
    }

    pub fn unregister(&mut self, id: &str) -> Result<()> {
        if self.entries.remove(id).is_none() {
            anyhow::bail!("Runtime '{}' not found", id);
        }
        self.crash_counters.remove(id);
        self.memory_samples.remove(id);
        self.last_heartbeats.remove(id);
        tracing::info!("Runtime unregistered: id={}", id);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn list(&self) -> Result<Vec<RuntimeEntry>> {
        Ok(self.entries.values().cloned().collect())
    }

    #[allow(dead_code)]
    pub fn get(&self, id: &str) -> Option<&RuntimeEntry> {
        self.entries.get(id)
    }

    pub fn update_status(&mut self, id: &str, status: &str) -> Result<()> {
        let entry = self
            .entries
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Runtime '{}' not found", id))?;
        entry.status = status.to_string();
        tracing::debug!("Runtime '{}' status -> {}", id, status);
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// 记录崩溃
    #[allow(dead_code)]
    pub fn record_crash(&mut self, id: &str) {
        *self.crash_counters.entry(id.into()).or_default() += 1;
        tracing::warn!("Crash recorded for '{}' (count: {})", id, self.crash_counters[id]);
    }

    /// 更新内存采样
    #[allow(dead_code)]
    pub fn update_memory(&mut self, id: &str, bytes: u64) {
        self.memory_samples.insert(id.into(), bytes);
    }

    /// 更新最后心跳时间
    #[allow(dead_code)]
    pub fn update_heartbeat(&mut self, id: &str, timestamp: &str) {
        self.last_heartbeats.insert(id.into(), timestamp.into());
    }

    /// 获取运行时快照（Dashboard Snapshot API）
    pub fn get_runtime_snapshot(&self) -> serde_json::Value {
        let entries: Vec<SnapshotEntry> = self
            .entries
            .values()
            .map(|e| SnapshotEntry {
                id: e.id.clone(),
                runtime_type: e.runtime_type.clone(),
                status: e.status.clone(),
                memory_bytes: self.memory_samples.get(&e.id).copied().unwrap_or(0),
                crash_count: self.crash_counters.get(&e.id).copied().unwrap_or(0),
                last_heartbeat: self.last_heartbeats.get(&e.id).cloned(),
            })
            .collect();

        serde_json::json!({
            "total": entries.len(),
            "plugins": entries,
        })
    }
}
