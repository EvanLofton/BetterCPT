// Phase 4: Tauri 嵌入 Host，Dashboard 为进程内 WebView 窗口
#![windows_subsystem = "windows"]

mod host;
mod ipc;
mod runtime;
mod scheduler;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::sync::Mutex;

use crate::host::context::AppContext;
use crate::host::logger::init_logger;
use crate::host::shutdown::ShutdownListener;
use crate::runtime::capability::CapabilityRegistry;
use crate::runtime::isolate_pool::IsolatePool;
use crate::runtime::plugin_loader::PluginLoader;
use crate::runtime::registry::RuntimeRegistry;
use crate::host::commands::{emit_runtime_event, PluginAction, RuntimeEvent};
use crate::runtime::widget_qt::{QtMessage, WidgetQtRuntime};

/// 非阻塞轮询 Qt 主动推送的事件（onDrop、onHover 等）并路由到 Isolate
fn flush_and_send(
    batch: &[crate::ipc::messages::IpcRequest],
    qt: &mut WidgetQtRuntime,
    pool: &mut IsolatePool,
) -> Result<()> {
    for cmd in batch {
        // system.launch / storage.* / dialog.* 是 Host 级命令，不发给 Qt
        if cmd.method == "system.launch" {
            let path = cmd.params.get("path").and_then(|v| v.as_str()).unwrap_or("");
            use windows::Win32::UI::Shell::ShellExecuteW;
            use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
            let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                ShellExecuteW(None, None, windows::core::PCWSTR::from_raw(wide.as_ptr()),
                    None, None, SW_SHOW);
            }
            continue;
        }
        if cmd.method == "plugin.log" {
            let pid = cmd.params.get("pluginId").and_then(|v| v.as_str()).unwrap_or("");
            let msg = cmd.params.get("message").and_then(|v| v.as_str()).unwrap_or("");
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    let log_dir = dir.join("plugins").join(pid);
                    let _ = std::fs::create_dir_all(&log_dir);
                    let _ = std::fs::OpenOptions::new()
                        .create(true).append(true)
                        .open(log_dir.join("plugin.log"))
                        .map(|mut f| {
                            use std::io::Write;
                            let _ = writeln!(f, "{}", msg);
                        });
                }
            }
            continue;
        }
        if cmd.method.starts_with("storage.") || cmd.method.starts_with("dialog.") || cmd.method.starts_with("notification.") {
            // v1: 这些 Host 命令尚未实现，跳过
            continue;
        }

        let params_str = cmd.params.to_string();
        qt.send_request(&cmd.method, &params_str)?;
        loop {
            match qt.read_any()? {
                QtMessage::Response { id: _, json } => {
                    // 高频操作（setScale/setOpacity/addChild/ping 等）不记 INFO
                    if cmd.method != "ping" && cmd.method != "setScale" && cmd.method != "setOpacity"
                        && cmd.method != "addChild" && cmd.method != "raise" && cmd.method != "lower"
                    {
                        tracing::info!("Qt: {}", json);
                    }
                    break;
                }
                QtMessage::Event { method, widget_id } => {
                    pool.route_event(&widget_id, &method);
                }
                QtMessage::EventJson { method, json } => {
                    // onDrop 等自定义事件：在 Isolate 中触发回调
                    if method == "onDrop" {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
                            let wid = v["params"]["widgetId"].as_str().unwrap_or("");
                            let paths = v["params"]["paths"].to_string();
                            let code = format!(
                                "(function(){{ var w=globalThis.__widgets['{id}']; if(w&&typeof w._onDrop==='function') w._onDrop({paths}); }})()",
                                id = wid, paths = paths
                            );
                            pool.fire_custom_event(&code);
                        }
                    }
                }
                QtMessage::Unknown(line) => {
                    tracing::warn!("Qt unknown: {}", line);
                }
            }
        }
    }
    Ok(())
}

fn check_hot_reload(
    loader: &mut PluginLoader,
    pool: &mut IsolatePool,
    registry: &mut RuntimeRegistry,
) -> Result<()> {
    for (plugin_id, new_code) in loader.check_hot_reload() {
        tracing::info!("Hot reload triggered for '{}'", plugin_id);
        pool.hot_reload(&plugin_id, &new_code)?;
        registry.update_status(&plugin_id, "running")?;
    }
    Ok(())
}

