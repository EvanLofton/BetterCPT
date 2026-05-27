# BetterCPT 系统架构

> 本文档定义系统拓扑、Runtime 边界和架构不变量。是项目的"宪法"。
> 
> 实现细节见 RUNTIME_SPEC.md、IPC_PROTOCOL.md、WIDGET_RUNTIME.md 等子文档。

---

## 一、系统拓扑

```
┌──────────────────────────────────────────────────────────────────┐
│                    BetterCPT Host（Rust + tokio）                 │
│                                                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌──────────────────┐   │
│  │ Widget Runtime │  │ Script Runtime │  │  Native Runtime  │   │
│  │   (常驻后台)   │  │  (Deno Core)   │  │  ⚠️ 默认关闭   │   │
│  └───────┬────────┘  └───────┬────────┘  └────────┬─────────┘   │
│          │                   │                     │             │
│  Qt 子进程(C++)        per-plugin Isolate     独立子进程         │
│  JSON-RPC IPC          V8 Isolate 池          任意语言           │
│  独立/共享模式          IPC 通信              需手动授权          │
│                                                                  │
│  ── Host 内部服务 ────────────────────────────────────────── │
│  Window Service  │  RuntimeRegistry  │  Render Scheduler         │
└──────────┬───────────────────┬─────────────────────┬─────────────┘
           │                   │                     │
      桌面 Widget          功能型工具            系统级插件
    Dock / 卡片 / Overlay  PDF / 自动化         游戏脚本等
```

### 组件清单

| 组件 | 进程模型 | 技术栈 | 生命周期 |
|---|---|---|---|
| Rust Host | 主进程 | Rust + tokio | 系统启动到关机 |
| Widget Runtime | 子进程（Qt） | C++ Qt 6.5 | Host 启动时拉起，随 Host 退出 |
| Widget 业务逻辑 | Host 内 V8 Isolate 池 | TypeScript (Deno Core, per-plugin Isolate) | 随 Host |
| Script Runtime | Host 内 Deno 实例 | TypeScript (Deno Core) | 按需启动，用完释放 |
| Dashboard | **Host 进程内** Tauri WebView 窗口 | React + TypeScript | 用户打开时启动，关闭时释放（窗口销毁，Host 进程不受影响） |
| Native Runtime | 子进程 | 任意语言 | 远期，默认关闭 |
| Window Service | Host 内部服务 | Tauri WebView / 系统原生 | 按需 |

---

## 二、Runtime 边界

### 状态所有权

**Host 是唯一真相源（Single Source of Truth）。**

```
TS SDK（Deno）
  │  发出命令，不持有状态
  ↓
Host State Tree（唯一状态源）
  │  所有 Widget 的 id、bounds、scale、blur、children、事件绑定
  │  状态变更时递增版本号
  ↓
Qt 子进程（Render Replica）
  │  仅持有渲染镜像
  │  不保存业务状态，崩溃可完整恢复
```

### 各层职责

| 层 | 负责 | 不负责 |
|---|---|---|
| TS SDK (Deno) | 业务逻辑、发出渲染命令 | 持有最终状态、直接操作渲染 |
| Rust Host | 状态管理、IPC 路由、生命周期、权限 | 渲染、执行插件业务逻辑 |
| Qt 子进程 | GPU 渲染、动画执行、窗口管理 | 持有业务状态、做业务决策 |
| Script Runtime (Deno) | 执行功能型插件逻辑 | 管理自身生命周期（Host 管理） |
| Dashboard (WebView) | 配置界面、插件管理 UI | 持有运行时状态 |

### Widget ID 生命周期

- **内部 ID**：Host 分配不透明递增整数（如 `42`），用于运行时索引和 IPC。全局唯一，生命周期内不复用
- **调试路径**：仅用于日志和 DevTools（如 `widget://dock/icon/0`），不参与业务逻辑，不存储在状态树中
- 崩溃恢复时 ID 保持不变（Host replay 使用相同内部 ID）
- 热更新匹配：按逻辑名（v2+）或调用顺序（v1 fallback）匹配已有 Widget，匹配成功后返回已有内部 ID
- TS SDK 和 Qt 均不可自生成 ID

详见 [RUNTIME_SPEC.md](./RUNTIME_SPEC.md)

---

## 三、Runtime Invariants（系统不变量）

以下是不可违反的架构规则。任何设计决策、迭代、重构必须遵守。

