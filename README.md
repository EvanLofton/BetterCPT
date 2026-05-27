#  BetterCPT <sup>v1</sup>

<div align="center">

  **让 Windows 桌面活起来**

  Dock 栏 · 天气卡片 · 系统监控 · PDF 工具箱 · 快捷启动器 · 自动化脚本

  *用 JavaScript 写一切 —— 桌面视觉组件 + 功能型工具 + 系统级脚本*

  [![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
  [![Platform](https://img.shields.io/badge/platform-Windows%2010%2B-blue)](https://www.microsoft.com/windows)
  [![Version](https://img.shields.io/badge/version-v1.0.0-brightgreen)](https://github.com/EvanLofton/BetterCPT/releases)

</div>

---

##  这是什么

BetterCPT 是一个 **Windows 桌面插件平台**。三种插件，一个 Host：

|   Plugin |  用途 |  举个例子 |
|---|------|------|
|   **Widget** | 常驻桌面的视觉组件 | Dock 栏、天气卡片、系统监控面板 |
|   **Script** | 按需启动的功能工具 | 文件行数统计、PDF 工具箱、自动化脚本 |
|   **Native** | 系统级操作 | 驱动工具、硬件管理（v2 开放） |

📌 **v1 现状**：Widget + Script 两种 Runtime 已就绪。Native Runtime 在 v2 开放，需要签名 + 开发者模式 + 用户确认。

##  截图展示

<div align="center">

  ** 启动加载页**
  <br>
  <img src="screenshots/loading.png" alt="启动页" width="60%" />
  <br>
  <sub>应用启动时的加载画面 — 轻快的品牌视觉</sub>

  ---

  **  Dashboard 管理后台**
  <br>
  <img src="screenshots/软件主页面.png" alt="Dashboard 主页" width="80%" />
  <br>
  <sub>Tauri v2 + React — 插件安装 / 启动 / 停止 / 卸载 / 监控，一站式管理</sub>

  ---

  **  Dock 插件**
  <br>
  <img src="screenshots/dock插件主页面.png" alt="Dock 插件" width="80%" />
  <br>
  <sub>拖放 .exe / .lnk 到 Dock →  自动提取图标 →  悬停放大 →  点击启动</sub>

  ---

  **  插件市场**
  <br>
  <img src="screenshots/插件市场.png" alt="插件市场" width="80%" />
  <br>
  <sub>浏览插件生态，一键安装（v1 为 Demo 数据，v2 正式开放）</sub>

</div>

##  特性

|   | 特性 | 说明 |
|---|------|------|
|   | **极轻内存** | 10 个 Widget < 50MB，闲置 CPU 接近零 |
|   | **GPU 加速** | Qt 6.5 硬件渲染，144Hz 丝滑动画 |
|   | **JavaScript** | v1 纯 JS 开发，Phase 5 支持 TypeScript |
|   | **崩溃自愈** | 单个插件崩溃不影响其他，3 秒内自动恢复 |
|   | **可视管理** | Dashboard 安装 / 启动 / 停止 / 卸载，实时监控 |
|   | **拖放创建** | 拖文件到 Dock →  自动图标 →  一键启动 |
|   | **三种插件** | Widget 常驻 + Script 按需 + Native 系统级 |
|   | **权限沙箱** | Capability / Permission 双层权限模型 |

##   5 分钟写一个插件

```javascript
//  ① 创建文件 my-widget/index.js
var card = createRect({
  width: 200, height: 120,
  x: 100, y: 100,
  radius: 16,
  color: "rgba(30, 30, 30, 0.85)"
});

card.onClick(function () {
  system.launch("calc.exe"); //  点击启动计算器
});
```

```json
//  ② 创建 my-widget/manifest.json
{
  "id": "@local/my-widget",
  "runtime": "widget",
  "capabilities": ["widget:overlay"],
  "permissions": ["storage"],
  "entry": "index.js"
}
```

  ️ ③ 把文件夹放到 `plugins/`，启动 BetterCPT。你的 Widget 就出现在桌面了。

📖 完整文档：[插件开发指南](docs/SDK_DEVELOPER_GUIDE.md)

##   为什么不用...

|   | 方案 |   | 问题 |
|---|------|---|------|
|  ️ | Rainmeter |   | Lua 脚本、无现代工具链、扩展性弱 |
|   | Electron |   | 每个 Widget 15-25MB，10 个直逼 300MB |
|   | 自研 GUI |   | focus / IME / clipboard 会失控 |

**BetterCPT 的解法**：Rust 保稳定 → Qt 保流畅 → JavaScript 保开发体验。各司其职，不重复造轮子。

##   架构亮点

###   宿主唯一状态源

所有 Widget 状态由 Rust Host 持有。Qt 只做"渲染镜像"——Qt 崩了，Host 从状态树重建所有 Widget，用户无感知。这是和 Electron / WebView 方案的**本质差异**。

###   命令式 SDK

`createRect()` → `setScale()` → `onClick()`。不做 Virtual DOM、不做 Hooks、不做响应式。你完全掌控渲染时机。

###   Build-first 模型

插件开发用多文件 + TypeScript → esbuild 打包 → Runtime 只执行一份 `bundle.js`。Figma / VSCode 同款架构。

###   per-plugin Isolate 隔离

每个插件独立的 V8 Isolate。死循环、内存泄漏、GC 压力互不影响。把"插件"当"微服务"看。

##   技术栈

|   | 层 |   | 技术 |
|---|------|---|------|
|   | Host 主进程 |   | Rust + tokio + Tauri v2 |
|   | 渲染引擎 |   | C++ Qt 6.5 (独立子进程) |
|   | 插件运行时 |   | Deno Core (V8 Isolate) |
|   | 管理后台 |   | React + Tailwind (Tauri WebView) |
|   | 进程通信 |   | JSON-RPC 2.0 (stdin/stdout pipe) |

##   路线图

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 0-4 |   Dock + Dashboard + SDK + 崩溃恢复 | ✅ v1 完成 |
| Phase 5 |   Batch IPC · 多线程 · TypeScript · 自动更新 | 📋 规划中 |
| v2 |  插件市场 · 签名审核 · Native Runtime | 📋 远期 |

详见 [ROADMAP.md](docs/ROADMAP.md) 和 [FUTURE_CAPABILITIES.md](docs/FUTURE_CAPABILITIES.md)。

##   从源码构建

```bash
git clone https://github.com/EvanLofton/BetterCPT.git

#   Qt 渲染器
cd widget-qt && cmake -B build && cmake --build build

#   Rust Host
cd ../host && cargo build

#   Dashboard 前端
cd ../dashboard && npm install && npm run build

#   运行
cd ../host && cargo run
```

##   协议

MIT License · [@EvanLofton](https://github.com/EvanLofton)

---

<div align="center">
  <sub>Built with   Rust ·   Qt ·   Deno Core ·   Tauri</sub>
</div>
