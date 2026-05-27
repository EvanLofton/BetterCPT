# BetterCPT 远期能力规划

> 记录 v1 平台能力状态、已确认不做的事项、远期备忘。
>
> **架构约定**：manifest 已拆分为三层——`runtime`（执行环境）、`capabilities`（系统能力）、`permissions`（资源访问）。详见 `SECURITY_MODEL.md`。

---

## 一、v1 平台能力总表

### Capability（系统能力）

| Capability | v1 状态 | 说明 |
|---|---|---|
| `widget:overlay` | ✅ | Widget Runtime 专属，Qt 渲染 |
| `window:create` | ✅ | Utility Window（WebView），Widget + Script 均可 |
| `notification` | ✅ | Windows Toast 通知 |
| Global Shortcut | 🔧 v1 Host 级 | 插件级放 v2 |

### Permission（资源访问）

| Permission | v1 状态 | 说明 |
|---|---|---|
| `storage` | ✅ | KV 存储，上限 1MB |
| `fs:read` / `fs:write` | ✅ | Script 专属 |
| `network:fetch` | ✅ | Script 全量，Widget 白名单 |
| `dialog:open` / `dialog:save` | ✅ | Script 专属 |
| `system:info` | ✅ | Script 专属 |

### 内置能力（无需声明）

| 能力 | v1 状态 | 说明 |
|---|---|---|
| Dashboard (Tauri) | ✅ | Host 内置管理界面 |
| Timer | ✅ | Deno 内置，随 Isolate 自动清理 |
| dialog.alert/confirm/prompt | ✅ | 系统原生消息弹窗 |
| system.memory() | ✅ | 性能指标 |
| 系统托盘 | ✅ | Host 内置 |
| 崩溃诊断报告 | ✅ | 自动保存 crash JSON |

---

## 二、v1 明确不做

| 项目 | 原因 |
|---|---|
| createButton / createInput | 一旦做 Input System，focus/IME/caret/selection/clipboard/accessibility 会爆炸——滑向自研 GUI Framework |
| 插件间通信 | 引入 message bus / service discovery / deadlock / permission graph |
| Widget 窗口拖拽移动 | v1 hardcode 位置（Dock 默认底部居中），跑通再说 |
| 国际化 (i18n) | v1 完全不需要 |
| Widget 音效 | 后期可加 |
| 声明式 UI（JSX/hooks/reactive） | 红线，不做框架 |
| Native Runtime | 安全基础设施不到位 |
| 插件市场 | 审核/签名/沙箱不到位不开放 |
| 跨平台 | v1 Windows only |
| 插件热更新（Qt 侧） | Qt 不热更新，TS 侧支持 |
| 多 Backend 动态切换 | v1 WebView only |
| 自定义窗口外观 | 无边框、异形窗口——v1 不做 |

---

## 三、远期能力清单

### 远期 Capability

| Capability | 状态 | 说明 |
|---|---|---|
| Global Shortcut（插件级） | 📋 远期 | 需防冲突和劫持 |
| Clipboard | 📋 远期 | 需防恶意读取敏感内容 |
| Screen Capture | 📋 远期 | 需强权限控制 |
| Shell Execute | 📋 远期 | 需允许命令白名单 |
| Native Runtime | 📋 远期 | 需签名+审核+沙箱 |

### 远期 Permission

| Permission | 状态 | 说明 |
|---|---|---|
| 插件间通信 | 📋 远期 | 需完整 service discovery |

### 远期基础设施

| 项目 | 状态 | 说明 |
|---|---|---|
| 插件市场 | 📋 v2 | 审核/签名/沙箱；使用 .bcpkg 格式，与 v1 本地加载复用 |
| 插件 ID 全局注册 | 📋 v2 | @author/plugin-name 命名空间，@bettercpt 保留给官方 |
| 跨平台支持 | 📋 远期 | v1 Windows only |
| DComp 增强 | 📋 Phase 5 | 渐进引入 |
| CAPABILITY_MODEL.md | 📋 等 2+ capability 成熟时提取 |

---

## 四、何时正式设计 CAPABILITY_MODEL.md

满足以下条件时：

1. v1 稳定运行
2. 至少 2 个远期能力进入正式设计阶段
3. 能力之间的共性足够抽取出统一模型

在此之前，各能力独立设计，本文档做索引。

---

