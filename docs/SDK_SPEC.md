# BetterCPT SDK 规范

> 定义 Widget SDK 的 API 设计、线程模型、红线、错误处理。
> 
> 这是插件开发者面向的接口。API 稳定性和向后兼容是最高优先级。

---

## 一、API 设计原则

**SDK 是命令式 API，不是 UI Framework。**

- 不做 virtual tree / diff tree
- 不做 reconciliation
- 不做 hooks / reactive graph / fiber
- 不做声明式 JSX / template

开发者直接调用方法操作 Widget 对象，SDK 通过 IPC 将命令发送给 Host。这不是限制——这是 Phase 1 的工程设计决策。声明式语法糖是后期的事。

### 兼容性承诺

- SDK API 在 Qt、D2D+DXGI、DComp 三种渲染后端下行为一致
- Renderer 可以增强（v1 Qt → v2+ DComp），SDK API 不变
- API 废弃需要至少一个主版本的过渡期

---

## 二、核心 API

### 创建元素

```typescript
import { createRect, createText, createIcon } from "bettercpt-sdk"

// 矩形（毛玻璃卡片背景）
const card = createRect({
  blur: true,        // 毛玻璃效果
  radius: 16,        // 圆角
  width: 200,
  height: 120,
  color: "rgba(30,30,30,0.85)"  // 可选
})

// 文本
const label = createText("Hello", {
  size: 32,
  weight: "bold",    // "normal" | "bold" | number(100-900)
  color: "#ffffff"
})

// 图标
const icon = createIcon({
  src: "chrome.png",  // 相对插件目录 assets/
  size: 48
})
```

### 层级操作

```typescript
card.addChild(label)
card.addChild(icon)
card.removeChild(icon)
```

### 视觉效果

```typescript
// 缩放动画（duration 单位 ms）
icon.setScale(1.2, 200)
icon.setScale(1.0, 200)

// 透明度动画
card.setOpacity(0.8, 150)

// 毛玻璃开关
card.setBlur(false)
```

### 交互事件

```typescript
icon.onHover(() => icon.setScale(1.2, 200))
icon.onLeave(() => icon.setScale(1.0, 200))
icon.onClick(() => {
  BetterCPT.system.launch("C:/Program Files/Chrome/chrome.exe")
})
```

### 鼠标事件原语

提供最底层的鼠标事件，由插件开发者自行组合实现拖拽、排序、手势等交互：

```typescript
icon.onMouseDown((e) => {
  // e.x, e.y — 鼠标在 Widget 坐标系中的位置
  // 拖拽开始、长按检测
})

icon.onMouseMove((e) => {
  // e.x, e.y — 实时跟踪鼠标位置
  // 跟手拖拽、hover 区域检测
})

icon.onMouseUp((e) => {
  // e.x, e.y
  // 拖拽结束、点击确认
})
```

- 坐标原点为 Widget 自身左上角
- 不做 onDragStart / onDragEnd 等高级封装——那是 UI Framework 的事

### 系统能力

```typescript
import { weather, system } from "bettercpt-sdk/system"

// 天气（Widget Runtime 允许的有限网络访问）
const temp = await weather.getCurrent("Beijing")
label.setText(temp + "°C")

// 启动应用
system.launch("C:/path/to/app.exe")

// 系统主题
system.onThemeChanged((mode) => {
  if (mode === "dark") card.setColor("rgba(30,30,30,0.85)")
})
```

### 数据持久化（KV Storage）

Host 为每个插件维护独立的 KV 存储。插件不接触文件系统：

```typescript
import { storage } from "bettercpt-sdk"

// 存储（键值对，值自动序列化为 JSON）
await storage.set("city", "Tokyo")
await storage.set("recentFiles", ["a.pdf", "b.pdf"])

// 读取
const city = await storage.get("city")

// 删除
await storage.delete("city")
```

- 数据保存在 `%AppData%/BetterCPT/plugins/<plugin-id>/config.json`，Host 维护
- 单个插件 storage 上限 1MB，超限抛出 `STORAGE_QUOTA_EXCEEDED`
- 插件不需要声明 `fs:write` 权限，`storage` 是独立的内置能力

