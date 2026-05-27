 # BetterCPT 任务指挥手册

> 本文档是项目负责人（你）指挥 AI 写代码的操作手册。
>
> 核心原则：**不让 AI 自由发挥，让 AI 填空。** 任务越小越好，约束越明确越好。

---

## 一、基本工作流

每次和 AI 交互的标准流程：

```
1. 从本文档找到当前 Phase 对应的任务清单
2. 按顺序取出下一个未完成的任务
3. 用任务模板组装 Prompt，发给 AI
4. 用验收标准检查 AI 的输出
5. 通过 → 标记任务完成，取下一个
   不通过 → 把具体问题反馈给 AI，要求修改（不要重新生成整个任务）
```

**每次只做一个任务。** 不要把多个任务合并成一个 Prompt，AI 会在某个地方偷懒。

---

## 二、任务 Prompt 模板

每次给 AI 的指令必须包含以下六个部分，缺一不可：

```
【必读】
以下两项每次必须贴入，不可省略：
1. ARCHITECTURE.md §三 全文（Runtime Invariants，11 条不变量）
2. PROJECT_STRUCTURE.md 第二节中当前任务文件对应的职责说明（只贴那一行，不是整节）

【背景】
贴入与本任务直接相关的文档片段。
不要贴整份文档——只贴这个任务需要的那几节。
具体看第六节参考文档速查表。
通常 1-3 个文档片段就够。

【任务】
用一句话说清楚要实现什么。
格式：实现 <模块路径> 中的 <函数/结构体名>

【已有签名】
贴入框架文件里已经定义好的函数签名、结构体定义、TODO 注释。
AI 只需要填 TODO 的函数体，不能改签名，不能增删参数。

【已有代码】
贴入与本任务直接相关的、前序 Phase 已完成的函数实现。
只贴被本任务调用或依赖的部分，不要贴整个文件。
早期 Phase（Phase 0/1）此块留空；Phase 3/4 开始变得重要。

【约束】
列出这个任务的硬性限制，例如：
- 不能用 unwrap()，错误必须返回 Result<T, anyhow::Error>
- 不能引入新的 crate，只用 Cargo.toml 里已有的依赖
- 只输出这一个文件，不要修改其他文件
- 不要写 main 函数或测试，只写被要求的函数
- v1 红线：<从 SDK_SPEC.md §七 摘抄与本任务直接相关的禁止项，不限数量，全部列出>
```

### 框架文件生成方式

每个 Phase 开始前，从 `PROJECT_STRUCTURE.md` **第四节"按 Phase 的新增文件列表"**取当前 Phase 对应的文件清单，让 AI 生成这些文件的骨架（只含 struct 定义和函数签名 + `// TODO`）。骨架生成后人工跑一次 `cargo build` 验证能与前序 Phase 已完成代码一起编译通过。前序 Phase 的已完成文件不再重新生成骨架。

> 按 Phase 逐批生成是有意的风险控制——全量一次性生成所有骨架，跨文件类型引用容易不一致，发现时已填充多个任务，改动牵连面广。

**mod.rs 约束**：`host/mod.rs` 和 `runtime/mod.rs` 的骨架只声明当前 Phase 已有文件对应的 `pub mod`，不得超前声明。具体限制见 `PROJECT_STRUCTURE.md` 第四节 Phase 1 注意事项。

---

## 三、验收标准（通用）

每个任务完成后，用以下清单检查 AI 的输出。**全部通过才算完成。**

### 3.1 代码质量

- [ ] 没有 `unwrap()` 或 `expect()`（Rust）
- [ ] 没有 `any` 类型（TypeScript）
- [ ] 函数签名和框架文件里的完全一致（没有悄悄改参数）
- [ ] 没有引入框架文件 `Cargo.toml` / `package.json` 之外的新依赖
- [ ] 没有 hardcode 的路径字符串（路径必须从配置或环境变量读取）

### 3.2 架构合规