## 五、远期架构扩展方案

> 以下方案的触发条件均为"v1 稳定运行 + 插件/指令量超出现有架构承载上限"。
> 当前 v1 暂不需要，方案先行记录，避免后期遗忘或重复调研。

---

### 5.1 Batch IPC 协议

**问题**：v1 的 `flush_and_send` 逐条 `send_request → read_response`。每条指令 = 2 次 pipe I/O。指令量上去后 pipe 往返次数线性增长。

| 规模 | Naive (当前) | Batch (数组打包) |
|------|-------------|-----------------|
| 13 条 | ~400ms | ~50ms |
| 100 条 | ~3s (不可用) | ~100ms |
| 500 条 | 不可用 | ~200ms |

**方案**：Scheduler flush 出的 `Vec<IpcRequest>` 序列化为 JSON 数组一次性写入 pipe，Qt 侧逐条处理后返回 JSON 数组响应。

**好处**：指令数不影响 pipe 往返次数，永远 1 写 + 1 读。Scheduler 已做好帧级去重，只需改 `flush_and_send` 的序列化方式。

**建议时机**：单帧指令稳定 > 20 时启用（Phase 5+）。

**相关文件**：`docs/IPC_PROTOCOL.md` §九

---

### 5.2 多 Isolate 多线程

**问题**：`JsRuntime: !Send`，v1 所有插件 Isolate 在单线程顺序执行。插件数 > 5 后，单个插件的 GC / 死循环 / 大量计算阻塞其他插件。

**方案**：

```
主线程                       Isolate 线程 A (Dock)
  │                               │
  ├── Qt pipe                     │
  ├── Scheduler                   │
  └── 事件分发器 ──mpsc──→       recv → execute_script → _send → mpsc → Scheduler
                      ──mpsc──→  Isolate 线程 B ...
                      ──mpsc──→  Isolate 线程 C ...
```

每个插件独立线程 + mpsc channel 通信。`route_event()` 通过 channel 向对应线程发送事件。主线程不直接操作 JsRuntime。

**好处**：插件完全隔离——一个插件的 GC 不阻塞其他插件；充分利用多核；架构对齐 deno_core 的设计本意。

**不需要改的**：Scheduler、Qt pipe、SDK preamble、v8 注入方式。

**建议时机**：插件数 > 5 时启用（Phase 5+）。

**相关文件**：`docs/RUNTIME_SPEC.md` §二

---

### 5.3 插件 TypeScript 转译（esbuild / SWC）

**问题**：v1 插件手写纯 JS。deno_core 的 `execute_script` 运行 V8 原生 JavaScript，不处理 TypeScript 语法（`interface`、`Array<T>`、`declare` 等）。当前 v1 三插件规模小，手写 JS 可接受。

**方案**：编译期用 esbuild（或 SWC）将插件 `.ts` 文件转译为单文件 `.js`，再交给 deno_core 执行。esbuild 支持 `--bundle --format=iife`，天然处理 ESM import/export 和多文件合并。

**好处**：
- 插件开发者可以写 TypeScript，享受类型检查和 IDE 补全
- 多文件插件自动打包为单文件
- 消除 v1 的 `import`/`export` 逐行过滤 hack
- esbuild 编译速度极快（< 10ms per plugin）

**建议时机**：插件数 > 3 或首个第三方插件出现时（Phase 5）。

**相关文件**：`host/src/runtime/deno_host.rs` execute() 注释

---

### 5.4 deno_core `#[op2]` 标准注册

**问题**：v1 绕过 deno_core 的 extension/op2 系统，用 v8 API 直接注入 `__bettercpt_tx` 全局对象。根因：`#[op2]` + `extension!` 在 deno_core 0.290 上未将 op 注册为 JS 全局函数。

**方案**：升级 deno_core 版本后，回归标准方式——用 `#[op2]` 宏定义 Rust 函数，用 `extension!` 宏声明扩展，用 `init_ops_and_esm()` 初始化。

**好处**：
- 删除 ~30 行 v8 手动注入代码
- 符合 deno_core 官方推荐模式
- 后续升级/迁移更安全

**建议时机**：deno_core 版本升级时顺手试验（Phase 5）。

**限时重试**：进入 Phase 4 前，给 `#[op2]` + `extension!` 初始化顺序调整留半天上限。若半天内未解决则接受 v8 直接注入方案，不再延期。

