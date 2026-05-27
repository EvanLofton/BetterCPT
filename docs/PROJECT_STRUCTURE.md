# BetterCPT 项目目录结构

> 本文档定义项目的完整文件结构、每个文件的职责边界。
>
> 用途：每个 Phase 骨架生成前，从本文档取对应 Phase 新增文件列表，作为 AI Prompt 的路径约束。
>
> 所有设计决策已定案，见第五节"已决事项"。

---

## 一、完整目录树

```
bettercpt/
├── host/                              # Rust Host 主进程
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                    # 入口：串联所有模块，启动顺序
│       ├── host/                      # Host 核心服务
│       │   ├── mod.rs                 # 模块入口，集中 re-export
│       │   ├── logger.rs              # 日志初始化
│       │   ├── singleton.rs           # Windows 命名 Mutex 单例锁
│       │   ├── context.rs             # AppContext 全局上下文
│       │   ├── tray.rs                # 系统托盘图标和右键菜单
│       │   ├── shutdown.rs            # 退出信号监听（托盘退出 + Ctrl-C）
│       │   ├── window_service.rs      # Window Service：创建/关闭/管理 Utility Window
│       │   └── commands.rs            # Tauri commands：get_runtime_snapshot / emit runtime_event
│       ├── ipc/                       # IPC 协议层
│       │   ├── mod.rs                 # 模块入口，集中 re-export
│       │   ├── messages.rs            # IpcRequest / IpcResponse / IpcNotification 类型定义
│       │   └── error_codes.rs         # IpcError enum（-32700 ~ -32003）
│       ├── runtime/                   # Runtime 管理层
│       │   ├── mod.rs                 # 模块入口，集中 re-export
│       │   ├── registry.rs            # RuntimeRegistry：注册/注销/快照
│       │   ├── manifest.rs            # manifest.json 解析和 engine 版本校验
│       │   ├── permissions.rs         # CapabilityCeiling + PermissionCeiling + check()
│       │   ├── capability.rs          # CapabilityRegistry：capability → 实现绑定
│       │   ├── deno_host.rs           # Deno Core 嵌入：JsRuntime 初始化和执行
│       │   ├── isolate_pool.rs        # per-plugin Isolate 池：spawn/kill/restart/list
│       │   ├── watchdog.rs            # Isolate Watchdog 三层保护（Timeout/Budget/Heartbeat）
│       │   ├── plugin_loader.rs       # 插件安装/加载/卸载流程
│       │   ├── recovery.rs            # 崩溃恢复：on_crash() + restart_isolate()
│       │   ├── crash_report.rs        # Crash Report 写入 logs/crash/
│       │   ├── storage.rs             # Plugin KV Storage + Quota 检查（1MB 上限）
│       │   └── widget_qt.rs           # Qt 子进程启动、pipe 管理、崩溃检测、状态树 Replay
│       └── scheduler/                 # Render Scheduler
│           ├── mod.rs                 # 模块入口 + RenderScheduler 实现（不拆子模块）
│           └── state_tree.rs          # StateTree + WidgetNode：insert/update/remove/get
│
├── widget-qt/                         # Qt C++ 子进程（Widget 渲染）
│   ├── CMakeLists.txt
│   ├── main.cpp                       # QApplication 初始化，ready 信号发送
│   ├── RenderServer.h
│   ├── RenderServer.cpp               # stdin/stdout pipe 读写，JSON-RPC 解析，分发到 WidgetManager
│   ├── WidgetManager.h
│   └── WidgetManager.cpp              # QWidget 创建/销毁，动画，事件回传，z-order，setMask
│
├── sdk/                               # TypeScript SDK（npm 包）
│   ├── package.json
│   └── src/
│       ├── index.ts                   # 统一导出入口（re-export widgets + system）
│       ├── widgets.ts                 # createRect/createIcon/createText/setScale/onHover 等
│       └── system.ts                  # BetterCPT.storage/dialog/notification/system
│
├── dashboard/                         # React 前端（纯前端，Tauri 后端在 host/ 里）
│   └── src/
│       ├── App.tsx                    # 根组件，路由，断连恢复遮罩
│       ├── main.tsx                   # React 入口
│       └── pages/
│           ├── Plugins.tsx            # 插件列表、安装、启动/停止/卸载操作
│           ├── CrashLog.tsx           # Crash 列表和 JSON 详情展示
│           └── Settings.tsx           # 设置页
│
├── plugins/                           # 内置验收插件，随安装包分发；打包格式为 .bcpkg（zip）
│   ├── dock/
│   │   ├── manifest.json              # id: @bettercpt/dock
│   │   └── index.ts                   # 验收插件：Dock
│   ├── text-line-counter/
│   │   ├── manifest.json              # id: @local/text-line-counter
│   │   └── index.ts                   # 验收插件：文件行数统计
│   └── pdf-toolkit/
│       ├── manifest.json              # id: @local/pdf-toolkit
│       └── index.ts                   # 验收插件：PDF 工具（Utility Window）
│
├── experiments/
│   └── qt-spike/                      # Phase 0 Spike 代码，不进入主分支合并，仅作参考实现
│       ├── CMakeLists.txt
│       ├── main.cpp
│       ├── RenderServer.cpp
│       └── WidgetManager.cpp
│
├── scripts/
│   └── measure_memory.ps1             # 10 Widget 内存测量脚本（Phase 0 验收用）
│
└── docs/                              # 项目文档
    ├── PROJECT_OVERVIEW.md
    ├── ARCHITECTURE.md
    ├── RUNTIME_SPEC.md
    ├── IPC_PROTOCOL.md
    ├── WIDGET_RUNTIME.md
    ├── SDK_SPEC.md
    ├── SECURITY_MODEL.md
    ├── BUILD_AND_DEPLOY.md
    ├── WINDOW_SYSTEM.md
    ├── TESTING_STRATEGY.md
    ├── ROADMAP.md
    ├── FUTURE_CAPABILITIES.md
    ├── TASK_GUIDE.md
    └── PROJECT_STRUCTURE.md           # 本文档
```

