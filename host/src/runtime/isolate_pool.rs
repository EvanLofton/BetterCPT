#![allow(dead_code)] // Phase 5 isolate management (restart/list)

use std::collections::HashMap;

use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::ipc::messages::IpcRequest;
use crate::runtime::deno_host::JsRuntimeHost;

/// per-plugin V8 Isolate 池
/// LIFO 销毁顺序：HashMap 做 O(1) 查找，Vec 记录创建顺序
pub struct IsolatePool {
    isolates: HashMap<String, JsRuntimeHost>,
    creation_order: Vec<String>,
    /// Scheduler sender，传递给每个新建的 JsRuntime
    scheduler_tx: mpsc::UnboundedSender<IpcRequest>,
}

impl IsolatePool {
    pub fn new(scheduler_tx: mpsc::UnboundedSender<IpcRequest>) -> Self {
        Self {
            isolates: HashMap::new(),
            creation_order: Vec::new(),
            scheduler_tx,
        }
    }

    /// 为插件创建新 Isolate，注入 SDK preamble，执行入口代码
    pub fn spawn(&mut self, plugin_id: &str, code: &str) -> Result<()> {
        if self.isolates.contains_key(plugin_id) {
            anyhow::bail!("Isolate for '{}' already exists", plugin_id);
        }

        let mut host = JsRuntimeHost::new(self.scheduler_tx.clone())
            .with_context(|| format!("failed to create JsRuntime for '{}'", plugin_id))?;

        // 注入插件 ID，供 console.log 标记来源
        let set_id = format!("globalThis.__pluginId = '{}';", plugin_id.replace('\'', "\\'"));
        host.execute_script_raw(&set_id).ok();

        host.execute(code)
            .with_context(|| format!("failed to execute plugin '{}'", plugin_id))?;

        self.isolates.insert(plugin_id.into(), host);
        self.creation_order.push(plugin_id.into());
        tracing::info!("Isolate spawned for plugin '{}'", plugin_id);
        Ok(())
    }

    /// 终止指定插件的 Isolate
    pub fn kill(&mut self, plugin_id: &str) -> Result<()> {
        if self.isolates.remove(plugin_id).is_none() {
            anyhow::bail!("Isolate for '{}' not found", plugin_id);
        }
        self.creation_order.retain(|id| id != plugin_id);
        tracing::info!("Isolate killed for plugin '{}'", plugin_id);
        Ok(())
    }

    /// 重启指定插件的 Isolate（先 kill 旧，再 spawn 新，保证 LIFO）
    pub fn restart(&mut self, plugin_id: &str, code: &str) -> Result<()> {
        self.kill(plugin_id)?;
        self.spawn(plugin_id, code)?;
        tracing::info!("Isolate restarted for plugin '{}'", plugin_id);
        Ok(())
    }

    /// Widget 热更新：重建 Isolate + 重新执行入口代码
    pub fn hot_reload(&mut self, plugin_id: &str, code: &str) -> Result<()> {
        tracing::info!("Hot reloading plugin '{}'", plugin_id);
        if self.has(plugin_id) {
            self.isolates.remove(plugin_id);
            self.creation_order.retain(|id| id != plugin_id);
        }
        self.spawn(plugin_id, code)?;
        tracing::info!("Hot reload complete for '{}'", plugin_id);
        Ok(())
    }

    /// 列出所有活动 Isolate 的 plugin_id
    pub fn list(&self) -> Vec<String> {
        self.isolates.keys().cloned().collect()
    }

    /// 检查指定 plugin_id 是否有活动 Isolate
    pub fn has(&self, plugin_id: &str) -> bool {
        self.isolates.contains_key(plugin_id)
    }

    /// 在所有 isolate 中执行自定义 JS 代码
    pub fn fire_custom_event(&mut self, code: &str) {
        for (plugin_id, host) in self.isolates.iter_mut() {
            if let Err(e) = host.execute_script_raw(code) {
                tracing::warn!("fire_custom_event in '{}' failed: {:?}", plugin_id, e);
            }
        }
    }

    /// 事件路由：遍历所有 isolate，找到拥有该 widget 的 isolate 并触发回调
    /// 返回 true 表示事件被成功路由
    pub fn route_event(&mut self, widget_id: &str, event_name: &str) -> bool {
        for (plugin_id, host) in self.isolates.iter_mut() {
            if host.try_fire_event(widget_id, event_name) {
                tracing::debug!(
                    "Event {} routed: widget={} → plugin={}",
                    event_name, widget_id, plugin_id
                );
                return true;
            }
        }
        tracing::warn!("Event {} dropped: widget={} not found in any isolate", event_name, widget_id);
        false
    }
}

impl Drop for IsolatePool {
    fn drop(&mut self) {
        // 严格逆序销毁，满足 V8 LIFO 要求
        while let Some(id) = self.creation_order.pop() {
            if let Some(_isolate) = self.isolates.remove(&id) {
                tracing::debug!("Isolate '{}' dropped (LIFO order)", id);
            }
        }
    }
}