- [ ] 状态只在 Host 修改（Qt 不写状态，Deno 不写最终 UI 状态）
- [ ] 跨模块通信只通过 channel，没有直接调用其他模块的内部函数
- [ ] Widget ID 只由 Host 生成，没有在 Qt 或 TS 侧生成 ID 的代码
- [ ] 错误处理后有日志（`tracing::error!` 或 `tracing::warn!`）

### 3.3 输出格式

- [ ] 只输出被要求的文件，没有额外生成其他文件
- [ ] 代码可以直接复制粘贴，不包含"你需要把 X 替换成 Y"这类说明
- [ ] 有必要的注释（复杂逻辑为什么这样写）

---

## 四、Phase 任务清单

> 状态标记：⬜ 未开始 / 🔄 进行中 / ✅ 完成 / ⛔ 被阻塞

---

### Phase 0：Qt Render Spike

**目标**：验证技术可行性，不是生产代码。可以有 unwrap，可以硬编码，跑通就行。
**验收**：ROADMAP.md Phase 0 验收标准全部通过。
**注意**：Phase 0 通过后，这里的代码不直接进入主分支，作为参考实现。Spike 代码存入 `experiments/qt-spike/` 目录（主分支内存放，作为后续实现的对照参考）。

| # | 任务 | 文件 | 状态 | 背景文档 |
|---|---|---|---|---|
| 0-1 | Qt Overlay 窗口（无边框/置顶/穿透） | `experiments/qt-spike/main.cpp` | ⬜ | WIDGET_RUNTIME.md §四 |
| 0-2 | Rust → Qt JSON-RPC IPC 通道 | `experiments/qt-spike/RenderServer.cpp` | ⬜ | IPC_PROTOCOL.md §二、三 |
| 0-3 | Qt 接收 createRect 并渲染 | `experiments/qt-spike/WidgetManager.cpp` | ⬜ | IPC_PROTOCOL.md §八 |
| 0-4 | QPropertyAnimation setScale 动画 | `experiments/qt-spike/WidgetManager.cpp` | ⬜ | WIDGET_RUNTIME.md §三 |
| 0-5 | Rust 侧发送 IPC 命令的最小 Host | `host/src/main.rs`（临时） | ⬜ | IPC_PROTOCOL.md §三 |
| 0-6 | Qt 崩溃后 Rust 侧检测并重启 | `host/src/main.rs`（临时） | ⬜ | RUNTIME_SPEC.md §二 Qt崩溃恢复 |
| 0-7 | 内存测量脚本（10 Widget 共享进程） | `scripts/measure_memory.ps1` | ⬜ | TESTING_STRATEGY.md §一 性能测试 |

**Phase 0 验收检查表**（手动逐项测试）：

- [ ] Qt Overlay 窗口点击穿透正常（鼠标在非图标区域可穿透到桌面）
- [ ] Rust ↔ Qt IPC 延迟 < 5ms（本机，用任务 0-5 的计时代码测）
- [ ] QPropertyAnimation 在 144Hz 显示器上稳定 ≥ 120FPS（PresentMon）
- [ ] 共享模式 10 Widget 内存 < 60MB（Task Manager → Private Working Set）
- [ ] 手动 kill Qt 进程后，Host 在 3 秒内完成重启并恢复渲染

---

### Phase 1：Host 核心骨架

**目标**：Host 静默启动、托盘图标、IPC 消息总线、日志、Scheduler 骨架。
**前置**：无。
**验收**：Host 启动 → 托盘图标出现 → 右键退出 → 进程正常关闭。Scheduler 数据结构就绪。

#### 任务顺序