**相关文件**：`docs/preview/PIPELINE_INTEGRATION_REPORT.md` P3

---

### 5.5 send_notification（Host→Qt fire-and-forget）

**问题**：当前 `send_request` 每条消息带 `"id"` 并等响应。未来 Scheduler flush 场景可能不需要逐条回复——发了就不管。

**方案**：恢复注释中的 `send_notification(method, params)` ——不带 `"id"` 字段，Qt 执行但不回复。适合高频 fire-and-forget 指令。

**好处**：减少不必要的 pipe 回读，降低延迟。

**建议时机**：出现"发了不管"的场景时（Scheduler batch flush 或高频事件流）。

**相关文件**：`host/src/runtime/widget_qt.rs` 注释

---

### 5.6 插件市场（.bcpkg 远程分发）

**问题**：当前仅支持本地 `plugins/` 目录安装。需要远程分发、审核、签名机制。

**方案**：v2 引入插件市场后端 + 签名校验。`.bcpkg`（zip 格式）复用本地安装流程。

**建议时机**：v2，审核/签名基础设施就绪后。

**相关文件**：本文档 §三

---

### 5.7 Utility Window（WebView Backend）

**问题**：`WindowService` 骨架已完成（create/close/resize/setMinSize/savePosition），但未接入实际的 WebView 后端。插件调用 `BetterCPT.window.create()` 只返回 windowId，不弹出真实窗口。

**方案**：Phase 4 集成 Tauri WebView 作为 Utility Window 后端。每个 `window.create()` 启动一个 Tauri WebView 实例。

**好处**：Script 类型插件（如 pdf-toolkit）可以弹出标准工具窗口，承载 HTML/JS 交互界面。

**建议时机**：Phase 5。Phase 4 Dashboard 集成已完成，但 window_service 未接入 Tauri WebView 后端。

**相关文件**：`host/src/host/window_service.rs` Phase 4 TODO

---

### 5.8 Widget 窗口拖拽移动

**问题**：v1 Widget 位置 hardcode（Dock 底部居中），用户无法移动。

**方案**：Qt 侧捕获 `QEvent::MouseMove` + 左键按下时调用 `QWidget::move()`。需区分"拖拽移动"和"点击交互"，在 `QEvent::MouseButtonPress` 记录起始位置，移动超过阈值后切换为拖拽模式。

**建议时机**：v1 稳定后（Phase 4/5），需要非固定位置 Widget 时。

**相关文件**：`widget-qt/WidgetManager.cpp`

---

### 5.9 崩溃恢复：Replay 不支持先子后父创建顺序

**问题**：`generate_replay_commands()` 三遍遍历（创建→属性→父子关系），假设插件始终先创建父节点再 addChild。若插件先创建子节点再加到父节点，回放时 `addChild` 引用的子节点尚未创建，恢复失败。

**方案**：回放前对 Widget 树做拓扑排序，确保父节点永远在子节点之前创建。

**建议时机**：是否有插件需要先子后父的创建顺序未知（v1 所有插件均为先父后子），第一个触发此问题的插件出现时处理（Phase 5+）。

**相关文件**：`host/src/scheduler/state_tree.rs` `generate_replay_commands()`

---

### 5.10 崩溃恢复：事件回调重新绑定

**问题**：Qt 崩溃恢复后，`generate_replay_commands()` 恢复 Widget 树结构和属性，但不恢复事件回调注册。当前 `onHover`/`onClick` 等回调在 Isolate 的 JS 侧（`globalThis.__widgets[id]._onHover`），Isolate 未崩溃时回调仍在，所以恰好工作。但以下场景不成立：

- 需要 Qt 侧配合的事件（`onMouseMove` 需要 `setMouseTracking(true)`）
- Isolate 和 Qt 同时崩溃时，JS 侧 `__widgets` 映射表也丢失

**方案**：回放第三遍（属性恢复后）追加第四遍——遍历 `__widgets` 映射表，对每个 widget 重新发送事件注册指令（`onHover`/`onClick`/`onMouseMove` 等）。同时 Qt 侧需要支持 `setMouseTracking` 等配置指令。

**建议时机**：Phase 5，多 Isolate 线程隔离方案同期处理（事件路由与 Isolate 线程绑定相关）。

**相关文件**：`host/src/scheduler/state_tree.rs`、`widget-qt/WidgetManager.cpp`

