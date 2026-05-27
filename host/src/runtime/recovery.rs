#![allow(dead_code)] // Phase 5 recovery manager

use std::collections::HashMap;

use anyhow::Result;

use crate::runtime::crash_report::CrashReport;
use crate::runtime::isolate_pool::IsolatePool;
use crate::runtime::registry::RuntimeRegistry;

/// 崩溃恢复管理器（Isolate 级），最多自动重启 5 次
pub struct RecoveryManager {
    max_restarts: u32,
    crash_counts: HashMap<String, u32>,
}

impl RecoveryManager {
    pub fn new() -> Self {
        Self {
            max_restarts: 5,
            crash_counts: HashMap::new(),
        }
    }

    /// 检测到崩溃，检查重启次数，决定是否重启
    pub fn on_crash(
        &mut self,
        plugin_id: &str,
        reason: &str,
        stack_trace: &str,
        isolate_pool: &mut IsolatePool,
        code: &str,
        runtime_registry: &mut RuntimeRegistry,
    ) -> Result<()> {
        let count = self.crash_counts.entry(plugin_id.into()).or_default();
        *count += 1;

        tracing::error!(
            "Plugin '{}' crashed (count: {}/{}): {}",
            plugin_id,
            count,
            self.max_restarts,
            reason
        );

        // 写入 crash report
        let report = CrashReport {
            plugin_id: plugin_id.into(),
            timestamp: chrono_like_now(),
            runtime: runtime_registry
                .get(plugin_id)
                .map(|e| e.runtime_type.clone())
                .unwrap_or_else(|| "unknown".into()),
            reason: reason.into(),
            stack_trace: stack_trace.into(),
            isolate_state: runtime_registry
                .get(plugin_id)
                .map(|e| e.status.clone())
                .unwrap_or_else(|| "unknown".into()),
            crash_count: *count,
        };
        if let Err(e) = report.save() {
            tracing::error!("Failed to save crash report: {}", e);
        }

        runtime_registry.record_crash(plugin_id);

        if *count > self.max_restarts {
            runtime_registry.update_status(plugin_id, "crashed (max restarts exceeded)")?;
            anyhow::bail!(
                "Plugin '{}' exceeded max restarts ({})",
                plugin_id,
                self.max_restarts
            );
        }

        runtime_registry.update_status(plugin_id, "recovering")?;
        self.restart_isolate(plugin_id, isolate_pool, code, runtime_registry)?;

        Ok(())
    }

    /// 重建 Isolate，重新加载插件代码
    pub fn restart_isolate(
        &mut self,
        plugin_id: &str,
        isolate_pool: &mut IsolatePool,
        code: &str,
        runtime_registry: &mut RuntimeRegistry,
    ) -> Result<()> {
        isolate_pool.restart(plugin_id, code)?;
        runtime_registry.update_status(plugin_id, "running")?;
        tracing::info!("Plugin '{}' recovered successfully", plugin_id);
        Ok(())
    }

    pub fn crash_count(&self, plugin_id: &str) -> u32 {
        self.crash_counts.get(plugin_id).copied().unwrap_or(0)
    }

    pub fn exceeded(&self, plugin_id: &str) -> bool {
        self.crash_count(plugin_id) > self.max_restarts
    }
}

fn chrono_like_now() -> String {
    // 避免为新依赖引入 chrono crate
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_else(|_| "unknown".into())
}
