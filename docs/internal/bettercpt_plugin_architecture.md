# BetterCPT 插件架构设计文档

> 最后更新：综合 Claude + ChatGPT 多轮讨论结果（2026-05-15）

---

## 一、整体架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                   BetterCPT Host（Rust + tokio）             │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │Widget Runtime │  │Script Runtime│  │  Native Runtime  │  │
│  │  (常驻后台)  │  │ (Deno Core) │  │  ⚠️ 默认关闭    │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
│         │                 │                    │            │
│  Qt 子进程(C++)     单一 Deno 实例         独立子进程       │
│  JSON-RPC IPC       嵌入 Rust Host         任意语言         │
│  独立/共享模式       IPC 通信              需手动授权        │
└─────────────────────────────────────────────────────────────┘
         │                 │                    │
    桌面 Widget         功能型工具            系统级插件
  Dock / 卡片 / Overlay  PDF / 自动化        游戏脚本等
```

---

## 二、三层插件 Runtime

### Layer 1：Widget Runtime（原生常驻层）

**定位**：负责所有常驻桌面的视觉组件，内存优先

**技术（v1）**：
- Qt（C++）作为 Widget 渲染引擎，以独立子进程方式运行
- 通过 JSON-RPC IPC 与 Rust Host 通信，和 Native Runtime 同一种隔离模型
- 每个桌面常驻 Widget 可独立为 Qt 子进程，崩溃互不影响
- 所有 Widget 的**业务逻辑**仍在 Deno Core（TS）中运行；Qt 只负责渲染
- Widget 之间通过 Deno module 命名空间隔离 + Qt 进程隔离，两层保护

**为什么选 Qt 做 Widget 渲染引擎**：

- 已验证：开发者此前用 Qt 实现过 Dock，实测内存极低
- 场景图（QWidget 层级）、动画系统（QPropertyAnimation）、渲染循环（QApplication 事件循环）均框架自带——v1 零自建成本
- 文档和社区极其成熟，AI 辅助可靠性高
- 独立进程模型天然解决崩溃隔离：一个 Widget 崩溃不影响其他

**两种部署模式**：

| 模式 | 内存（10 个 Widget） | 崩溃隔离 | 适用场景 |
|---|---|---|---|
| 共享 Qt 进程 | ~30-50MB | 一个崩全崩 | Dock + 轻量卡片，追求极致内存 |
| 每 Widget 独立 Qt 进程 | ~100-120MB | 一个崩不影响其他 | 复杂 Widget，追求稳定性 |

具体选择需在 Phase 0 Spike 中实测后决定。推荐默认使用**共享 Qt 进程模式**（与当前架构"所有 Widget 共享 Deno 实例"的设计一致），但架构上保持"可随时切为独立进程"的能力。

**关键架构决策：Qt 和 Rust 的事件循环不协同**

Qt 子进程有自己完整的 `QApplication::exec()` 事件循环。Rust Host 有自己的 tokio 运行时。两者仅通过 JSON-RPC IPC 通信，不存在事件循环冲突。Qt 对 Rust Host 而言就是一个普通的子进程——和 Native Runtime 子进程、Script Runtime 的 Deno 实例没有本质区别。

**关键架构决策：GPU Context 不跨进程共享**

每个 Qt 子进程有自己独立的渲染上下文（OpenGL 或 Direct3D，由 Qt 自动管理）。DWM 在合成层面统一处理所有窗口的最终显示。v1 不需要跨进程 GPU 资源共享。

**v2+ 引入 DComp 时**，Qt 窗口可渲染到离屏 Surface → 通过 DXGI 共享句柄传给 Rust Host → Host 创建 DComp Visual 绑定该 Surface。这是成熟的跨进程 GPU 共享技术（`IDXGIResource::GetSharedHandle`），但 v1 完全不需要。

**v1 为什么不用 DComp / D2D+DXGI 直接手写渲染**：

- Qt 提供了 D2D+DXGI 方案中需要自建的场景图、动画系统、渲染循环——这些在 Qt 中都是框架自带
- 手写 GPU 渲染意味着"写 Runtime"；用 Qt 意味着"写应用"——后者的开发效率高一个数量级
- 开发者已有 Qt 实战经验，学习成本为零

**v1 为什么不用 WebView**：

- 每个 WebView 渲染进程 ~15-25MB，10 个 Widget 总计 200-320MB
- WebView 的内存模型不适合常驻后台的桌面 Widget
- Overlay 点击穿透、z-order 控制在 WebView 上远不如原生窗口可控
- WebView 在 BetterCPT 中的正确位置是 Dashboard（偶尔打开的配置界面）

**适用插件类型**：
- macOS 风格 Dock
- Dynamic Island（灵动岛）
- 天气卡片、日历卡片
- 系统监控卡片
- 音乐控制组件
- 音量浮层、通知中心

**开发者体验**：
- 使用命令式 Widget SDK（JS/TS）编写
- SDK 直接操作对象，不做 virtual tree / diff / reconciliation
- 通过 IPC 告知 Runtime 渲染指令
- 开发者完全不接触 Direct2D / DXGI / GPU 渲染细节

**内存目标**：
- 后台常驻 10 个 Widget 总计 < 100MB
- Deno Core 单实例固定开销，不随 Widget 数量线性增长

**⚠️ SDK 设计红线（第一阶段严格遵守）**：
- 不做 virtual tree
- 不做 diff / reconciliation
- 不做 hooks / reactive graph / fiber
- 先用命令式 API 跑通，声明式语法糖是后期的事

---

### Layer 2：Script Runtime（脚本功能层）

**定位**：负责功能型工具插件，按需启动，用完释放

**技术**：
- 嵌入 Deno Core（独立实例，与 Widget Runtime 分开）
- 沙盒隔离，权限由 manifest 声明
- IPC / JSON-RPC 与 Host 通信

**适用插件类型**：
- PDF 处理（转换、合并、拆分）
- 文件格式转换
- 自动化脚本
- 数据处理工具

**开发者体验**：
- 使用标准 TypeScript 编写
- 原生支持文件系统、网络等 Deno API
- 权限由 manifest 显式声明，Host 审批

**为什么 Script Runtime 用 Deno 而不是 QuickJS**：
- 功能插件不常驻，内存不是首要问题
- 需要完整 JS 生态（文件处理、网络、加密等）
- TypeScript 原生支持，开发者体验好
- QuickJS 生态贫乏，会严重限制插件能做的事

---

### Layer 3：Native Runtime（系统级层）⚠️

**定位**：负责需要完整系统权限的高级插件

**⚠️ 这是整个系统最大的安全风险点，MVP 阶段不实现，默认关闭**

**技术**：
- 独立子进程，任意编译型语言
- 标准 IPC 协议与 Host 通信

**适用插件类型**：
- 游戏脚本（内存读写、进程注入）
- 输入设备控制
- 系统底层操作

**开启条件（必须同时满足）**：
- 用户手动开启开发者模式
- 插件经过签名验证
- 用户手动二次确认授权

**已知风险（用户必须知晓）**：
- 可能进行键盘监听、进程注入
- 可能读取敏感文件或 Token
- 可能绕过权限系统

**后期必须补充**：
- 插件签名系统和官方审核机制
- 安装时强制风险提示
- 运行时行为监控

---

## 三、插件选型对照表

| 需求 | 推荐 Runtime | 原因 |
|---|---|---|
| 常驻桌面卡片 | Widget Runtime | 内存低，GPU 渲染 |
| Dock 栏 | Widget Runtime | 需要高帧率动画 |
| PDF 工具 | Script Runtime | 文件操作，按需启动 |
| 自动化脚本 | Script Runtime | JS/TS 生态丰富 |
| 游戏脚本 | Native Runtime ⚠️ | 需要进程级权限，默认关闭 |
| 系统监控后台 | Native Runtime ⚠️ | 长驻进程，高权限，默认关闭 |

---

## 四、统一 manifest 格式

```json
{
  "id": "weather-card",
  "name": "天气卡片",
  "version": "1.0.0",
  "author": "developer-name",
  "runtime": "widget",
  "permissions": [
    "network:fetch",
    "storage:read",
    "storage:write"
  ],
  "entry": "index.ts",
  "widget": {
    "defaultSize": { "width": 200, "height": 120 },
    "resizable": false,
    "alwaysOnTop": true
  }
}
```

**runtime 字段**：`"widget"` / `"script"` / `"native"`（native 需开发者模式）

**权限系统**：
- 插件只能申请 manifest 中声明的权限
- Host 安装时向用户展示权限列表
- 运行时拦截所有未声明权限的调用
- Native 插件额外需要用户手动二次确认

**权限列表**：

```
fs:read          → 读取文件
fs:write         → 写入文件
network:fetch    → 网络请求
dialog:open      → 打开文件对话框
dialog:save      → 保存文件对话框
system:info      → 读取系统信息
widget:render    → 创建和管理 Widget（Widget Runtime 专属）
widget:overlay   → Overlay 窗口权限（Widget Runtime 专属）
```

### Runtime 权限上限（Permission Ceiling）

每种 Runtime 类型有**硬编码的权限上限**，manifest 中声明的权限超出上限时，Host 拒绝安装。这个上限不是"建议"，是强制规则。

| Runtime | 权限上限 | 适用场景 |
|---|---|---|
| Widget | `widget:render`、`widget:overlay`、`network:fetch`（仅限天气/日历类 API） | 纯视觉组件，不触碰文件系统 |
| Script | `fs:read`、`fs:write`、`network:fetch`、`dialog:open`、`dialog:save`、`system:info` | 功能型工具插件 |
| Native | 全权限（需开发者模式 + 签名 + 二次确认） | 系统级操作 |

**Widget Runtime 的强制约束**：

Widget 插件**禁止**声明以下权限，即使写在 manifest 中也会被 Host 拒绝安装：

- `fs:write` — Widget 不应该是文件编辑器
- `fs:read` — 数据获取通过 SDK 内置 API（如 `weather.getCurrent()`），不直接读文件
- `dialog:open` / `dialog:save` — 文件对话框是工具型操作，Widget 不需要
- `system:info` — 系统信息通过 SDK 内置 API 获取，不直接调用

Widget 允许的唯一外部权限：`network:fetch`，且仅限以下用途的白名单域名：

```json
{
  "permissions": [
    "network:fetch"
  ],
  "network": {
    "allowedDomains": ["api.weather.com", "api.calendar.com"]
  }
}
```

超出白名单的网络请求被 Host 拦截。

**为什么这样设计**：

Widget 的本质是"常驻桌面的视觉组件"。如果不加权限上限，Widget 开发者会逐渐加入文件操作、系统调用、任意网络请求——Widget 最终退化为功能插件，失去"轻量常驻"的意义。用户安装一个天气卡片时，它不应该能读写磁盘。

如果开发者确实需要 `fs:write` 等能力，他们应该写一个 Script 插件，而不是 Widget。Script 插件按需启动、用完释放，权限范围天然更大。

**权限上限的检查时机**：

```
安装插件
  → 解析 manifest.json
  → 提取 runtime 字段，确定权限上限表
  → 逐一检查 permissions 数组中的每一项：
      ✅ 在权限上限内 → 通过
      ❌ 超出上限 → 拒绝安装，显示错误信息
  → Native Runtime 额外弹出二次确认
