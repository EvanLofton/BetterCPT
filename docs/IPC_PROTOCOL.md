# BetterCPT IPC 协议规范

> 定义 Host 与各 Runtime 之间的通信协议：JSON-RPC 格式、消息类型、错误码、版本、重连。
> 
> 所有跨进程通信必须遵守本规范。

---

## 一、协议版本

```
当前版本：bettercpt-ipc/1
```

所有 JSON-RPC 消息必须包含协议版本标识。未来协议升级时，通过版本号保证兼容。

---

## 二、传输层

| 通道 | 使用方 | 方式 |
|---|---|---|
| Host ↔ Qt 子进程 | Widget Runtime 渲染指令 | stdin/stdout pipe |
| Host ↔ Deno (Widget) | Widget 业务逻辑 | 进程内 mpsc channel |
| Host ↔ Deno (Script) | Script 插件 | 进程内 mpsc channel |
| Host ↔ Utility Window | 功能插件界面（WebView） | Tauri IPC (invoke/emit) |
| Host ↔ Dashboard | 管理界面 | Tauri IPC (invoke/emit) |
| Host ↔ Native | 系统级插件（远期） | 命名管道 |

---

## 三、消息格式（JSON-RPC 2.0）

### 请求（Request）

```json
{
  "jsonrpc": "2.0",
  "method": "widget.setScale",
  "params": {
    "id": "widget://dock/icon/0",
    "scale": 1.2,
    "duration": 200
  },
  "id": 1
}
```

### 响应（Response）

```json
{
  "jsonrpc": "2.0",
  "result": { "widgetId": "widget://dock/icon/0" },
  "id": 1
}
```

