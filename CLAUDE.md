# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

BetterCPT 是一个 **Windows 桌面插件平台** —— Rust 主进程 + Qt Widget 渲染 + Deno Core（TypeScript 插件）+ Tauri 管理后台。目标：10 个 Widget 内存 < 50MB，GPU 加速渲染，崩溃可恢复。

## 当前状态

**v1 已完成。** Phase 0~4 全部验收通过，核心管线（SDK → Deno → Scheduler → Qt 渲染）已打通。Dock 插件可运行。Phase 5 为远期优化。

## 构建命令

```bash
# Qt Widget 渲染器
cd widget-qt && cmake -B build -DCMAKE_BUILD_TYPE=Debug && cmake --build build

# Rust Host（含 Tauri Dashboard）
cd host && cargo build
cd host && cargo build --release
cd host && cargo test

# Dashboard 前端
cd dashboard && npm install && npm run build

# 打包安装程序
cd host && cargo tauri build --bundles nsis
```

## 系统架构

**三种 Runtime，一个 Host：**

| Runtime | 进程模型 | 技术栈 | 定位 |
|---------|---------|--------|------|
| Widget | Qt 子进程 | C++ Qt 6.5 | 常驻桌面组件（Dock、卡片、Overlay） |
| Script | Host 内 V8 Isolate | JavaScript (Deno Core) | 按需功能型插件 |
| Native | 独立子进程 | 任意语言 | 系统级插件（默认关闭） |

**数据流向：** TS SDK → Host State Tree（唯一真相源）→ Render Scheduler → IPC Batch → Qt 渲染镜像。Qt 不持有业务状态，崩溃后通过 Replay 完整恢复。

**Host 内部服务：** Window Service（工具窗口管理）、RuntimeRegistry（生命周期追踪）、Render Scheduler（16ms 刷新、帧级去重、批量打包）。

## 架构不变量（来自 ARCHITECTURE.md §三）

以下是不可违反的 11 条规则，任何设计决策、迭代、重构都必须遵守：

1. **Host 是唯一状态源。** Qt 不能持有业务状态。Deno 不能持有最终 UI 状态。
2. **Widget ID 由 Host 统一分配。** 全局唯一，生命周期内不复用。
3. **所有 Widget API 必须串行进入 Render Scheduler。** 不允许多个 async task 并发修改 Widget Tree。
4. **IPC 不保证实时性，只保证顺序性。** 需要帧级同步的场景使用 Scheduler batch 机制。
5. **Qt 是 Render Replica，不是 Render Authority。** Qt 只响应 Host 同步指令。用户交互回传 Host 决策。
6. **Widget SDK 不允许直接访问 Renderer。** 不能直接操作 Qt 对象、创建原生窗口、绕过 IPC。
7. **Runtime 权限上限不可绕过。** Widget 永远不能获取 fs:write。Native 需要开发者模式 + 签名 + 确认。
8. **每个 Runtime 的崩溃必须可恢复。** 不存在"崩溃后系统不可用"的 Runtime 类型。
9. **Widget SDK API 向后兼容优先于 Renderer 能力。** SDK API 在 Qt、DComp 等后端下行为一致。
10. **不能因引入新技术而提高最低系统要求。** DComp 不可用时回退到 Qt 实现。
11. **第一阶段不做声明式框架。** 不做 virtual tree、diff、hooks、reactive graph。

## 任务指挥系统（TASK_GUIDE.md）

所有代码实现工作遵循 `docs/TASK_GUIDE.md` 中的规范。核心规则：

- **骨架先行**：先生成文件骨架（struct 定义 + 函数签名 + `// TODO`），再填充实现。骨架必须通过 `cargo build` 编译后才开始写函数体。
- **每次只做一个任务**：不要把多个任务合并。发给 AI 的 Prompt 必须包含六部分：架构不变量、文件职责说明、相关文档片段、任务描述、已有签名、约束条件。
- **Rust 代码禁止 `unwrap()` 和 `expect()`**：错误处理统一用 `Result<T, anyhow::Error>` 和 `?` 操作符。
- **禁止引入新依赖**：只能用骨架 `Cargo.toml` 中已有的 crate。
- **mod.rs 约束**：`host/mod.rs` 和 `runtime/mod.rs` 只声明当前 Phase 已有文件对应的 `pub mod`，不得超前声明未存在的模块。
- **Phase 0 的 spike 代码**放在 `experiments/qt-spike/` —— 验证代码可以用 unwrap、可以硬编码，不作为生产代码进入主分支。

### Prompt 模板六要素

给 AI 发任务时，必须包含：
1. **必读**：ARCHITECTURE.md §三 全文 + PROJECT_STRUCTURE.md 中当前文件对应的职责说明
2. **背景**：与本任务直接相关的文档片段（1-3 个即可）
3. **任务**：一句话说明要实现什么，格式为"实现 `<模块路径>` 中的 `<函数/结构体名>`"
4. **已有签名**：框架文件中已定义好的函数签名、结构体定义、TODO 注释
5. **已有代码**：前序 Phase 已完成的、被本任务调用的函数实现
6. **约束**：硬性限制（禁止 unwrap、禁止新依赖、只输出当前文件等）+ v1 红线

## 项目目录结构