```

### Capability Layer（能力注册层）

当前权限是扁平字符串（`fs:read`），但随着系统增长，会出现更复杂的能力：Win32 API、Shell API、Clipboard、Overlay Capture 等。如果继续用扁平模型，"权限系统越来越乱"是必然结果。

**三层模型**：`Permission → Capability → Runtime Binding`

```
manifest.json 声明：
  permissions: ["fs:read", "network:fetch"]

        ↓ 解析

Capability Registry（Host 内部注册表）：
  "fs:read"    → Capability::FileSystem { access: Read }
  "network:fetch" → Capability::Network { domains: AllowedList }

        ↓ 运行时绑定

Runtime Binding（每个 Runtime 类型有自己的实现）：
  Script Runtime: Deno op → fs:read 绑定到 op_fs_read()
  Widget Runtime: SDK API → network:fetch 绑定到 weather.getCurrent()
  Native Runtime: IPC → 直接透传给子进程
```

**Capability 定义**（Rust 侧 trait）：

```rust
/// 一个 Capability 代表 Host 提供的一项"能力"
/// 每个权限字符串映射到一个 Capability
trait Capability {
    /// 唯一标识符，对应 manifest 中的权限字符串
    fn id(&self) -> &str;

    /// 该能力在指定 Runtime 类型下的绑定
    fn bind_for_runtime(&self, runtime: RuntimeType) -> Result<Box<dyn CapabilityBinding>>;

    /// 能力依赖的其他能力（如 clipboard 依赖 system:info 检测剪贴板可用性）
    fn dependencies(&self) -> Vec<&str> { vec![] }
}

/// 能力的运行时绑定：Deno op / SDK API / IPC command
trait CapabilityBinding {
    /// 在 Deno Core 中注册此能力（Script Runtime 用）
    fn register_deno_op(&self, runtime: &mut deno_core::JsRuntime);

    /// 注入到 TS SDK 全局对象（Widget Runtime 用）
    fn inject_sdk_api(&self, sdk: &mut SdkGlobal);

    /// 暴露为 IPC command（Native Runtime 用）
    fn ipc_methods(&self) -> Vec<String>;
}
```

**当前 Capability Registry**：

| Permission | Capability | Script 绑定 | Widget 绑定 | Native 绑定 |
|---|---|---|---|---|
| `fs:read` | FileSystemRead | Deno op `op_fs_read` | ❌ 禁止 | IPC `fs.readFile` |
| `fs:write` | FileSystemWrite | Deno op `op_fs_write` | ❌ 禁止 | IPC `fs.writeFile` |
| `network:fetch` | NetworkFetch | Deno op `op_net_fetch` | SDK `weather.getCurrent()` | IPC `net.fetch` |
| `dialog:open` | DialogOpen | Deno op `op_dialog_open` | ❌ 禁止 | IPC `dialog.open` |
| `dialog:save` | DialogSave | Deno op `op_dialog_save` | ❌ 禁止 | IPC `dialog.save` |
| `system:info` | SystemInfo | Deno op `op_sys_info` | ❌ 禁止 | IPC `sys.info` |
| `widget:render` | WidgetRender | ❌ 禁止 | SDK `createRect/createIcon/...` | ❌ 禁止 |
| `widget:overlay` | WidgetOverlay | ❌ 禁止 | SDK 窗口管理 | ❌ 禁止 |

**未来扩展例子**：

```
v2 新增:
  "clipboard:read"  → Capability::Clipboard { access: Read }
  "shell:execute"    → Capability::Shell { allowedCommands: [...] }
  "overlay:capture"  → Capability::OverlayCapture { requireUserConsent: true }
