# BetterCPT 插件开发指南 (v1)

> 用纯 JavaScript 写 Windows 桌面插件。无需构建工具，无需 import。

---

## 快速开始

最小插件：一个 `manifest.json` + 一个 `index.js`。

### 目录结构

```
my-plugin/
├── manifest.json       # 插件元数据
├── index.js            # 入口文件（纯 JS，无 import/export）
└── assets/             # 图标等静态资源（可选）
```

### manifest.json

```json
{
  "id": "@local/my-plugin",
  "name": "我的插件",
  "version": "0.1.0",
  "runtime": "widget",
  "capabilities": ["widget:overlay"],
  "permissions": ["storage"],
  "entry": "index.js",
  "engine": ">=0.1.0"
}
```

| 字段 | 说明 |
|------|------|
| `id` | 全局唯一标识。`@local/` 命名空间留给本地开发 |
| `runtime` | `"widget"` = 有 Qt 渲染 / `"script"` = 纯 JS 无 UI |
| `capabilities` | 系统能力声明。Widget 插件必填 `"widget:overlay"` |
| `permissions` | 资源访问声明，如 `"storage"` |
| `entry` | 入口文件名 |
| `engine` | 兼容的 Host 版本（semver） |

### 安装

把插件目录拖到 `%AppData%/BetterCPT/plugins/` 下，或通过 Dashboard 拖拽 `.bcpkg` 文件安装。

---

## API 参考

**以下 API 均为全局函数，直接调用，无需 import。**

### 创建元素

```javascript
// 矩形（可用作卡片背景、Dock 面板等）
var rect = createRect({
  width: 400,
  height: 80,
  x: 0,                   // 屏幕坐标（可选，默认 0）
  y: 0,                   // 屏幕坐标（可选，默认 0）
  blur: true,             // 毛玻璃效果
  radius: 16,             // 圆角
  color: "rgba(30,30,30,0.85)"   // 可选
});

// 图标
var icon = createIcon({
  src: "chrome.png",      // 相对插件目录的图标文件
  size: 48,               // 图标大小（正方形）
  x: 100,                 // 屏幕坐标（可选）
  y: 200                  // 屏幕坐标（可选）
});

// 文本
var label = createText("Hello World", {
  size: 32,               // 字号
  weight: "bold",         // "normal" | "bold" | 数字(100-900)
  color: "#ffffff",       // 文字颜色
  x: 50,                  // 屏幕坐标（可选）
  y: 30                   // 屏幕坐标（可选）
});
```

每个创建函数返回一个 `Widget` 对象。

### 层级操作

```javascript
rect.addChild(icon);      // 添加子元素
rect.removeChild(icon);   // 移除子元素
```

### 视觉效果

```javascript
icon.setScale(1.2, 200);  // 缩放到 1.2 倍，动画 200ms
icon.setScale(1.0, 200);  // 恢复原始大小
icon.setOpacity(0.8, 150); // 透明度 0.8，动画 150ms
icon.setBlur(true);       // 开启/关闭毛玻璃
```

### Z 轴顺序

```javascript
icon.raise();             // 置于顶层
icon.lower();             // 置于底层
```

### 交互事件

```javascript
icon.onHover(function() {
  icon.setScale(1.2, 200);
});

icon.onLeave(function() {
  icon.setScale(1.0, 200);
});

icon.onClick(function() {
  // 点击处理
});
```

**可用事件：** `onHover` / `onLeave` / `onClick`

> **v1 限制：** `onMouseDown` / `onMouseMove` / `onMouseUp` 的 callback 参数（鼠标坐标）暂不可用。Phase 5 支持。

---

### 系统能力

```javascript
// 启动外部应用
system.launch("C:/path/to/app.exe");

// 获取内存使用（返回 undefined，数值通过 IPC 回传，v1 未实现返回）
system.memory();
```

### 数据持久化（KV Storage）

```javascript
// 存储（值自动序列化为 JSON）
storage.set("key", "value");
storage.set("obj", { a: 1, b: 2 });

// 读取（当前 v1 返回 undefined，值通过 IPC 回传）
storage.get("key");

// 删除
storage.delete("key");
```

- 数据保存在 `%AppData%/BetterCPT/plugins/<plugin-id>/config.json`
- 单插件上限 1MB

### 弹窗与通知

```javascript
// 系统弹窗（原生 Win32 MessageBox）
dialog.alert("操作成功");

// 确认弹窗
dialog.confirm("确认删除？");

// 输入弹窗
dialog.prompt("请输入名称：", "默认值");

// Windows Toast 通知（需 manifest 声明 notification 权限）
notification.show("转换完成", "report.pdf 已生成");
```

### 控制台输出

```javascript
console.log("Dock initialized");
```

输出到 Host 日志文件（`%AppData%/BetterCPT/logs/`）。

---

## 完整示例：Dock 插件