---

## 二、各文件职责说明

### Rust Host

| 文件 | 负责 | 不负责 |
|---|---|---|
| `main.rs` | 启动顺序串联（日志→单例锁→AppContext→托盘→Scheduler→等待退出），关闭时反向清理 | 任何业务逻辑 |
| `host/logger.rs` | tracing 初始化，日志文件轮转，日志目录创建 | 业务日志内容 |
| `host/singleton.rs` | Windows 命名 Mutex 获取和释放，`SingletonGuard` RAII | 进程间通信 |
| `host/context.rs` | `AppContext` 结构体定义，持有各服务的 Arc 引用，`new()` 初始化 | 具体服务实现 |
| `host/tray.rs` | 托盘图标创建，右键菜单渲染，菜单事件分发 | 退出信号的最终处理 |
| `host/shutdown.rs` | 监听托盘退出事件和 Ctrl-C，统一触发 shutdown channel | 资源释放（各模块自己清理） |
| `host/window_service.rs` | Utility Window 的创建/关闭/resize/minSize，窗口位置记忆持久化 | Overlay Widget 窗口（归 widget_qt.rs） |
| `ipc/messages.rs` | `IpcRequest` / `IpcResponse` / `IpcNotification` 的结构体定义和序列化/反序列化 | 消息路由和处理 |
| `ipc/error_codes.rs` | `IpcError` enum，错误码常量（-32700 ~ -32003），到 JSON 的转换 | 错误恢复逻辑 |
| `runtime/registry.rs` | Phase 1：`RuntimeRegistry` 数据结构 + `register()` / `unregister()` / `list()`；Phase 2 追加：`get_runtime_snapshot()` → JSON（记录状态/内存/崩溃次数/最后心跳） | 插件的具体生命周期操作 |
| `runtime/manifest.rs` | 解析 `manifest.json`，校验 `engine` 版本范围（semver），提取 runtime/capabilities/permissions | 权限上限检查（归 permissions.rs） |
| `runtime/permissions.rs` | `CapabilityCeiling` 和 `PermissionCeiling` 按 runtime 类型的硬编码上限表，`check()` 校验插件声明是否超限 | capability 的运行时绑定 |
| `runtime/capability.rs` | `CapabilityRegistry`：capability/permission → Deno op / SDK API / IPC command 的运行时绑定 | 权限上限规则（归 permissions.rs） |
| `runtime/deno_host.rs` | Deno Core `JsRuntime` 初始化，注入 BetterCPT SDK 对象，`execute()` 执行 TS 入口文件 | Isolate 池管理（归 isolate_pool.rs） |
| `runtime/isolate_pool.rs` | per-plugin V8 Isolate 的 `spawn()` / `kill()` / `restart()` / `list()`，Isolate 与 plugin-id 的映射 | 崩溃检测和恢复决策（归 recovery.rs） |
| `runtime/watchdog.rs` | 三层保护：Execution Timeout（500ms）/ Microtask Budget（10000）/ Event Loop Heartbeat（5s），粒度为单个 Isolate | 崩溃恢复操作（归 recovery.rs） |
| `runtime/plugin_loader.rs` | 插件目录扫描，`install()` / `load()` / `uninstall()` 流程，调用 manifest 解析和权限检查 | Isolate 创建（调用 isolate_pool.rs） |
| `runtime/recovery.rs` | `on_crash()` 检测崩溃，`restart_isolate()` 重建 Isolate，重启次数上限（5次）检查，超限后通知用户 | Crash Report 写入（归 crash_report.rs） |
| `runtime/crash_report.rs` | 构造 `CrashReport` 结构体，序列化为 JSON，写入 `%AppData%/BetterCPT/logs/crash/<plugin-id>-<timestamp>.json` | 崩溃恢复逻辑 |
| `runtime/storage.rs` | Plugin KV Storage 的 `set()` / `get()` / `delete()`，1MB 配额检查，持久化到 `config.json` | 文件系统的直接访问（通过 Host 管理路径） |
| `runtime/widget_qt.rs` | Qt 子进程启动（`std::process::Command`），stdin/stdout pipe 建立，崩溃检测，IPC 冻结/解冻，状态树 Replay（含分页） | Widget 渲染逻辑（归 Qt 侧） |
| `scheduler/mod.rs` | `RenderScheduler`：mpsc channel 接收 SDK 命令，帧级 dedup（duration=0 合并，duration>0 保留），按帧 flush 为 IPC batch | 状态树的读写（调用 state_tree.rs） |
| `scheduler/state_tree.rs` | `StateTree` + `WidgetNode` 数据结构，`insert()` / `update()` / `remove()` / `get()`，版本号递增 | 渲染指令的生成和发送 |