### 消息弹窗

系统原生 MessageBox，不经过 Qt 也不经过 WebView：

```typescript
import { dialog } from "bettercpt-sdk"

// 提示（仅确定按钮）
await dialog.alert("操作成功")

// 确认（确定 / 取消，返回 boolean）
const confirmed = await dialog.confirm("确认删除此图标？")
if (confirmed) { /* 执行删除 */ }

// 输入（输入框 + 确定/取消，返回 string | null）
const name = await dialog.prompt("请输入名称：", "默认值")
```

- 底层 Win32 `MessageBoxW` / `TaskDialog`，系统原生样式
- Widget 和 Script 均可调用，不需要额外权限声明

### 系统通知

```typescript
import { notification } from "bettercpt-sdk"

notification.show("转换完成", "report.pdf 已生成")
```

- v1 使用 Windows Toast 通知
- 需在 manifest 中声明 `notification` 权限

### 性能指标

```typescript
import { system } from "bettercpt-sdk"

const mem = await system.memory()
// { heapUsed: 4200000, heapTotal: 8000000, cpuMs: 15 }
```

---

## 三、线程模型

**所有 Widget API 调用串行进入 Render Scheduler。**

```
TS async task 1: icon.setScale(1.2, 200)  // duration=200ms，有动画
TS async task 2: icon.setScale(1.2, 0)    // duration=0，即时生效
                        ↓
              mpsc channel（串行）
                        ↓
              Render Scheduler（帧级 dedup）
              → task1 的 setScale(1.2, 200)：duration>0，不合并，保留
              → task2 的 setScale(1.2, 0)：   duration=0，覆盖同类即时命令
                        ↓
              最终执行两条：先 scale(1.2, 200)，后 scale(1.2, 0)
```

- 多个 async task 可以同时调用 SDK 方法——SDK 内部序列化
- duration=0 的同 Widget 同属性命令在帧内合并（只保留最后一条）
- duration>0 的动画命令不合并，按顺序执行
- 不同 Widget 的调用不会互相覆盖

---

## 四、热更新与 Widget 匹配

### 问题：Deno 重启后如何匹配已有 Widget

Deno 崩溃恢复或代码热更新时，TS 侧会重新执行 `createRect`/`createIcon` 等调用。此时 Qt 进程中已有对应的 QWidget（因为 Qt 没有崩溃，窗口还在）。Host 必须判断：新的 create 调用是"创建新 Widget"还是"匹配已有 Widget"。

### v2+：逻辑名匹配（推荐）

开发者在创建 Widget 时显式声明逻辑名：

```typescript
const icon = createIcon({
  src: "chrome.png",
  size: 48,
  logicalName: "chrome-icon"  // 热更新匹配 key
})
```

Host 按逻辑名匹配已有 Widget。逻辑名在插件内唯一。开发者重构代码顺序不影响匹配。

### v1 Fallback：位置匹配（无逻辑名时）

v1 不强制要求逻辑名。当没有逻辑名时，Host 使用**调用顺序位置匹配**：

```
Deno 重启后，TS 重新执行：
  第 1 次 createRect(...)  →  匹配已有 Widget 列表中第 1 个同类型 Widget
  第 2 次 createIcon(...)  →  匹配已有 Widget 列表中第 2 个同类型 Widget
  第 3 次 createText(...)  →  匹配已有 Widget 列表中第 1 个 Text 类型 Widget
```

**这是确定性规则**：同类型 Widget 按原创建顺序排列，新的 create 调用按调用顺序匹对。

**代价**：如果开发者在重构插件时改变了 create 调用的顺序，热更新后 Widget 会错位（图标 A 的属性被应用到图标 B 上）。

**这被接受为 v1 的限制**——在 SDK_SPEC.md 中明确注明，开发者在重构时知道需要重新启动插件（冷启动会重新分配 ID），而不是热更新。

### 匹配流程