### 错误（Error）

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Widget not found",
    "data": { "widgetId": "widget://nonexistent" }
  },
  "id": 1
}
```

### 事件推送（Notification，无 id，不期待响应）

```json
{
  "jsonrpc": "2.0",
  "method": "system.event",
  "params": {
    "type": "theme.changed",
    "data": { "mode": "dark" }
  }
}
```

---

## 四、错误码

| 代码 | 含义 | 场景 |
|---|---|---|
| -32700 | Parse error | JSON 格式错误 |
| -32600 | Invalid request | 不符合 JSON-RPC 规范 |
| -32601 | Method not found | 方法名不存在 |
| -32602 | Invalid params | 参数类型或必填字段错误 |
| -32603 | Internal error | 未预期的内部错误 |
| -32000 | Permission denied | 权限不足 |
| -32001 | Widget not found | id 对应的 Widget 不存在 |
| -32002 | Runtime not ready | Runtime 处于 recovering/stopped 状态 |
| -32003 | Timeout | 操作超时 |

---

## 五、消息顺序保证

- 同一 IPC 通道内消息**严格按发送顺序到达**
- Host 不保证不同通道之间的顺序（Qt IPC 和 Deno IPC 各自独立）
- 需要跨通道同步的场景，使用状态树版本号

---

## 六、重连与 Replay

### Qt 进程崩溃重连

1. Host 检测到 pipe 断开
2. Host 冻结所有 Widget IPC
3. 启动新 Qt 进程，建立新 pipe
4. Host 全量同步状态树（replay）到新 Qt 进程
5. 解除 IPC 冻结

### Replay 消息格式

```json
{
  "jsonrpc": "2.0",
  "method": "sync.widgetTree",
  "params": {
    "version": 42,
    "widgets": [
      {
        "id": "widget://dock",
        "type": "rect",
        "bounds": { "x": 0, "y": 1000, "w": 400, "h": 64 },
        "blur": true,
        "radius": 16,
        "children": ["widget://dock/icon/0", "widget://dock/icon/1"]
      },
      {
        "id": "widget://dock/icon/0",
        "type": "icon",
        "src": "chrome.png",
        "size": 48,
        "scale": 1.0,
        "opacity": 1.0
      }
    ]
  }
}
```

### Replay 分页

Widget 树超过 1MB 时，分页发送。

**分页规则**：

1. **分页单位**：以 Widget 为最小单位切割，不在单个 Widget 内部断页
2. **顺序保证**：深度优先，父节点必须在子节点之前发送
3. **接收状态**：Qt 收到第一页时进入 `replaying` 状态，拒绝处理普通渲染指令；收到最后一页（`is_last: true`）后切换回 `active`

**分页消息格式**：

```json
{
  "jsonrpc": "2.0",
  "method": "sync.widgetTree",
  "params": {
    "version": 42,
    "page": 1,
    "total_pages": 3,
    "is_last": false,
    "widgets": [ ... ]
  }
}
```

**Qt 侧实现**：维护一个 merge buffer。收齐所有页后一次性应用到 QWidget 树，保证原子性。如果分页传输中途 pipe 断开，Qt 丢弃 buffer，等待 Host 重新发起全量同步。

---

## 八、渲染指令参考

### Widget 创建

| Method | 参数 | 说明 |
|---|---|---|
| createRect | id, bounds, blur, radius, color | 创建矩形区域 |
| createText | id, text, size, weight, color | 创建文本 |
| createIcon | id, src, size | 创建图标 |

### Widget 操作

| Method | 参数 | 说明 |
|---|---|---|
| setScale | id, scale, duration | 缩放动画 |
| setOpacity | id, opacity, duration | 透明度动画 |
| setBlur | id, enabled | 毛玻璃开关 |
| addChild | parentId, childId | 添加子 Widget |
| removeChild | parentId, childId | 移除子 Widget |
| setBounds | id, bounds | 设置位置和大小 |

### 事件绑定

| Method | 参数 | 说明 |
|---|---|---|
| onHover | id, callbackId | 注册 hover 回调 |
| onLeave | id, callbackId | 注册 leave 回调 |
| onClick | id, callbackId | 注册 click 回调 |

### Qt → Host 事件回传

| Method | 参数 | 说明 |
|---|---|---|
| event.hover | widgetId | Widget 被 hover |
| event.leave | widgetId | Widget leave |
| event.click | widgetId | Widget 被点击 |
| event.mouseDown | widgetId, x, y | 鼠标在 Widget 内按下 |
| event.mouseMove | widgetId, x, y | 鼠标在 Widget 内移动 |
| event.mouseUp | widgetId, x, y | 鼠标在 Widget 内松开 |
| event.animationCompleted | widgetId, property, finalValue | 动画完成 |

---

## 九、Batching

Scheduler flush 时，多条渲染指令打包为单条 IPC 消息：

```json
{
  "jsonrpc": "2.0",
  "method": "batch",
  "params": {
    "commands": [
      { "method": "setScale", "params": { "id": "widget://dock/icon/0", "scale": 1.2, "duration": 200 } },
      { "method": "setOpacity", "params": { "id": "widget://dock/icon/0", "opacity": 0.9, "duration": 150 } },
      { "method": "setScale", "params": { "id": "widget://dock/icon/1", "scale": 1.0, "duration": 200 } }
    ]
  }
}
```

Batch 内的命令按数组顺序执行。不期待逐条响应——batch 整体返回 OK 或错误。

### Batch vs Naive 性能对比（v1 → 后期）

| 规模 | Naive (逐条 send→read) | Batch (数组打包) |
|------|----------------------|-----------------|
| 13 条 | ~400ms | ~50ms |
| 100 条 | ~3s (不可用) | ~100ms |
| 500 条 | 不可用 | ~200ms |

启用条件：单帧指令 > 20 时启用（Phase 5+）。

**Qt 侧改动**：RenderServer 检测数组格式消息 → 逐条处理 → 数组格式响应。

**Rust 侧改动**：`flush_and_send` 改为单次 `write_all` + 单次 `read_line`：
```rust
fn flush_batch(batch: &[IpcRequest], qt: &mut WidgetQtRuntime) -> Result<Vec<QtResponse>> {
    qt.stdin.write_all(serde_json::to_string(batch)?.as_bytes())?;
    qt.stdin.write_all(b"\n")?;
    let resp = qt.read_line()?;
    Ok(serde_json::from_str(&resp)?)
}
```

---

## 十、消息大小限制

| 消息类型 | 最大大小 | 超限处理 |
|---|---|---|
| 单条渲染指令 | 4KB | 拒绝并返回错误 |
| Batch | 64KB | 拆分为多个 batch |
| Replay (sync.widgetTree) | 1MB | 分页发送 |