### Qt 子进程

| 文件 | 负责 | 不负责 |
|---|---|---|
| `main.cpp` | `QApplication` 初始化，`AA_EnableHighDpiScaling`，向 Host 发送 ready 信号，`exec()` 启动事件循环 | 业务逻辑 |
| `RenderServer.cpp` | stdin 读取，JSON-RPC 解析，方法名分发到 `WidgetManager`，stdout 写入响应 | Widget 状态管理 |
| `WidgetManager.cpp` | `QWidget` / `QLabel` / `QPixmap` 创建销毁，`QPropertyAnimation` 动画，`setMask()` 穿透，`raise()` / `lower()` z-order，鼠标事件捕获和回传，多显示器 DPI 和热插拔 | 业务决策（只执行 Host 指令） |

### TypeScript SDK

| 文件 | 负责 | 不负责 |
|---|---|---|
| `src/index.ts` | 统一导出入口（re-export widgets + system） | 具体实现 |
| `src/widgets.ts` | `createRect()` / `createIcon()` / `createText()`，`setScale()` / `setOpacity()` / `setBlur()`，`addChild()` / `removeChild()`，`onHover()` / `onLeave()` / `onClick()` / `onMouseDown()` / `onMouseMove()` / `onMouseUp()`，IPC 命令序列化 | 状态管理、渲染后端实现 |
| `src/system.ts` | `BetterCPT.storage`（set/get/delete），`BetterCPT.dialog`（alert/confirm/prompt），`BetterCPT.notification.show()`，`BetterCPT.system.launch()` / `memory()` | Widget 渲染相关 API |

