# BetterCPT Runtime 规范

> 定义所有 Runtime 类型的统一行为：生命周期、状态机、崩溃恢复、Watchdog、热更新。
> 
> 不包含 Widget 渲染细节（见 WIDGET_RUNTIME.md）和 IPC 协议细节（见 IPC_PROTOCOL.md）。

---

## 一、Runtime 生命周期

所有 Runtime 类型遵循统一的生命周期协议。

### 状态机

```
                 install
  [None] ─────────────────→ [Installed]
                                │
                              start
                                ↓
                            [Running] ←──────────┐
                                │                  │
                      ┌─────────┼─────────┐        │
                      ↓         ↓         ↓        │
                  [Crashed]  [Paused]  [Stopping]  │
                      │         │         │        │
                    recover   resume     stop      │
                      │         │         │        │
                      └─────────┴─────────┘        │
                                │                  │
                              stop                 │
                                ↓                  │
                            [Stopped] ─── start ───┘
```

### 状态定义

| 状态 | 含义 | 在此状态下 |
|---|---|---|
| Installed | 插件已安装，未运行 | 可以 start |
| Running | 正常运行 | 心跳正常，IPC 连通 |
| Paused | 暂时挂起 | IPC 保持，但不处理命令（v1 不实现） |
| Crashed | 异常退出 | 等待 Host 恢复 |
| Recovering | 恢复中 | Host 正在重启进程或重放状态 |
| Stopped | 正常停止 | 资源已释放，可以重新 start |

### 生命周期操作

| 操作 | 触发 | 前置状态 | 后置状态 |
|---|---|---|---|
| install | 用户安装插件 | None | Installed |
| start | 用户启动 / Host 自动启动 | Installed, Stopped | Running |
| stop | 用户停止 / Host 关闭 | Running | Stopped |
| crash | Runtime 进程异常退出 | Running | Crashed |
| recover | Host 自动恢复 | Crashed | Recovering → Running |
| pause | 用户暂停（v1 不实现） | Running | Paused |
| resume | 用户恢复（v1 不实现） | Paused | Running |

---

## 二、崩溃恢复

### 恢复次数限制

| Runtime | 最大自动重启次数 | 超限后行为 |
|---|---|---|
| Widget Qt 进程 | 5 | 通知用户，停止重启 |
| Widget Isolate（单个插件） | 5 | 通知用户，停止重启 |
| Script Runtime | 5 | 通知用户，停止重启 |
| Native Runtime | 0 | 仅通知用户 |

### Qt 进程崩溃恢复顺序

```
1. Host 检测到 Qt 进程退出（IPC pipe 断开 / try_wait）
2. Host 冻结所有 Widget 相关 IPC（入队不执行）
   → 向 Deno 推送 "qt.recovering" 事件
3. Host 标记状态树为 "recovering"，暂停版本号递增
   → 记录崩溃前状态树快照
4. Host 启动新 Qt 子进程
   → 等待 QApplication ready 信号
   → 建立新 IPC pipe
5. Host 全量同步状态树到新 Qt 进程
   → 深度优先遍历，逐一发送 create 指令
   → 动画状态回退到起始值（非中间帧）
6. Host 重放交互状态
   → 重新注册 Hover 区域、Click 回调
   → 焦点重置为无
7. Host 标记状态树为 "active"，恢复版本号
8. Host 解除 IPC 冻结
   → 执行步骤 2 中入队的命令
   → 向 Deno 推送 "qt.recovered"
```

**约束**：Qt 启动超时 5 秒则放弃恢复。动画回退到起始值（不误触发动画完成事件）。焦点统一重置。

### Isolate 崩溃恢复顺序（per-plugin Isolate 模型）

v1 采用 per-plugin Isolate 池：每个 Widget 插件运行在独立的 V8 Isolate 中，共享同一个 JsRuntime 进程。每个 Isolate 有独立的堆和 globalThis。一个插件的内存泄漏、GC 压力、未捕获异常不会跨越 Isolate 边界。

```
Host Isolate Pool
  ├── Isolate A → Widget 插件 "dock"
  ├── Isolate B → Widget 插件 "weather-card"
  └── Isolate C → Widget 插件 "system-monitor"
```

每个 Isolate 开销约 1-2MB。10 个插件额外增加 10-20MB，在 60MB 总目标内可接受。

**Isolate 隔离边界**：

- 每个 Isolate 拥有独立的堆和 globalThis
- Isolate 之间**不共享任何对象引用**
- 单个 Isolate 的内存上限通过 V8 的 heap limit 约束
- 一个插件的 GC 压力、内存泄漏不会跨越 Isolate 边界

**Isolate 级崩溃恢复**：

