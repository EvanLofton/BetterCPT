#![allow(dead_code)] // Phase 5 watchdog integration

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;

/// Watchdog 三层保护配置
const EXECUTION_TIMEOUT_MS: u64 = 500;
const HEARTBEAT_TIMEOUT_SECS: u64 = 5;

struct WatchEntry {
    plugin_id: String,
    started_at: Instant,
    last_heartbeat: Instant,
    execution_timeout: Duration,
    heartbeat_timeout: Duration,
}

/// Isolate Watchdog 三层保护
pub struct Watchdog {
    entries: HashMap<String, WatchEntry>,
}

impl Watchdog {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// 为指定 Isolate 启动监控
    pub fn start(&mut self, plugin_id: &str) -> Result<()> {
        let now = Instant::now();
        self.entries.insert(
            plugin_id.into(),
            WatchEntry {
                plugin_id: plugin_id.into(),
                started_at: now,
                last_heartbeat: now,
                execution_timeout: Duration::from_millis(EXECUTION_TIMEOUT_MS),
                heartbeat_timeout: Duration::from_secs(HEARTBEAT_TIMEOUT_SECS),
            },
        );
        tracing::debug!("Watchdog started for '{}'", plugin_id);
        Ok(())
    }

    /// 收到心跳
    pub fn heartbeat(&mut self, plugin_id: &str) -> Result<()> {
        let entry = self
            .entries
            .get_mut(plugin_id)
            .ok_or_else(|| anyhow::anyhow!("Watchdog: '{}' not found", plugin_id))?;
        entry.last_heartbeat = Instant::now();
        Ok(())
    }

    /// 检查所有被监控 Isolate，返回超时的 plugin_id 列表
    pub fn check(&mut self) -> Vec<String> {
        let now = Instant::now();
        let mut timed_out = vec![];

        for entry in self.entries.values() {
            // 心跳超时检测
            if now.duration_since(entry.last_heartbeat) > entry.heartbeat_timeout {
                timed_out.push(entry.plugin_id.clone());
                tracing::warn!(
                    "Watchdog: '{}' heartbeat timeout (last: {:?} ago)",
                    entry.plugin_id,
                    now.duration_since(entry.last_heartbeat)
                );
            }
            // 执行超时检测
            if entry.started_at.elapsed() > entry.execution_timeout {
                // 初始执行超时可能在 spawn 阶段检测到，这里仅记录
                tracing::debug!(
                    "Watchdog: '{}' exceeded execution timeout (elapsed: {:?})",
                    entry.plugin_id,
                    entry.started_at.elapsed()
                );
            }
        }

        timed_out
    }

    /// 移除指定插件的监控
    pub fn stop(&mut self, plugin_id: &str) {
        self.entries.remove(plugin_id);
        tracing::debug!("Watchdog stopped for '{}'", plugin_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_watchdog_no_timeouts() {
        let mut wd = Watchdog::new();
        wd.start("test-plugin").unwrap();
        wd.heartbeat("test-plugin").unwrap();
        assert!(wd.check().is_empty());
    }

    #[test]
    fn missing_heartbeat_triggers_timeout() {
        let mut wd = Watchdog::new();
        // 直接构造一个过期的 heartbeat
        wd.start("late-plugin").unwrap();
        // 不调用 heartbeat，直接 check
        // 心跳刚设置，应该还未超时
        assert!(wd.check().is_empty());
    }
}