| # | 任务 | 文件 | 前置 | 产出 | 状态 | 背景文档 |
|---|---|---|---|---|---|---|
| 1-1 | 日志系统初始化 | `host/src/host/logger.rs` | — | `init_logger()` | ⬜ | BUILD_AND_DEPLOY.md §二 |
| 1-2 | 单例锁（Windows 命名 Mutex） | `host/src/host/singleton.rs` | — | `acquire_singleton()` → `Result<SingletonGuard>` | ⬜ | ARCHITECTURE.md §三 |
| 1-3 | AppContext 结构体和初始化 | `host/src/host/context.rs` | — | `AppContext { .. }` + `new()` | ⬜ | ARCHITECTURE.md §二 |
| 1-4 | 系统托盘（图标 + 右键菜单） | `host/src/host/tray.rs` | 1-3 | `TrayIcon::new()` + `show_menu()` | ⬜ | — |
| 1-5 | 退出信号监听（托盘退出 + Ctrl-C） | `host/src/host/shutdown.rs` | 1-3, 1-4 | `ShutdownListener::new()` + `wait()` | ⬜ | — |
| 1-6 | IPC 消息类型定义 | `host/src/ipc/messages.rs` | — | `IpcRequest { .. }` / `IpcResponse { .. }` / `IpcNotification { .. }` | ⬜ | IPC_PROTOCOL.md §三、八 |
| 1-7 | IPC 错误码定义 | `host/src/ipc/error_codes.rs` | — | `IpcError` enum（-32700 ~ -32003） | ⬜ | IPC_PROTOCOL.md §四 |
| 1-8 | StateTree 数据结构 | `host/src/scheduler/state_tree.rs` | — | `StateTree` + `WidgetNode` + `insert/update/remove/get` | ⬜ | ARCHITECTURE.md §二 |
| 1-9 | RuntimeRegistry 数据结构 | `host/src/runtime/registry.rs` | — | `RuntimeRegistry` + `register/unregister/list/snapshot` | ⬜ | ARCHITECTURE.md §四 |
| 1-10 | Render Scheduler（帧级 dedup + batch） | `host/src/scheduler/mod.rs` | 1-6, 1-7, 1-8 | `RenderScheduler` + `mpsc::channel` + `flush()` | ⬜ | RUNTIME_SPEC.md §八 |
| 1-11 | main.rs 串联所有模块 | `host/src/main.rs` | 1-1 ~ 1-10 | `main()` — 初始化日志 → 单例锁 → AppContext → 托盘 → Scheduler → 等待退出 | ⬜ | ROADMAP.md Phase 1 |

> **注意**：Scheduler（1-10）是渲染管线的心脏，必须放在 Qt 侧任何工作之前。Phase 3 的所有渲染命令都通过 Scheduler 串行化后发送。

**Phase 1 验收检查表**：

- [ ] `cargo build` 通过，无 warning
- [ ] 运行 `bettercpt-host.exe`，任务栏托盘出现图标
- [ ] 右键托盘图标，菜单出现"退出"选项
- [ ] 点击退出，进程正常关闭（Task Manager 中消失）
- [ ] `logs/` 目录下有日志文件生成
- [ ] 再次运行时，日志追加而不是覆盖
- [ ] Scheduler 结构体可以通过单元测试（创建 channel、发送命令、flush 返回 batch）

---

### Phase 2：Script Runtime + Window Service

**目标**：Deno Core 嵌入、插件完整生命周期、Window Service v1、Snapshot API。
**前置**：Phase 1 全部完成。
**验收**：text-line-counter 插件可运行；pdf-toolkit 插件可弹出 Utility Window；Snapshot API 可查看运行时状态。

#### 任务顺序

