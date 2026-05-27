pub mod state_tree;

use std::collections::HashMap;

use anyhow::Result;
use tokio::sync::mpsc;

use crate::ipc::messages::IpcRequest;

/// RenderScheduler：异步 channel 接收 SDK 命令，帧级 dedup，按帧 flush 为 IPC batch
pub struct RenderScheduler {
    rx: mpsc::UnboundedReceiver<IpcRequest>,
}

impl RenderScheduler {
    /// 创建 Scheduler 和对应的 sender（外部通过 sender 提交命令）
    pub fn new() -> (mpsc::UnboundedSender<IpcRequest>, Self) {
        let (tx, rx) = mpsc::unbounded_channel();
        (tx, Self { rx })
    }

    /// 帧级 flush：收集 channel 中所有待处理命令，按规则去重后返回
    ///
    /// Dedup 规则：
    /// - duration=0 的相同 (method, widget_id) 只保留最后一条（瞬时操作，最终状态为准）
    /// - duration>0 的操作不过滤（动画序列必须完整保留）
    /// - create* 方法不过滤（每次创建都是独立的）
    /// - 跨 Widget 命令互不影响
    pub fn flush(&mut self) -> Result<Vec<IpcRequest>> {
        let mut pending: Vec<IpcRequest> = vec![];

        // 收集 channel 中所有待处理命令
        while let Ok(cmd) = self.rx.try_recv() {
            pending.push(cmd);
        }

        if pending.is_empty() {
            return Ok(vec![]);
        }

        // 帧级去重
        let mut deduped: Vec<IpcRequest> = vec![];
        let mut last_index: HashMap<(String, String), usize> = HashMap::new();

        for cmd in pending {
            let widget_id = extract_widget_id(&cmd.params);
            let duration = extract_duration(&cmd.params);
            let is_create = cmd.method.starts_with("create");

            // 瞬时操作（duration=0 且非 create）允许覆盖
            if duration == 0 && !is_create && !widget_id.is_empty() {
                let key = (cmd.method.clone(), widget_id);
                if let Some(&idx) = last_index.get(&key) {
                    deduped[idx] = cmd; // 替换前一条
                    continue;
                }
                last_index.insert(key, deduped.len());
            }

            deduped.push(cmd);
        }

        tracing::debug!(
            "Scheduler flush: {} commands -> {} after dedup",
            deduped.len() + last_index.len().saturating_sub(deduped.len().saturating_sub(last_index.len())),
            deduped.len()
        );

        Ok(deduped)
    }
}

fn extract_widget_id(params: &serde_json::Value) -> String {
    match params.get("id") {
        Some(v) => match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        },
        None => String::new(),
    }
}

fn extract_duration(params: &serde_json::Value) -> u64 {
    params
        .get("duration")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cmd(method: &str, id: &str, duration: u64) -> IpcRequest {
        IpcRequest {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params: json!({"id": id, "duration": duration}),
            id: 1,
        }
    }

    fn cmd_create(method: &str, id: &str) -> IpcRequest {
        IpcRequest {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params: json!({"id": id, "x": 0, "y": 0, "w": 100, "h": 100}),
            id: 1,
        }
    }

    #[test]
    fn empty_flush_returns_empty() {
        let (_, mut scheduler) = RenderScheduler::new();
        let batch = scheduler.flush().unwrap();
        assert!(batch.is_empty());
    }

    #[test]
    fn preserves_animation_commands() {
        let (tx, mut scheduler) = RenderScheduler::new();
        tx.send(cmd("setScale", "w1", 200)).unwrap();
        tx.send(cmd("setScale", "w1", 150)).unwrap();
        tx.send(cmd("setOpacity", "w1", 100)).unwrap();

        let batch = scheduler.flush().unwrap();
        // duration>0 全部保留
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn dedups_instant_same_method_same_widget() {
        let (tx, mut scheduler) = RenderScheduler::new();
        tx.send(cmd("setOpacity", "w1", 0)).unwrap(); // 被覆盖
        tx.send(cmd("setOpacity", "w1", 0)).unwrap(); // 被覆盖
        tx.send(cmd("setOpacity", "w1", 0)).unwrap(); // 最终保留

        let batch = scheduler.flush().unwrap();
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].method, "setOpacity");
    }

    #[test]
    fn no_dedup_across_widgets() {
        let (tx, mut scheduler) = RenderScheduler::new();
        tx.send(cmd("setOpacity", "w1", 0)).unwrap();
        tx.send(cmd("setOpacity", "w2", 0)).unwrap();
        tx.send(cmd("setOpacity", "w3", 0)).unwrap();

        let batch = scheduler.flush().unwrap();
        // 不同 widget id，每个都保留
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn no_dedup_across_methods() {
        let (tx, mut scheduler) = RenderScheduler::new();
        tx.send(cmd("setScale", "w1", 0)).unwrap();
        tx.send(cmd("setOpacity", "w1", 0)).unwrap();

        let batch = scheduler.flush().unwrap();
        // 不同 method，都保留
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn never_dedups_create_commands() {
        let (tx, mut scheduler) = RenderScheduler::new();
        tx.send(cmd_create("createRect", "w1")).unwrap();
        tx.send(cmd_create("createRect", "w1")).unwrap();
        tx.send(cmd_create("createIcon", "w1")).unwrap();

        let batch = scheduler.flush().unwrap();
        // create* 方法永远不去重
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn mixed_animation_and_instant() {
        let (tx, mut scheduler) = RenderScheduler::new();
        tx.send(cmd("setScale", "w1", 200)).unwrap(); // 动画，保留
        tx.send(cmd("setScale", "w1", 0)).unwrap();   // 瞬时，覆盖前一条同 method+id 的瞬时
        tx.send(cmd("setOpacity", "w1", 0)).unwrap();  // 保留
        tx.send(cmd("setOpacity", "w1", 0)).unwrap();  // 覆盖上一条

        let batch = scheduler.flush().unwrap();
        // 应保留: setScale(dur=200), setScale(dur=0), setOpacity(dur=0, 最后一条)
        assert_eq!(batch.len(), 3);
        assert_eq!(batch[0].method, "setScale");
        assert_eq!(batch[1].method, "setScale");
        assert_eq!(batch[2].method, "setOpacity");
    }
}
