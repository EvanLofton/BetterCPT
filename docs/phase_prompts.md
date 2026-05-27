# BetterCPT 开发会话 Prompt 集合

> 每个 Phase 开启一个新会话，把对应的 Prompt 完整发给 DeepSeek。
> DeepSeek 可以直接读取项目文件，所有文档引用均为文件路径。
> 每次只完成一个任务，完成后告知你，你确认后再继续下一个。
> **每个 Phase 验收通过后必须编写验收报告，追加到 `docs/ACCEPTANCE_REPORT.md`。**

---

## Phase 0 会话 Prompt

```
你是 BetterCPT 项目的开发者。请先读取以下文件，完整理解项目背景后再开始写代码：

【必读文件】
- docs/ARCHITECTURE.md §三（Runtime Invariants，11 条不变量，所有代码必须遵守）
- docs/PROJECT_STRUCTURE.md 第二节（当前 Phase 涉及文件的职责说明）
  重点看：experiments/qt-spike/ 下三个文件 + scripts/measure_memory.ps1

【背景文档】
- docs/WIDGET_RUNTIME.md §二、§三、§四（Qt 集成、动画模型、窗口管理）
- docs/IPC_PROTOCOL.md §二、§三、§八（传输层、消息格式、渲染指令参考）
- docs/RUNTIME_SPEC.md §二 Qt进程崩溃恢复（恢复顺序、超时约束）
- docs/TESTING_STRATEGY.md §一 性能测试（内存测量方法）

【任务清单】
Phase 0 共 7 个任务，按以下顺序逐个完成：

| # | 任务 | 文件 |
|---|---|---|
| 0-1 | Qt Overlay 窗口（无边框/置顶/穿透）+ CMakeLists.txt | experiments/qt-spike/main.cpp + CMakeLists.txt |
| 0-2 | Rust → Qt JSON-RPC IPC 通道 | experiments/qt-spike/RenderServer.cpp |
| 0-3 | Qt 接收 createRect 并渲染 | experiments/qt-spike/WidgetManager.cpp |
| 0-4 | QPropertyAnimation setScale 动画 | experiments/qt-spike/WidgetManager.cpp |
| 0-5 | Rust 侧发送 IPC 命令的最小 Host | host/src/main.rs（临时） |
| 0-6 | Qt 崩溃后 Rust 侧检测并重启 | host/src/main.rs（临时） |
| 0-7 | 内存测量脚本（10 Widget 共享进程） | scripts/measure_memory.ps1 |

【全局约束】
- Phase 0 是 Spike 验证代码，不是生产代码，可以有 hardcode，可以用 unwrap()
- 每次只输出一个任务的文件，完成后告诉我"任务 0-X 完成"，等我确认后再继续
- Widget ID 由 Host 传入，Qt 不自己生成 ID
- JSON 解析使用 Qt 自带的 QJsonDocument，不引入第三方库
- Rust 侧不引入新 crate，只用 std 标准库
- v1 红线：不做声明式框架；不做 virtual tree/diff/hooks/reactive graph；Qt 不持有业务状态
- 验收通过后编写验收报告，追加到 `docs/ACCEPTANCE_REPORT.md`

【验收标准】（全部完成后对照）
- Qt Overlay 窗口点击穿透正常（鼠标在非图标区域可穿透到桌面）
- Rust ↔ Qt IPC 延迟 < 5ms（0-5 的计时代码输出）
- QPropertyAnimation 在 144Hz 显示器上稳定 ≥ 120FPS
- 共享模式 10 Widget 内存 < 60MB（measure_memory.ps1 输出）
- 手动 kill Qt 进程后，Host 在 3 秒内完成重启并恢复渲染

现在开始任务 0-1。
```

> Phase 0 验收报告：`docs/ACCEPTANCE_REPORT.md` § Phase 0

---

## Phase 1 会话 Prompt