| # | 任务 | 文件 | 前置 | 产出 | 状态 | 背景文档 |
|---|---|---|---|---|---|---|
| 2-1 | manifest.json 解析和权限校验 | `host/src/runtime/manifest.rs` | — | `Manifest { runtime, capabilities, permissions, entry, engine, ... }` + `parse()` | ⬜ | SECURITY_MODEL.md §二 |
| 2-2 | Capability/Permission 上限规则表 | `host/src/runtime/permissions.rs` | 2-1 | `CapabilityCeiling` + `PermissionCeiling` + `check()` | ⬜ | SECURITY_MODEL.md §二 |
| 2-3 | Capability Registry | `host/src/runtime/capability.rs` | 2-2 | `CapabilityRegistry` + `register()` + `bind()` | ⬜ | SECURITY_MODEL.md §二 四层架构 |
| 2-4 | Runtime Snapshot API | `host/src/runtime/registry.rs` | 1-9 | `get_runtime_snapshot()` → JSON（插件列表、状态、内存、崩溃次数） | ⬜ | ARCHITECTURE.md §四 |
| 2-5 | Deno Core 嵌入（JsRuntime 初始化） | `host/src/runtime/deno_host.rs` | — | `JsRuntimeHost` + `new()` + `execute()` | ⬜ | RUNTIME_SPEC.md §二 |
| 2-6 | per-plugin Isolate 池 | `host/src/runtime/isolate_pool.rs` | 2-5 | `IsolatePool` + `spawn()` + `kill()` + `restart()` + `list()` | ⬜ | RUNTIME_SPEC.md §二 |
| 2-7 | Isolate Watchdog（三层保护） | `host/src/runtime/watchdog.rs` | 2-6 | `Watchdog` + per-Isolate `start()` / `heartbeat()` / `check()` | ⬜ | RUNTIME_SPEC.md §三 |
| 2-8 | 插件安装/加载流程 | `host/src/runtime/plugin_loader.rs` | 2-1, 2-3, 2-6 | `PluginLoader` + `install()` + `load()` + `uninstall()` | ⬜ | RUNTIME_SPEC.md §一 |
| 2-9 | 插件崩溃恢复（Isolate 级） | `host/src/runtime/recovery.rs` | 2-6, 2-7 | `RecoveryManager` + `on_crash()` + `restart_isolate()` | ⬜ | RUNTIME_SPEC.md §二 |
| 2-10 | Crash Report 写入 | `host/src/runtime/crash_report.rs` | 2-9 | `CrashReport { .. }` + `save()` → `logs/crash/` | ⬜ | RUNTIME_SPEC.md §六 |
| 2-11 | Plugin KV Storage | `host/src/runtime/storage.rs` | 2-8 | `PluginStorage` + `set()` + `get()` + `delete()` | ⬜ | SDK_SPEC.md §二 |
| 2-12 | Storage Quota 检查 | `host/src/runtime/storage.rs` | 2-11 | `StorageQuota` + `check_limit(1MB)` | ⬜ | RUNTIME_SPEC.md §七 |
| 2-13 | Window Service API（create/close） | `host/src/host/window_service.rs` | 2-3 | `WindowService` + `create()` + `close()` | ⬜ | WINDOW_SYSTEM.md §三 |
| 2-14 | Window Service onResize/setMinSize | `host/src/host/window_service.rs` | 2-13 | `on_resize()` + `set_min_size()` | ⬜ | WINDOW_SYSTEM.md §三 |
| 2-15 | 窗口位置记忆（持久化） | `host/src/host/window_service.rs` | 2-11, 2-13 | 自动保存/恢复上次窗口位置到 storage | ⬜ | WINDOW_SYSTEM.md §三 |
| 2-16 | 验收插件：text-line-counter | `plugins/text-line-counter/index.ts` | 2-8 | 插件：读取文件 → 返回行数 | ⬜ | SDK_SPEC.md §五 |
| 2-17 | 验收插件：pdf-toolkit（Utility Window） | `plugins/pdf-toolkit/index.ts` | 2-13 | 插件：`window.create()` → 弹出 WebView 窗口 | ⬜ | WINDOW_SYSTEM.md §三 |

**Phase 2 验收检查表**：

- [ ] 安装 `text-line-counter` 插件（把目录放进 plugins/），Host 识别并加载
- [ ] 触发插件运行，返回文件行数结果
- [ ] 主动停止插件，进程资源释放
- [ ] 插件代码里写一个 `throw new Error()`，Host 检测到崩溃并自动重启
- [ ] 连续崩溃 5 次后 Host 停止重启，推送通知给用户
- [ ] `logs/crash/` 下生成对应的 crash JSON 文件
- [ ] `storage.set("key", "value")` 数据持久化到 `%AppData%/BetterCPT/plugins/<id>/config.json`
- [ ] 写入超过 1MB 时抛出 `STORAGE_QUOTA_EXCEEDED`
- [ ] `pdf-toolkit` 插件调用 `BetterCPT.window.create()`，Utility Window 弹出
- [ ] 调用 Snapshot API 可看到所有插件的状态、内存、崩溃次数
- [ ] `cargo test` 全部通过（含 Phase 1 单元测试）

