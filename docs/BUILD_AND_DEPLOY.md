# BetterCPT 构建与部署

> 工程文档：Rust + Qt + CMake 构建链、CI/CD、打包。

---

## 一、项目结构

```
bettercpt/
├── host/                    # Rust Host（主进程）
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── host/            # 托盘、IPC、日志、生命周期
│       └── runtime/         # Runtime 管理器
├── widget-qt/               # Qt C++ 子进程（Widget 渲染）
│   ├── CMakeLists.txt
│   ├── main.cpp
│   ├── RenderServer.cpp/h
│   └── WidgetManager.cpp/h
├── dashboard/               # React 前端（Vite + Tailwind，Tauri 后端在 host/）
├── sdk/                     # TypeScript SDK（npm 包）
│   └── src/
└── docs/                    # 本文档
```

---

## 二、依赖

### Rust Host

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"
deno_core = "0.290"
tauri = { version = "2", features = ["tray-icon"] }
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_Security",
    "Win32_System_LibraryLoader",
    "Win32_System_Threading",
    "Win32_UI_Shell",
    "Win32_UI_WindowsAndMessaging",
] }

[build-dependencies]
tauri-build = { version = "2", features = ["isolation"] }
```

### Qt 子进程

- **Qt 6.5 LTS**（MinGW 或 MSVC 版本，取决于 Rust Host 的 ABI）
- CMake ≥ 3.20

```cmake
cmake_minimum_required(VERSION 3.20)
project(BetterCPT_WidgetQt)

find_package(Qt6 REQUIRED COMPONENTS Widgets)

add_executable(widget-qt
    main.cpp
    RenderServer.cpp
    WidgetManager.cpp
)

target_link_libraries(widget-qt Qt6::Widgets)
```

### Dashboard（2026-05-19 更新）

- Tauri v2（嵌入式，运行在 Host 进程内）
- React + TypeScript + Vite
- Tailwind CSS
- Rust 后端在 `host/src/commands.rs`，通过 `State<Arc<RuntimeRegistry>>` 直接访问 Host 状态

---

## 三、构建流程

### 开发构建

```bash
# 1. 构建 Qt 子进程
cd widget-qt
cmake -B build -DCMAKE_BUILD_TYPE=Debug
cmake --build build

# 2. 构建 Rust Host
cd ../host
cargo build

# 3. 构建 Dashboard 前端
cd ../dashboard
npm install
npm run build   # Vite 构建 → host 通过 Tauri 加载
```

### Release 构建

```bash
# Qt Release
cd widget-qt
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release

# Rust Release
cd ../host
cargo build --release

# Dashboard 前端打包（Vite 输出，Tauri 加载）
cd ../dashboard
npm run build
```

---

## 四、CI/CD（建议）

```yaml
# .github/workflows/build.yml
jobs:
  build-qt:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: jurplel/install-qt-action@v4
        with:
          version: '6.5.*'
      - run: cmake -B build && cmake --build build
        working-directory: widget-qt

  build-host:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo build --release
        working-directory: host
```

---

## 五、打包

### 最终产物

```
BetterCPT/
├── bettercpt-host.exe       # Rust Host（主进程）
├── bettercpt-widget-qt.exe  # Qt 子进程
├── dashboard/               # React 前端产物（HTML/JS/CSS）
├── plugins/                 # 内置插件（Dock 等）
└── Qt6*.dll                 # Qt 运行时（静态链接可省略）
```

### Qt DLL 部署

- **动态链接**：需随包携带 `Qt6Widgets.dll`、`Qt6Gui.dll`、`Qt6Core.dll` 等（~20MB）
- **静态链接**：Qt exe 单体文件，但需注意 LGPL 合规

---

## 六、版本管理

- **Host 版本**：`Cargo.toml` 中的 `version`
- **Qt 版本**：`CMakeLists.txt` 中的 `VERSION`
- **SDK 版本**：`sdk/package.json` 中的 `version`
- **协议版本**：IPC 协议版本 `bettercpt-ipc/1`