fn main() -> Result<()> {
    init_logger()?;
    tracing::info!("BetterCPT Host starting (Phase 4 — Tauri integrated)...");

    // 单例锁，程序结束前保持持有
    let _singleton = crate::host::singleton::acquire_singleton()
        .context("failed to acquire singleton lock")?;

    // 原生 splash 窗口 — 在 WebView2 初始化期间即时显示
    crate::host::splash::start();

    let ctx = AppContext::new()?;
    let scheduler = ctx.scheduler.clone();
    let shutdown_tx = ctx.shutdown_tx.clone();
    let sched_tx = ctx.scheduler_tx.clone();
    let widget_cmd_tx = ctx.scheduler_tx.clone();
    let state_tree: Arc<Mutex<crate::scheduler::state_tree::StateTree>> =
        Arc::new(Mutex::new(crate::scheduler::state_tree::StateTree::new()));

    // Runtime registry + PluginLoader: Arc<Mutex<>> 供 Tauri commands 共享
    let registry_arc: Arc<Mutex<RuntimeRegistry>> = Arc::new(Mutex::new(RuntimeRegistry::new()));
    let loader_arc: Arc<Mutex<PluginLoader>> = Arc::new(Mutex::new(PluginLoader::new()?));

    // Dashboard 就绪信号：Dashboard 加载完毕 → Host 开始启动插件
    let ready_notify = Arc::new(tokio::sync::Notify::new());

    // Plugin action channel: Tauri command → host loop
    let (action_tx, action_rx) = tokio::sync::mpsc::unbounded_channel::<PluginAction>();

    tauri::Builder::default()
        .manage(registry_arc.clone())
        .manage(loader_arc.clone())
        .manage(action_tx)
        .manage(crate::host::commands::WidgetCommandSender(widget_cmd_tx))
        .manage(crate::host::commands::DashboardReadyNotifier(ready_notify.clone()))
        .invoke_handler(tauri::generate_handler![
            crate::host::commands::get_runtime_snapshot,
            crate::host::commands::install_plugin,
            crate::host::commands::start_plugin,
            crate::host::commands::stop_plugin,
            crate::host::commands::uninstall_plugin,
            crate::host::commands::list_crash_logs,
            crate::host::commands::read_crash_log,
            crate::host::commands::start_drag,
            crate::host::commands::get_settings,
            crate::host::commands::update_settings,
            crate::host::commands::clear_crash_logs,
            crate::host::commands::send_widget_command,
            crate::host::commands::update_plugin_config,
            crate::host::commands::dashboard_ready,
            crate::host::commands::show_dashboard,
        ])
        .setup(move |app| {
            let reg = registry_arc.clone();
            let ld = loader_arc.clone();
            let handle = app.handle().clone();

            // ── Tauri 托盘（替代 Win32 tray）──
            use tauri::{
                menu::{MenuBuilder, MenuItemBuilder},
                tray::TrayIconBuilder,
                Manager, WebviewWindowBuilder, WebviewUrl,
            };

            let dashboard_item = MenuItemBuilder::with_id("dashboard", "Dashboard").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&dashboard_item)
                .item(&quit_item)
                .build()?;

            // 构造 16x16 tray 图标（主色 #006c4a）
            let mut rgba = vec![0u8; 16 * 16 * 4];
            for i in 0..16 * 16 {
                rgba[i * 4] = 0;
                rgba[i * 4 + 1] = 108;
                rgba[i * 4 + 2] = 74;
                rgba[i * 4 + 3] = 255;
            }
            let tray_icon = tauri::image::Image::new_owned(rgba, 16, 16);
            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&menu)
                .on_menu_event(move |app_handle, event| {
                    match event.id().as_ref() {
                        "dashboard" => {
                            if let Some(w) = app_handle.get_webview_window("dashboard") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // ── 预创建 Dashboard 窗口 ──
            let app_handle = handle.clone();
            let w = WebviewWindowBuilder::new(
                &app_handle,
                "dashboard",
                WebviewUrl::App("index.html".into()),
            )
            .title("BetterCPT Dashboard")
            .inner_size(1000.0, 680.0)
            .resizable(true)
            .decorations(false)
            .center()
            .devtools(true)
            .visible(false)
            .background_color(tauri::webview::Color(255, 255, 255, 255))
            .build()?;

            // 拦截关闭事件：设置中开启"最小化到托盘"时，隐藏而非关闭
            let wc = w.clone();
            w.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let settings = crate::host::settings::load_settings().unwrap_or_default();
                    if settings.minimize_to_tray {
                        api.prevent_close();
                        let _ = wc.hide();
                    }
                }
            });

            // Window shown from JavaScript after page renders — eliminates white flash

            // ── 启动 Host 主循环（独立线程）──
            let shutdown_tx_clone = shutdown_tx.clone();
            let stc = state_tree.clone();
            let ready_notify_host = ready_notify.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    run_host_loop(scheduler, sched_tx, shutdown_tx_clone, reg, ld, action_rx, handle, stc, ready_notify_host).await;
                });
            });

            tracing::info!("Tauri setup complete, Dashboard available from tray");
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("Tauri run error: {}", e))?;

    Ok(())
}