### Dashboard（2026-05-19 更新：Tauri 嵌入 Host，不再有独立 src-tauri/）

| 文件 | 负责 | 不负责 |
|---|---|---|
| `host/src/host/commands.rs` | Tauri commands：`get_runtime_snapshot()`、`emit("runtime_event", ...)`，通过 `State<Arc<RuntimeRegistry>>` 直接读 Host 状态 | 具体页面逻辑 |
| `dashboard/src/App.tsx` | 根组件，路由，断连恢复遮罩 | 具体页面内容 |
| `dashboard/src/pages/Plugins.tsx` | 插件列表（状态/内存/崩溃次数），安装（拖拽 .bcpkg），启动/停止/卸载 | Crash 日志展示 |
| `dashboard/src/pages/CrashLog.tsx` | Crash 文件列表，选中后展示 JSON 详情 | 插件操作 |
| `dashboard/src/pages/Settings.tsx` | 设置页 | 插件管理 |

### 验收插件

| 文件 | 负责 | 说明 |
|---|---|---|
| `plugins/dock/index.ts` | 毛玻璃 Dock，6 个图标，hover/click 动画，应用启动 | Phase 3 验收插件；id: @bettercpt/dock |
| `plugins/text-line-counter/index.ts` | 读取文件，返回行数 | Phase 2 验收插件（仅需 fs:read）；id: @local/text-line-counter |
| `plugins/pdf-toolkit/index.ts` | 调用 `BetterCPT.window.create()`，弹出 Utility Window | Phase 2 验收插件；id: @local/pdf-toolkit |

---

## 三、Rust mod 树

> 以下为所有 Phase 完成后的**最终状态**。各 Phase 骨架生成时按当前 Phase 已有文件逐步追加 `pub mod`，不得超前声明未存在的文件。

`main.rs` 顶层声明：

```rust
mod host;
mod ipc;
mod runtime;
mod scheduler;
```

`host/mod.rs`：

```rust
pub mod logger;
pub mod singleton;
pub mod context;
pub mod tray;
pub mod shutdown;
pub mod window_service;
pub mod commands;
```

`ipc/mod.rs`：

```rust
pub mod messages;
pub mod error_codes;
```

`runtime/mod.rs`：

```rust
pub mod registry;
pub mod manifest;
pub mod permissions;
pub mod capability;
pub mod deno_host;
pub mod isolate_pool;
pub mod watchdog;
pub mod plugin_loader;
pub mod recovery;
pub mod crash_report;
pub mod storage;
pub mod widget_qt;
```

`scheduler/mod.rs` 同时作为模块文件和 `RenderScheduler` 的实现文件（不再拆子模块）：

```rust
// RenderScheduler 实现在此文件
pub mod state_tree;
```

---

## 四、按 Phase 的新增文件列表

> 给 AI 生成骨架时，按当前 Phase 取对应列表，不包含前序 Phase 的文件。

### Phase 0（experiments/qt-spike/）

```
experiments/qt-spike/CMakeLists.txt
experiments/qt-spike/main.cpp
experiments/qt-spike/RenderServer.cpp
experiments/qt-spike/WidgetManager.cpp
scripts/measure_memory.ps1
```

### Phase 1

> **注意**：`host/mod.rs` 骨架只声明 `pub mod logger / singleton / context / tray / shutdown;` 五个（`window_service` 等 Phase 2 追加）。`runtime/mod.rs` 骨架只声明 `pub mod registry;`（其余等 Phase 2 追加）。超前声明未存在的文件会导致 `cargo build` 报错。

```
host/src/main.rs
host/src/host/mod.rs
host/src/host/logger.rs
host/src/host/singleton.rs
host/src/host/context.rs
host/src/host/tray.rs
host/src/host/shutdown.rs
host/src/ipc/mod.rs
host/src/ipc/messages.rs
host/src/ipc/error_codes.rs
host/src/runtime/mod.rs
host/src/runtime/registry.rs
host/src/scheduler/mod.rs
host/src/scheduler/state_tree.rs
```

### Phase 2