```
Deno 重启 → TS 重新执行入口文件
  ↓
createRect({...})
  → SDK 发送: { method: "createRect", params: { ..., logicalName: "dock" | undefined } }
  ↓
Host 收到 create 请求：
  1. 查找 matching widget:
     a. 有 logicalName → 按 logicalName 精确匹配
     b. 无 logicalName → 按 (pluginId, WidgetType, callOrder) 位置匹配
  2. 匹配成功 → 返回已有 widgetId，不创建新 QWidget
  3. 匹配失败（无对应 Widget）→ 分配新 widgetId，创建新 QWidget
  ↓
SDK 收到 widgetId，后续 setScale/onHover 都带此 ID
```

**约束**：位置匹配只在 Deno 重启恢复/热更新的上下文中生效。用户主动调用 `createRect` 且没有匹配目标时，始终创建新 Widget。

---

## 六、错误处理

```typescript
try {
  await icon.setScale(1.2, 200)
} catch (e) {
  if (e.code === "WIDGET_NOT_FOUND") {
    // Widget 已被销毁
  } else if (e.code === "RUNTIME_NOT_READY") {
    // Qt 进程正在恢复中
  } else if (e.code === "PERMISSION_DENIED") {
    // 尝试了 Widget 不允许的操作
  }
}
```

### 错误码

| 错误码 | 含义 |
|---|---|
| WIDGET_NOT_FOUND | id 对应的 Widget 不存在 |
| RUNTIME_NOT_READY | Qt 正在崩溃恢复 |
| PERMISSION_DENIED | 操作超出 Widget Runtime 权限 |
| INVALID_PARAMS | 参数类型或范围错误 |
| TIMEOUT | 操作超时（500ms） |
| STORAGE_QUOTA_EXCEEDED | KV Storage 容量超限（单插件 1MB） |

---

## 五、SDK 与 Script Runtime 的区别

Widget SDK 和 Script Plugin API 的对比：

### Capability（系统能力）

| Capability | Widget SDK | Script API |
|---|---|---|
| createRect / setScale（widget:overlay） | ✅ | ❌ |
| BetterCPT.window.create（window:create） | ✅ | ✅ |
| BetterCPT.notification.show（notification） | ✅ | ✅ |

### Permission（资源访问）

| Permission | Widget SDK | Script API |
|---|---|---|
| BetterCPT.storage（storage） | ✅ | ✅ |
| fs.readFile（fs:read） | ❌ | ✅ (BetterCPT.invoke) |
| network.fetch（network:fetch） | ❌ (仅 SDK 内置 API) | ✅ |
| dialog.open / dialog.save | ❌ | ✅ |

### 内置（无需声明）

| API | Widget SDK | Script API |
|---|---|---|
| dialog.alert / confirm / prompt | ✅ | ✅ |
| system.memory | ✅ | ✅ |
| setTimeout / setInterval | ✅ | ✅ |

Widget 和 Script 的 API 形态不同，但底层共享同一套 Capability/Permission 检查。

---

## 七、红线（v1 绝对不做）

- ❌ 不做 virtual tree / diff / reconciliation
- ❌ 不做 hooks（useState / useEffect）
- ❌ 不做 reactive graph / signal
- ❌ 不做声明式 JSX 模板
- ❌ 不做跨 Widget 状态共享（Context / Provider）
- ❌ 不做 createButton / createInput 等输入控件 — Input System 会引入 focus/IME/caret/selection/clipboard，属于 GUI Framework 范畴
- ❌ 不做 Widget 嵌套路由
- ❌ 不做 Widget 跨插件通信
- ❌ 不做 ESM import/export（Runtime 只执行单文件 JS；多文件插件使用 esbuild 构建时拼接，Runtime 不负责模块解析）

Phase 1 SDK 就是：创建对象 → 调方法 → 绑定事件。简单、明确、可交付。

---

## 八、v1 插件 Runtime 模型

### Runtime 边界

BetterCPT v1 采用 **build-first runtime** 模型：

```
插件源码（多文件 TS/JS）
    │  esbuild
    ▼
bundle.js（单文件 JS）
    │  deno_core execute_script()
    ▼
V8 沙箱执行
```

**Runtime 负责**：沙箱执行单文件 JS、SDK API 注入、生命周期管理。

**Runtime 不负责**：模块解析、TS 编译、npm 包管理——这些属于构建工具链。