```

新增一个能力只需：实现 `Capability` trait → 注册到 Registry → 各 Runtime 实现对应 binding。权限检查逻辑不变，因为检查的是 capability id 是否在 manifest 的 permissions 列表里，而这个流程已经在安装时跑通了。

**为什么现在就用 trait 而不是等 v2**：

目前只有 8 种权限，用 match 语句硬编码也能跑。但随着能力增长（v2 的 clipboard、shell、overlay capture；v3 的 input hook、screenshot 等），match 会膨胀为不可维护的 50+ 分支。`Capability` trait 把"能力的定义"和"能力的权限检查"解耦，新增能力不需要改权限系统的代码。

---

## 五、Widget SDK 设计原则

### 第一阶段：严格使用命令式 API

开发者直接操作对象，SDK 映射到 Host 侧的渲染操作，没有中间层：

```typescript
import { createRect, createText } from "bettercpt-sdk"
import { weather } from "bettercpt-sdk/system"

// 创建元素
const card = createRect({ blur: true, radius: 16, width: 200, height: 120 })
const temp = createText(weather.temp + "°C", { size: 32, weight: "bold" })
const city = createText(weather.city, { size: 14, color: "#888888" })

// 组织层级
card.addChild(temp)
card.addChild(city)

// 注册交互（触发 Host 侧动画系统）
card.onHover(() => card.setScale(1.02, 150))
card.onLeave(() => card.setScale(1.0, 150))
```

### SDK 内部工作流程（v1：Qt 渲染后端）

```
开发者调用命令式 SDK API（TS）
        ↓
Deno Core 执行（所有 Widget 共享单实例）
        ↓
SDK 通过 IPC（JSON-RPC）发送渲染指令给 Rust Host
        ↓
Rust Host 转发渲染指令给 Qt 子进程（JSON-RPC IPC）
        ↓
Qt 子进程执行渲染操作（QWidget / QPropertyAnimation）
        ↓
Qt 渲染到屏幕（GPU 加速，由 Qt 自动管理）
```

**v1 零自建模块**。场景图、动画系统、渲染循环全部由 Qt 提供。Rust Host 的角色是 IPC 消息代理 + 生命周期管理者，不直接参与渲染。

---

## 六、Host 生命周期管控

```
安装插件
  → 验证 manifest 格式
  → Native Runtime：弹出安全警告，要求用户确认
  → 审核权限列表，注册插件

启动插件
  → Widget：加入共享 Deno 实例 + 通过 IPC 向 Qt 进程注册新 Widget
  → Script：启动独立 Deno 实例
  → Native：检查签名 → 启动子进程
  → 建立 IPC 通道，开始心跳监控

运行中监控
  → 心跳检测（每 5 秒）
  → 内存 / CPU 监控
  → 崩溃自动恢复

停止插件
  → 发送停止信号，等待清理（最多 3 秒）
  → 超时强制终止，回收资源，注销 IPC 通道
```

**崩溃隔离策略**：

| Runtime | 崩溃影响范围 | 恢复策略 | 最大重启次数 |
|---|---|---|---|
| Widget Qt 进程（共享模式） | 所有 Widget 的渲染层 | 重启 Qt 进程 + 重新注册所有 Widget | 5次 |
| Widget Qt 进程（独立模式） | 单个 Widget | 3秒后自动重启该 Widget 的 Qt 进程 | 5次 |
| Widget Deno 实例 | 所有 Widget 的业务逻辑（TS） | 重启整个 Deno 实例，Qt 进程不受影响 | 5次 |
| Script Runtime | 单个插件进程 | 3秒后自动重启 | 5次 |
| Native Runtime | 单个子进程 | 通知用户，不自动重启 | 0次 |

### 崩溃恢复顺序（Host Recovery Sequence）

恢复不是"重启进程就行"，顺序错了会导致动画中间状态、Hover 状态、焦点状态漂移。

**Qt 进程崩溃恢复顺序**：

```
1. Host 检测到 Qt 进程退出（IPC pipe 断开 / try_wait 返回 Exit）
         ↓
2. Host 冻结所有 Widget 相关 IPC
   → 阻塞 TS → Host 的渲染命令（入队不执行）
   → 向 Deno 推送 "qt.recovering" 事件，Deno 侧暂停渲染调用
         ↓
3. Host 标记状态树为 "recovering"，暂停版本号递增
   → 记录崩溃前的状态树快照（此时所有 Widget 的最终状态是可靠的）
         ↓
4. Host 启动新的 Qt 子进程
   → 等待 Qt 进程 ready 信号（QApplication 启动完成）
   → 建立新的 IPC pipe
         ↓
5. Host 全量同步状态树到新 Qt 进程
   → 按状态树深度优先遍历，逐一发送 create 指令
   → 每条 create 指令携带该 Widget 的完整属性（bounds、scale、blur、opacity）
   → 注意：动画中间状态丢失，发送的是动画的**起始值**
         ↓
6. Host 重放交互状态
   → 重新注册 Hover 区域（Qt 侧 setMask / enterEvent 区域）
   → 重新注册 Click 回调
   → 焦点状态重置为默认（崩溃前哪个图标有焦点不可知）
         ↓
7. Host 标记状态树为 "active"，恢复版本号递增
         ↓
8. Host 解除 IPC 冻结
   → 将步骤 2 中入队的命令逐条执行
   → 向 Deno 推送 "qt.recovered" 事件
         ↓
9. 恢复完成。用户看到窗口在 500ms-2s 内重新出现。
```

**关键约束**：

- 步骤 2 的 IPC 冻结时间应 < 2 秒。如果 Qt 启动超过 5 秒，Host 放弃本次恢复，通知用户。
- 步骤 5 的全量同步中，动画状态回退到起始值。如果崩溃前 `icon-1` 正在从 scale 1.0 动画到 1.2 且进行到一半——恢复后 `icon-1` 显示为 scale 1.0，不会自动重新播放动画。依赖动画完成事件的逻辑（如"动画结束后打开应用"）不会误触发。
- 步骤 6 的焦点重置：不尝试恢复崩溃前的焦点状态（无法可靠获取），统一重置为无焦点。

**Deno 实例崩溃恢复顺序**（更简单，因为 Qt 进程仍在运行）：

```
1. Host 检测到 Deno task panic
         ↓
2. Host 向 Qt 推送 "deno.recovering"，Qt 保留当前渲染但不响应交互
         ↓
3. Host 重启 Deno 实例，重新加载 TS 入口文件
         ↓
4. TS 侧重新执行 createRect / createIcon 等创建调用
   → Host 检测到 id 匹配已有 Widget → 不重复创建 QWidget
   → Host 将已有 Widget 的当前状态返回给 TS SDK 对象
         ↓
5. Host 重新绑定事件回调（onHover / onClick / onLeave）
         ↓
6. Host 向 Qt 推送 "deno.active"，恢复交互
```

**Deno + Qt 同时崩溃**（最坏情况，概率极低）：

先执行 Qt 恢复（步骤 1-9），Qt 恢复完成后立即执行 Deno 恢复。Deno 恢复在 Qt 已恢复的基础上进行，TS 侧创建调用匹配到已存在的 QWidget。

### Deno 实例 Watchdog（执行超时保护）

所有 Widget 共享一个 Deno Core 实例。如果一个 Widget 的代码进入死循环：

```typescript
// 某个天气插件的 bug
while (true) { /* 永远不会返回 */ }
```

整个 Deno 事件循环被阻塞——所有 Widget 的交互全部冻结。Qt 进程仍在运行（窗口还在、动画继续、hover 仍能检测），但 TS 侧的所有回调都不会执行。

**三层保护机制**：

**1. Execution Timeout（执行超时）**

Deno Core 支持 per-op 级别的超时。对每个从 TS 调用的 SDK 方法设置硬超时：

```rust
// Host 侧：每个 Deno op 执行前设置超时
let timeout = tokio::time::timeout(
    Duration::from_millis(500),  // 单个 op 执行不超过 500ms
    js_runtime.execute_script(script)
);