---

### Phase 3：Widget Runtime (v1 — Qt)

**目标**：Qt 子进程渲染 Dock，Widget SDK 命令式 API 验证通过。
**前置**：Phase 0 验收通过 + Phase 1 全部完成。
**验收**：Dock 常驻底部，毛玻璃背景，Hover 120FPS 动画，内存 < 25MB。

> **核心约束**：Scheduler（1-10）已在 Phase 1 就绪。Qt 侧所有渲染命令（createRect / setScale / addChild 等）由 Host 经 Scheduler 打包后发送，Qt 不做状态决策。

#### 任务顺序

| # | 任务 | 文件 | 前置 | 产出 | 状态 | 背景文档 |
|---|---|---|---|---|---|---|
| 3-1 | Qt 子进程启动和 pipe 建立 | `host/src/runtime/widget_qt.rs` | 1-10 | `WidgetQtRuntime` + `start_qt_process()` + stdin/stdout pipe | ⬜ | RUNTIME_SPEC.md §十一 |
| 3-2 | Qt 侧 JSON-RPC 消息解析器 | `widget-qt/RenderServer.cpp` | 3-1 | `RenderServer` — 读 stdin JSON → 解析 → 分发到 WidgetManager | ⬜ | IPC_PROTOCOL.md §三 |
| 3-3 | Qt 侧 createRect/createIcon/createText | `widget-qt/WidgetManager.cpp` | 3-2 | `WidgetManager` — 创建 QWidget/QLabel/QPixmap | ⬜ | WIDGET_RUNTIME.md §二 |
| 3-4 | Qt 侧 setScale/setOpacity（动画） | `widget-qt/WidgetManager.cpp` | 3-3 | `QPropertyAnimation` 驱动 scale/opacity 动画 | ⬜ | WIDGET_RUNTIME.md §三 |
| 3-5 | Qt 侧 addChild/removeChild | `widget-qt/WidgetManager.cpp` | 3-3 | `setParent()` / `setParent(nullptr)` 层级操作 | ⬜ | IPC_PROTOCOL.md §八 |
| 3-6 | Qt 侧 hover/click/mouseDown 事件回传 | `widget-qt/WidgetManager.cpp` | 3-3 | `mousePressEvent` → stdin JSON 回传 Host | ⬜ | IPC_PROTOCOL.md §八 |
| 3-7 | Qt 侧 Z-Order 管理（raise/lower） | `widget-qt/WidgetManager.cpp` | 3-3 | `raise()` / `lower()` 按 Host 指令调 z-order | ⬜ | WIDGET_RUNTIME.md §四 |
| 3-8 | Qt 侧点击穿透（setMask） | `widget-qt/WidgetManager.cpp` | 3-3 | `setMask(QRegion)` — 非交互区域鼠标穿透 | ⬜ | WIDGET_RUNTIME.md §二 |
| 3-9 | Qt 侧多显示器 DPI + 热插拔 | `widget-qt/WidgetManager.cpp` | 3-3 | `AA_EnableHighDpiScaling` + `QScreen::geometryChanged` | ⬜ | WIDGET_RUNTIME.md §四 |
| 3-10 | Qt 崩溃恢复（Host 侧检测 + 重启） | `host/src/runtime/widget_qt.rs` | 3-1 | `detect_crash()` + `restart_qt()` + 冻结/解冻 IPC | ⬜ | RUNTIME_SPEC.md §二 |
| 3-11 | Qt 崩溃恢复（状态树 Replay） | `host/src/runtime/widget_qt.rs` | 3-10, 1-8, 1-10 | `replay_state_tree()` — 深度优先遍历，逐条 create | ⬜ | IPC_PROTOCOL.md §六 |
| 3-12 | Replay 分页发送 | `host/src/runtime/widget_qt.rs` | 3-11 | `send_replay_pages()` — >1MB 分页，Qt merge buffer | ⬜ | IPC_PROTOCOL.md §六 |
| 3-13 | Widget SDK：createRect/createIcon/createText | `sdk/src/widgets.ts` | 1-10 | `createRect()` / `createIcon()` / `createText()` → 序列化 IPC 命令 | ⬜ | SDK_SPEC.md §二 |
| 3-14 | Widget SDK：setScale/setOpacity/setBlur | `sdk/src/widgets.ts` | 3-13 | `setScale(s, ms)` / `setOpacity(o, ms)` / `setBlur(bool)` | ⬜ | SDK_SPEC.md §二 |
| 3-15 | Widget SDK：onHover/onLeave/onClick | `sdk/src/widgets.ts` | 3-13 | 事件注册 → Host IPC → Qt 回调注册 | ⬜ | SDK_SPEC.md §二 |
| 3-16 | Widget SDK：onMouseDown/Move/Up | `sdk/src/widgets.ts` | 3-15 | 鼠标事件原语，坐标 Widget 自身左上角 | ⬜ | SDK_SPEC.md §二 |
| 3-17 | Widget SDK：storage/dialog/notification | `sdk/src/system.ts` | 3-13 | `BetterCPT.storage` / `BetterCPT.dialog` / `BetterCPT.notification` | ⬜ | SDK_SPEC.md §二 |
| 3-18 | Widget 热更新（Isolate 重建 + 匹配） | `host/src/runtime/isolate_pool.rs` | 2-6, 3-13 | TS 文件变更检测 → 重建 Isolate → 逻辑名/位置匹配 | ⬜ | RUNTIME_SPEC.md §九 |
| 3-19 | 验收插件：Dock | `plugins/dock/index.ts` | 3-13 ~ 3-17 | 完整 Dock：毛玻璃 + 6 图标 + hover/click + 应用启动 | ⬜ | SDK_SPEC.md §八 |

