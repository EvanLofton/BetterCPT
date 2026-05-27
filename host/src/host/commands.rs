use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use serde::Serialize;
use crate::runtime::plugin_loader::PluginLoader;
use crate::runtime::registry::{RuntimeRegistry, RuntimeEntry};
use crate::host::settings::{self, AppSettings};
use crate::ipc::messages::IpcRequest;

/// Widget 命令发送端（Tauri managed state，供 send_widget_command 使用）
pub struct WidgetCommandSender(pub mpsc::UnboundedSender<IpcRequest>);

/// Dashboard 就绪信号（Dashboard 加载完毕 → Host 开始启动插件）
pub struct DashboardReadyNotifier(pub Arc<tokio::sync::Notify>);

/// Dashboard 加载完成后调用，通知 Host 可以启动插件
#[tauri::command]
pub async fn dashboard_ready(notify: State<'_, DashboardReadyNotifier>) -> Result<(), String> {
    notify.0.notify_one();
    Ok(())
}

/// Dashboard 页面渲染后调用，显示窗口（避免 WebView2 白屏闪烁）
#[tauri::command]
pub async fn show_dashboard(app: AppHandle) -> Result<(), String> {
    // 先关闭原生 splash，再显示 WebView
    crate::host::splash::close();
    if let Some(w) = app.get_webview_window("dashboard") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

/// Host loop 接收的插件操作请求
#[derive(Debug, Clone)]
pub enum PluginAction {
    Start { plugin_id: String },
    Stop { plugin_id: String },
    Uninstall { plugin_id: String },
    ConfigUpdate { plugin_id: String, config_json: String },
}

/// 操作请求 sender（Tauri command → host loop）
pub type ActionSender = mpsc::UnboundedSender<PluginAction>;

/// 返回所有插件状态的 JSON 快照（Dashboard 启动时调用）
#[tauri::command]
pub async fn get_runtime_snapshot(
    registry: State<'_, Arc<Mutex<RuntimeRegistry>>>,
) -> Result<String, String> {
    let reg = registry.lock().await;
    Ok(reg.get_runtime_snapshot().to_string())
}

/// 推送事件到 Dashboard 前端（从 Host 主循环调用）
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEvent {
    pub event: String,
    pub plugin_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crash_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
}

pub fn emit_runtime_event(app: &AppHandle, event: RuntimeEvent) {
    let _ = app.emit("runtime_event", event);
}

/// 窗口拖动——Rust 直接调原生 API，绕过 WebView2 转发限制
#[tauri::command]
pub fn start_drag(window: WebviewWindow) {
    let _ = window.start_dragging();
}

/// 安装 .bcpkg 插件：复制到用户插件目录 → 返回新 manifest
#[tauri::command]
pub async fn install_plugin(
    path: String,
    loader: State<'_, Arc<Mutex<PluginLoader>>>,
    registry: State<'_, Arc<Mutex<RuntimeRegistry>>>,
) -> Result<String, String> {
    let mut ld = loader.lock().await;
    match ld.install(&path) {
        Ok(manifest) => {
            let mut reg = registry.lock().await;
            reg.register(RuntimeEntry {
                id: manifest.id.clone(),
                runtime_type: manifest.runtime.clone(),
                status: "installed".into(),
            }).map_err(|e| e.to_string())?;
            Ok(manifest.id)
        }
        Err(e) => Err(format!("Install failed: {}", e)),
    }
}

#[tauri::command]
pub async fn start_plugin(
    plugin_id: String,
    action_tx: State<'_, ActionSender>,
) -> Result<(), String> {
    action_tx.send(PluginAction::Start { plugin_id }).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_plugin(
    plugin_id: String,
    action_tx: State<'_, ActionSender>,
) -> Result<(), String> {
    action_tx.send(PluginAction::Stop { plugin_id }).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn uninstall_plugin(
    plugin_id: String,
    action_tx: State<'_, ActionSender>,
) -> Result<(), String> {
    action_tx.send(PluginAction::Uninstall { plugin_id }).map_err(|e| e.to_string())
}

/// 返回崩溃日志文件列表
#[tauri::command]
pub async fn list_crash_logs() -> Result<String, String> {
    let crash_dir = crate::runtime::crash_report::get_crash_dir().map_err(|e| e.to_string())?;
    let mut files = vec![];
    if let Ok(entries) = std::fs::read_dir(&crash_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().map_or(false, |e| e == "json") {
                files.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    serde_json::to_string(&files).map_err(|e| e.to_string())
}

/// 读取指定崩溃日志内容
#[tauri::command]
pub async fn read_crash_log(filename: String) -> Result<String, String> {
    let crash_dir = crate::runtime::crash_report::get_crash_dir().map_err(|e| e.to_string())?;
    let path = crash_dir.join(&filename);
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", filename, e))
}

/// 获取应用设置
#[tauri::command]
pub async fn get_settings() -> Result<String, String> {
    let s = settings::load_settings().map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&s).map_err(|e| e.to_string())
}

/// 更新并持久化应用设置
#[tauri::command]
pub async fn update_settings(settings_json: String) -> Result<(), String> {
    let s: AppSettings = serde_json::from_str(&settings_json).map_err(|e| e.to_string())?;
    settings::save_settings(&s).map_err(|e| e.to_string())
}

/// 清除所有崩溃日志
#[tauri::command]
pub async fn clear_crash_logs() -> Result<(), String> {
    let crash_dir = crate::runtime::crash_report::get_crash_dir().map_err(|e| e.to_string())?;
    if crash_dir.exists() {
        for entry in (std::fs::read_dir(&crash_dir)).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.path().extension().map_or(false, |e| e == "json") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

/// 更新插件配置并注入到 V8
#[tauri::command]
pub async fn update_plugin_config(
    plugin_id: String,
    config_json: String,
    action_tx: State<'_, ActionSender>,
) -> Result<(), String> {
    action_tx
        .send(PluginAction::ConfigUpdate { plugin_id, config_json })
        .map_err(|e| e.to_string())
}

/// 发送 Widget 命令到 Qt（Dashboard 配置面板调用）
#[tauri::command]
pub async fn send_widget_command(
    method: String,
    params_json: String,
    sender: State<'_, WidgetCommandSender>,
) -> Result<(), String> {
    let params: serde_json::Value =
        serde_json::from_str(&params_json).map_err(|e| e.to_string())?;
    let req = IpcRequest {
        jsonrpc: "2.0".into(),
        method,
        params,
        id: 0,
    };
    sender.0.send(req).map_err(|e| e.to_string())
}