```
你是 BetterCPT 项目的开发者。请先读取以下文件，完整理解项目背景后再开始写代码：

【必读文件】
- docs/ARCHITECTURE.md §三（Runtime Invariants，11 条不变量，所有代码必须遵守）
- docs/PROJECT_STRUCTURE.md 第二节（当前 Phase 涉及文件的职责说明）
  重点看：host/src/ 下所有文件的职责说明
- docs/PROJECT_STRUCTURE.md 第三节（Rust mod 树，注意 Phase 1 的 mod.rs 约束）
- docs/PROJECT_STRUCTURE.md 第四节 Phase 1 注意事项（mod.rs 不得超前声明）

【背景文档】
- docs/ARCHITECTURE.md §二（状态所有权、各层职责）
- docs/ARCHITECTURE.md §四（Dashboard 数据流，了解 RuntimeRegistry 的用途）
- docs/IPC_PROTOCOL.md §三、§四（消息格式、错误码）
- docs/RUNTIME_SPEC.md §八（Render Scheduler 调度规则、帧级 dedup）
- docs/BUILD_AND_DEPLOY.md §二（Rust 依赖列表）

【已有代码参考】
- experiments/qt-spike/（Phase 0 完成的 Spike 代码，了解 IPC pipe 的实际用法）

【任务清单】
Phase 1 共 11 个任务，按以下顺序逐个完成：

| # | 任务 | 文件 | 前置 |
|---|---|---|---|
| 1-1 | 日志系统初始化 | host/src/host/logger.rs | — |
| 1-2 | 单例锁（Windows 命名 Mutex） | host/src/host/singleton.rs | — |
| 1-3 | AppContext 结构体和初始化 | host/src/host/context.rs | — |
| 1-4 | 系统托盘（图标 + 右键菜单） | host/src/host/tray.rs | 1-3 |
| 1-5 | 退出信号监听（托盘退出 + Ctrl-C） | host/src/host/shutdown.rs | 1-3, 1-4 |
| 1-6 | IPC 消息类型定义 | host/src/ipc/messages.rs | — |
| 1-7 | IPC 错误码定义 | host/src/ipc/error_codes.rs | — |
| 1-8 | StateTree 数据结构 | host/src/scheduler/state_tree.rs | — |
| 1-9 | RuntimeRegistry 数据结构（Phase 1 部分） | host/src/runtime/registry.rs | — |
| 1-10 | Render Scheduler（帧级 dedup + batch） | host/src/scheduler/mod.rs | 1-6, 1-7, 1-8 |
| 1-11 | main.rs 串联所有模块 | host/src/main.rs | 1-1 ~ 1-10 |

【全局约束】
- 不能用 unwrap() 或 expect()，错误必须返回 Result<T, anyhow::Error>
- 不能引入 Cargo.toml 之外的新 crate
- 每次只输出一个任务的文件，完成后告诉我"任务 1-X 完成"，等我确认后再继续
- mod.rs 约束：host/mod.rs 只声明 logger/singleton/context/tray/shutdown 五个 pub mod；runtime/mod.rs 只声明 registry 一个 pub mod（其余等 Phase 2 追加）
- 错误处理后必须有日志（tracing::error! 或 tracing::warn!）
- 状态只在 Host 修改，跨模块通信只通过 channel
- v1 红线：不做声明式框架；不做 virtual tree/diff/hooks/reactive graph
- 验收通过后编写验收报告，追加到 `docs/ACCEPTANCE_REPORT.md`

【验收标准】（全部完成后对照）
- cargo build 通过，无 warning
- 运行 bettercpt-host.exe，任务栏托盘出现图标
- 右键托盘图标，菜单出现"退出"选项，点击后进程正常关闭
- logs/ 目录下有日志文件生成，再次运行时日志追加而不是覆盖
- Scheduler 单元测试通过（创建 channel、发送命令、flush 返回 batch）
- 前序 Phase 单元测试全部通过

现在开始任务 1-1。
```

> Phase 1 验收报告：`docs/ACCEPTANCE_REPORT.md` § Phase 1

---

## Phase 2 会话 Prompt