```
host/src/runtime/manifest.rs
host/src/runtime/permissions.rs
host/src/runtime/capability.rs
host/src/runtime/deno_host.rs
host/src/runtime/isolate_pool.rs
host/src/runtime/watchdog.rs
host/src/runtime/plugin_loader.rs
host/src/runtime/recovery.rs
host/src/runtime/crash_report.rs
host/src/runtime/storage.rs
host/src/host/window_service.rs
plugins/text-line-counter/manifest.json
plugins/text-line-counter/index.ts
plugins/pdf-toolkit/manifest.json
plugins/pdf-toolkit/index.ts
```

### Phase 3

```
host/src/runtime/widget_qt.rs
widget-qt/CMakeLists.txt
widget-qt/main.cpp
widget-qt/RenderServer.h
widget-qt/RenderServer.cpp
widget-qt/WidgetManager.h
widget-qt/WidgetManager.cpp
sdk/package.json
sdk/src/index.ts
sdk/src/widgets.ts
sdk/src/system.ts
plugins/dock/manifest.json
plugins/dock/index.ts
```

### Phase 4（2026-05-19 更新：Tauri 嵌入 Host，无独立 src-tauri/）

```
host/src/host/commands.rs
dashboard/src/main.tsx
dashboard/src/App.tsx
dashboard/src/pages/Plugins.tsx
dashboard/src/pages/CrashLog.tsx
dashboard/src/pages/Settings.tsx
```

---

## 五、已决事项

> 以下问题已定案，对应目录树已更新。

### #1：Rust 模块风格 → `mod.rs` 风格

`host/`、`ipc/`、`runtime/` 均使用 `mod.rs` 作为模块入口。

理由：`runtime/` 子模块 12 个，`runtime/mod.rs` 可集中 re-export，调用方只写 `use crate::runtime::PluginLoader` 而非 `use crate::runtime::plugin_loader::PluginLoader`，代码整洁且不暴露内部文件结构。

### #2：SDK 入口文件 → `index.ts` 统一导出

```typescript
// sdk/src/index.ts
export { createRect, createIcon, createText } from "./widgets"
export { storage, dialog, notification, system } from "./system"
```

插件开发者只写 `import { createRect } from "bettercpt-sdk"`。

**实现注意**：`sdk/package.json` 必须声明 `"main"` 和 `"types"` 字段，否则运行时 import 失败：
```json
{ "name": "bettercpt-sdk", "main": "dist/index.js", "types": "dist/index.d.ts" }
```
Phase 3 骨架生成的 Prompt【约束】中加一条：`package.json 必须包含 main 和 types 字段`。

### #3：plugins/ 目录定位 → 两个目录，独立加载

| 位置 | 放什么 |
|---|---|
| `bettercpt/plugins/`（项目目录） | 内置验收插件，随安装包分发 |
| `%AppData%/BetterCPT/plugins/` | 用户安装的第三方插件 |

安装流程：Dashboard 拖拽 → Host 复制到 `%AppData%/BetterCPT/plugins/<id>/`。

加载规则：两个目录独立加载，**不做 fallback**。同名插件在日志中记录 `warn`，v1 不做冲突 UI。

### #4：widget-qt/ 文件拆分 → v1 单文件，按职责方向拆

v1 保持单文件 `WidgetManager.cpp`。

拆分时机：跑完 Dock 验收后，按职责方向拆分——优先将"主动上报"部分（鼠标事件回传、显示器热插拔通知）拆为 `EventRouter.cpp`。该部分不依赖 QWidget 创建状态，独立性最强，拆分后职责边界清晰。其余"接收指令执行"部分（createRect / setScale / addChild 等）保留在 `WidgetManager.cpp`。

### #5：插件包格式 → .bcpkg

插件以 `.bcpkg` 文件分发，本质是 zip 压缩包，改扩展名。

内部结构：
```
my-plugin.bcpkg (zip)
├── manifest.json
├── index.ts / index.js
├── assets/
└── node_modules/
```

v1 本地加载时解压到 `%AppData%/BetterCPT/plugins/<plugin-id>/`。v2 插件市场下载的 `.bcpkg` 使用相同格式，安装流程完全复用。