```javascript
// dock/index.js — 屏幕底部居中 Dock，6 个图标

// ── 配置 ──
var ICONS = [
  { src: "chrome.png",   app: "C:/Program Files/Google/Chrome/Application/chrome.exe" },
  { src: "vscode.png",   app: "C:/Users/YourName/AppData/Local/Programs/Microsoft VS Code/Code.exe" },
  { src: "terminal.png", app: "C:/Windows/System32/cmd.exe" },
  { src: "explorer.png", app: "C:/Windows/explorer.exe" },
  { src: "notepad.png",  app: "C:/Windows/System32/notepad.exe" },
  { src: "calc.png",     app: "C:/Windows/System32/calc.exe" },
];

var ICON_SIZE = 48;
var ICON_GAP = 16;
var DOCK_PADDING = 16;
var DOCK_HEIGHT = 80;

// ── 计算位置 ──
// v1 不提供屏幕尺寸 API，硬编码或手动计算
var SCREEN_W = 1920;
var SCREEN_H = 1080;

var dockW = ICONS.length * (ICON_SIZE + ICON_GAP) + DOCK_PADDING * 2 - ICON_GAP;
var dockX = (SCREEN_W - dockW) / 2;
var dockY = SCREEN_H - DOCK_HEIGHT - 8;

// ── Dock 背景 ──
var dockBg = createRect({
  blur: true,
  radius: 16,
  width: dockW,
  height: DOCK_HEIGHT,
  color: "rgba(30, 30, 30, 0.75)",
  x: dockX,
  y: dockY,
});

// ── 图标 ──
for (var i = 0; i < ICONS.length; i++) {
  (function(idx) {
    var icon = ICONS[idx];
    var x = dockX + DOCK_PADDING + idx * (ICON_SIZE + ICON_GAP);
    var y = dockY + (DOCK_HEIGHT - ICON_SIZE) / 2;

    var w = createIcon({
      src: icon.src,
      size: ICON_SIZE,
      x: x,
      y: y,
    });

    w.onHover(function() { w.setScale(1.2, 200); });
    w.onLeave(function() { w.setScale(1.0, 200); w.setOpacity(1.0, 150); });
    w.onClick(function() { system.launch(icon.app); });

    dockBg.addChild(w);
  })(i);
}

console.log("[Dock] " + ICONS.length + " icons, " + dockW + "x" + DOCK_HEIGHT);
```

---



## 错误处理

```javascript
// v1 不支持 try-catch 获取错误码
// 操作失败时错误写入 Host 日志
// 错误码（Phase 5 支持）：
//   WIDGET_NOT_FOUND   — Widget 不存在
//   RUNTIME_NOT_READY  — Qt 正在恢复
//   PERMISSION_DENIED  — 操作超出权限
//   INVALID_PARAMS     — 参数错误
//   TIMEOUT            — 操作超时
//   STORAGE_QUOTA_EXCEEDED — 存储超 1MB
```

---

## 已知限制（v1 — Phase 5 解决）

> 每个限制的正式方案见 `docs/FUTURE_CAPABILITIES.md` §5.11–5.13。

### 1. 无屏幕尺寸查询 API

插件无法获取屏幕分辨率。Dock 示例中 `SCREEN_W = 1920; SCREEN_H = 1080` 是硬编码的，换了显示器或分辨率会错位。

- **临时方案**：手动查自己屏幕分辨率，硬编码到插件里
- **正式方案**：Phase 5 新增 `system.screenWidth` / `system.screenHeight`（§5.11）

### 2. storage.get / dialog.confirm 无返回值

`storage.get(key)` 和 `dialog.confirm(msg)` 调用后立即返回 `undefined`，JS 拿不到存储的值或用户的确认结果。`storage.set` 和 `dialog.alert` 不受影响（不需要返回值）。

- **临时方案**：用 `storage.set` 配合硬编码默认值；确认操作拆为两次（先 `dialog.alert` 提示，用户手动执行后续逻辑）
- **正式方案**：Phase 5 将 `send()` 改为返回 Promise（§5.12）

### 3. 坐标全靠手算

所有 Widget 使用屏幕绝对坐标，没有"居中""底部对齐"等逻辑定位。Dock 示例里 `dockX = (SCREEN_W - dockW) / 2` 要自己算。

- **临时方案**：自己算坐标，参照示例公式
- **正式方案**：Phase 5 支持 `x: "center"` / `y: "bottom"` 逻辑锚点（§5.13）

---

## 限制与红线

### v1 不做

- ❌ `import` / `export` — 不支持模块语法，插件必须是单文件纯 JS
- ❌ TypeScript — 需手动转为 JS
- ❌ 嵌套 Widget（Widget 不能嵌套其他 Widget）
- ❌ 跨插件通信
- ❌ virtual tree / hooks / reactive graph / JSX

### Widget 限制

- Widget 只能由 Widget 类型的插件创建
- Widget 不能访问文件系统（`fs:read` / `fs:write` 对 Widget 不可用）
- 坐标系统：屏幕绝对坐标，原点在左上角
- v1 不提供屏幕尺寸查询 API

### 事件限制

- `onHover` / `onLeave` / `onClick` 可用
- `onMouseDown` / `onMouseMove` / `onMouseUp` 的 callback 参数暂不可用
- 事件回调通过 Qt EventFilter → stdout → Host → V8 火灾回调，延迟 ~16ms

---

## 热更新

修改 `index.js` 后保存，Host 自动检测文件变更并重新加载插件。Qt 进程不重启，Widget 位置和状态通过 `logicalName` 保持。

v1 热更新使用**调用顺序位置匹配**：同类型 Widget 按原创建顺序排列，新的 create 调用按顺序匹配已有 Widget。重构代码时如果改变了 create 调用顺序，Widget 会错位——此时需要冷重启插件。

---

## 调试

1. `console.log()` 输出到 `%AppData%/BetterCPT/logs/` 和 Host 控制台
2. Dashboard 崩溃日志页可查看 Deno 崩溃详情
3. Host 启动参数 `--verbose` 可开启 DEBUG 级别日志

---

## 分发包格式

`.bcpkg` 文件 — 本质是 zip：

```
my-plugin.bcpkg
├── manifest.json
├── index.js
└── assets/
    └── icon.png
```

通过 Dashboard 拖拽安装，或手动解压到 `%AppData%/BetterCPT/plugins/<plugin-id>/`。