```
你是 BetterCPT 项目的开发者。请先读取以下文件，完整理解项目背景后再开始写代码：

【必读文件】
- docs/ARCHITECTURE.md §三（Runtime Invariants，11 条不变量，所有代码必须遵守）
- docs/PROJECT_STRUCTURE.md 第二节（当前 Phase 涉及文件的职责说明）
  重点看：host/src/runtime/ 下所有文件 + host/src/host/window_service.rs
- docs/PROJECT_STRUCTURE.md 第四节 Phase 2（新增文件列表）

【背景文档】
- docs/SECURITY_MODEL.md §二（Capability/Permission 模型、四层架构、检查流程）
- docs/RUNTIME_SPEC.md §一（Runtime 生命周期状态机）
- docs/RUNTIME_SPEC.md §二（Isolate 崩溃恢复顺序）
- docs/RUNTIME_SPEC.md §三（Isolate Watchdog 三层保护）
- docs/RUNTIME_SPEC.md §六（Plugin Crash Report 格式）
- docs/RUNTIME_SPEC.md §七（Plugin Storage Quota）
- docs/ARCHITECTURE.md §四（Dashboard 数据流、RuntimeRegistry 事件类型）
- docs/WINDOW_SYSTEM.md §三（Window Service API）
- docs/SDK_SPEC.md §二（storage/dialog/notification API）
- docs/SDK_SPEC.md §五（Widget SDK 与 Script Runtime 的区别）

【已有代码】
Phase 1 已完成，可直接读取以下文件了解已有实现：
- host/src/runtime/registry.rs（Phase 1 部分：register/unregister/list/snapshot）
- host/src/scheduler/mod.rs
- host/src/ipc/messages.rs
- host/src/ipc/error_codes.rs

【任务清单】
Phase 2 共 17 个任务，按以下顺序逐个完成：

| # | 任务 | 文件 | 前置 |
|---|---|---|---|
| 2-1 | manifest.json 解析、插件 ID 格式校验和权限校验 | host/src/runtime/manifest.rs | — |
| 2-2 | Capability/Permission 上限规则表 | host/src/runtime/permissions.rs | 2-1 |
| 2-3 | Capability Registry | host/src/runtime/capability.rs | 2-2 |
| 2-4 | Runtime Snapshot API | host/src/runtime/registry.rs | 1-9 |
| 2-5 | Deno Core 嵌入（JsRuntime 初始化） | host/src/runtime/deno_host.rs | — |
| 2-6 | per-plugin Isolate 池 | host/src/runtime/isolate_pool.rs | 2-5 |
| 2-7 | Isolate Watchdog（三层保护） | host/src/runtime/watchdog.rs | 2-6 |
| 2-8 | 插件安装/加载流程 | host/src/runtime/plugin_loader.rs | 2-1, 2-3, 2-6 |
| 2-9 | 插件崩溃恢复（Isolate 级） | host/src/runtime/recovery.rs | 2-6, 2-7 |
| 2-10 | Crash Report 写入 | host/src/runtime/crash_report.rs | 2-9 |
| 2-11 | Plugin KV Storage | host/src/runtime/storage.rs | 2-8 |
| 2-12 | Storage Quota 检查 | host/src/runtime/storage.rs | 2-11 |
| 2-13 | Window Service API（create/close） | host/src/host/window_service.rs | 2-3 |
| 2-14 | Window Service onResize/setMinSize | host/src/host/window_service.rs | 2-13 |
| 2-15 | 窗口位置记忆（持久化） | host/src/host/window_service.rs | 2-11, 2-13 |
| 2-16 | 验收插件：text-line-counter | plugins/text-line-counter/index.ts + manifest.json | 2-8 |
| 2-17 | 验收插件：pdf-toolkit（Utility Window） | plugins/pdf-toolkit/index.ts + manifest.json | 2-13 |

【全局约束】
- 不能用 unwrap() 或 expect()，错误必须返回 Result<T, anyhow::Error>
- 不能引入 Cargo.toml 之外的新 crate
- 每次只输出一个任务的文件，完成后告诉我"任务 2-X 完成"，等我确认后再继续
- 错误处理后必须有日志（tracing::error! 或 tracing::warn!）
- 状态只在 Host 修改，跨模块通信只通过 channel
- Widget ID 由 Host 统一分配，不在 Qt 或 TS 侧生成
- 插件 ID 必须符合 @author/plugin-name 格式（全局约束，manifest 解析时校验，不符合格式拒绝安装）
- 验收插件 ID 分别为 @local/text-line-counter 和 @local/pdf-toolkit
- v1 红线：不做声明式框架；不做 createButton/createInput；不做插件间通信
- 验收通过后编写验收报告，追加到 `docs/ACCEPTANCE_REPORT.md`

【验收标准】（全部完成后对照）
- 安装 text-line-counter 插件，Host 识别并加载，触发运行返回文件行数
- 插件代码 throw new Error()，Host 检测到崩溃并自动重启，连续 5 次后停止
- logs/crash/ 下生成 crash JSON 文件
- storage.set() 数据持久化，写入超 1MB 抛出 STORAGE_QUOTA_EXCEEDED
- pdf-toolkit 调用 window.create()，Utility Window 弹出
- 调用 Snapshot API 可看到所有插件的状态、内存、崩溃次数
- cargo test 全部通过（含 Phase 1 单元测试）

现在开始任务 2-1。
```