Dashboard 安装入口：拖拽 `.bcpkg` 文件（非目录）。

### #6：插件 ID 规范 → @author/plugin-name

格式：`@author/plugin-name`，全小写，只允许字母、数字、连字符。

- `@bettercpt` 命名空间保留给官方插件
- 本地开发使用 `@local/plugin-name`，不与市场 ID 冲突
- manifest 解析时校验格式，不符合格式拒绝安装

---

## 六、DeepSeek 处理结果

本节供 Claude 复核——以下列出了对原始文档的每项改动及依据。

### 删除内容

- 原第六节"DeepSeek 分析结果"（#1~#4 建议）——已采纳并融入第五节决策，原文不再保留
- 原第七节"Claude 分析结果"（#1~#4 补充意见）——同上

### 第五节改写：待定事项 → 已决事项

| 项目 | 决策 | 关键依据 |
|---|---|---|
| #1 Rust 模块风格 | `mod.rs` 风格 | `runtime/` 12 个子模块，`mod.rs` 集中 re-export 可避免调用方写全路径（Claude 的实质理由被采纳） |
| #2 SDK 入口 | `index.ts` 统一导出 | npm 包标准做法；`package.json` 必须含 `main` 和 `types` 字段（Claude 的补坑被写入决策并加入 Phase 3 骨架约束） |
| #3 plugins 目录 | 两个目录独立加载，不做 fallback | 同名插件不做静默覆盖，日志 warn 是底线（Claude 对 fallback 方案的风险修正被采纳） |
| #4 Qt 文件拆分 | v1 单文件，按职责方向拆 | 优先拆"主动上报"（EventRouter），按职责边界而非行数切割（Claude 的拆分依据被采纳） |

### 目录树同步

- 4 处 `mod.rs` 的 `❓见待定事项 #1` 注释 → 改为 `模块入口，集中 re-export`
- `sdk/src/index.ts` 的 `❓见待定事项 #2` → 改为 `统一导出入口（re-export widgets + system）`
- `plugins/` 的 `❓见待定事项 #3` → 改为 `内置验收插件，随安装包分发`
- 文档抬头标注说明从"❓待定"改为"所有设计决策已定案"

### 未改动

- 目录树结构未变（所有决策对目录结构无影响）
- Rust mod 树未变
- 第二节各文件职责说明未变（仅清除了 index.ts 一行的 ❓ 标记）

### Claude 复审修复（第二轮）

Claude 发现 3 个问题，2 个确认修复，1 个自行排除：

| # | 问题 | 处理 |
|---|---|---|
| 1 | Cargo.toml 路径 | Claude 自行排除——结构正确 |
| 2 | `runtime/mod.rs` 声明了 Phase 2 才存在的子模块 | **修复**：TASK_GUIDE.md 框架生成说明加"mod.rs 约束"；PROJECT_STRUCTURE.md Phase 1 列表加注意注释 |
| 3 | Dashboard 职责表缺 `src-tauri/src/main.rs` | **修复**：职责表补一行 |

### Claude 复审修复（第三轮）

| # | 问题 | 处理 |
|---|---|---|
| 1 | `scheduler/mod.rs` 注释与其他 `mod.rs` 风格不一致 | **修复**：改为"模块入口 + RenderScheduler 实现（不拆子模块）" |
| 2 | `experiments/qt-spike/` 缺 CMakeLists.txt | **修复**：目录树和 Phase 0 文件列表各补一行 |

### Claude 复审修复（第四轮）

Claude 发现同类问题蔓延：

| # | 问题 | 处理 |
|---|---|---|
| 1 | `host/mod.rs` 同样声明了 Phase 2 才存在的 `pub mod window_service` | **修复**：Phase 1 注意事项从只提 runtime/mod.rs 扩展为同时提 host/mod.rs 和 runtime/mod.rs |
| 2 | 第三节 mod 树是"最终状态"但未说明，AI 容易误以为 Phase 1 就要照写 | **修复**：第三节标题下加注——"以下为所有 Phase 完成后的最终状态" |

