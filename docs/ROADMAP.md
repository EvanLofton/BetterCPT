# BetterCPT 路线图

> 阶段计划、里程碑、风险、暂不实现。

---

## 总览

```
Phase 0：Qt Render Spike          → 验证 Qt IPC + 内存数据（可与 1/2 并行）
Phase 1：Host 核心骨架            → 托盘 + IPC + 日志
Phase 2：Script Runtime           → Deno Core + 生命周期
Phase 3：Widget Runtime (v1)     → Qt + Dock
Phase 4：Dashboard                → Tauri + React
Phase 5：DComp 增强层（远期）     → 渐进引入
```

---

## Phase 0：Qt Render Spike

**目标**：验证 Qt 作为 Widget Runtime 渲染后端的可行性。

**核心验证项**：
- Qt Overlay 窗口（无边框、置顶、点击穿透）
- Rust ↔ Qt JSON-RPC IPC 通信
- QPropertyAnimation 120FPS 动画
- 共享 vs 独立进程 10 Widget 内存实测
- Qt 崩溃恢复

**与 Phase 1/2 并行推进。**

### 阶段启动条件（硬性前置）

```
Phase 1 启动条件：无前置
Phase 2 启动条件：无前置（可与 Phase 0 并行）
Phase 3 启动条件：Phase 0 验收通过（硬性前置）
Phase 4 启动条件：Phase 1 + Phase 3 验收通过
Phase 5 启动条件：v1 发布并稳定运行 ≥ 3 个月
```

### Phase 0 验收标准（全部通过才算）

| # | 验收项 | 测量方式 |
|---|---|---|
| 1 | Qt Overlay 窗口点击穿透正常 | 手动测试：鼠标在非图标区域可穿透 |
| 2 | Rust ↔ Qt JSON-RPC IPC 延迟 < 5ms（本机） | 计时 TS setScale → Qt QWidget 属性变更 |
| 3 | QPropertyAnimation 在 144Hz 显示器上稳定 ≥ 120FPS | PresentMon 录制 |
| 4 | 共享模式 10 Widget 内存 < 60MB（实测，非估算） | Task Manager Private Working Set |
| 5 | 手动 kill Qt 进程后 Host 在 3 秒内完成重启 | 手动计时 kill → 窗口恢复 |

**如果 Phase 2 先于 Phase 0 完成**：Phase 2 可以验收，但 Phase 3 不得启动。需要评估 Phase 2 的工作是否需要因 Phase 0 结果而返工。这条约束具有文档效力。

详见 [internal/qt-render-spike.md](./internal/qt-render-spike.md)（迁移自原路线图 Phase 0 详细内容）

---

## Phase 1：Host 核心骨架

**难度**：★★☆☆☆

**目标**：Host 静默启动、托盘图标、IPC 消息总线、日志。

**验收**：Host 启动 → 托盘图标出现 → 右键退出 → 进程正常关闭。

---

## Phase 2：Script Runtime + Window Service

**难度**：★★★☆☆

**目标**：Deno Core 嵌入，插件生命周期（安装→启动→崩溃恢复→停止），Window Service v1（WebView Backend）。

**验收**：
- 文件行数统计工具（text-line-counter，仅需 fs:read 权限）
- PDF 工具插件：启动 → 打开 Utility Window → 显示 WebView 界面 → 关闭窗口 → 插件停止

---

## Phase 3：Widget Runtime (v1 — Qt)

**难度**：★★★☆☆

**目标**：Qt 子进程渲染 Dock，Widget SDK 命令式 API 验证通过。

**验收**：Dock 常驻底部，毛玻璃背景，Hover 120FPS 动画，Dock 插件内存增量 < 25MB。

> **内存目标更新（2026-05-19 L5 实测）**：
> Qt6 + V8 刚性基准 ~35MB（任何应用都逃不掉），不应算在插件头上。
> 增量度量：Dock V8 Isolate ~10MB + 7 Widgets ~11MB ≈ 21MB。
> 原始 25MB 目标不含基准开销，改为"插件增量 < 25MB"。

---

## Phase 4：Dashboard

**难度**：★★☆☆☆

**前置准备（Phase 3 验收通过后，Phase 4 开始前完成）**：
- Dashboard 功能清单确认：有哪些页面、每个页面做什么
- Dashboard UI 设计稿：视觉风格、布局、交互

未完成上述两项，Phase 4 不得启动。

**目标**：Tauri v2 + React 管理后台。

**验收**：安装 → 启动 → 停止 → 卸载插件全流程。

---

## Phase 5：DComp 增强层（远期）

**难度**：★★★★☆

**前提**：v1 Runtime 稳定运行 ≥ 3 个月，有真实用户。

**子阶段**：

| 步骤 | 内容 |
|---|---|
| 5a | DComp Blur（Qt 离屏渲染 → DWM Blur） |
| 5b | DComp Animation（QPropertyAnimation → DComp Animation） |
| 5c | DComp Visual Tree（QWidget 层级 → DComp Visual 树） |
| 5d | TS 转译支持（esbuild/SWC）：插件恢复 TypeScript 开发体验 |
| 5e | Runtime 权限拦截：per-plugin 请求校验（与 Phase 5 多 Isolate 架构一起做） |

每一步增量替换，出问题回退到 v1 Qt 实现。

---

## 暂不实现

- ❌ Native Runtime（安全基础设施不到位）
- ❌ 插件市场（审核、签名、沙箱不到位不开放）
- ❌ 声明式 Widget SDK（virtual DOM / hooks / reactive graph）
- ❌ 跨平台支持（v1 Windows only）
- ❌ 插件热更新（v1 TS 热更新支持，Qt 热更新不支持）

---

## 风险总览

| 风险 | 严重程度 | 应对 |
|---|---|---|
| Qt 构建依赖过重 | 中 | Phase 0 验证构建流程可行性 |
| 多 Qt 进程内存偏高 | 中 | 默认共享 Qt 进程，需要时切独立 |
| Widget SDK 演变为 UI 框架 | 高 | Phase 1 严格限制命令式 API |
| 插件 JS 死循环阻塞自身 Isolate | 中 | per-plugin Isolate 隔离 + Execution Timeout + Watchdog（影响范围限于单个插件） |
| 过早平台化 | 高 | 先跑通 Dock，Runtime 成立后再扩展 |
