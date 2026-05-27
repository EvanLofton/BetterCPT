#![allow(dead_code)] // Phase 5 utility window infrastructure

use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::runtime::storage::PluginStorage;

/// 窗口位置配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 窗口条目
struct WindowEntry {
    id: u32,
    title: String,
    width: u32,
    height: u32,
    min_width: u32,
    min_height: u32,
    position: Option<WindowPosition>,
}

/// Window Service：Utility Window 的创建/关闭/缩放/位置记忆
pub struct WindowService {
    next_id: u32,
    windows: HashMap<u32, WindowEntry>,
}

impl WindowService {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            windows: HashMap::new(),
        }
    }

    /// 创建 Utility Window（v1: WebView Backend，Phase 4 集成）
    pub fn create(&mut self, title: &str, width: u32, height: u32) -> Result<u32> {
        let id = self.next_id;
        self.next_id += 1;

        let entry = WindowEntry {
            id,
            title: title.into(),
            width,
            height,
            min_width: 300,
            min_height: 200,
            position: None,
        };

        self.windows.insert(id, entry);
        tracing::info!("Window created: id={}, title='{}', {}x{}", id, title, width, height);
        Ok(id)
    }

    /// 关闭窗口
    pub fn close(&mut self, window_id: u32) -> Result<()> {
        if self.windows.remove(&window_id).is_none() {
            anyhow::bail!("Window {} not found", window_id);
        }
        tracing::info!("Window closed: id={}", window_id);
        Ok(())
    }

    /// Resize 窗口
    pub fn resize(&mut self, window_id: u32, width: u32, height: u32) -> Result<()> {
        let entry = self
            .windows
            .get_mut(&window_id)
            .ok_or_else(|| anyhow::anyhow!("Window {} not found", window_id))?;

        let w = width.max(entry.min_width);
        let h = height.max(entry.min_height);
        entry.width = w;
        entry.height = h;

        tracing::debug!("Window {} resized to {}x{}", window_id, w, h);
        Ok(())
    }

    /// 设置最小尺寸
    pub fn set_min_size(&mut self, window_id: u32, width: u32, height: u32) -> Result<()> {
        let entry = self
            .windows
            .get_mut(&window_id)
            .ok_or_else(|| anyhow::anyhow!("Window {} not found", window_id))?;
        entry.min_width = width;
        entry.min_height = height;

        // 如果当前尺寸小于新的最小尺寸，则扩展
        if entry.width < entry.min_width {
            entry.width = entry.min_width;
        }
        if entry.height < entry.min_height {
            entry.height = entry.min_height;
        }

        tracing::debug!("Window {} min size set to {}x{}", window_id, width, height);
        Ok(())
    }

    /// 持久化当前窗口位置到 PluginStorage
    pub fn save_position(&self, window_id: u32, storage: &mut PluginStorage) -> Result<()> {
        let entry = self
            .windows
            .get(&window_id)
            .ok_or_else(|| anyhow::anyhow!("Window {} not found", window_id))?;

        let pos = entry.position.clone().unwrap_or_else(|| WindowPosition {
            x: 100,
            y: 100,
            width: entry.width,
            height: entry.height,
        });

        let key = format!("window_pos_{}", window_id);
        storage.set(&key, serde_json::to_value(&pos)?)?;
        tracing::debug!("Window {} position saved", window_id);
        Ok(())
    }

    /// 从 PluginStorage 恢复上次窗口位置
    pub fn restore_position(&mut self, window_id: u32, storage: &PluginStorage) -> Result<()> {
        let entry = self
            .windows
            .get_mut(&window_id)
            .ok_or_else(|| anyhow::anyhow!("Window {} not found", window_id))?;

        let key = format!("window_pos_{}", window_id);
        if let Some(value) = storage.get(&key) {
            if let Ok(pos) = serde_json::from_value::<WindowPosition>(value.clone()) {
                tracing::debug!("Window {} position restored: {}x{} at ({},{})",
                    window_id, pos.width, pos.height, pos.x, pos.y);
                entry.position = Some(pos);
            }
        }

        Ok(())
    }

    pub fn list(&self) -> Vec<u32> {
        self.windows.keys().copied().collect()
    }

    pub fn get(&self, window_id: u32) -> Option<(String, u32, u32)> {
        self.windows
            .get(&window_id)
            .map(|e| (e.title.clone(), e.width, e.height))
    }
}