```
bettercpt/
├── host/                      # Rust Host 主进程（Tauri v2 嵌入）
│   └── src/
│       ├── main.rs            # 入口：Tauri setup + Host 主循环
│       ├── host/              # logger, singleton, context, shutdown, window_service, commands, settings, splash
│       ├── ipc/               # messages（含 method 常量）, error_codes（JSON-RPC 2.0）
│       ├── runtime/           # registry, manifest, permissions, capability, deno_host, isolate_pool,
│       │                      # watchdog, plugin_loader, recovery, crash_report, storage, widget_qt
│       └── scheduler/         # RenderScheduler + state_tree（apply_ipc, generate_replay_commands）
├── widget-qt/                 # Qt C++ 子进程 —— CMake, Qt 6.5 Widgets, RenderServer, WidgetManager
├── sdk/                       # SDK（.d.ts 类型声明 + JS Preamble 运行时）
├── dashboard/                 # React 前端（Vite + Tailwind），Tauri WebView 加载
├── plugins/                   # 内置验收插件（.bcpkg 格式），含 dock
├── experiments/qt-spike/      # Phase 0 Spike 代码（仅作参考，容器模型已废弃）
├── scripts/                   # 辅助脚本
└── docs/                      # 全部设计文档
```

## 设计文档索引

| 文档 | 内容 | 何时读 |
|------|------|--------|
| `docs/PROJECT_OVERVIEW.md` | 项目总纲：是什么、为什么、核心原则 | 入门 |
| `docs/ARCHITECTURE.md` | 系统拓扑、Runtime 边界、11 条不变量 | 理解架构全局 |
| `docs/PROJECT_STRUCTURE.md` | 完整目录树、每个文件职责、按 Phase 文件列表 | 生成骨架 / 定位文件 |
| `docs/TASK_GUIDE.md` | 指挥 AI 写代码的操作手册：Prompt 模板、任务清单、验收标准 | 每次写代码前 |
| `docs/RUNTIME_SPEC.md` | Runtime 生命周期、崩溃恢复、Watchdog、热更新 | 实现 Runtime 管理 |
| `docs/IPC_PROTOCOL.md` | JSON-RPC 2.0 通信协议、错误码、Batching、Replay | 实现 IPC 层 |
| `docs/WIDGET_RUNTIME.md` | Qt 渲染管线、动画模型、窗口管理 | 实现 Widget Runtime |
| `docs/SDK_SPEC.md` | SDK API 设计、线程模型、红线、v1 Runtime 模型 | 写插件 / 实现 SDK |
| `docs/SDK_DEVELOPER_GUIDE.md` | 插件开发实战指南 | 插件开发者入门 |
| `docs/SECURITY_MODEL.md` | 权限模型、Capability/Permission 分层、Runtime 隔离 | 理解安全边界 |
| `docs/WINDOW_SYSTEM.md` | 窗口所有权、类型、Backend、Window Service API | 实现窗口系统 |
| `docs/BUILD_AND_DEPLOY.md` | 构建链、依赖、CI/CD、打包 | 搭建环境 |
| `docs/TESTING_STRATEGY.md` | 测试层次、验收指标、性能基准 | 写测试 / 验收 |
| `docs/FUTURE_CAPABILITIES.md` | v1 能力总表、明确不做清单、Phase 5 远期规划 | 控制范围 / 规划后期 |
| `docs/ROADMAP.md` | 阶段计划、风险、里程碑 | 规划下一步 |
| `docs/ACCEPTANCE_REPORT.md` | 各 Phase 验收报告汇总、问题修复记录、遗留项 | 验收 / 排查问题 / 新 Phase 开始前 |
| `docs/DASHBOARD_SPEC.md` | Dashboard 功能清单与 UI 设计规范 | Phase 4 开发 |
| `docs/SDK_DEVELOPER_GUIDE.md` | 插件开发实战指南：manifest、API 速查、调试 | 写插件 |

## IPC 协议

JSON-RPC 2.0，传输层为 stdin/stdout pipe（Host ↔ Qt）和进程内 mpsc channel（Host ↔ Deno）。协议版本：`bettercpt-ipc/1`。消息类型：Request（有 id）、Response（有 id）、Error（有 id）、Notification（无 id，不期待响应）。Batch 消息将多条渲染指令打包为单条 IPC 消息。错误码范围：-32700 ~ -32003。

## 插件系统

- `runtime` 字段：`"widget"`（常驻视觉组件）或 `"script"`（按需功能工具）
- `capabilities` 字段：系统能力（`widget:overlay`、`window:create`、`notification`）
- `permissions` 字段：资源访问（`fs:read`、`fs:write`、`network:fetch`、`storage` 等）
- 插件目录有两个：`bettercpt/plugins/`（内置）和 `%AppData%/BetterCPT/plugins/`（用户安装），各自独立加载，不做 fallback

## 崩溃恢复

每个 Runtime 最多自动重启 5 次。Qt 崩溃恢复流程：冻结 IPC → 标记状态树为 recovering → 启动新 Qt 进程 → 全量 Replay 状态树（>1MB 时分页发送）→ 解除冻结。Isolate 崩溃恢复：Watchdog 三层检测（Timeout / Budget / Heartbeat）→ 重建 Isolate → 重新加载插件代码。超限后通知用户并停止重启。