> Phase 2 验收报告：`docs/ACCEPTANCE_REPORT.md` § Phase 2

---

## Phase 3 会话 Prompt

```
你是 BetterCPT 项目的开发者。请先读取以下文件，完整理解项目背景后再开始写代码：

【必读文件】
- docs/ARCHITECTURE.md §三（Runtime Invariants，11 条不变量，所有代码必须遵守）
- docs/PROJECT_STRUCTURE.md 第二节（当前 Phase 涉及文件的职责说明）
  重点看：host/src/runtime/widget_qt.rs + widget-qt/ 下所有文件 + sdk/src/ 下所有文件
- docs/PROJECT_STRUCTURE.md 第四节 Phase 3（新增文件列表）

【背景文档】
- docs/WIDGET_RUNTIME.md（全文：渲染管线、Qt 集成、动画模型、窗口管理、事件路由）
- docs/IPC_PROTOCOL.md §三、§六、§八（消息格式、Replay 分页、渲染指令参考）
- docs/RUNTIME_SPEC.md §二（Qt 进程崩溃恢复顺序）
- docs/RUNTIME_SPEC.md §八（Render Scheduler 调度规则）
- docs/RUNTIME_SPEC.md §九（热更新流程）
- docs/SDK_SPEC.md §二（Widget SDK 全部 API）
- docs/SDK_SPEC.md §三（线程模型）
- docs/SDK_SPEC.md §四（热更新与 Widget 匹配）
- docs/SDK_SPEC.md §七（红线，Phase 1 绝对不做的事）
- docs/SDK_SPEC.md §八（完整 Dock 插件示例）

【已有代码】
Phase 0 Spike 参考实现（了解 Qt IPC 实现思路，不要直接复制）：
- experiments/qt-spike/RenderServer.cpp
- experiments/qt-spike/WidgetManager.cpp

Phase 1/2 已完成，可直接读取：
- host/src/scheduler/mod.rs（Render Scheduler，Phase 3 所有渲染命令经此串行化）
- host/src/scheduler/state_tree.rs
- host/src/runtime/isolate_pool.rs

【任务清单】
Phase 3 共 19 个任务，按以下顺序逐个完成：

| # | 任务 | 文件 | 前置 |
|---|---|---|---|
| 3-1 | Qt 子进程启动和 pipe 建立 | host/src/runtime/widget_qt.rs | 1-10 |
| 3-2 | Qt 侧 JSON-RPC 消息解析器 | widget-qt/CMakeLists.txt + widget-qt/main.cpp + widget-qt/RenderServer.h + widget-qt/RenderServer.cpp | 3-1 |
| 3-3 | Qt 侧 createRect/createIcon/createText | widget-qt/WidgetManager.h + widget-qt/WidgetManager.cpp | 3-2 |
| 3-4 | Qt 侧 setScale/setOpacity（动画） | widget-qt/WidgetManager.cpp | 3-3 |
| 3-5 | Qt 侧 addChild/removeChild | widget-qt/WidgetManager.cpp | 3-3 |
| 3-6 | Qt 侧 hover/click/mouseDown 事件回传 | widget-qt/WidgetManager.cpp | 3-3 |
| 3-7 | Qt 侧 Z-Order 管理（raise/lower） | widget-qt/WidgetManager.cpp | 3-3 |
| 3-8 | Qt 侧点击穿透（setMask） | widget-qt/WidgetManager.cpp | 3-3 |
| 3-9 | Qt 侧多显示器 DPI + 热插拔 | widget-qt/WidgetManager.cpp | 3-3 |
| 3-10 | Qt 崩溃恢复（Host 侧检测 + 重启） | host/src/runtime/widget_qt.rs | 3-1 |
| 3-11 | Qt 崩溃恢复（状态树 Replay） | host/src/runtime/widget_qt.rs | 3-10, 1-8, 1-10 |
| 3-12 | Replay 分页发送 | host/src/runtime/widget_qt.rs | 3-11 |
| 3-13 | Widget SDK：createRect/createIcon/createText | sdk/package.json + sdk/src/index.ts + sdk/src/widgets.ts | 1-10 |
| 3-14 | Widget SDK：setScale/setOpacity/setBlur | sdk/src/widgets.ts | 3-13 |
| 3-15 | Widget SDK：onHover/onLeave/onClick | sdk/src/widgets.ts | 3-13 |
| 3-16 | Widget SDK：onMouseDown/Move/Up | sdk/src/widgets.ts | 3-15 |
| 3-17 | Widget SDK：storage/dialog/notification | sdk/src/system.ts | 3-13 |
| 3-18 | Widget 热更新（Isolate 重建 + 匹配） | host/src/runtime/isolate_pool.rs | 2-6, 3-13 |
| 3-19 | 验收插件：Dock | plugins/dock/index.ts + manifest.json | 3-13 ~ 3-17 |

【全局约束】
- Rust 文件：不能用 unwrap() 或 expect()，错误必须返回 Result<T, anyhow::Error>
- TypeScript 文件：不能用 any 类型
- 不能引入 Cargo.toml / package.json 之外的新依赖
- 每次只输出一个任务的文件，完成后告诉我"任务 3-X 完成"，等我确认后再继续
- Qt 不持有业务状态，只执行 Host 指令；Widget ID 由 Host 传入，Qt 不自己生成
- context.rs 中的 TODO（Scheduler receiver）必须在此 Phase 处理，spawn background task 持有 receiver 并循环调用 flush()
- V8 Isolate 池必须保证 LIFO 销毁顺序（Phase 2 遗留问题，见 Phase 2 验收报告问题 3）。restart() 实现前必须先解决此问题，否则 3-18 无法完成。
  建议方案：保留 HashMap<String, JsRuntimeHost> 做 O(1) 查找，新增 Vec<String> 记录创建顺序。spawn 时 HashMap::insert + creation_order.push，销毁时按 creation_order 逆序执行，保证 LIFO 销毁顺序。
  （备选方案 IndexMap / LinkedHashMap 可替代 HashMap + Vec，但需引入新依赖，v1 不考虑）
- 所有渲染命令必须经过 Scheduler 串行化后发送，不能绕过
- 验收插件 ID 为 @bettercpt/dock，插件包格式为 .bcpkg（zip 压缩包）
- v1 红线：不做声明式框架；不做 virtual tree/diff/hooks/reactive graph；不做 createButton/createInput
- 验收通过后编写验收报告，追加到 `docs/ACCEPTANCE_REPORT.md`

【验收标准】（全部完成后对照）
- Dock 出现在屏幕底部，毛玻璃背景正常
- 鼠标 Hover 图标，放大动画流畅（PresentMon ≥ 120FPS）
- 点击图标，对应应用启动
- 鼠标在 Dock 非图标区域可点穿到桌面
- Task Manager 内存 < 25MB（Private Working Set）
- 手动 kill bettercpt-widget-qt.exe，3 秒内 Dock 自动恢复
- 修改 dock/index.ts 触发热更新，Dock 不闪烁更新
- text-line-counter 和 pdf-toolkit 插件恢复完整 TS 代码（Phase 2 简化为占位代码，Phase 3 SDK ops 已就绪）
- cargo test 全部通过（含 Phase 1 + Phase 2 单元测试）

现在开始任务 3-1。
```