```
1. Host 检测到 Isolate [plugin-id] panic
2. Host 向 Qt 推送 "widget.recovering"，携带 plugin-id
   → Qt 仅冻结该插件对应的 Widget，其他 Widget 正常交互
3. Host 销毁该 Isolate，重新创建 Isolate
4. 重新加载该插件的 TS 入口文件
5. TS 侧重新执行 createRect/createIcon
   → Host 按逻辑名（v2+）或调用顺序位置（v1 fallback）匹配已有 Widget
   → 不重复创建 QWidget
6. Host 重新绑定该插件的事件回调
7. Host 向 Qt 推送 "widget.active"
```

**对比旧模型**：旧模型（单 Deno 实例共享）一个插件死循环 → 所有 Widget 停止响应。新模型（per-plugin Isolate）一个插件死循环 → 只有该插件恢复，其他插件无感知。

### Deno + Qt 同时崩溃（概率极低）

先执行 Qt 恢复，Qt 完成后立即执行 Deno 恢复。Deno 在 Qt 已恢复基础上执行。

### 多 Isolate 多线程架构（Phase 5+ 启用）

**v1 现状**：所有 Isolate 在单线程顺序执行（`JsRuntime: !Send` 限制）。

**后期架构**：插件数 > 5 后，每个 Isolate 独立线程，通过 mpsc channel 与主线程通信：

```
主线程 (Host main loop)
    │
    ├── Qt pipe (WidgetQtRuntime)
    ├── Scheduler (Arc<Mutex<RenderScheduler>>)
    └── 事件分发器
         ├── mpsc::Sender → Isolate 线程 A (Dock)
         ├── mpsc::Sender → Isolate 线程 B (Weather Card)
         └── mpsc::Sender → Isolate 线程 C (Script)

每个 Isolate 线程:
    loop { recv(event) → execute_script → _send → mpsc → Scheduler }
```

**关键改动**：
- `IsolatePool::spawn()` 改为 `std::thread::spawn`，每个 isolate 独立线程
- `route_event()` 查 widget→plugin 映射表，通过 `mpsc::Sender` 发送事件
- 主线程不直接操作 JsRuntime，只通过 channel 收发

**不需要改的**：Scheduler、Qt pipe、SDK preamble、v8 注入方式——全部复用。

---

## 三、Isolate Watchdog

### 三层保护（per-Isolate 粒度）

**Execution Timeout**：单个 Isolate 的 op 执行超时 500ms → 终止当前 op。

**Microtask Budget**：单个 Isolate 每轮事件循环最多 10,000 个微任务 → 超预算中断该 Isolate。

**Event Loop Watchdog**：Rust 侧为每个 Isolate 维护独立的心跳监控。单个 Isolate 5 秒无响应 → 触发该 Isolate 的崩溃恢复，其他 Isolate 不受影响。

---

## 四、Timer 生命周期

Deno Core 自带 `setTimeout` / `setInterval`，直接暴露给插件使用。

**关键约束**：插件 stop 时，Host 销毁该 Isolate → 所有 pending timer 随 Isolate 自动取消，无需手动清理。

Isolate 重启后 timer 不自动恢复——由插件代码在入口文件中重新注册。

**开发者须知**：在模块顶层直接写的 `setInterval`，热更新时旧 Isolate 销毁、新 Isolate 重新执行入口文件，会自动重新注册——这是预期行为。但如果在运行时动态创建的 timer（如用户点击后 `setTimeout`），热更新后不会恢复，需插件自行处理。

**热更新后 JS 状态重置**：Isolate 重建后所有 JS 变量、闭包、运行时状态会归零（如 `let count = 0` 重新从 0 开始）。需要跨热更新持久化的状态请使用 `BetterCPT.storage`。

## 五、Async Cancellation

插件在运行期间可能发起异步操作（fetch、文件读取等）。当插件 stop 或 crash 时：

- Host 销毁 Isolate → 所有 pending Promise 随 Isolate 消失
- 网络请求走 Deno op，Isolate 销毁后 op 自然断开
- 文件操作同样随 op 断开而终止

不需要额外的取消机制——Deno Core 的 Isolate 销毁天然处理。

## 六、Plugin Crash Report

插件崩溃时 Host 自动保存诊断信息到 `%AppData%/BetterCPT/logs/crash/<plugin-id>-<timestamp>.json`。

> **文件名 sanitize 规则**：plugin-id 中的 `@` 替换为空字符串，`/` 替换为 `-`。例：`@local/pdf-toolkit` → `local-pdf-toolkit-<timestamp>.json`。Windows 文件名不允许 `@` 和 `/` 字符。

```json
{
  "pluginId": "weather-card",
  "timestamp": "2026-05-16T10:30:00Z",
  "runtime": "widget",
  "error": "Uncaught TypeError: ...",
  "stack": "...",
  "isolateState": "running",
  "crashCount": 2
}
```

最小集，不做自动上报。

## 七、Plugin Storage Quota

每个插件的 KV Storage（`config.json`）上限 **1MB**。

