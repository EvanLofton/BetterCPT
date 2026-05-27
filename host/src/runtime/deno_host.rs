use std::sync::OnceLock;

use anyhow::{Context, Result};
use deno_core::{JsRuntime, RuntimeOptions};
use tokio::sync::mpsc;

use crate::ipc::messages::IpcRequest;

static SCHEDULER_TX: OnceLock<mpsc::UnboundedSender<IpcRequest>> = OnceLock::new();

// ── JS SDK Preamble（内联 op_send 实现，不依赖 deno_core extension）──
// 直接在 JS 侧通过全局 tx 代理对象将命令写入 Rust

const SDK_PREAMBLE: &str = r#"
(function() {
  // widget 追踪表：widgetId → Widget 实例（事件回路路由用）
  globalThis.__widgets = {};
  // 事件回调：由 Rust 侧收到 Qt 事件后调用
  globalThis.__fireEvent = function(widgetId, eventName) {
    var w = globalThis.__widgets[widgetId];
    if (!w) return;
    // eventName: "onHover" → callback: w._onHover
    var cb = w['_' + eventName];
    if (typeof cb === 'function') cb();
  };

  // __bettercpt_tx 由 Rust 侧在 globalThis 上注入
  function __bettercpt_send(method, params) {
    var tx = globalThis.__bettercpt_tx;
    if (tx && tx.send) {
      tx.send(method, JSON.stringify(params || {}));
    }
  }

  let __nextId = 0;
  function __nextWidgetId() { return 'widget_' + (++__nextId); }

  globalThis.Widget = function(id) {
    this.id = id;
    this._children = [];
  };
  Widget.prototype.addChild = function(child) {
    this._children.push(child);
    __bettercpt_send('addChild', { parentId: this.id, childId: child.id });
  };
  Widget.prototype.removeChild = function(child) {
    this._children = this._children.filter(function(c) { return c.id !== child.id; });
    __bettercpt_send('removeChild', { parentId: this.id, childId: child.id });
  };
  Widget.prototype.setScale = function(scale, duration) {
    __bettercpt_send('setScale', { id: this.id, scale: scale, duration: duration || 200 });
  };
  Widget.prototype.setOpacity = function(opacity, duration) {
    __bettercpt_send('setOpacity', { id: this.id, opacity: opacity, duration: duration || 150 });
  };
  // 事件注册仅存 JS 回调（Qt 通过 EventFilter → stdout → Host → __fireEvent 触发）
  Widget.prototype.onHover = function(cb) { if (cb) this._onHover = cb; };
  Widget.prototype.onLeave = function(cb) { if (cb) this._onLeave = cb; };
  Widget.prototype.onClick = function(cb) { if (cb) this._onClick = cb; };
  Widget.prototype.onDrop = function(cb) { if (cb) this._onDrop = cb; };
  Widget.prototype.raise = function() { __bettercpt_send('raise', { id: this.id }); };
  Widget.prototype.lower = function() { __bettercpt_send('lower', { id: this.id }); };

  globalThis.createRect = function(options) {
    var id = __nextWidgetId();
    var w = new Widget(id);
    globalThis.__widgets[id] = w;
    __bettercpt_send('createRect', { id: id, x: options.x || 0, y: options.y || 0,
      w: options.width, h: options.height,
      color: options.color, radius: options.radius, blur: options.blur,
      containerId: options.containerId || '' });
    return w;
  };
  globalThis.createIcon = function(options) {
    var id = __nextWidgetId();
    var w = new Widget(id);
    globalThis.__widgets[id] = w;
    __bettercpt_send('createIcon', { id: id, x: options.x || 0, y: options.y || 0,
      size: options.size, src: options.src,
      containerId: options.containerId || '' });
    return w;
  };
  globalThis.createText = function(text, options) {
    var id = __nextWidgetId();
    var opts = options || {};
    var w = new Widget(id);
    globalThis.__widgets[id] = w;
    __bettercpt_send('createText', { id: id, x: opts.x, y: opts.y,
      text: text, size: opts.size, color: opts.color, weight: opts.weight });
    return w;
  };

  globalThis.storage = {
    get: function(key) { return __bettercpt_send('storage.get', { key: key }); },
    set: function(key, value) { __bettercpt_send('storage.set', { key: key, value: value }); },
    delete: function(key) { __bettercpt_send('storage.delete', { key: key }); },
  };
  globalThis.dialog = {
    alert: function(msg) { __bettercpt_send('dialog.alert', { message: msg }); },
    confirm: function(msg) { return __bettercpt_send('dialog.confirm', { message: msg }); },
    prompt: function(msg, def) { return __bettercpt_send('dialog.prompt', { message: msg, defaultValue: def }); },
  };
  globalThis.notification = {
    show: function(title, body) { __bettercpt_send('notification.show', { title: title, body: body }); },
  };
  globalThis.system = {
    launch: function(path) { __bettercpt_send('system.launch', { path: path }); },
    memory: function() { return __bettercpt_send('system.memory', {}); },
    screenWidth: 0,
    screenHeight: 0,
    scaleFactor: 1.0,
  };

  // 插件配置对象 — Dashboard 修改后通过 update_plugin_config 注入
  globalThis.config = {};
  globalThis.onConfigChange = null;

  // console.log 截获 → 写入插件目录 plugin.log
  globalThis.__pluginId = '';
  var _origConsoleLog = console.log;
  console.log = function() {
    var msg = Array.prototype.slice.call(arguments).join(' ');
    __bettercpt_send('plugin.log', { pluginId: globalThis.__pluginId, message: msg });
    if (typeof _origConsoleLog === 'function') _origConsoleLog.apply(console, arguments);
  };
})();
"#;

