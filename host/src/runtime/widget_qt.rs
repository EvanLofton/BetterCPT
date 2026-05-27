use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use anyhow::{Context, Result};

/// Qt 发来的消息类型
pub enum QtMessage {
    /// 响应：带 "id"，是之前 send_request 的回复
    Response { #[allow(dead_code)] id: u64, json: String },
    /// 事件：无 "id"，Qt 主动推送（如鼠标 hover/click）
    Event { method: String, widget_id: String },
    /// 事件（保留完整 JSON，用于需要 params 数据的自定义事件如 onDrop）
    EventJson { method: String, json: String },
    /// 无法识别的消息
    Unknown(String),
}

/// Widget Qt Runtime — 管理 Qt 子进程的生命周期、IPC pipe 和崩溃恢复
///
/// L1 resolve: BufReader<ChildStdout> + read_line() 本身无 bug。
/// 早期返回 0 字节的根因：send_request 缺少 JSON-RPC "id" 字段
/// → Qt id.isUndefined()=true → 执行但不回复 → read_line 等到 EOF。
/// 修复：send_request 补自增 id，BufReader 正常使用。
/// send_notification 作为注释保留以备未来 Scheduler batch 等场景。
pub struct WidgetQtRuntime {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    frozen: bool,
    crash_count: u32,
    max_restarts: u32,
    /// JSON-RPC message id counter
    next_id: u64,
}

impl WidgetQtRuntime {
    pub fn new() -> Self {
        Self {
            child: None,
            stdin: None,
            stdout: None,
            frozen: false,
            crash_count: 0,
            max_restarts: 5,
            next_id: 1,
        }
    }

    /// 启动 Qt 子进程，建立 stdin/stdout pipe
    pub fn start_qt_process(&mut self) -> Result<()> {
        let qt_exe = resolve_qt_exe_path()?;

        let mut child = Command::new(&qt_exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("failed to start Qt process: {}", qt_exe.display()))?;

        let stdin = child
            .stdin
            .take()
            .context("failed to take Qt stdin")?;
        let stdout = child
            .stdout
            .take()
            .context("failed to take Qt stdout")?;

        self.stdin = Some(stdin);
        self.stdout = Some(BufReader::new(stdout));
        self.child = Some(child);

        tracing::info!(
            "Qt process started: {} (PID: {})",
            qt_exe.display(),
            self.child.as_ref().map(|c| c.id()).unwrap_or(0)
        );

        // 等待 Qt ready 信号
        self.wait_for_ready()?;

        Ok(())
    }

    /// 等待 Qt 子进程发送 ready 通知
    fn wait_for_ready(&mut self) -> Result<()> {
        let reader = self.stdout.as_mut().context("stdout not connected")?;
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader
                .read_line(&mut line)
                .context("failed to read Qt ready signal")?;
            if n == 0 {
                anyhow::bail!("Qt stdout closed before ready signal");
            }
            let trimmed = line.trim();
            tracing::info!("Qt startup line: '{}' (len={})", trimmed, trimmed.len());
            if trimmed.contains("\"ready\"") {
                tracing::info!("Qt ready signal received");
                return Ok(());
            }
        }
    }

    /// JSON-RPC Request：带自增 id，Qt **必须回复**（调用后需 read_response）
    pub fn send_request(&mut self, method: &str, params: &str) -> Result<()> {
        if self.frozen {
            tracing::debug!("IPC frozen, dropping request: {}", method);
            return Ok(());
        }

        let id = self.next_id;
        self.next_id += 1;

        let writer = self.stdin.as_mut().context("stdin not connected")?;
        let json = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"method\":\"{}\",\"params\":{}}}",
            id, method, params
        );
        writer.write_all(json.as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush().context("failed to flush Qt stdin")?;

        tracing::debug!("IPC request [id={}]: {}", id, method);
        Ok(())
    }