---

### 5.11 SDK：屏幕尺寸查询 API

**问题**：v1 插件开发者无法在 JS 中获取屏幕尺寸。当前 Dock 示例用硬编码 `SCREEN_W = 1920; SCREEN_H = 1080`，换了显示器或分辨率就错位。根源是 SDK 没有暴露 `system.screenSize()` 或其他查询接口，且 Qt 创建 Widget 时的坐标是全屏绝对坐标，开发者必须知道自己屏幕多大才能居中放置。

**方案**：

- 方案 A（推荐）：新增 `system.screenSize()` API，Host 通过 Win32 `GetSystemMetrics(SM_CXSCREEN)` 获取主显示器尺寸，在 Deno 全局对象上注入为只读属性 `system.screenWidth` / `system.screenHeight`。插件启动时即可读取。
- 方案 B：SDK 增加 `system.screenInfo()` 返回多显示器信息（主屏尺寸 + 所有显示器 Rect 列表），为多显示器 Dock 场景做预留。
- 方案 C：支持相对定位语法（`x: "center"` / `y: "bottom"`），Host 侧在 IPC 处理时将逻辑位置转换为屏幕绝对坐标。好处是开发者不需要手算坐标。

**建议时机**：Phase 5，第一个需要动态定位的第三方插件出现时。方案 A 成本最低（Rust 侧 5 行 + JS preamble 1 行），方案 C 用户体验最好但需改 IPC 协议。

**相关文件**：`host/src/runtime/deno_host.rs` `SDK_PREAMBLE`、`host/src/scheduler/state_tree.rs`

---

### 5.12 SDK：storage.get / dialog.confirm 等同步返回值

**问题**：v1 的 `storage.get(key)` 和 `dialog.confirm(msg)` 发出 IPC 命令后立即返回 `undefined`，JS 侧无法获取结果。根因是当前 IPC 架构为单向 fire-and-forget：`__bettercpt_send()` 通过 V8 callback 直接把命令写入 mpsc channel，不等待 Qt/Rust 侧的响应。对比 `console.log`（纯 JS 无 IO）和 `system.launch`（不期待返回值），storage/dialog 的使用场景需要拿到结果才能继续逻辑。

**方案**：

- 方案 A（推荐）：将 `send_fn` 改为 Rust 侧持有 Promise resolve/reject 句柄。`send(method, params)` 返回一个 Promise，Rust 在收到 Qt 响应（或超时）后调用 V8 `resolve`/`reject`。需要改用 `#[op2(async)]` 或 `deno_core::op2` 的异步机制，放弃当前的手动 v8 注入。
- 方案 B：给 `storage.get` 专用一个同步通道——在 Isolate 执行上下文中直接读写 Rust 侧 `HashMap<String, String>`，不经过 IPC。`dialog.confirm` 则用 Win32 `MessageBoxW` 同步阻塞（在独立线程调用，通过 oneshot channel 回传结果）。

**影响**：这是 5.4（deno_core `#[op2]` 标准注册）的前置依赖——回归标准 op2 之前，需要确认 op2 的异步模式能支撑 Promise 返回值。

**建议时机**：Phase 5，与 5.4 同步处理。

**相关文件**：`host/src/runtime/deno_host.rs` `JsRuntimeHost::new()`、`host/src/runtime/storage.rs`

---

### 5.13 SDK：布局引擎（相对定位语法）

**问题**：v1 所有 Widget 使用屏幕绝对坐标定位，开发者需要手动计算每个 Widget 的 x/y。`SCREEN_W`、居中偏移、间距全在 JS 里算。UI 越复杂，坐标计算越脆弱。这是故意的——v1 红线明确不做声明式框架。但反馈表明即使不做完整布局引擎，也需要极简的辅助定位能力。

**方案**：不引入完整布局引擎（flexbox/auto-layout），只支持两种逻辑定位：

- 位置锚点：`x: "center"` / `y: "bottom"` — Host 侧在 IPC 处理时将逻辑位置转换为屏幕绝对坐标
- 相对父容器：`addChild` 时子节点坐标自动相对父节点左上角偏移（而非当前的全屏绝对坐标）

**红线保持**：不引入 flex/grid/auto-layout、不做 wrapping/reflow、不做 min/max constraints、不做 spacing/gap 自动计算。这两个原语只解决"居中"和"跟随"两个最常见的手算场景。

