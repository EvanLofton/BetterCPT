# BetterCPT — 项目总纲

> 这是整个项目的唯一入口文档。只回答"是什么、为什么、核心原则、系统全景"。实现细节见各子文档。

---

## BetterCPT 是什么

BetterCPT 是一个 **Windows 桌面插件平台**。它在桌面上提供一个 GPU 加速的轻量 Widget Runtime，允许用户通过 TypeScript SDK 编写常驻桌面的视觉组件（Dock、天气卡片、系统监控等），同时支持按需启动的功能型脚本插件。

## 为什么存在

现有方案的两难：
- **Rainmeter / Desktop Widgets**：轻量但开发体验差（Lua 脚本、无现代工具链）、扩展性弱
- **Electron / WebView 方案**：开发体验好但内存沉重（单 Widget 15-25MB，10 个直逼 300MB）

BetterCPT 的目标是：**常驻 Widget 内存 < 50MB，开发体验接近现代前端，同时保持桌面级 GPU 渲染性能。**

## 核心原则

以下原则贯穿整个项目，不可违反：

1. **Host 是唯一状态源** — 渲染进程不保存业务状态，崩溃可完整恢复
2. **Widget 必须轻量常驻** — 后台闲置内存极小，CPU 接近零
3. **插件按需分层** — 常驻用 Widget Runtime，功能用 Script Runtime，高风险用 Native Runtime（默认关闭）
4. **SDK 是命令式 API，不是 UI Framework** — 不做 virtual DOM，不做 hooks，不做 reactive graph
5. **崩溃必须可恢复** — 任何 Runtime 的崩溃不应导致其他 Runtime 不可用
6. **v1 追求可交付，v2+ 追求更优** — Qt 作为 v1 Widget 渲染引擎（已验证可行），DirectComposition 作为远期增强

## 系统全景

```
┌─────────────────────────────────────────────────────────────┐
│                   BetterCPT Host（Rust + tokio）             │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │Widget Runtime │  │Script Runtime│  │  Native Runtime  │  │
│  │  (常驻后台)  │  │ (Deno Core) │  │  ⚠️ 默认关闭    │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
│         │                 │                    │            │
│  Qt 子进程(C++)     per-plugin Isolate     独立子进程       │
│  JSON-RPC IPC       V8 Isolate 池          任意语言         │
│  独立/共享模式       IPC 通信              需手动授权        │
│                                                             │
│  ── Host 内部服务 ─────────────────────────────────────── │
│  Window Service  │  RuntimeRegistry  │  Render Scheduler    │
└─────────────────────────────────────────────────────────────┘
         │                 │                    │
    桌面 Widget         功能型工具            系统级插件
  Dock / 卡片 / Overlay  PDF / 自动化        游戏脚本等
```

## 为什么 Qt

- 开发者已用 Qt 实现过 Dock，实测内存极低
- 场景图、动画系统、渲染循环全部框架自带 — v1 零自建模块
- 独立进程模型天然提供崩溃隔离
- 文档和社区极其成熟

详见 [WIDGET_RUNTIME.md](./WIDGET_RUNTIME.md)

## 为什么不用 WebView 做 Widget Runtime

- 每 WebView 进程 15-25MB，10 个 Widget 总计 200-320MB
- 不适合常驻后台
- Overlay 点击穿透、z-order 控制受限

WebView 的正确位置：Dashboard（Tauri v2 + React，偶尔打开的配置界面）

## 为什么 Widget / Script / Native 分层

| Runtime | 定位 | 内存目标 | 权限 |
|---|---|---|---|
| Widget | 常驻视觉组件 | < 50MB（10 Widget） | widget:overlay |
| Script | 按需功能工具 | 按需分配，用完释放 | 普通（fs, network, dialog） |
| Native | 系统级操作 | 不限 | 全权限（需开发者模式） |

## MVP 路线图（简版）

```
Phase 0：Qt Render Spike          → 验证 Qt IPC + 内存数据
Phase 1：Host 核心骨架            → 托盘 + IPC + 日志
Phase 2：Script Runtime           → Deno Core + 生命周期
Phase 3：Widget Runtime (v1)     → Qt + Dock
Phase 4：Dashboard                → Tauri + React
Phase 5：DComp 增强层（远期）     → 渐进引入
```

详见 [ROADMAP.md](./ROADMAP.md)

---

## 文档索引

| 文档 | 内容 | 何时读 |
|---|---|---|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | 系统拓扑、边界、Invariants | 理解架构全局 |
| [RUNTIME_SPEC.md](./RUNTIME_SPEC.md) | Runtime 生命周期、状态机、恢复 | 实现 Runtime 管理 |
| [IPC_PROTOCOL.md](./IPC_PROTOCOL.md) | 通信协议、错误码、版本 | 实现 IPC 层 |
| [WIDGET_RUNTIME.md](./WIDGET_RUNTIME.md) | Widget 渲染管线、Qt、Scheduler | 实现 Widget Runtime |
| [SDK_SPEC.md](./SDK_SPEC.md) | API 设计、线程模型 | 写插件 / 实现 SDK |
| [SECURITY_MODEL.md](./SECURITY_MODEL.md) | 权限、隔离、信任 | 理解安全边界 |
| [BUILD_AND_DEPLOY.md](./BUILD_AND_DEPLOY.md) | 构建链、CI/CD | 搭建开发环境 |
| [WINDOW_SYSTEM.md](./WINDOW_SYSTEM.md) | 窗口所有权、类型、Backend | 实现功能插件界面 |
| [TESTING_STRATEGY.md](./TESTING_STRATEGY.md) | 测试计划、验收指标 | 写测试 / 验收 |
| [ROADMAP.md](./ROADMAP.md) | 阶段、风险、里程碑 | 规划下一步 |
| [FUTURE_CAPABILITIES.md](./FUTURE_CAPABILITIES.md) | 远期能力规划、备忘 | 规划 v2+ |

历史讨论和设计过程见 [internal/](./internal/)。