// ── JsRuntimeHost ──
pub struct JsRuntimeHost {
    runtime: JsRuntime,
}

impl JsRuntimeHost {
    pub fn new(scheduler_tx: mpsc::UnboundedSender<IpcRequest>) -> Result<Self> {
        SCHEDULER_TX.set(scheduler_tx.clone()).ok();

        let mut runtime = JsRuntime::new(RuntimeOptions::default());

        // 注入 __bettercpt_tx 代理对象（绕过 extension/op2，直接用 v8）
        {
            let scope = &mut runtime.handle_scope();
            let context = scope.get_current_context();
            let global = context.global(scope);

            // 创建一个 JS 函数 send(method, params_json)
            let send_fn = deno_core::v8::Function::new(
                scope,
                |scope: &mut deno_core::v8::HandleScope,
                 args: deno_core::v8::FunctionCallbackArguments,
                 _rv: deno_core::v8::ReturnValue| {
                    if args.length() < 2 { return; }
                    let method_v = args.get(0);
                    let params_v = args.get(1);
                    let method = method_v.to_rust_string_lossy(scope);
                    let params_json = params_v.to_rust_string_lossy(scope);
                    if let Some(tx) = SCHEDULER_TX.get() {
                        let params: serde_json::Value = match serde_json::from_str(&params_json) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::warn!(
                                    "Plugin sent malformed JSON params for '{}': {} | raw: {}",
                                    method, e, params_json
                                );
                                return;
                            }
                        };
                        let req = IpcRequest {
                            jsonrpc: "2.0".into(),
                            method,
                            params,
                            id: 0,
                        };
                        let _ = tx.send(req);
                    }
                },
            );

            // tx = { send: send_fn }
            let tx_obj = deno_core::v8::Object::new(scope);
            let send_key = deno_core::v8::String::new(scope, "send").unwrap();
            let send_val: deno_core::v8::Local<'_, deno_core::v8::Value> = send_fn.unwrap().into();
            tx_obj.set(scope, send_key.into(), send_val);