**建议时机**：Phase 5，与 5.11 屏幕尺寸 API 配合使用。先有 screenSize 让开发者能算，再有 anchor 让开发者不用算。

**相关文件**：`host/src/ipc/messages.rs`（新增 `setAnchorX` / `setAnchorY` 指令）、`widget-qt/WidgetManager.cpp`、`host/src/scheduler/state_tree.rs` `apply_ipc()`

---

### 5.14 SDK：JS 布局工具函数（纯 JS 方案，替代 5.13 的 Host 侧方案）

**问题**：同 5.13——开发者需要手算每个 Widget 的坐标。但与 5.13 的 Host 侧 IPC 改动不同，此方案只在 JS SDK 层提供纯函数封装。

**方案**：SDK 提供布局工具函数，在 JS 侧完成坐标计算后，仍通过现有的 `x/y` 参数传给 Host。Qt 侧零改动。

```javascript
// sdk/src/layout.js（Phase 5 随 SDK 发布）
function hStack(container, items, options = {}) {
    const { gap = 12, padding = 16 } = options;
    let x = padding;
    const centerY = container.height / 2;
    const widgets = [];
    items.forEach(item => {
        const sz = item.size || 48;
        const y = centerY - sz / 2;
        const w = createIcon({ ...item, size: sz, x, y, containerId: container.id });
        widgets.push(w);
        x += sz + gap;
    });
    return widgets;
}

function vStack(container, items, options = {}) { /* 同样模式，y 递增 */ }
```

插件开发者用 `hStack(dockBg, icons, { gap: 12 })` 替代手动循环计算坐标。

**好处**：
- Qt / IPC / Host 全部零改动——仅 JS SDK 增加一个文件
- 灵活——插件可以不调用工具函数，手动算坐标
- 纯 JS——布局算法不受 Qt/C++ 约束
- 不违反 v1 红线——不是声明式框架，只是 JS 辅助函数

**对比 5.13（Host 侧锚点方案）**：
| | 5.13 Host 锚点 | 5.14 JS 布局库 |
|---|---|---|
| Qt 改动 | 需要 | 零 |
| IPC 改动 | 新增指令 | 零 |
| 灵活性 | 固定的 anchor 语法 | 任意 JS 逻辑 |
| 插件可覆写 | 否 | 是 |

**建议时机**：Phase 5，插件生态超过 5 个插件且手算坐标成为显著痛点时。优先采用 5.14，因为它零架构改动。

**相关文件**：`sdk/src/layout.js`（新增）

---

### 5.15 Plugin Capability：system:lnk-resolve / system:file-icon

**问题**：Dock 插件需要解析 `.lnk` 快捷方式和提取 `.exe` 内嵌图标。v1 当前方案是 Qt 侧 `createIcon` 内部完成——检测 `.lnk` 后缀 → `IShellLinkW` COM 解析 → `QFileIconProvider::icon()` 提取。这绕过了 SDK 和 Capability 模型，是临时方案。

**当前 v1 方案**：`widget-qt/WidgetManager.cpp` `createIcon` 里 `.lnk` 分支，约 15 行 Qt/Win32 COM 代码。

**局限性**：
- Qt 专属——换到 DComp 后端需要重新实现
- 未走 Capability 模型——插件 manifest 里不声明也能用
- 无法扩展到其他系统级能力

**方案**：Phase 5 正式将这两个能力提升为 Plugin Capability：

```json
{
    "permissions": ["system:lnk-resolve", "system:file-icon"]
}
```

Host 通过 Deno op 暴露 `system.resolveLnk(path)` 和 `system.extractIcon(path)`，返回 Promise。实现与渲染后端无关——Windows 走 COM，未来 Linux 走 `.desktop` 解析。

**前置依赖**：5.4（deno_core `#[op2]` 标准注册）和 5.12（op2 Promise 返回值）。当前 v8 注入是 fire-and-forget，不支持 `async fn` 的 `Resolve` 回调。回归 `#[op2]` 后才能实现 Promise-based Capability API。

**迁移路径**：
1. Phase 5 实现 `#[op2]` + Promise 返回值
2. 将 `WidgetManager.cpp` 中的 `.lnk` 逻辑抽取为独立能力
3. Dock manifest 声明 `system:lnk-resolve` + `system:file-icon`
4. 删除 `createIcon` 中的 Qt 侧 `.lnk` 分支