match timeout.await {
    Ok(result) => { /* 正常 */ }
    Err(_) => {
        // 超时 → 记录日志、终止当前 op、通知用户
        log::error!("Widget op timed out after 500ms");
    }
}
```

**2. Microtask Budget（微任务预算）**

在每轮事件循环中限制微任务的总执行次数：

```rust
// V8 / Deno Core 中设置 microtask 预算
js_runtime.set_microtask_budget(10_000);  // 每轮最多 10000 个微任务
```

超过预算时 Deno Core 会中断当前执行并返回错误，而不是继续阻塞。

**3. Event Loop Watchdog（事件循环看门狗）**

Rust Host 侧独立于 Deno 的监控 task：

```rust
tokio::spawn(async move {
    let mut last_heartbeat = Instant::now();

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;

        // 通过 IPC 向 Deno 发送心跳请求
        if let Err(_) = deno_heartbeat_tx.send(()).await {
            // Deno 已崩溃，触发恢复
            break;
        }

        // 检查 Deno 响应
        match tokio::time::timeout(Duration::from_millis(200), deno_heartbeat_rx.recv()).await {
            Ok(Some(())) => {
                last_heartbeat = Instant::now();  // 正常
            }
            _ => {
                // 超时未响应
                if last_heartbeat.elapsed() > Duration::from_secs(5) {
                    log::error!("Deno event loop blocked for 5+ seconds, triggering restart");
                    // 触发 Deno 崩溃恢复流程
                    trigger_deno_recovery().await;
                }
            }
        }
    }
});
```

**Watchdog 触发后的处理**：

Watchdog 检测到 Deno 事件循环阻塞 → 不等待 Deno 自己恢复 → 直接触发 Deno 崩溃恢复流程（已有定义，在"崩溃恢复顺序"的 Deno 部分）。恢复期间 Qt 进程不受影响，窗口仍在显示。

**重要**：Watchdog 的 5 秒阈值需要平衡误报和用户体验。正常的 Widget JS 代码（天气 API 调用、DOM 计算）在 500ms 内完成。5 秒的阈值足够宽松，避免网络慢导致误杀，但也足够快让用户不会觉得"卡死了"。

---

## 七、Runtime 状态所有权（Source of Truth）

这是整个多进程架构中最关键的架构决策。当前系统涉及三个可能持有 Widget 状态的位置：

| 层 | 组件 | 持有内容 |
|---|---|---|
| TS SDK（Deno Core） | `createRect(...)` 返回的对象 | Widget 配置（位置、大小、样式） |
| Rust Host | Widget Registry | 所有 Widget 的完整状态树 |
| Qt 子进程 | QWidget 实例 | 窗口句柄、渲染属性、动画状态 |

**v1 明确规则：Host 是唯一真相源（Single Source of Truth）。**

```
TS SDK（Deno Core）
  │  发出命令（createRect / setScale / onHover）
  │  不持有最终状态
  ↓
Rust Host — Widget State Tree（唯一状态）
  │  所有 Widget 的 id、bounds、scale、blur、children、事件绑定
  │  状态变更时递增版本号
  ↓
Qt 子进程 — Render Replica（纯渲染镜像）
  │  接收 Host 同步的状态，创建/更新 QWidget
  │  不保存业务状态
  │  Qt 崩溃 → Host reply 即可完整恢复
```

### 为什么必须这样定义

**热更新场景**：开发者修改 TS 代码 → Deno 重新加载模块 → TS 侧对象全部重建。如果 TS 持有状态，热更新会丢失所有 Widget 配置。Host 持有状态 → Deno 重启后重新 attach → Host 将当前 Widget 树推送给新的 Deno 实例 → 状态无丢失。

**Qt 崩溃恢复场景**：Qt 进程崩溃 → 所有 QWidget 销毁。如果 Qt 持有状态，恢复需要从某个地方重建。Host 持有状态 → 重启 Qt 进程 → Host 将当前 Widget 树完整同步给新 Qt 进程 → 所有 Widget 窗口恢复显示。

**DComp 替换 Qt 场景（v2+）**：Qt 是 Render Replica → 替换为 DComp 时，只需将 Host State Tree 的同步目标从 Qt IPC 改为 DComp Visual 操作。SDK 开发者 API 不变，TS 代码不变，Host 状态树不变。如果 Qt 持有业务状态，这个替换就不可能做到无缝。

**Deno 崩溃恢复场景**：Widget Deno 实例崩溃 → 所有 TS 对象销毁，但 Qt 进程仍在运行，窗口仍在显示。Host 持有状态 → 重启 Deno → 重新加载 TS 代码 → TS 侧重新创建 SDK 对象 → Host 检测到这些对象对应已有的 Widget（id 匹配）→ 不重复创建 QWidget，只重新绑定事件回调。

### 状态同步协议

Host State Tree 是唯一的写入口。所有状态变更必须经过 Host：

```
TS 侧：
  dock.addChild(icon)                          // SDK 方法调用
  → IPC: {"method":"widget.addChild","params":{"parentId":"dock","childId":"icon-3"}}

Host 侧：
  widget_tree.add_child("dock", "icon-3")      // 更新状态树
  → IPC → Qt: {"method":"addChild",...}        // 同步到渲染镜像
  → 递增版本号
```

**关键约束**：

- TS SDK 的方法调用是**命令（Command）**，不是直接状态修改。Host 决定是否接受、如何执行。
- Qt 永远不主动修改状态。Qt 只响应 Host 的同步指令。Qt 侧的 QWidget 属性变更（如用户拖动窗口）通过 IPC 回传 Host，Host 更新状态树后再确认。
- 状态树版本号用于崩溃恢复：Qt 重启后，Host 检查 Qt 侧的版本号 → 如果落后，全量同步当前状态树。

### 各层对状态的权利

| | 读状态 | 写状态 | 持久化状态 |
|---|---|---|---|
| TS SDK（Deno） | ✅ 可读（通过 IPC 查询） | ❌ 不可直接写（只能发命令） | ❌ |
| Rust Host | ✅ 全部 | ✅ **唯一写入口** | ✅ 可持久化到磁盘 |
| Qt 子进程 | ✅ 渲染所需子集 | ❌ 不可主动修改 | ❌ |

### 这条规则的唯一例外

Qt 侧的动画中间状态（QPropertyAnimation 的当前帧值）由 Qt 本地管理，不同步回 Host。原因是：

- 动画中间值每秒变化 120 次，同步回 Host 毫无意义且消耗 IPC 带宽
- 动画结束时 Qt 发送 `animation.completed` 事件给 Host，Host 将最终值写入状态树
- Qt 崩溃时，动画中间状态丢失，Host replay 时从动画起始值重新开始——用户看到一瞬间的跳变，但状态一致性不破坏

### Widget ID 生命周期

**ID 生成权归 Host。** TS SDK 和 Qt 子进程均不允许自己生成 ID。

**ID 格式**：采用层级路径方案（可读、可定位父 Widget、便于调试和 DevTools）：

```
widget://dock                    ← 根 Widget
widget://dock/icon/0            ← Dock 的第一个图标
widget://dock/icon/1            ← Dock 的第二个图标
widget://weather-card           ← 独立 Widget
widget://weather-card/temp      ← 天气卡片内的温度文本
```

层级路径优于 UUID v7 的原因：
- 崩溃恢复时，父子关系可以从 ID 直接解析，不需要额外的 parent_id 字段
- DevTools inspector 中可以直接看出 Widget 层级
- 日志和错误信息中一眼知道是哪个 Widget
- replay 时按路径前缀批量定位 Widget 子树

**ID 生命周期规则**：

| 规则 | 说明 |
|---|---|
| 全局唯一 | 同一时刻不存在两个相同 ID 的 Widget |
| 创建时由 Host 分配 | TS 调用 `createRect(...)` → Host 分配 ID → 返回给 TS |
| 生命周期内不复用 | Widget 销毁后，其 ID 永久废弃 |
| 崩溃恢复时保持 | Qt 崩溃 → Host replay → 使用相同的 ID 重建 QWidget |
| 热更新时保持 | Deno 重启 → TS 重新创建对象 → Host 按 ID 匹配已有 Widget |
| Qt 不可自生成 | Qt 侧新建 QWidget 必须使用 Host 分配的 ID |

**TS SDK 侧的行为**：

```typescript
// TS 侧调用 createRect，不传 ID
const dock = createRect({ blur: true, radius: 16, width: 400, height: 64 })
// → IPC → Host 分配 "widget://dock"
// → Host 返回 { widgetId: "widget://dock" }
// → SDK 对象内部记录此 ID

