# BetterCPT 文档目录

> 每个文档的职责和阅读顺序。

---

## 阅读顺序（新人入门）

1. `PROJECT_OVERVIEW.md` — 项目是什么、核心原则、系统全景图
2. `ARCHITECTURE.md` — 系统拓扑、Runtime 边界、11 条不变量（宪法）
3. `ROADMAP.md` — 分阶段计划、验收标准、风险
4. `FUTURE_CAPABILITIES.md` — v1 能力总表、不做清单、远期备忘

---

## 设计文档

| 文档 | 内容 | 何时读 |
|---|---|---|
| `PROJECT_OVERVIEW.md` | 项目总纲：是什么、为什么、核心原则、文档索引 | 入门 |
| `ARCHITECTURE.md` | 系统拓扑、Runtime 边界、架构不变量（11 条） | 理解全局 |
| `RUNTIME_SPEC.md` | 生命周期、崩溃恢复、Watchdog、Scheduler、热更新 | 实现 Runtime |
| `IPC_PROTOCOL.md` | JSON-RPC 2.0 通信协议、错误码、Batching、Replay | 实现 IPC |
| `WIDGET_RUNTIME.md` | Qt 渲染管线、动画模型、窗口管理、事件路由 | 实现 Widget |
| `SDK_SPEC.md` | TypeScript SDK API 设计、线程模型、红线、v1 插件 Runtime 模型 | 写插件 / 实现 SDK |
| `SDK_DEVELOPER_GUIDE.md` | 插件开发实战指南：manifest、API 速查、调试、示例 | 插件开发者入门 |
| `SECURITY_MODEL.md` | 权限模型、Capability/Permission 分层、Runtime 隔离 | 理解安全边界 |
| `WINDOW_SYSTEM.md` | 窗口所有权、类型、Backend、Window Service API | 实现窗口系统 |
| `DASHBOARD_SPEC.md` | Dashboard 功能清单与 UI 设计规范 | Phase 4 开发前必读 |
| `BUILD_AND_DEPLOY.md` | 构建链、依赖、CI/CD、打包 | 搭建环境 |
| `TESTING_STRATEGY.md` | 测试层次、验收指标、性能基准 | 写测试 / 验收 |

---

## 工程文档

| 文档 | 内容 | 何时读 |
|---|---|---|
| `TASK_GUIDE.md` | 指挥 AI 写代码的操作手册：Prompt 模板、任务清单、验收标准 | 每次写代码前 |
| `PROJECT_STRUCTURE.md` | 完整目录树、每个文件职责、Rust mod 树、按 Phase 文件列表 | 生成骨架 / 定位文件 |
| `phase_prompts.md` | 每个 Phase 的完整会话 Prompt，可直接发给 AI | 开始新 Phase |
| `ROADMAP.md` | 阶段计划、启动条件、验收标准、风险总览 | 规划下一步 |
| `FUTURE_CAPABILITIES.md` | v1 能力总表、明确不做清单、远期规划 | 控制范围 |
| `ACCEPTANCE_REPORT.md` | 各 Phase 最终验收报告、问题修复记录、跨 Phase 遗留项追踪 | 验收 / 新 Phase 开始前 |

---

## 历史文档

`internal/` 目录下为设计讨论过程的记录，不作为正式规范，仅供参考。