**建议时机**：Phase 5，与 5.4 / 5.12 同步处理。

**相关文件**：`widget-qt/WidgetManager.cpp`（待删除的临时分支）、`host/src/runtime/deno_host.rs`（新增 Deno op）、`plugins/dock/manifest.json`（新增 permissions）

---

### 5.15 Hover 标签：顶层窗口透明动画不生效

**问题**：Dock 图标 hover 时需要显示应用名称标签（Mac 风格 tooltip）。标签是 `createText` 创建的 `QLabel`，带 `FramelessWindowHint`。`setOpacity` 通过 `QGraphicsOpacityEffect` 设置透明度，但 `QGraphicsOpacityEffect` 对 Windows 上层窗口无效——渲染到离屏 buffer 后 DWM 不拾取 alpha 通道，标签无法淡入淡出。

**方案**：
- 方案 A：标签不设为独立顶层窗口，改为 Dock 背景的子 Widget（`containerId`）。子 Widget 在父容器内渲染，`QGraphicsOpacityEffect` 正常工作。需调整 Dock 高度为标签预留空间。
- 方案 B：用 `setWindowOpacity`（DWM 窗口级透明度）代替 `QGraphicsOpacityEffect`。标签作为独立窗口，通过 `SetLayeredWindowAttributes` 实现平滑淡入淡出。

**建议时机**：Phase 5。

**相关文件**：`widget-qt/WidgetManager.cpp`（`setOpacity` 方法）、`plugins/dock/index.js`

---

### 5.16 SDK：`setTimeout` / `setInterval` Polyfill

**问题**：`deno_core` 不包含 `setTimeout` 和 `setInterval` API。Dock 弹跳动画（点击 → 缩小 → 弹回）和任何延时操作都依赖此 API。当前方案是去掉弹跳，仅保留直接启动。

**方案**：
- 在 Rust 侧注册 `op_set_timeout(delay_ms)` Deno op，接收延时参数
- Host 主循环维护一个 `Vec<TimerEntry>`（timer_id, plugin_id, deadline, callback_id）
- 主循环每次 tick 检查到期 timer，调用 `isolate_pool.fire_custom_event` 执行回调
- SDK preamble 注入 `setTimeout(fn, ms)` → 发送 `setTimeout` 命令到 Rust → 到期后执行 `globalThis.__timers[id]()`
- `clearTimeout` 同理

**建议时机**：Phase 5，与 5.4（`#[op2]` 标准注册）同步处理——`setTimeout` 是第一个需要异步回调的 op，是验证 op2 异步能力的好用例。

**相关文件**：`host/src/runtime/deno_host.rs`（新增 op2 op）、`host/src/main.rs`（主循环 timer 检查）、`plugins/dock/index.js`（恢复弹跳动画）

---

### 5.17 App 自动更新（Tauri updater plugin）

**问题**：v1 无内置更新机制。用户安装后，软件版本永久停留在安装时的版本。插件可通过 Dashboard 手动更新，但 Host 本身无法更新。

**方案**：接入 `tauri-plugin-updater`。Tauri v2 原生支持，无需自研。

```json
// tauri.conf.json
"plugins": {
    "updater": {
        "endpoints": ["https://releases.bettercpt.com/update.json"],
        "pubkey": "<Ed25519 public key>"
    }
}
```

用户流程：Dashboard → 设置页 → "检查更新" → 检测到新版本 → 下载 → 安装 → 重启。

**好处**：
- 零研发成本——Tauri 团队维护
- 签名校验——公钥验证更新包，防篡改
- 增量更新——只下载差异部分

**前置依赖**：需要部署一个静态文件服务器存放 `update.json` + 安装包签名。

**分发 CDN 备选方案**：
- **方案 A（推荐）**：GitHub Releases + Gitee 镜像双推。国内用户直连 Gitee，延迟低且免费。
- **方案 B（用户量上来后）**：阿里云 OSS / 腾讯云 COS，按流量付费，v1 无用户时月费可忽略。

**建议时机**：Phase 5，首个公开发布版本之后。v1 内测阶段用户可手动下载安装包覆盖安装。

**相关文件**：`host/tauri.conf.json`（plugins 配置）、`host/src-tauri/capabilities/default.json`（updater 权限）