// 后续操作都带上 ID
dock.addChild(icon)
// → IPC: {"method":"widget.addChild","params":{"parentId":"widget://dock","childId":"widget://dock/icon/0"}}
```

**崩溃恢复时的 ID 匹配**：

```
Qt 进程崩溃 → 所有 QWidget 销毁
  ↓
Host 重启 Qt 进程
  ↓
Host replay 状态树：遍历所有 Widget，逐一发送 create 指令
  → {"method":"createRect","params":{"id":"widget://dock","bounds":{...},"blur":true}}
  → {"method":"createIcon","params":{"id":"widget://dock/icon/0","src":"chrome.png"}}
  → {"method":"addChild","params":{"parentId":"widget://dock","childId":"widget://dock/icon/0"}}
  ↓
Qt 进程使用相同的 ID 重建 QWidget
  ↓
Deno 重启 → TS 侧重新 createRect(...) → Host 检测到 widget://dock 已存在
  → 不重复创建 QWidget，返回已有 widgetId，重新绑定事件回调
```

**ID 与持久化**：

Host 可选地将 Widget 状态树持久化到磁盘（`%AppData%/BetterCPT/widget-state.json`）。持久化时 ID 完整保留。下次 Host 启动时，如果检测到持久化文件，可以直接恢复到上次的 Widget 布局，无需 TS 侧重新创建。但 MVP 阶段不强制实现持久化，崩溃恢复靠内存中的状态树即可。

---

## 八、IPC 通信协议

所有 Runtime 与 Host 之间统一使用 JSON-RPC 2.0。

**插件 → Host（渲染指令）**：

```json
{
  "jsonrpc": "2.0",
  "method": "widget.setScale",
  "params": { "id": "weather-card", "scale": 1.05, "duration": 150 },
  "id": 1
}
```

**Host → 插件（系统事件推送）**：

```json
{
  "jsonrpc": "2.0",
  "method": "system.event",
  "params": { "type": "theme.changed", "data": { "mode": "dark" } }
}
```

**Host → Qt 子进程（渲染同步，含版本号）**：

```json
{
  "jsonrpc": "2.0",
  "method": "sync.widgetTree",
  "params": {
    "version": 42,
    "widgets": [
      { "id": "dock", "type": "rect", "bounds": {...}, "blur": true, "children": ["icon-1"] },
      { "id": "icon-1", "type": "icon", "src": "chrome.png", "scale": 1.0 }
    ]
  }
}
```

**Qt 子进程 → Host（事件回传）**：

```json
{
  "jsonrpc": "2.0",
  "method": "event.animationCompleted",
  "params": { "widgetId": "icon-1", "property": "scale", "finalValue": 1.2 }
}
```

**内部消息总线**：Host 内部各模块通过 `tokio::sync::mpsc` 通信：

```rust
#[derive(Serialize, Deserialize)]
pub struct IpcMessage {
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}
```

---

## 九、Render Scheduler（渲染调度层）

### 为什么必须有调度层

TS SDK 的命令式 API 允许开发者连续调用：

```typescript
icon.onHover(() => {
    icon.setScale(1.2, 200)
    icon.setOpacity(0.9, 150)
})
```

如果没有调度层，每一次 `setScale`、`setOpacity` 都是一条独立的 JSON-RPC 消息。Hover 动画在 120FPS 下每 8ms 触发一次——如果每个 Widget 的每帧都直接发 IPC，10 个 Widget 同时动画就是每秒上千条消息。

### 调度层位置

调度层位于 Rust Host 内部，在 IPC 接收端和状态树之间：

```
TS SDK（Deno）
  ↓ 命令（每次 SDK 调用 = 一条内部消息）
Rust Host — Command Queue（无界或有界队列）
  ↓ 每 16ms（一帧）flush 一次
Render Scheduler
  ├── coalesce：同一 Widget 同一属性只保留最后一条
  ├── merge：layout 变更合并为一次计算
  └── batch：多条独立命令打包为一个 IPC batch
  ↓
Host State Tree（唯一状态源）
  ↓ 同步