### 状态

1. **Host 是唯一状态源。** Qt 不能持有业务状态。Deno 不能持有最终 UI 状态。
2. **Widget ID 由 Host 统一分配。** 全局唯一，生命周期内不复用。

### 调度

3. **所有 Widget API 必须串行进入 Render Scheduler。** 不允许多个 async task 并发修改 Widget Tree。
4. **IPC 不保证实时性，只保证顺序性。** 需要帧级同步的场景使用 Scheduler batch 机制。

### 渲染

5. **Qt 是 Render Replica，不是 Render Authority。** Qt 只响应 Host 同步指令。用户交互回传 Host 决策。
6. **Widget SDK 不允许直接访问 Renderer。** 不能直接操作 Qt 对象、创建原生窗口、绕过 IPC。

### 安全

7. **Runtime 权限上限不可绕过。** Widget 永远不能获取 fs:write。Native 需要开发者模式+签名+确认。
8. **每个 Runtime 的崩溃必须可恢复。** 不存在"崩溃后系统不可用"的 Runtime 类型。

### 兼容性

9. **Widget SDK API 向后兼容优先于 Renderer 能力。** SDK API 在 Qt、DComp 等后端下行为一致。
10. **不能因引入新技术而提高最低系统要求。** DComp 不可用时回退到 Qt 实现。

### 复杂性

11. **第一阶段不做声明式框架。** 不做 virtual tree、diff、hooks、reactive graph。
12. **每个新增架构层次必须有"为什么不能没有它"的回答。** 用数据证明必要性，而非假设。

---

## 四、Dashboard 数据流

Dashboard（Tauri v2 + React）是管理界面，不持有运行时状态。所有状态由 Host 维护，Dashboard 通过订阅获取。

**进程模型（2026-05-19 锁定）**：Dashboard 不是独立进程，而是 Host 进程内的 Tauri WebView 窗口。
Tauri 运行在 Host 进程中，Rust 后端通过 `State<Arc<RuntimeRegistry>>` 直接访问 Host 状态，零 IPC 开销。
Dashboard 窗口关闭时仅销毁 WebView，Host 主循环和 Qt 子进程不受影响。

```
Host 进程 (bettercpt-host.exe, Tauri app)
  ├── tokio runtime（主循环、Scheduler、Deno Isolate 池）
  ├── Qt 子进程（widget-qt.exe, stdin/stdout pipe）
  └── Dashboard（Tauri WebView 窗口, 按需创建/销毁）
       ├── 启动时：invoke("get_runtime_snapshot") → State<Arc<RuntimeRegistry>> 直接返回
       └── 运行时：listen("runtime_event") → Tauri emit 推送增量更新
```

**事件类型**：

| 事件 | 触发时机 | payload |
|---|---|---|
| plugin_started | 插件启动成功 | pluginId, runtime |
| plugin_stopped | 插件正常停止 | pluginId |
| plugin_crashed | 插件崩溃 | pluginId, crashCount |
| plugin_recovered | 崩溃恢复成功 | pluginId |
| memory_sample | 每 5 秒采样 | pluginId, bytes |

**Dashboard 断连恢复**：Dashboard 本地缓存最后一次快照。IPC 断开时显示"正在恢复..."遮罩，重连后重新拉取快照。Tauri 的 `onDisconnect` 事件可直接使用。

---

## 五、文档边界

每个子文档的明确职责：

| 文档 | 职责 | 不负责 |
|---|---|---|
| RUNTIME_SPEC.md | Runtime 生命周期、状态机、恢复流程 | Widget 渲染细节 |
| IPC_PROTOCOL.md | 通信协议格式、错误码、版本 | 业务逻辑 |
| WIDGET_RUNTIME.md | Widget 渲染管线、Qt 集成、Scheduler | Script Runtime |
| SDK_SPEC.md | Widget SDK API 设计、线程规则 | 渲染后端实现 |
| WINDOW_SYSTEM.md | 窗口系统、Window Service API、Backend | Widget 渲染 |
| SECURITY_MODEL.md | 权限模型、隔离、信任 | 具体权限实现代码 |
| BUILD_AND_DEPLOY.md | 构建链、CI/CD | 架构设计 |
| TESTING_STRATEGY.md | 测试计划、验收标准 | 具体测试用例 |
| ROADMAP.md | 阶段计划、里程碑、风险 | 实现细节 |