- 超限写入抛出 `STORAGE_QUOTA_EXCEEDED` 错误
- Host 在每次 `storage.set()` 调用时检查容量
- 插件开发者应管理自身数据大小

## 八、Render Scheduler

位于 Rust Host 内部，在 Deno IPC 接收端和状态树之间。

```
TS SDK → Command Queue (mpsc) → Scheduler → State Tree → Qt
```

### 调度规则

- **按帧 flush**：默认 16ms（60FPS），Dock 可声明 8ms（120FPS）
- **帧级 dedup**（非属性级 coalesce）：flush 时对同一 widgetId + 同一 property 的命令队列：
  - `duration = 0`（即时生效，无动画）→ 只保留最后一条，丢弃前面的
  - `duration > 0`（有动画过渡）→ 不合并，全部保留，按顺序执行
- **不合并跨 Widget 命令**
- **create/addChild/removeChild 不参与 dedup**
- **所有 Widget 操作串行进入**：mpsc channel 天然保证

### 为什么用帧级 dedup 而非属性级 coalesce

旧规则"同 Widget 同属性只保留最后一条"会在以下场景出错：

```typescript
// 开发者意图：先放大到 1.2，再缩小到 1.0（两个独立动画）
icon.setScale(1.2, 200)   // duration > 0，有动画
icon.setScale(1.0, 200)   // duration > 0，有动画
// 旧规则：合并为 setScale(1.0, 200)，丢失放大动画
// 新规则：保留两条，顺序执行两段动画
```

新规则区分"无动画的冗余调用"和"有动画的动画序列"：

```typescript
// 冗余调用：鼠标快速抖动触发了 5 次相同的 hover
icon.setScale(1.2, 0)  // duration = 0，无动画
icon.setScale(1.2, 0)  // 合并
icon.setScale(1.2, 0)  // 合并
// → 最终只执行一次 setScale(1.2, 0)

// 动画序列：每段动画独立保留
icon.setScale(1.2, 200)  // duration > 0
icon.setScale(1.0, 200)  // duration > 0
// → 两段动画顺序执行
```

### IPC 削减效果

无调度层：10 Widget × 120fps × 3 条 = 3,600 条/秒
有调度层：10 Widget × 60fps × 1 batch = 60 条/秒（↓60 倍）

---

## 九、热更新

Widget 业务逻辑（TS）可热更新，Qt 渲染引擎不可热更新。

### 流程

1. 开发者修改 TS 代码
2. Host 检测到文件变更（或手动触发）
3. Host 向 Qt 推送 "widget.recovering"，携带 plugin-id → 仅冻结该插件对应的 Widget
4. 销毁并重建该插件的 V8 Isolate，重新加载该插件的 TS 入口文件
5. TS 侧重新执行 createRect → Host 按逻辑名或调用顺序匹配已有 Widget，不创建新 QWidget
6. Host 重新绑定该插件的事件回调
7. Host 向 Qt 推送 "widget.active"

Qt 进程不重启。其他插件的 Widget 不受影响，窗口不闪烁。

> **注意**：Isolate 重建后所有 JS 运行时状态（变量、闭包、动态 timer）归零。持久状态请用 `BetterCPT.storage`。模块顶层注册的 `setInterval`/`setTimeout` 会随入口文件重新执行而自动恢复。详见第四节 Timer 生命周期。

---

## 十、心跳监控

| Runtime | 心跳方式 | 间隔 | 超时判定 |
|---|---|---|---|
| Widget Qt | IPC pipe alive | 5 秒 | pipe 断开 |
| Widget Deno | Event Loop Watchdog | 1 秒 | 5 秒无响应 |
| Script | tokio task 存活 | — | task panic |
| Utility Window (WebView) | Tauri 进程存活 | 5 秒 | 进程退出 |
| Native | 子进程存活 | 5 秒 | 进程退出 |

### Script + Utility Window 生命周期

Script 插件可以申请创建 Utility Window（见 `WINDOW_SYSTEM.md`），二者的生命周期关系：

- Script 启动时：窗口不自动创建（由插件代码主动调用 `BetterCPT.window.create()`）
- Script 停止时：其拥有的所有 Utility Window 强制关闭
- 用户关闭窗口时：Script 不自动停止（窗口关闭只是关闭界面，插件可以继续后台运行或主动退出）
- Script 崩溃时：窗口关闭，Host 视情况重启 Script（是否重开窗口由插件代码决定）

---


## 十一、各 Runtime 启动顺序

```
Host 启动
  → 启动 Qt 子进程（Widget Runtime 渲染层）
  → 初始化 Deno Core（Widget 业务逻辑层）
  → 加载已安装的 Widget 插件
  → 启动系统托盘
  → 等待用户打开 Dashboard（Tauri WebView，按需启动）
```

关闭时反向：Widget 插件 → Deno → Qt → 托盘 → Host 退出。