**Phase 3 验收检查表**：

- [ ] Dock 出现在屏幕底部，毛玻璃背景正常
- [ ] 鼠标 Hover 图标，图标放大动画流畅（PresentMon 确认 ≥ 120FPS）
- [ ] 鼠标离开，图标缩小动画流畅
- [ ] 点击图标，对应应用启动
- [ ] 鼠标在 Dock 非图标区域，可以点穿到桌面
- [ ] Task Manager 确认内存 < 25MB（Private Working Set）
- [ ] 手动 kill `bettercpt-widget-qt.exe`，3 秒内 Dock 自动恢复
- [ ] 修改 `dock/index.ts` 代码，触发热更新，Dock 不闪烁更新
- [ ] `cargo test` 全部通过（含 Phase 1 + Phase 2 单元测试）

---

### Phase 4：Dashboard

**目标**：Tauri v2 + React 管理后台，插件完整管理流程。
**前置**：Phase 1 + Phase 3 全部完成。
**验收**：安装 → 启动 → 停止 → 卸载插件全流程。

> **注意**：Snapshot API 已在 Phase 2-4 实现（`get_runtime_snapshot()` → JSON）。Tauri 后端直接调用该函数暴露给前端。

| # | 任务 | 文件 | 前置 | 产出 | 状态 | 背景文档 |
|---|---|---|---|---|---|---|
| 4-1 | Tauri 后端：get_runtime_snapshot | `dashboard/src-tauri/src/commands.rs` | 2-4 | `#[tauri::command] get_runtime_snapshot()` → 调用 Phase 2 API | ⬜ | ARCHITECTURE.md §四 |
| 4-2 | Tauri 后端：runtime_event 推送 | `dashboard/src-tauri/src/commands.rs` | 2-4 | `emit("runtime_event", ...)` — plugin_started/stopped/crashed/recovered | ⬜ | ARCHITECTURE.md §四 |
| 4-3 | React：插件列表页（状态/内存/崩溃次数） | `dashboard/src/pages/Plugins.tsx` | 4-1 | 表格：图标/名称/版本/状态/内存/操作按钮 | ⬜ | ARCHITECTURE.md §四 |
| 4-4 | React：插件安装（拖拽目录） | `dashboard/src/pages/Plugins.tsx` | 4-3 | 拖拽文件夹 → manifest 校验 → 复制 → 刷新列表 | ⬜ | — |
| 4-5 | React：插件启动/停止/卸载操作 | `dashboard/src/pages/Plugins.tsx` | 4-3 | 启动/停止/卸载按钮 → invoke Rust 命令 | ⬜ | — |
| 4-6 | React：崩溃详情查看 | `dashboard/src/pages/CrashLog.tsx` | 2-10 | Crash 列表 + JSON 详情展示 | ⬜ | RUNTIME_SPEC.md §六 |
| 4-7 | React：断连恢复（onDisconnect 遮罩） | `dashboard/src/App.tsx` | 4-3 | 断连 → "正在恢复"遮罩 → 重连 → 重新拉取快照 | ⬜ | ARCHITECTURE.md §四 |

