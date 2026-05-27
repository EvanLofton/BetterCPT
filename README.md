# BetterCPT

**Windows 桌面插件平台** — Rust 主进程 + Qt Widget 渲染 + Deno Core JavaScript 插件 + Tauri 管理后台。

目标：10 个常驻 Widget 内存 < 50MB，GPU 加速渲染，崩溃可恢复。

## 系统架构

```
TS/JS SDK (Deno Core V8 Isolate)
  │  命令式 API：createRect / setScale / onHover
  ▼
Host State Tree (Rust) — 唯一真相源
  │
  ▼
Render Scheduler (16ms flush, 帧级去重)
  │
  ▼
IPC Batch (JSON-RPC 2.0, stdin/stdout pipe)
  │
  ▼
Qt 子进程 (C++ Qt 6.5) — 渲染镜像
  │
  ▼
GPU 渲染 → DWM → 桌面
```

## 快速开始

```bash
# 构建 Qt 渲染器
cd widget-qt && cmake -B build && cmake --build build

# 构建 Rust Host
cd ../host && cargo build

# Dashboard 前端
cd ../dashboard && npm install && npm run build

# 运行
cd ../host && cargo run
```

## 插件开发

插件是纯 JavaScript 单文件，无需构建工具。

```
my-plugin/
├── manifest.json       # {"id": "@local/my-plugin", "runtime": "widget", ...}
└── index.js            # createRect({...}), widget.onClick(...)
```

详见 [SDK_DEVELOPER_GUIDE.md](docs/SDK_DEVELOPER_GUIDE.md)。

## 技术栈

| 层 | 技术 |
|---|------|
| Host | Rust + tokio + Tauri v2 |
| 渲染 | C++ Qt 6.5 Widgets (独立子进程) |
| 插件 | JavaScript (Deno Core V8 Isolate) |
| 管理 | Tauri WebView + React + Tailwind |
| IPC | JSON-RPC 2.0 (stdin/stdout pipe) |

## 文档

- [项目总纲](docs/PROJECT_OVERVIEW.md)
- [系统架构](docs/ARCHITECTURE.md)
- [SDK 规范](docs/SDK_SPEC.md)
- [插件开发指南](docs/SDK_DEVELOPER_GUIDE.md)
- [构建与部署](docs/BUILD_AND_DEPLOY.md)
- [路线图](docs/ROADMAP.md)
- [验收报告](docs/ACCEPTANCE_REPORT.md)

## License

MIT