async fn run_host_loop(
    scheduler: Arc<Mutex<crate::scheduler::RenderScheduler>>,
    scheduler_tx: tokio::sync::mpsc::UnboundedSender<crate::ipc::messages::IpcRequest>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    registry_arc: Arc<Mutex<RuntimeRegistry>>,
    loader_arc: Arc<Mutex<PluginLoader>>,
    mut action_rx: tokio::sync::mpsc::UnboundedReceiver<PluginAction>,
    app_handle: tauri::AppHandle,
    state_tree: Arc<Mutex<crate::scheduler::state_tree::StateTree>>,
    ready_notify: Arc<tokio::sync::Notify>,
) {
    // ── Qt 子进程 ──
    let mut qt = WidgetQtRuntime::new();
    if let Err(e) = qt.start_qt_process() {
        tracing::error!("Qt startup failed: {}", e);
        return;
    }

    // ── 等待 Dashboard 加载完毕 ──
    tracing::info!("Waiting for Dashboard ready signal...");
    ready_notify.notified().await;
    tracing::info!("Dashboard ready, starting plugins...");

    // ── 插件系统 ──
    let mut isolate_pool = IsolatePool::new(scheduler_tx);
    let mut cap_registry = CapabilityRegistry::new();
    cap_registry.register_capability("widget:overlay", "桌面覆盖层渲染").ok();
    cap_registry.register_capability("window:create", "创建工具窗口").ok();
    cap_registry.register_capability("notification", "系统通知").ok();

    let manifests = {
        let ld = loader_arc.lock().await;
        ld.scan_plugins().unwrap_or_default()
    };
    tracing::info!("Found {} plugin(s)", manifests.len());

    {
        let mut ld = loader_arc.lock().await;
        let mut reg = registry_arc.lock().await;
        for manifest in &manifests {
            match ld.load(&manifest.id, &mut isolate_pool, &cap_registry, &mut reg) {
                Ok(()) => {
                    tracing::info!("Plugin loaded: {}", manifest.id);
                    emit_runtime_event(&app_handle, RuntimeEvent {
                        event: "plugin_started".into(),
                        plugin_id: manifest.id.clone(),
                        crash_count: None,
                        memory_bytes: None,
                    });
                }
                Err(e) => tracing::warn!("Failed to load '{}': {:#}", manifest.id, e),
            }
        }
        tracing::info!("Plugin loading complete: {} running", reg.count());
    }

    // ── 首次 flush ──
    {
        let mut sched = scheduler.lock().await;
        let batch = sched.flush().unwrap_or_default();
        drop(sched);
        if !batch.is_empty() {
            tracing::info!("Initial flush: {} commands", batch.len());
            if flush_and_send(&batch, &mut qt, &mut isolate_pool).is_ok() {
                let mut tree = state_tree.lock().await;
                for cmd in &batch {
                    if let Err(e) = tree.apply_ipc(cmd) {
                        tracing::warn!("StateTree apply_ipc (init) failed: {}", e);
                    }
                }
                tracing::info!("StateTree synced after init: {} nodes", tree.node_count());
            }
        }
    }

    // ── 主循环 ──
    let mut shutdown = ShutdownListener::new(shutdown_tx).unwrap();
    let mut tick: u64 = 0;
    loop {
        // 处理来自 Dashboard 的插件操作请求
        while let Ok(action) = action_rx.try_recv() {
            match action {
                PluginAction::Start { plugin_id } => {
                    tracing::info!("Action: start {}", plugin_id);
                    // 读取插件代码 → spawn Isolate
                    let code = {
                        let ld = loader_arc.lock().await;
                        ld.get_plugin_code(&plugin_id).unwrap_or_default()
                    };
                    if !code.is_empty() {
                        if let Err(e) = isolate_pool.spawn(&plugin_id, &code) {
                            tracing::error!("Start failed for '{}': {}", plugin_id, e);
                        }
                    }
                    let mut reg = registry_arc.lock().await;
                    reg.update_status(&plugin_id, "running").ok();
                    emit_runtime_event(&app_handle, RuntimeEvent {
                        event: "plugin_started".into(),
                        plugin_id: plugin_id.clone(),
                        crash_count: None,
                        memory_bytes: None,
                    });
                }
                PluginAction::Stop { plugin_id } => {
                    tracing::info!("Action: stop {}", plugin_id);
                    // 1) kill Isolate（停止 SDK 命令流）
                    isolate_pool.kill(&plugin_id).ok();
                    // 2) 立即发送 removeWidget 给 Qt
                    {
                        let tree = state_tree.lock().await;
                        let ids: Vec<String> = tree.all_nodes().iter().map(|n| n.id.clone()).collect();
                        tracing::info!("Stop: {} widgets in StateTree", ids.len());
                        if !ids.is_empty() {
                            let batch: Vec<crate::ipc::messages::IpcRequest> = ids.iter().map(|wid| {
                                crate::ipc::messages::IpcRequest {
                                    jsonrpc: "2.0".into(),
                                    method: crate::ipc::messages::METHOD_REMOVE_WIDGET.into(),
                                    params: serde_json::json!({"id": wid}),
                                    id: 0,
                                }
                            }).collect();
                            tracing::info!("Stop: sending removeWidget batch");
                            match flush_and_send(&batch, &mut qt, &mut isolate_pool) {
                                Ok(()) => tracing::info!("Stop: removeWidget OK"),
                                Err(e) => tracing::error!("Stop: removeWidget FAILED: {}", e),
                            }
                        } else {
                            tracing::warn!("Stop: StateTree empty, no widgets to remove");
                        }
                    }
                    // 3) 更新状态
                    let mut reg = registry_arc.lock().await;
                    reg.update_status(&plugin_id, "stopped").ok();
                    emit_runtime_event(&app_handle, RuntimeEvent {
                        event: "plugin_stopped".into(),
                        plugin_id: plugin_id.clone(),
                        crash_count: None,
                        memory_bytes: None,
                    });
                }
                PluginAction::ConfigUpdate { plugin_id, config_json } => {
                    tracing::info!("Action: config_update {}", plugin_id);
                    // 防注入：仅当 config_json 是合法 JSON 时才注入
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&config_json) {
                        let safe_json = serde_json::to_string(&v).unwrap_or_default();
                        let code = format!(
                            "(function(){{ var cfg={}; for(var k in cfg){{ config[k]=cfg[k]; }} if(typeof onConfigChange==='function') onConfigChange(); }})()",
                            safe_json
                        );
                        isolate_pool.fire_custom_event(&code);
                    } else {
                        tracing::warn!("ConfigUpdate: invalid JSON for plugin '{}'", plugin_id);
                    }
                }
                PluginAction::Uninstall { plugin_id } => {
                    tracing::info!("Action: uninstall {}", plugin_id);
                    let mut reg = registry_arc.lock().await;
                    reg.unregister(&plugin_id).ok();
                    emit_runtime_event(&app_handle, RuntimeEvent {
                        event: "plugin_uninstalled".into(),
                        plugin_id: plugin_id.clone(),
                        crash_count: None,
                        memory_bytes: None,
                    });
                }
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(16)) => {
                tick += 1;
                let batch = {
                    let mut sched = scheduler.lock().await;
                    sched.flush().unwrap_or_default()
                };
                if !batch.is_empty() {
                    // 1) 先发送 IPC 到 Qt，再同步到 StateTree——StateTree 只记录 Qt 确认收到的命令
                    if let Err(e) = flush_and_send(&batch, &mut qt, &mut isolate_pool) {
                        tracing::error!("flush_and_send: {}", e);
                        if qt.detect_crash().unwrap_or(false) {
                            tracing::warn!("Qt crash detected, restarting...");
                            if qt.restart_qt().is_ok() {
                                // 2) 崩溃恢复：从 StateTree 全量回放到新 Qt 进程
                                let tree = state_tree.lock().await;
                                let replay = tree.generate_replay_commands();
                                if !replay.is_empty() {
                                    tracing::info!("Replaying {} commands from StateTree after Qt restart", replay.len());
                                    if let Err(e) = flush_and_send(&replay, &mut qt, &mut isolate_pool) {
                                        tracing::error!("StateTree replay failed: {}", e);
                                    }
                                }
                            }
                        }
                    } else {
                        // Qt 确认收到 → 同步到 StateTree
                        let mut tree = state_tree.lock().await;
                        for cmd in &batch {
                            if let Err(e) = tree.apply_ipc(cmd) {
                                tracing::warn!("StateTree apply_ipc failed: {}", e);
                            }
                        }
                        tracing::debug!("StateTree: {} nodes, version {}", tree.node_count(), tree.version());
                    }
                } else {
                    // 无待发送命令 → 发 ping 逼 Qt 响应，顺便清空事件队列
                    let ping = crate::ipc::messages::IpcRequest {
                        jsonrpc: "2.0".into(),
                        method: "ping".into(),
                        params: serde_json::json!({}),
                        id: 99999,
                    };
                    let _ = flush_and_send(&[ping], &mut qt, &mut isolate_pool);
                }
                if tick % 60 == 0 {
                    let mut ld = loader_arc.lock().await;
                    let mut reg = registry_arc.lock().await;
                    check_hot_reload(&mut ld, &mut isolate_pool, &mut reg).ok();
                }
            }
            result = shutdown.recv() => {
                result.ok();
                break;
            }
        }
    }
    tracing::info!("Host main loop exiting");
}