    // ── 备选方案：send_notification ──
    // 当前 v1 所有 Host→Qt 消息均需响应，notification 暂无用途。
    // 未来若有"发了不管"的场景（如 Scheduler batch flush），恢复此方法即可。
    //
    // pub fn send_notification(&mut self, method: &str, params: &str) -> Result<()> {
    //     if self.frozen { return Ok(()); }
    //     let writer = self.stdin.as_mut().context("stdin not connected")?;
    //     let json = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"{}\",\"params\":{}}}", method, params);
    //     writer.write_all(json.as_bytes())?;
    //     writer.write_all(b"\n")?;
    //     writer.flush().context("failed to flush Qt stdin")?;
    //     Ok(())
    // }

    /// 从 Qt stdout 读取一条响应
    #[allow(dead_code)]
    pub fn read_response(&mut self) -> Result<String> {
        let reader = self.stdout.as_mut().context("stdout not connected")?;
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .context("failed to read Qt response")?;
        if n == 0 {
            anyhow::bail!("Qt stdout closed unexpectedly");
        }
        let trimmed = line.trim().to_string();
        tracing::debug!("Qt response: '{}' ({} bytes)", trimmed, n);
        Ok(trimmed)
    }

    /// 从 stdout 读取一条消息，区分 event / response / unknown
    pub fn read_any(&mut self) -> Result<QtMessage> {
        let line = {
            let reader = self.stdout.as_mut().context("stdout not connected")?;
            let mut buf = String::new();
            let n = reader.read_line(&mut buf).context("failed to read Qt message")?;
            if n == 0 {
                anyhow::bail!("Qt stdout closed unexpectedly");
            }
            buf.trim().to_string()
        };

        if line.is_empty() {
            return Ok(QtMessage::Unknown(line));
        }
        let v: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => return Ok(QtMessage::Unknown(line)),
        };
        // 有 "id" → Response
        if let Some(id) = v.get("id").and_then(|i| i.as_u64()) {
            return Ok(QtMessage::Response { id, json: line });
        }
        // 有 "method" 且无 "id" → Event
        if let Some(method) = v.get("method").and_then(|m| m.as_str()) {
            let widget_id = v
                .get("params")
                .and_then(|p| p.get("widgetId"))
                .and_then(|w| w.as_str())
                .unwrap_or("")
                .to_string();
            // onDrop / 需要完整 params 的事件 → EventJson
            if method == "onDrop" {
                return Ok(QtMessage::EventJson {
                    method: method.to_string(),
                    json: line,
                });
            }
            return Ok(QtMessage::Event {
                method: method.to_string(),
                widget_id,
            });
        }
        Ok(QtMessage::Unknown(line))
    }

    /// 检测 Qt 进程是否已退出
    pub fn detect_crash(&mut self) -> Result<bool> {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    tracing::warn!("Qt process exited with status: {:?}", status);
                    Ok(true)
                }
                Ok(None) => Ok(false),
                Err(e) => {
                    tracing::error!("Failed to check Qt process status: {}", e);
                    Ok(true)
                }
            }
        } else {
            Ok(false)
        }
    }

    /// 重启 Qt 进程
    pub fn restart_qt(&mut self) -> Result<()> {
        self.crash_count += 1;
        if self.crash_count > self.max_restarts {
            anyhow::bail!(
                "Qt process exceeded max restarts ({}/{})",
                self.crash_count,
                self.max_restarts
            );
        }

        if let Some(mut child) = self.child.take() {
            child.kill().ok();
            child.wait().ok();
        }

        self.stdin = None;
        self.stdout = None;

        self.freeze_ipc();
        self.start_qt_process()?;
        self.unfreeze_ipc();

        tracing::info!(
            "Qt process restarted (crash count: {}/{})",
            self.crash_count,
            self.max_restarts
        );
        Ok(())
    }

    /// 冻结 IPC（暂停向 Qt 发送命令）
    pub fn freeze_ipc(&mut self) {
        self.frozen = true;
        tracing::info!("IPC frozen");
    }

    /// 解冻 IPC
    pub fn unfreeze_ipc(&mut self) {
        self.frozen = false;
        tracing::info!("IPC unfrozen");
    }

    #[allow(dead_code)]
    pub fn crash_count(&self) -> u32 {
        self.crash_count
    }

    /// 全量 Replay 状态树到 Qt（崩溃恢复核心）
    #[allow(dead_code)]
    pub fn replay_state_tree(&mut self, tree: &crate::scheduler::state_tree::StateTree) -> Result<()> {
        let nodes = tree.all_nodes();
        tracing::info!("Replaying state tree: {} nodes", nodes.len());

        for node in nodes {
            let params = serde_json::json!({
                "id": node.id,
                "x": node.bounds.x,
                "y": node.bounds.y,
                "w": node.bounds.w,
                "h": node.bounds.h,
                "radius": node.radius,
                "blur": node.blur,
                "scale": node.scale,
                "opacity": node.opacity,
            });

            let method = match node.widget_type {
                crate::scheduler::state_tree::WidgetType::Rect => "createRect",
                crate::scheduler::state_tree::WidgetType::Icon => "createIcon",
                crate::scheduler::state_tree::WidgetType::Text => "createText",
            };

            self.send_request(method, &params.to_string())?;
        }

        for node in tree.all_nodes() {
            for child_id in &node.children {
                let params = serde_json::json!({
                    "parentId": node.id,
                    "childId": child_id,
                });
                self.send_request("addChild", &params.to_string())?;
            }
        }

        tracing::info!("State tree replay complete");
        Ok(())
    }

    /// 分页发送 Replay（>1MB 时，以 Widget 为单位切割）
    #[allow(dead_code)]
    pub fn send_replay_pages(&mut self, tree: &crate::scheduler::state_tree::StateTree) -> Result<()> {
        const PAGE_LIMIT: usize = 1024 * 1024; // 1MB

        let nodes = tree.all_nodes();
        let mut current_page: Vec<&crate::scheduler::state_tree::WidgetNode> = vec![];
        let mut current_size: usize = 0;
        let mut pages: Vec<Vec<&crate::scheduler::state_tree::WidgetNode>> = vec![];

        for node in nodes {
            let node_json = serde_json::to_string(&node).unwrap_or_default();
            let node_size = node_json.len();

            if !current_page.is_empty() && current_size + node_size > PAGE_LIMIT {
                pages.push(std::mem::take(&mut current_page));
                current_size = 0;
            }

            current_size += node_size;
            current_page.push(node);
        }
        if !current_page.is_empty() {
            pages.push(current_page);
        }

        let total_pages = pages.len();
        tracing::info!("Replay paginated: {} pages for {} nodes", total_pages, tree.all_nodes().len());

        for (i, page) in pages.iter().enumerate() {
            let is_last = i + 1 == total_pages;

            for node in page {
                let method = match node.widget_type {
                    crate::scheduler::state_tree::WidgetType::Rect => "createRect",
                    crate::scheduler::state_tree::WidgetType::Icon => "createIcon",
                    crate::scheduler::state_tree::WidgetType::Text => "createText",
                };

                let params = serde_json::json!({
                    "id": node.id,
                    "x": node.bounds.x,
                    "y": node.bounds.y,
                    "w": node.bounds.w,
                    "h": node.bounds.h,
                    "page": i + 1,
                    "total_pages": total_pages,
                    "is_last": is_last,
                });
                self.send_request(method, &params.to_string())?;
            }
        }

        tracing::info!("Replay pages sent: total={}", total_pages);
        Ok(())
    }
}

impl Drop for WidgetQtRuntime {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            child.kill().ok();
            child.wait().ok();
        }
        tracing::info!("Qt process terminated");
    }
}

/// 解析 Qt 可执行文件路径
fn resolve_qt_exe_path() -> Result<std::path::PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // 1) 生产安装：widget-qt.exe 与 Host exe 同目录
            let prod = exe_dir.join("widget-qt.exe");
            if prod.exists() {
                return Ok(prod);
            }
            // 2) 开发构建：exe 在 host/target/debug/ → 往上 3 层到项目根
            let dev = exe_dir
                .join("..").join("..").join("..")
                .join("widget-qt").join("build").join("widget-qt.exe");
            if dev.exists() {
                return Ok(dev);
            }
        }
    }
    // 3) CWD fallback（cargo run 时 CWD 在 host/）
    let cwd_path = std::path::PathBuf::from("../widget-qt/build/widget-qt.exe");
    if cwd_path.exists() {
        return Ok(cwd_path);
    }
    anyhow::bail!("widget-qt.exe not found. Build it first: cd widget-qt && cmake -B build && cmake --build build")
}