Qt 子进程（渲染镜像）
```

### Scheduler 核心规则

**1. 按帧 flush**：默认 16ms（约 60FPS）flush 一次。可由 Widget manifest 声明更高频率（如 Dock 可声明 8ms / 120FPS）。

**2. Coalesce 同 Widget 同属性**：

```
队列中：
  setScale(widget://dock/icon/0, 1.1, 200)
  setScale(widget://dock/icon/0, 1.2, 200)   ← 覆盖上一条
  setScale(widget://dock/icon/0, 1.0, 200)   ← 覆盖上一条
  setOpacity(widget://dock/icon/0, 0.9, 150)  ← 不同属性，保留

flush 后实际执行：
  setScale(widget://dock/icon/0, 1.0, 200)    ← 只剩最后一条
  setOpacity(widget://dock/icon/0, 0.9, 150)
```

**3. 不合并跨 Widget 的命令**：不同 Widget 的命令独立保留。`widget://dock/icon/0` 和 `widget://dock/icon/1` 的 setScale 不互相覆盖。

**4. Layout 合并**：同一帧内的多次 layout 相关变更（bounds 调整、children 增删）合并为一次 layout pass，在 flush 时计算最终布局并一次性同步到 Qt。

**5. 关键命令不参与 coalesce**：

| 命令类型 | coalesce？ | 原因 |
|---|---|---|
| setScale / setOpacity / setBlur | ✅ 可合并 | 只有最终值有意义 |
| createRect / createIcon | ❌ 不合并 | 每个创建指令产生新 Widget |
| addChild / removeChild | ❌ 不合并 | 层级变更是结构性操作 |
| onHover / onClick 绑定 | ❌ 不合并 | 事件绑定是声明性的 |

### Scheduler 对 IPC 的影响

```
无调度层：
  Hover 事件 → 每帧可能发出 3-5 条 IPC 消息
  10 个 Widget × 120fps × 3 条 = 3600 条/秒

有调度层（16ms flush）：
  每帧合并为一个 batch，一条 IPC 消息包含该帧所有变更
  10 个 Widget × 60fps × 1 batch = 60 条/秒（减少 60 倍）
```

### Scheduler 对 Qt 同步的影响

Scheduler flush 后，Host 状态树已更新。然后有两种同步策略：

**默认策略（每帧全量 diff）**：比较 flush 前后的状态树 → 生成变更集 → IPC 发送给 Qt。适合 Widget 数量 < 20 的场景。

**可选策略（dirty marker）**：Scheduler 在 coalesce 时标记被修改的 Widget 为 dirty。flush 时只发送 dirty Widget 的变更。

v1 默认使用全量 diff（实现简单），后期按需切换到 dirty marker。

### SDK 线程模型：强制串行化

**核心规则：所有 Widget 操作必须串行进入 Render Scheduler。**

```
TS SDK（Deno async tasks）
  ↓  多个 async task 可能同时调用 SDK
  ↓
Host — Command Queue（mpsc channel，天然串行）
  │  tokio::sync::mpsc 保证消息顺序
  │  先进先出，不存在并发写
  ↓
Render Scheduler（单线程消费）
  │  逐条出队、coalesce、batch
  ↓
Host State Tree（单写者）
  │  只有 Scheduler 能写
  ↓
Qt 主线程（接收同步指令，串行执行）
```

**为什么必须串行**：

如果不强制串行，以下场景会出 race condition：

```typescript
// Deno 侧两个 async task 同时执行
task1: dock.addChild(iconA)
task2: dock.removeChild(iconB)

// 如果并发执行，可能出现：
// - iconA 在 dock.children 中但 QWidget 未创建
// - iconB 的 QWidget 被移除但状态树仍引用
// - replay 时行为不可重现（因为并发顺序不确定）
```

React、Figma、Flutter 最终都走向了单 UI 调度线程——这不是设计偏好，是"UI 状态一致性"的工程必然。

**实现方式**：

Host 侧使用 `tokio::sync::mpsc::unbounded_channel` 作为 Command Queue。所有来自 Deno 的渲染命令（通过 IPC 到达 Host 后）立即入队。Scheduler 在独立的 tokio task 中逐条消费：

```rust
struct RenderScheduler {
    cmd_rx: mpsc::UnboundedReceiver<RenderCommand>,
    state_tree: WidgetStateTree,
    qt_tx: IpcSender,  // 到 Qt 进程的 IPC 发送端
}

impl RenderScheduler {
    async fn run(mut self) {
        let mut batch = Vec::new();
        let mut tick = tokio::time::interval(Duration::from_millis(16));

        loop {
            tokio::select! {
                // 新命令入队
                cmd = self.cmd_rx.recv() => {
                    if let Some(cmd) = cmd {
                        Self::coalesce_into(&mut batch, cmd);
                    }
                }
                // 帧定时器到期 → flush
                _ = tick.tick() => {
                    if !batch.is_empty() {
                        self.flush(&mut batch).await;
                    }
                }
            }
        }
    }
}
```

**关键约束**：

- Deno 侧：多个 async task 可以同时调用 SDK 方法，但 SDK 内部将调用序列化为 IPC 消息后发送到 mpsc channel——mpsc 天生串行
- Host 侧：只有 Scheduler 的这一个 tokio task 能读写 State Tree。其他 Host 模块（生命周期管理、IPC 路由器）只能读
- Qt 侧：所有渲染指令在 Qt 主线程（GUI thread）中执行，因为 QWidget 只能在主线程操作。Host 通过 IPC 发送的指令已经是串行化后的，Qt 接收端逐条执行即可，不需要额外加锁
- 崩溃恢复：恢复期间 Scheduler 暂停消费，恢复完成后从队列中继续消费，不丢消息

**这不是过度设计**。多线程并发改 UI 树的问题不是在"压力测试"时才会出现——两个 async task 同时操作同一个 Widget 的场景在正常使用中就会发生（一个定时器更新天气数据，用户同时点击 Widget 触发 hover 动画）。不串行的话，第一周就会遇到 bug。

---

## 十、插件目录结构

### Widget 插件
```
weather-card/
  manifest.json
  index.ts
  assets/icon.png
```

### Script 插件
```
pdf-toolkit/
  manifest.json
  index.ts
  lib/
    converter.ts
    merger.ts
  assets/icon.png
```

### Native 插件
```
game-script/
  manifest.json
  plugin.exe
  signature.sig     ← 签名文件（后期必须）
  assets/icon.png
```

---

## 十一、Dashboard（管理后台）

**技术**：Tauri v2 + React，只在用户主动打开时启动

**负责内容**：
- 插件市场、插件配置页面
- 生命周期管理（启动 / 停止 / 卸载）
- 权限审批、更新管理、用户账号
- Native Runtime 开关（开发者模式，默认隐藏）

**关键设计**：
- Dashboard 关闭后 WebView 完全释放
- 后台常驻只有 Widget Runtime 和 Host 进程
- Dashboard 本身不持有插件状态，状态全在 Host

---

## 十二、架构风险总览

| 风险 | 严重程度 | 当前应对 |
|---|---|---|
| Widget SDK 演变为自研 UI 框架 | 高 | 第一阶段严格限制命令式 API，禁止做 diff/tree |
| Deno 每 Widget 一个 isolate 导致内存爆炸 | 高 | 所有 Widget 共享单一 Deno 实例 |
| Native Runtime 被恶意插件利用 | 极高 | 默认关闭，MVP 阶段不实现 |
| Qt 构建依赖增加维护成本 | 中 | Qt SDK 安装 ~500MB-2GB；C++/CMake 进入构建链；Phase 0 验证构建流程可行性 |
| 多 Qt 进程模式内存可能偏高 | 中 | Phase 0 Spike 实测共享 vs 独立进程的内存数据，按数据决策 |

---

## 十三、Widget Runtime 渲染后端策略

### v1 默认后端：Qt（C++ 子进程 + JSON-RPC IPC）

**为什么选 Qt 作为 v1 Widget 渲染引擎**：

Qt 在"轻量桌面 Widget"这一场景下是经过实战验证的方案——开发者此前用 Qt 实现过 Dock，实测内存极低。将 Qt 纳入 BetterCPT 的 Widget Runtime，意味着：

- 场景图（QWidget 层级 / QGraphicsScene）——框架自带，零开发成本
- 动画系统（QPropertyAnimation / QML Animation）——框架自带，零开发成本
- 渲染循环（QApplication 事件循环）——框架自带，零开发成本
- 鼠标事件处理（enterEvent / leaveEvent / mousePressEvent）——框架自带
- 文本和图标渲染（QPainter / QLabel / QPixmap）——框架自带
- 毛玻璃背景（Qt::WA_TranslucentBackground + 手动绘制模糊）——少量代码

**v1 零自建模块**。和 D2D+DXGI 方案（需自建场景图、动画插值系统、Render Loop）相比，Qt 把这些基础设施全部拿掉，v1 的开发聚焦在 Widget 业务逻辑和 IPC 协议上。

**v1 内存预期**：

| 模式 | 内存（Private Working Set，10 个 Widget） |
|---|---|
| 共享 Qt 进程 | ~30-50MB |
| 每 Widget 独立 Qt 进程 | ~100-120MB |

推荐默认使用共享 Qt 进程模式，与当前架构"所有 Widget 共享 Deno 实例"保持一致。独立进程模式作为可选的后备，当某个 Widget 稳定性要求高于内存要求时使用。

### 架构：Qt 子进程如何集成

```
Rust Host（tokio 运行时）
    │
    ├── JSON-RPC IPC ──→ Qt Widget 进程
    │                        ├── Dock 窗口
    │                        ├── 天气卡片窗口
    │                        └── 系统监控窗口
    │
    ├── Deno Core ──→ Widget 业务逻辑（TS）
    │                    │
    │                    └── IPC: createRect / setScale / onHover
    │                         映射到 Qt 进程的渲染原语
    │
    └── Tauri WebView ──→ Dashboard（管理界面）
```

**关键设计**：

- **Qt 事件循环不跟 tokio 协同**：Qt 子进程有自己完整的 `QApplication::exec()`，Rust Host 有自己的 tokio 运行时。两者仅通过 JSON-RPC 通信，互不干扰。这和 Native Runtime 子进程是同一种隔离模型。
- **GPU Context 不跨进程共享**：每个 Qt 子进程独立管理自己的渲染上下文（OpenGL/Direct3D，Qt 自动选择）。DWM 在合成层面统一处理所有窗口的最终显示。v1 不需要跨进程 GPU 资源。
- **热更新**：Widget 的业务逻辑（TS，跑在 Deno Core 中）可以热更新——改 TS 代码 → Deno 重新加载 → 不影响 Qt 进程。Qt 本身只在 BetterCPT 版本升级时更新。
- **渲染原语通过 IPC 暴露**：Qt 进程提供一套渲染原语（createRect、createIcon、setScale、setBlur 等），Deno Core 中的 Widget TS 代码通过 JSON-RPC 调用。Widget SDK 的开发者 API 保持不变。

### v2+ 增强方向：DirectComposition 渐进引入

当 v1 Runtime 成立后，DComp 可作为增强层逐步引入。在 Qt 方案下，引入方式是：

```
v1:   Qt 独立窗口             ← 每个 Widget 是 Qt QWidget 窗口
v2+:  Qt → 离屏渲染           ← Qt 渲染到离屏 Surface
      + DComp Visual 树       ← Rust Host 将 Surface 绑定到 DComp Visual
      + DComp Animation       ← DComp 接管动画插值
      + DComp Blur            ← DWM 系统级毛玻璃替代 Qt 手动模糊
```

关键点：引入 DComp 后，Qt 不再是独立窗口，而是变成"GPU Surface 生产者"。Rust Host 创建 DComp Visual 树，将 Qt 渲染的 Surface 作为 Visual 的内容。这保留了 Qt 的渲染能力，同时获得 DComp 的合成优化。但这是 v2+ 的事，v1 不需要。

### 三种 Widget 渲染方案对比（最终决策依据）

| | Qt（v1 选择） | D2D+DXGI 手写 | WebView2 |
|---|---|---|---|
| 场景图 | QWidget，自带 | 需自建 | DOM，自带 |
| 动画系统 | QPropertyAnimation，自带 | 需自建插值 + 每帧重绘 | CSS transition/animation |
| 内存（10 Widget） | ~30-50MB（共享进程） | ~50-80MB | ~200-320MB |
| GPU 渲染 | Qt 自动（OpenGL/D3D） | 手动 D2D 硬件加速 | Chromium 合成器 |
| 学习成本 | 低（已有经验） | 中 | 低 |
| 文档和社区 | 极大 | 中 | 极大 |
| AI 辅助 | 高 | 中高 | 高 |
| 构建依赖 | Qt SDK（~500MB-2GB） | windows crate（~50MB） | WebView2 Runtime |
| 进程模型 | 独立子进程，天然隔离 | 进程内，共享 Deno 实例 | 每 WebView 独立进程 |
| Windows 集成 | 原生 Win32 窗口 | 原生 Win32 窗口 | WebView 窗口，受限 |
| 适合 | **常驻桌面 Widget** | 性能极致场景 | Dashboard、复杂 UI |

WebView 在 BetterCPT 中的正确位置仍是 **Dashboard**（Tauri v2 + React），不参与 Widget Runtime 竞争。

---

## 十四、Windows 版本兼容性

### 最低支持版本（v1）

| 组件 | 最低要求 | 原因 |
|---|---|---|
| Qt（C++） | Qt 6.5 LTS（推荐） | Qt 6.5+ 对 Windows 10/11 支持成熟 |
| Rust Host + Deno Core | 不限制 | 嵌入到 Host，不依赖系统 |
| Tauri v2 | Windows 10 1809+ | WebView2 最低要求 |

### v2+ 引入 DComp 后的版本要求

| 组件 | 最低要求 | 原因 |
|---|---|---|
| DirectComposition | Windows 8+ | DComp API 从 Win8 开始可用 |
| DWM Blur（DwmEnableBlurBehindWindow） | Windows Vista+ | 但 Win8 移除，Win10 1803+ 行为不同 |
| Acrylic Blur（SetWindowCompositionAttribute） | Windows 10 1803+ | 更现代的毛玻璃方案 |

### 建议策略

- **v1 阶段**：仅支持 Windows 10 21H2+ 和 Windows 11。Qt 6.5 在这些版本上行为一致。
- **v2+ 阶段**：引入 DComp 增强时，运行时检测当前系统能力，不可用时回退到 v1 的 Qt 渲染。不因 DComp 而提高最低版本要求。
- **Win8/Win7**：不作为目标平台。

---

## 十五、测试策略

对于这样一个涉及多进程 IPC、Deno 嵌入、Qt 子进程管理的系统，手动验证会很快成为瓶颈。以下测试策略按优先级排列：

### 单元测试层（Rust 侧）

- manifest 解析器（合法 / 非法 / 边界 JSON）
- 权限系统（声明权限通过、未声明权限拦截）
- Capability Registry（capability → runtime binding 映射）
- IpcMessage 序列化 / 反序列化
- Render Scheduler coalesce 逻辑（同 Widget 同属性合并、跨 Widget 不合并）
- 日志系统写入和轮转
- 单例锁获取和释放

### 集成测试层

- Host 启动 → 托盘图标 → 退出，完整生命周期
- Script Runtime：加载插件 → 调用 Deno op → 返回结果 → 停止
- 崩溃恢复：模拟 Qt 进程退出 / Deno task panic，验证 Host 捕获并按正确顺序恢复
- IPC 消息收发（超时、格式错误、大 payload、版本号匹配）

### Widget Runtime 测试（手动为主）

- 帧率：PresentMon 录制 QPropertyAnimation 动画帧间隔
- 内存：启动 10 分钟和 2 小时后的 Qt 进程 Private Working Set 对比
- Qt 崩溃恢复：手动 kill Qt 进程，验证 Host 检测、重启、replay、交互恢复全流程
- 多显示器热插拔测试

### 不追求的目标

- Widget 视觉效果的像素级自动化测试（成本极高，收益低）
- 跨版本全矩阵兼容性测试（MVP 阶段只测 Win10 21H2+ / Win11）

---

## 十六、插件生态冷启动

技术上跑通之后，BetterCPT 面临插件生态的冷启动问题：没有插件就没有用户，没有用户就没有开发者写插件。

### 当前无需过度设计

这属于"Runtime 成立之后才需要认真面对的问题"，MVP 阶段过度设计插件市场机制是本末倒置。但以下几件事可以在架构层面提前布局：

- **内置插件先行**：Dock 作为系统级内置 Widget，不依赖第三方生态即可提供核心价值。用户安装 BetterCPT 后立即获得 Dock 体验，这是吸引第一批用户的基础。
- **示例插件库**：提供 5-10 个高质量示例插件（天气、日历、系统监控、音乐控制），既是功能也是开发者参考。
- **插件开发体验优先于插件数量**：SDK 好用、文档清晰、调试方便，比插件市场里有多少插件更重要。前 10 个高质量插件都会是作者自己写的。
- **避免过早开放市场**：在没有插件审核、签名验证、安全沙箱到位之前，不开放第三方插件市场。安全基础设施的缺失会在出事时毁掉整个项目。

---

## 十七、DevTools 协议预留（Host Inspector Protocol）

现在不实现 DevTools，但必须在 Host 架构中预留检查接口。否则后续 DevTools 需要"拆内核"才能获取 Widget 树、IPC 消息、帧率数据——到那时成本极高。

### Host Inspector Protocol

Host 暴露一个内部 HTTP 或 WebSocket 端点（仅 `127.0.0.1`），供 DevTools 客户端连接。以下接口定义的是**协议格式**，不要求 MVP 阶段全部实现：

**1. Widget Tree Inspector**

```
GET /inspector/widgets
→ {
    "widgets": [
      {
        "id": "widget://dock",
        "type": "rect",
        "bounds": { "x": 0, "y": 1000, "w": 400, "h": 64 },
        "blur": true,
        "children": ["widget://dock/icon/0", "widget://dock/icon/1"],
        "state": {
          "scale": 1.0,
          "opacity": 1.0,
          "animating": false
        }
      }
    ],
    "version": 42
  }
```

**2. IPC Message Inspector**

```
GET /inspector/ipc?limit=100&filter=setScale
→ {
    "messages": [
      {
        "timestamp": "2026-05-15T10:30:01.234Z",
        "direction": "host→qt",
        "method": "widget.setScale",
        "params": { "id": "widget://dock/icon/0", "scale": 1.2, "duration": 200 }
      }
    ],
    "total": 3421,
    "dropped": 0
  }
```

**3. Render Scheduler Stats**

```
GET /inspector/scheduler
→ {
    "flushIntervalMs": 16,
    "lastFlushDurationMs": 0.8,
    "queueDepth": 3,
    "coalescedDropped": 12,
    "batchesPerSecond": 60,
    "ipcMessagesPerSecond": 58
  }
```

**4. Frame Time Measurement**

```
GET /inspector/frame-time?window=last60
→ {
    "fps": 118,
    "p50Ms": 7.2,
    "p95Ms": 11.4,
    "p99Ms": 15.1,
    "dropped": 2
  }
```

**5. Event Replay**

```
POST /inspector/replay
Body: { "fromVersion": 40, "toVersion": 42 }
→ {
    "events": [
      { "version": 41, "method": "widget.setScale", "params": {...} },
      { "version": 42, "method": "widget.addChild", "params": {...} }
    ]
  }
```

### 为什么现在预留

这些端点在 MVP 阶段可以用最简实现（返回硬编码或空数据），但协议格式一旦定义，后续 DevTools 开发不需要改 Host 代码——只改 DevTools 客户端即可。具体来说：

- `GET /inspector/widgets` 当前返回 State Tree 的内存 dump（已有，因为 Widget State Tree 是 Host 内核结构）
- `GET /inspector/ipc` 当前只需在 IpcMessage 上挂一个环形缓冲区（ring buffer），记录最近 1000 条消息，每条消息 16 字节开销 → 总共 16KB 内存，无感知
- `GET /inspector/scheduler` 返回 Render Scheduler 的内部计数器（flush 次数、coalesce 次数——这些 Scheduler 自己已经在维护）
- `GET /inspector/frame-time` v1 可以先返回空，等 Phase 5 DComp 引入后再实现
- `POST /inspector/replay` 利用状态树版本号直接实现

### 安全约束

- Inspector 端点仅在 `127.0.0.1` 监听，不暴露到局域网
- Production 构建中默认禁用，通过 `--devtools` flag 或注册表键启用
- 不修改状态的操作（GET）无需认证；修改状态的操作（POST replay）需要用户在 DevTools 客户端中确认

---

## 十八、Runtime Invariants（系统不变量）

以下是 BetterCPT 架构中**不可违反**的核心规则。它们不是"建议"或"最佳实践"——它们是系统正确性的前提。任何架构决策、功能迭代、重构，都必须遵守这些规则。

### 状态相关

**Invariant 1：Host 是唯一状态源（Single Source of Truth）**

Qt 不能持有业务状态。Deno 不能持有最终 UI 状态。只有 Rust Host 的 State Tree 是真相。违反此条的后果：崩溃恢复无法可靠重放、热更新状态丢失、DComp 替换 Qt 时无法无缝迁移。

**Invariant 2：Widget ID 由 Host 统一分配，全局唯一，生命周期内不复用**

TS SDK 不传 ID，Qt 不自生成 ID。ID 格式为层级路径（`widget://dock/icon/0`），可解析父子关系。

### 调度相关

**Invariant 3：所有 Widget API 必须串行进入 Render Scheduler**

不允许多个 async task 并发修改 Widget Tree。串行化通过 mpsc channel 天然保证。

**Invariant 4：IPC 不保证实时性，只保证顺序性**

JSON-RPC 消息按发送顺序到达，但不保证帧级延迟。需要帧级同步的场景使用 Scheduler 的 batch 机制。

### 渲染相关

**Invariant 5：Qt 是 Render Replica，不是 Render Authority**

Qt 只能响应 Host 的同步指令渲染，不能主动修改渲染状态。用户交互事件（hover、click）回传 Host 决策。

**Invariant 6：Widget SDK 不允许直接访问 Renderer**

TS 代码不能直接操作 Qt 对象、不能创建原生窗口、不能绕过 IPC。所有渲染必须经过 SDK → Host → Qt 管道。

### 安全相关

**Invariant 7：Runtime 权限上限不可绕过**

Widget 永远不能获取 `fs:write`。Script 永远不能获取 `widget:overlay`。Native 需要开发者模式 + 签名 + 二次确认。这不是 manifest 可配置的——是硬编码的。

**Invariant 8：每个 Runtime 的崩溃必须可恢复**

Qt 崩溃 → 窗口重建。Deno 崩溃 → 业务逻辑恢复。Script 崩溃 → 独立重启。不存在"崩溃后系统不可用"的 Runtime 类型。

### 兼容性相关

**Invariant 9：Widget SDK API 向后兼容优先于 Renderer 能力**

TS 开发者写的 `createRect({blur:true})` 在 D2D+DXGI、Qt、DComp 三种渲染后端下行为一致。Renderer 可以增强（v1 Qt → v2+ DComp），但 SDK API 不变。

**Invariant 10：不能因引入新技术而提高最低系统要求**

v2+ 引入 DComp 时，不支持 DComp 的系统必须回退到 v1 的 Qt 实现。不强制用户升级 Windows。

### 复杂性控制

**Invariant 11：第一阶段不做声明式框架**

不做 virtual tree、不做 diff/reconciliation、不做 hooks、不做 reactive graph。SDK 保持命令式 API，直到 Runtime 成立且真实需求推动框架演进。

**Invariant 12：每个新增的架构层次必须有"为什么不能没有它"的回答**

Render Scheduler 存在是因为没有它 IPC 会炸（3600 条/秒 vs 60 条/秒）。Capability Layer 存在是因为没有它权限系统会随能力增长而崩溃。Watchdog 存在是因为没有它一个死循环干掉所有 Widget。任何新增层次如果不能用具体数据证明必要性，则不应加入。

---

> 这十二条 Invariant 是 BetterCPT 架构的宪法。宪法可以