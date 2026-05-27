#   BetterCPT <sup>v1</sup>

<div align="center">

  **AI 时代的桌面，不该还是 2006 年的样子**

  *你的桌面能做的事，远超你的想象*

  [![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
  [![Platform](https://img.shields.io/badge/platform-Windows%2010%2B-blue)](https://www.microsoft.com/windows)
  [![Version](https://img.shields.io/badge/version-v1.0.0-brightgreen)](https://github.com/EvanLofton/BetterCPT/releases)
  [![Made with](https://img.shields.io/badge/made%20with-%E2%9D%A4%EF%B8%8F-red)]()

</div>

---

##   AI 帮你写代码，那 AI 能帮你用桌面吗？

2026 年了。你让 ChatGPT 写了一个自动化脚本。你让 Claude 生成了一个数据分析面板。然后呢？

然后你把它放在文件夹里，忘了。

**BetterCPT 解决的问题**：让你的桌面 **可编程**。AI 生成的代码 → 直接变成桌面上的交互式 Widget。不再是"写代码 → 存文件 → 手动执行"，而是 **"写代码 → 桌面上活起来"**。

---

##   三种插件，覆盖一切桌面需求

|   | 插件类型 |   | 做什么 |   | 举个例子 |
|---|---------|---|--------|---|------|
|   | **Widget** |   | 常驻桌面的视觉组件 |   | Dock 栏 · 天气卡片 · 股票面板 · 系统监控 |
|   | **Script** |   | 按需启动的功能工具 |   | PDF 工具箱 · 文件批处理 · 自动化工作流 |
|   | **Native** |   | 系统级操作（v2 开放） |   | 驱动管理 · 硬件监控 · 深度系统集成 |

📌 **v1 已就绪**：Widget + Script。Native Runtime 在 v2 开放。

---

##   截图详解

###   启动加载 — 第一眼就是品质

<div align="center">
  <img src="screenshots/loading.png" alt="启动加载" width="70%" />
</div>

>   **品牌化启动体验**：BetterCPT 大字渐显 +   旋转图标 +   绿色进度条匀速滑动 + v1.0.0 版本号标注。加载页不是占位符，是产品设计的第一印象。

|   | 亮点 |
|---|------|
|   | 毛玻璃风格背景，Material Icons 浮动装饰 |
|   | 字母逐字淡入动画，品牌感拉满 |
|   | 进度条 `left` 动画从左侧外侧滑到右侧外侧，无跳帧 |
|   | 加载完成后淡出 → 通知 Host 插件就绪 → 无缝衔接 |

---

###   Dashboard 管理后台 — 插件指挥中心

<div align="center">
  <img src="screenshots/软件主页面.png" alt="Dashboard 管理后台" width="80%" />
</div>

>   **一站式插件管理**：左侧导航栏（首页 / 插件管理 / 插件市场 / 崩溃日志 / 设置），右侧主内容区。Tauri v2 + React，所有操作由 Rust Host 直接响应，不走跨进程通信。

|   | 功能亮点 |
|---|------|
|   | 统计卡片：已安装插件数 / 运行中 / 崩溃次数 / 总内存占用 |
|   | 插件列表表：图标 + 名称 + 状态（运行中 / 已停止 / 已崩溃）+ 操作按钮 |
|   |   实时状态推送（`runtime_event`），不轮询 |
|   |   深色 / 浅色主题切换，CSS 变量驱动全局色板 |
|   |   系统托盘常驻，关闭窗口不退出程序 |
|   |   拖放 .bcpkg 文件即可安装插件 |

---

###   Dock 插件 — 桌面上的启动器

<div align="center">
  <img src="screenshots/dock插件主页面.png" alt="Dock 插件" width="80%" />
</div>

>   **macOS 风格的 Dock 栏，纯 JavaScript 实现**。拖一个 .exe 或 .lnk 文件上去 → 自动解析路径、提取图标、分配色彩 → 悬停放大 → 点击启动。这个插件本身就在展示 BetterCPT 的 SDK 能力上限。

|   | 功能亮点 |
|---|------|
|   |   拖放 .exe / .lnk → `IShellLink` COM 解析 → `QFileIconProvider` 提取图标 |
|   |   悬停 `setScale(1.1, 150)` + `OutBack` 缓动 → 图标弹性放大 |
|   |   离开恢复 `setScale(1.0, 150)` → 丝滑缩小 |
|   |   点击 `ShellExecuteW` 启动应用，.lnk 透明处理 |
|   |   图标色彩从 10 色调色板自动轮换 |
|   |   最多 12 个图标，水平排列，超出不追加 |
|   |   非图标区域鼠标穿透到桌面 |
|   |   通过 Dashboard 配置图标大小 / 间距 / 背景色 |

---

###   插件市场 — 生态起点

<div align="center">
  <img src="screenshots/插件市场.png" alt="插件市场" width="80%" />
</div>

>   分类浏览 · 搜索筛选 · 插件卡片（名称 / 作者 / 版本 / 下载量 / 评分）。v1 为 Demo 预览 — v2 正式开放社区生态，支持 .bcpkg 远程下载和自动更新。

|   | 功能亮点 |
|---|------|
|   | 卡片式布局，每个插件独立展示 |
|   | 分类筛选（全部 / Widget / Script / 热门） |
|   | 搜索框实时过滤 |
|   | 插件详情页：描述 / 版本 / 作者 / 权限声明 |
|   | v2 接入真实后端后，一键安装到本地 |

---

##   为什么选择 BetterCPT

###   对用户：你的桌面你做主

|   | 痛点 |   | BetterCPT |
|---|------|---|------|
|   | AI 生成的代码不知道放哪 |   | 直接变成桌面 Widget，0 秒触达 |
|   | 桌面工具散落各处 |   | 一个 Dashboard 管理所有插件 |
|   | 工具崩溃影响工作 |   | 单插件隔离崩溃，3 秒自恢复 |
|   | 现成方案太重 |   | 10 个 Widget < 50MB |

###   对开发者：写 JS 就够了

|   | 痛点 |   | BetterCPT |
|---|------|---|------|
|   | 想写桌面应用要学 C++ / Qt |   | `createRect({...})` 三行代码出界面 |
|   | 渲染性能难优化 |   | Qt 6.5 GPU 加速，你只管逻辑 |
|   | 插件分发困难 |   | `.bcpkg` 打包 → 拖到 Dashboard → 安装 |
|   | TypeScript 还不行？ |   | v1 纯 JS，Phase 5 支持 TS + SWC |

---

##   架构特色

###   主机唯一状态源

```
你写的 JS 插件
    ↓
Host State Tree (Rust) — 唯一真相
    ↓
Render Scheduler (16ms flush, 帧级去重)
    ↓
Qt 子进程 (C++ GPU 渲染) — 只是"镜像"
```

Qt 崩了？  Host 从状态树全量重建 —— 用户无感知。不是靠"尽量不崩溃"，是靠"崩了能恢复"。

###   命令式 API — 你完全掌控

```javascript
var icon = createIcon({ src: "chrome.png", size: 48 });
icon.onHover(() => icon.setScale(1.2, 200));  //   悬停放大
icon.onLeave(() => icon.setScale(1.0, 200));  //   离开恢复
icon.onClick(() => system.launch("chrome.exe")); //   点击启动
```

不做 Virtual DOM。不做 Hooks。不做响应式。**你决定什么时候渲染，不是框架。**

###   Build-first 模型

```
开发时：多文件 TypeScript + npm 依赖
构建时：esbuild 打包 → single bundle.js
运行时：deno_core 执行 — 零模块解析，零依赖管理
```

Figma / VSCode / Obsidian 同款架构。Runtime 保持极简 = 稳定 + 安全。

###   per-plugin V8 Isolate 隔离

```
插件 A (Dock)        插件 B (天气)        插件 C (监控)
    ↓                    ↓                    ↓
Isolate A            Isolate B            Isolate C
每个有独立的堆 · 全局作用域 · 内存上限
```

一个插件死循环？  **只影响它自己**。这是把"插件"当"微服务"做的。

---

##   5 分钟写一个插件

```javascript
//   my-widget/index.js
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
//   my-widget/manifest.json
{
  "id": "@local/my-widget",
  "runtime": "widget",
  "capabilities": ["widget:overlay"],
  "permissions": ["storage"],
  "entry": "index.js"
}
```

  ️ 放到 `plugins/` →   启动 BetterCPT →   Widget 出现

📖 [插件开发指南](docs/SDK_DEVELOPER_GUIDE.md) · [SDK API 参考](docs/SDK_SPEC.md)

---

##   技术栈

|   | 层 |   | 技术 |
|---|------|---|------|
|   | Host 主进程 |   | Rust + tokio + Tauri v2 |
|   | 渲染引擎 |   | C++ Qt 6.5 (独立子进程) |
|   | 插件运行时 |   | Deno Core (per-plugin V8 Isolate) |
|   | 管理后台 |   | React + Tailwind CSS |
|   | 进程间通信 |   | JSON-RPC 2.0 (stdin/stdout pipe) |

---

##   路线图

|   | 阶段 |   | 内容 | 状态 |
|---|------|---|------|------|
| ✅ | Phase 0-4 |  ️ | Dock + Dashboard + SDK + 崩溃恢复 | 已完成 |
| 📋 | Phase 5 |   | Batch IPC · 多线程 · TypeScript · 自动更新 | 规划中 |
| 📋 | v2 |   | 插件市场 · 签名审核 · Native Runtime · 跨平台探索 | 远期 |

---

##   从源码构建

```bash
git clone https://github.com/EvanLofton/BetterCPT.git
cd widget-qt && cmake -B build && cmake --build build     #   Qt 渲染器
cd ../host && cargo build                                   #   Rust Host
cd ../dashboard && npm install && npm run build             #   Dashboard
cd ../host && cargo run                                     #   启动
```

---

<div align="center">

  **BetterCPT — AI 时代的桌面，由你编程**

  [![GitHub stars](https://img.shields.io/github/stars/EvanLofton/BetterCPT?style=social)](https://github.com/EvanLofton/BetterCPT)

  <sub>MIT License · Built with   Rust ·   Qt ·   Deno Core ·   Tauri</sub>

</div>