> Phase 3 验收报告：`docs/ACCEPTANCE_REPORT.md` § Phase 3

---

## Phase 4 会话 Prompt

```
【启动前检查】
开始 Phase 4 前，以下两项必须已完成并附在本 Prompt 末尾：
1. Dashboard 功能清单（有哪些页面、每个页面做什么）
2. Dashboard UI 设计稿（视觉风格、布局、交互）

如果本 Prompt 末尾没有上述内容，请停止并提醒用户补充后再继续。

你是 BetterCPT 项目的开发者。请先读取以下文件，完整理解项目背景后再开始写代码：

【必读文件】
- docs/ARCHITECTURE.md §三（Runtime Invariants，11 条不变量，所有代码必须遵守）
- docs/PROJECT_STRUCTURE.md 第二节（当前 Phase 涉及文件的职责说明）
  重点看：dashboard/ 下所有文件
- docs/PROJECT_STRUCTURE.md 第四节 Phase 4（新增文件列表）

【背景文档】
- docs/ARCHITECTURE.md §四（Dashboard 数据流、事件类型、断连恢复）
- docs/RUNTIME_SPEC.md §六（Plugin Crash Report 格式）
- docs/BUILD_AND_DEPLOY.md §二（Dashboard 依赖：Tauri v2 + React + Vite + Tailwind）

【已有代码】
Phase 2 已完成，可直接读取：
- host/src/runtime/registry.rs（get_runtime_snapshot() 已实现）
- host/src/host/tray.rs（Win32 托盘实现，Phase 4 需改用 Tauri tray plugin 替代）
- docs/DASHBOARD_SPEC.md（功能清单 + UI 设计稿）
- docs/preview/dashboard_merged.html（完整可交互 UI 预览）

【Phase 3 遗留项（Phase 4 开始前已全部解决）】✅
以下问题已在 Phase 4 前置工作中修复，详细报告见 `docs/preview/PHASE4_PREREQ_REPORT.md`：

| # | 问题 | 修复方案 | 状态 |
|---|------|---------|------|
| L1 | `BufReader<Box<dyn Read>>` pipe bug | 根因是缺 JSON-RPC "id"，非 BufReader bug。`send_request` 补自增 id | ✅ |
| L2 | `hideFromTaskbar` Qt crash | `QTimer::singleShot(0)` 延迟 + `IsWindow()` 校验 | ✅ |
| L3 | 任务栏图标 | 同 L2 | ✅ |
| L4 | Dock 插件验收 | SDK→Deno→Scheduler→Qt 管线已通，Dock 7 widget 正常渲染 | ✅ |
| L5 | 内存 < 25MB | Qt6 + V8 刚性基准 ~35MB，Dock 增量 ~21MB。目标改为"插件增量 < 25MB"（已同步 ROADMAP.md） | ✅ |
| L6 | 热更新验收 | mtime 文件检测 + isolate 重建 + 代码重新执行 | ✅ |
| L7 | 插件 TS 代码恢复 | TS 版保留，新增 JS 版适配 deno_core | ✅ |
| L8 | resolve_builtin_plugin_dir | 生产→开发→CWD 三路径优先级 | ✅ |
| L9 | check_engine_version | 完整 semver 解析 + `>=X.Y.Z` 比较 + `env!("CARGO_PKG_VERSION")` | ✅ |

【Phase 4 期间遗留项（Dashboard 完成后处理）】
以下问题在 Phase 4 前置工作中暴露，不影响 Dashboard 开发，但需在 Phase 4 验收前或 Phase 5 解决：

| # | 问题 | 说明 | 涉及文件 |
|---|------|------|----------|
| L10 | StateTree 未接入管线 | 崩溃恢复已接入主循环（detect_crash → restart_qt），但 Qt 重启后无法恢复已有 widget——StateTree 未在 SDK 命令执行时同步填充。完整崩溃恢复需等 StateTree 集成 | host/src/scheduler/state_tree.rs, host/src/main.rs |
| L11 | Naive IPC（逐条 send→read） | 13 条指令 = 13 次 pipe 往返，单帧 > 20 条时不可用。Batch IPC 方案已记录在 IPC_PROTOCOL.md §九 | host/src/main.rs |
| L12 | 插件只支持纯 JS | TypeScript 需手动转译。esbuild/SWC 方案已记录在 FUTURE_CAPABILITIES.md 5.3 | plugins/*/ |

【任务清单】
Phase 4 共 7 个任务，按以下顺序逐个完成。

> **架构决策（2026-05-19）**：Dashboard 不是独立 Tauri 项目，而是 Host 进程内的 Tauri WebView 窗口。
> 不存在 `dashboard/src-tauri/` —— Rust 后端在 `host/src/` 中，Tauri 直接嵌入 Host。
> `dashboard/` 仅为纯 React 前端（Vite + Tailwind）。

| # | 任务 | 文件 | 前置 |
|---|---|---|---|
| 4-1 | Host 集成 Tauri + get_runtime_snapshot 命令 | host/Cargo.toml + host/src/main.rs + host/src/commands.rs + dashboard/ (npm create vite) | 2-4 |
| 4-2 | Tauri runtime_event 推送 + listen | host/src/commands.rs + dashboard/src/App.tsx | 2-4 |
| 4-3 | React：插件列表页 | dashboard/src/pages/Plugins.tsx | 4-1 |
| 4-4 | React：插件安装（拖拽 .bcpkg 文件） | dashboard/src/pages/Plugins.tsx | 4-3 |
| 4-5 | React：插件启动/停止/卸载操作 | dashboard/src/pages/Plugins.tsx | 4-3 |
| 4-6 | React：崩溃详情查看 | dashboard/src/pages/CrashLog.tsx | 2-10 |
| 4-7 | React：断连恢复（onDisconnect 遮罩） | dashboard/src/App.tsx | 4-3 |

【全局约束】
- TypeScript/React：不能用 any 类型
- 不能引入 package.json 之外的新依赖
- 每次只输出一个任务的文件，完成后告诉我"任务 4-X 完成"，等我确认后再继续
- Dashboard 不持有运行时状态，所有状态从 Host 订阅获取
- commands.rs 新增的 command 必须同步在 main.rs 的 invoke_handler 里注册
- v1 红线：不做声明式框架；不做多窗口管理；不做窗口状态持久化
- 验收通过后编写验收报告，追加到 `docs/ACCEPTANCE_REPORT.md`
- Phase 3 遗留项 L1~L9 已全部解决（2026-05-19）✅
- Phase 4 期间遗留项 L10~L12 在 Dashboard 完成后处理

【验收标准】（全部完成后对照）
- Dashboard 从托盘菜单打开
- 插件列表显示所有已安装插件的状态、内存用量
- 安装新插件（拖拽 .bcpkg 文件），列表更新
- 启动/停止/卸载插件全流程正常
- 触发崩溃后，崩溃次数在 Dashboard 实时更新
- 关闭并重新打开 Dashboard，数据恢复正确
- cargo test 全部通过（含 Phase 1 + Phase 2 + Phase 3 单元测试）
- Phase 3 遗留项 L1~L9 全部解决 ✅
- Phase 4 期间遗留项 L10~L12 有方案有记录，不阻塞 Phase 4 验收

现在开始任务 4-1。
```

> Phase 4 验收报告：`docs/ACCEPTANCE_REPORT.md` § Phase 4

---

## 会话中断恢复说明

如果一个 Phase 会话中途中断（上下文过长或意外退出），开新会话时在对应 Phase Prompt 末尾追加：

```
【已完成任务】
以下任务已完成，代码已保存到文件，请直接读取对应文件了解已有实现，不要重新生成：
- 任务 X-1：已完成（读取 <文件路径>）
- 任务 X-2：已完成（读取 <文件路径>）
...

现在从任务 X-N 继续。
```