            let key = deno_core::v8::String::new(scope, "__bettercpt_tx").unwrap();
            let tx_val: deno_core::v8::Local<'_, deno_core::v8::Value> = tx_obj.into();
            global.set(scope, key.into(), tx_val);
        }

        runtime
            .execute_script("<sdk-preamble>", SDK_PREAMBLE)
            .with_context(|| "failed to inject SDK preamble")?;

        // 注入真实屏幕尺寸（Win32 GetSystemMetrics + GetDeviceCaps DPI）
        {
            use windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, LOGPIXELSX};
            use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
            unsafe {
                let sys_w = GetSystemMetrics(SM_CXSCREEN);
                let sys_h = GetSystemMetrics(SM_CYSCREEN);
                let scale = {
                    let hdc = GetDC(None);
                    if hdc.is_invalid() { 1.0 }
                    else {
                        let dpi = GetDeviceCaps(hdc, LOGPIXELSX) as f64;
                        dpi / 96.0
                    }
                };
                let logical_w = (sys_w as f64 / scale).round() as i32;
                let logical_h = (sys_h as f64 / scale).round() as i32;
                let script = format!(
                    "system.screenWidth = {}; system.screenHeight = {}; system.scaleFactor = {};",
                    logical_w, logical_h, scale
                );
                let _ = runtime.execute_script("<screen-info>", script);
                tracing::info!("Screen: logical {}x{} dpi={} scale={}", logical_w, logical_h, (scale * 96.0) as i32, scale);
            }
        }

        tracing::info!("JsRuntime initialized (v8 direct injection)");
        Ok(Self { runtime })
    }

    /// 执行任意 JS 代码（不返回结果）
    pub fn execute_script_raw(&mut self, code: &str) -> Result<()> {
        self.runtime
            .execute_script("<custom>", code.to_owned())
            .map(|_| ())
            .with_context(|| "execute_script_raw failed")
    }

    /// 触发插件 widget 事件回调（事件回路：Qt → Host → Deno）
    #[allow(dead_code)]
    pub fn fire_event(&mut self, widget_id: &str, event_name: &str) {
        let code = format!(
            "__fireEvent('{}', '{}');",
            widget_id.replace('\'', "\\'"),
            event_name.replace('\'', "\\'")
        );
        if let Err(e) = self.runtime.execute_script("<event>", code) {
            tracing::warn!("fire_event {} {}: {:?}", widget_id, event_name, e);
        }
    }

    /// 检查是否拥有指定 widget，有则触发事件回调。返回 true 表示事件被处理。
    pub fn try_fire_event(&mut self, widget_id: &str, event_name: &str) -> bool {
        let escaped_id = widget_id.replace('\'', "\\'");
        let escaped_ev = event_name.replace('\'', "\\'");

        // 合并 check + fire 为单次 execute_script，避免 double borrow
        let code = format!(
            "(function() {{ var w = globalThis.__widgets['{id}']; if (w) {{ var cb = w['_{ev}']; if (typeof cb === 'function') cb(); return true; }} return false; }})()",
            id = escaped_id, ev = escaped_ev
        );
        match self.runtime.execute_script("<event>", code) {
            Ok(global) => {
                let scope = &mut self.runtime.handle_scope();
                let local = deno_core::v8::Local::new(scope, &global);
                deno_core::serde_v8::from_v8::<bool>(scope, local).unwrap_or(false)
            }
            Err(e) => {
                tracing::warn!("try_fire_event failed: {:?}", e);
                false
            }
        }
    }

    /// 执行插件代码。v1 禁止 import/export，含 ESM 语句则明确拒绝。
    ///
    /// NOTE: import/export 检测为逐行前缀匹配，若字符串字面量内出现这些关键词会误判。
    /// v1 插件规模小且为手写 JS，不会触发；Phase 5 接入 SWC 后替换为 AST 级检测。
    pub fn execute(&mut self, code: &str) -> Result<serde_json::Value> {
        for line in code.lines() {
            let t = line.trim();
            if t.starts_with("import ") || t.starts_with("export ") {
                anyhow::bail!(
                    "ESM import/export is not supported in v1. Use esbuild to bundle multi-file plugins into a single JS file.\n  offending line: \"{}\"",
                    t
                );
            }
        }

        let global = self
            .runtime
            .execute_script("<plugin>", code.to_owned())
            .with_context(|| "JsRuntime execution failed")?;

        let scope = &mut self.runtime.handle_scope();
        let local = deno_core::v8::Local::new(scope, &global);
        let value = deno_core::serde_v8::from_v8(scope, local)
            .with_context(|| "failed to deserialize JS result")?;

        Ok(value)
    }
}
