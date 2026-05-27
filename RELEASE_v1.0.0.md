# BetterCPT v1.0.0

首个公开测试版本。欢迎尝鲜、反馈、提 Issue。

---

## 这个版本有什么

### Dock 插件
- 拖放 .exe / .lnk 到 Dock → 自动解析路径、提取图标 → 悬停放大 → 点击启动应用
- macOS 风格，纯 JavaScript 实现

### Dashboard 管理后台
- 插件安装 / 启动 / 停止 / 卸载，一站式管理
- 实时状态监控（运行中 / 已停止 / 已崩溃）
- 崩溃日志查看、设置持久化、深色 / 浅色主题

### SDK — 写插件只要三行代码
- `createRect()` / `createIcon()` / `createText()` — 创建桌面元素
- `setScale()` / `setOpacity()` — 动画
- `onHover()` / `onLeave()` / `onClick()` / `onDrop()` — 交互事件
- `system.launch()` — 启动应用
- `storage.*` / `dialog.*` / `notification.*` — 系统能力

### 崩溃恢复
- Qt 进程崩溃 → 3 秒内自动重启 → 从状态树全量恢复所有 Widget
- V8 Isolate 崩溃 → 自动重建 → 重新加载插件代码
- 每个插件最多自动恢复 5 次，超限后通知用户

### 权限模型
- Capability + Permission 双层权限
- Widget 永远不能写文件系统
- 插件 manifest 声明权限，Host 校验上限

---

## 已完成的 Phase

| Phase | 内容 | 状态 |
|-------|------|------|
| Phase 0 | Qt 渲染验证（Overlay 窗口 / IPC / 动画 / 崩溃恢复） | ✅ |
| Phase 1 | Host 核心骨架（托盘 / 日志 / StateTree / Scheduler） | ✅ |
| Phase 2 | Script Runtime + Window Service + Deno Core 嵌入 | ✅ |
| Phase 3 | Widget Runtime（Qt 子进程 / SDK 命令式 API） | ✅ |
| Phase 4 | Dashboard（Tauri v2 + React + 插件管理全流程） | ✅ |

29/29 单元测试通过。

---

## 已知限制

-   v1 插件为纯 JS 单文件，不支持 TypeScript / import（Phase 5 接入 SWC）
-   `storage.get()` 和 `dialog.confirm()` 暂不返回结果（Phase 5 `#[op2]` Promise）
-   `setTimeout` / `setInterval` 暂未实现（Phase 5 Deno op）
-   Dock 仅支持单屏幕主显示器
-   任务栏隐藏依赖 `QTimer::singleShot(0)` 延迟，部分环境可能不生效

---

## 系统要求

- Windows 10 x64 或更高
- WebView2 Runtime（Win11 自带，Win10 大部分已有）
- 无需安装 Qt、无需安装 Node.js、无需管理员权限

---

## 安装

下载 `BetterCPT_v1.0.0_x64-setup.exe`，一路下一步。

---

## 反馈

- [GitHub Issues](https://github.com/EvanLofton/BetterCPT/issues)
- 欢迎提交 Bug、功能建议、插件 idea

---

## 下一步

Phase 5 规划中：
-   Batch IPC（提升指令吞吐量）
-   多线程 Isolate（插件性能隔离）
-   TypeScript 支持（esbuild / SWC 编译）
-   自动更新（Tauri updater plugin）
-   `setTimeout` / `setInterval`（Deno op 异步回调）

---

**让桌面可编程——从 BetterCPT 开始。**