### 为什么

每往 Runtime 加一层能力，复杂度成倍增长：

| 能力 | 引入的连带问题 |
|------|-------------|
| ESM import/export | 模块解析、缓存、循环依赖、异步求值、top-level await |
| TS 编译器 | source map、loader graph、transpile cache |
| npm 包管理 | 依赖隔离、版本冲突、security audit |

保持 Runtime 极简 = 稳定、安全、可维护。开发者体验由构建工具保证。

### v1 推荐构建方案

```bash
npm install -D esbuild
npx esbuild src/main.js --bundle --outfile=dist/bundle.js --format=iife
```

插件目录结构：
```
my-plugin/
├── src/              # 开发源码（多文件 TS/JS）
│   ├── main.ts
│   ├── dock.ts
│   └── theme.ts
├── dist/
│   └── bundle.js     # 构建产物（Runtime 执行这个）
├── manifest.json     # entry: "dist/bundle.js"
└── package.json
```

v1 三个验收插件（<150 行）直接用单文件 JS 即可，不需要构建步骤。插件规模增长后按需引入 esbuild。

### 类型支持

SDK 导出 `.d.ts` 声明文件，VSCode 提供完整的智能提示和类型检查。开发者写 JS 同样获得 API 文档和自动补全。

---

## 九、Plugin 开发者体验

一个完整的 Dock 插件：

```typescript
// dock/index.ts
import { createRect, createIcon } from "bettercpt-sdk"
import { system } from "bettercpt-sdk/system"

const dock = createRect({ blur: true, radius: 16, width: 400, height: 64 })

const apps = [
  { icon: "chrome.png",  path: "C:/Program Files/Chrome/chrome.exe" },
  { icon: "vscode.png",  path: "C:/Program Files/VS Code/Code.exe" },
  { icon: "terminal.png", path: "C:/Windows/System32/cmd.exe" },
]

for (const app of apps) {
  const icon = createIcon({ src: app.icon, size: 48 })
  dock.addChild(icon)
  icon.onHover(() => icon.setScale(1.2, 200))
  icon.onLeave(() => icon.setScale(1.0, 200))
  icon.onClick(() => system.launch(app.path))
}
```

manifest.json：

```json
{
  "id": "@bettercpt/dock",
  "name": "Dock",
  "version": "1.0.0",
  "engine": ">=1.0.0",
  "runtime": "widget",
  "capabilities": ["widget:overlay", "window:create"],
  "permissions": ["storage", "notification"],
  "entry": "index.ts",
  "widget": {
    "defaultSize": { "width": 400, "height": 64 },
    "resizable": false,
    "alwaysOnTop": true
  }
}
```

- `runtime`：执行环境（widget / script / native），决定生命周期和隔离模型
- `capabilities`：系统能力声明（如 `widget:overlay`、`window:create`、`notification`）
- `permissions`：资源访问声明（如 `storage`、`network:fetch`、`fs:read`）
- `engine`：兼容的 Host 版本范围（semver 语法），Host 启动前校验

---

## 十、插件包格式

### 分发格式

插件以 `.bcpkg` 文件分发，本质是 zip 压缩包，改扩展名。

内部结构：

```
my-plugin.bcpkg (zip)
├── manifest.json       # 插件元数据
├── index.ts / index.js # 入口文件
├── assets/             # 图标、图片等静态资源
└── node_modules/       # 依赖（如果有）
```

v1 本地加载时解压到 `%AppData%/BetterCPT/plugins/<plugin-id>/`。
v2 插件市场下载的 `.bcpkg` 使用相同格式，安装流程完全复用。

### 插件 ID 规范

格式：`@author/plugin-name`

规则：
- 全小写，只允许字母、数字、连字符
- `@bettercpt` 命名空间保留给官方插件
- 本地开发使用 `@local/plugin-name`，不与市场 ID 冲突

示例：
```
@johnsmith/dock
@johnsmith/weather-card
@bettercpt/system-monitor
@local/my-test-plugin
```

manifest.json 中的 `id` 字段必须符合此规范：

```json
{
  "id": "@johnsmith/dock",
  ...
}
```