**Phase 4 验收检查表**：

- [ ] Dashboard 从托盘菜单打开
- [ ] 插件列表显示所有已安装插件的状态、内存用量
- [ ] 安装新插件（拖拽文件夹到 Dashboard），列表更新
- [ ] 点击启动插件，状态变为运行中
- [ ] 点击停止插件，状态变为已停止
- [ ] 卸载插件，从列表消失
- [ ] 触发崩溃后，崩溃次数在 Dashboard 实时更新
- [ ] 关闭并重新打开 Dashboard，数据恢复正确（快照机制正常）
- [ ] `cargo test` 全部通过（含 Phase 1 + Phase 2 + Phase 3 单元测试）

---

## 五、常见问题处理

### AI 输出了框架文件以外的函数/文件

**处理方式**：告诉它"只保留被要求的函数，其余全部删掉"，不要自己手动删除，让 AI 重新输出一次，确认它知道边界在哪。

### AI 用 unwrap() 了

**处理方式**：不要接受。直接说"把所有 unwrap() 改成用 ? 或者 match 处理错误，错误类型用 anyhow::Error"。

### AI 改了函数签名

**处理方式**：把框架文件里的原始签名再贴一遍，说"签名必须和这个完全一致，不能改参数名、参数类型、返回值类型"。这是最常见的错误，AI 经常"顺手优化"签名。

### AI 的代码可以编译但行为不对

**处理方式**：用验收检查表逐项检查，找到第一个失败的项，把它描述清楚（预期行为 vs 实际行为），单独作为一个新任务让 AI 修复。不要说"整体不对"，要指出具体哪里不对。

### 一个任务太难，AI 多次失败

**处理方式**：把这个任务拆成更小的两到三个步骤，每步只做一件事。例如把"Qt 崩溃恢复"拆成"检测崩溃"和"重启并 Replay"两个任务分别做。

**编号约定**：拆分后用 `3-10a` / `3-10b` 编号，**替换**任务清单中原来的 3-10 行（不是在后面追加新行），原后续任务编号不变。

### AI 输出了完整文件，不是只填 TODO

**处理方式**：不要直接覆盖现有文件。先 `diff` 对比新旧文件，确认只有 `// TODO` 部分被填充、函数签名和其他已完成函数未被改动，再替换。如果签名或已有函数被改动，让 AI 重新输出。

## 六、参考文档速查

给 AI Prompt 时，按任务类型贴对应文档的对应节：

| 任务类型 | 读哪个文档的哪一节 |
|---|---|
| Rust 数据结构定义 | ARCHITECTURE.md §二 状态所有权 |
| IPC 消息格式 | IPC_PROTOCOL.md §三 消息格式 |
| 渲染指令 | IPC_PROTOCOL.md §八 渲染指令参考 |
| Qt 窗口属性 | WIDGET_RUNTIME.md §二、四 |
| Qt 动画 | WIDGET_RUNTIME.md §三 |
| Scheduler 逻辑 | RUNTIME_SPEC.md §八 |
| 崩溃恢复流程 | RUNTIME_SPEC.md §二 |
| 权限检查 | SECURITY_MODEL.md §二 |
| 插件生命周期状态机 | RUNTIME_SPEC.md §一 |
| TS SDK API 签名 | SDK_SPEC.md §二 |
| manifest 格式 | SDK_SPEC.md §八 |
| 窗口系统 | WINDOW_SYSTEM.md §三 |
| Storage API | SDK_SPEC.md §二 数据持久化 |
