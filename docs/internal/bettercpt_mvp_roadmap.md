# BetterCPT MVP 详细开发路线

> 最后更新：综合 Claude + ChatGPT 多轮讨论结果（2026-05-15）

---

## 总览

```
Phase 0：Qt Render Spike          → 验证 Qt 作为 Widget Runtime 渲染后端（可与 Phase 1/2 并行）
第一阶段：Host 核心骨架            → 让 Host 能跑起来
第二阶段：Script Runtime           → 验证插件生命周期
第三阶段：Widget Runtime (v1)     → Qt 子进程 + JSON-RPC IPC 实现 Dock 和核心 Widget
第四阶段：Dashboard                → 插件管理界面
第五阶段：DComp 增强层（远期）     → 按能力逐步引入 DirectComposition
```

**重要原则**：

- 每个阶段结束都有验收标准，达到了再进入下一阶段。不要几个阶段并行推进。
- **例外**：Phase 0（Qt Render Spike）可以与 Phase 1/2 并行推进，因为 Phase 1/2 是纯后端逻辑，不涉及任何渲染。Phase 0 必须在 Phase 3 启动前完成并通过验收。

**关于 Native Runtime**：MVP 阶段完全不实现，等 Runtime 整体成立、有安全审核机制后再开放。

**关于 DirectComposition**：v1 不引入 DComp。Qt 作为 v1 Widget Runtime 渲染引擎。DComp 作为 v2+ 增强层，Qt 渲染到离屏 Surface → DComp 合成。

---

---

# Phase 0：Qt Render Spike

## 目标

在正式进入 Widget Runtime 开发（第三阶段）之前，用最小技术验证实验确认 Qt 作为 v1 Widget Runtime 渲染后端的可行性。

Phase 0 的目标不是"做出一个 Dock"，而是回答以下核心问题：
- Qt 子进程能否通过 JSON-RPC IPC 接收渲染指令并正确渲染？
- 共享 Qt 进程 vs 独立 Qt 进程，10 个 Widget 的内存数据是多少？
- Qt Overlay 窗口的 Click-through、置顶、多显示器行为是否稳定？
- Qt 的动画系统能否达到 120FPS？
- Qt 子进程崩溃后，Rust Host 能否正确检测并重启？

## 为什么 Phase 0 验证 Qt

Qt 在"轻量桌面 Widget"场景下已被开发者实战验证（此前用 Qt 实现过 Dock，内存极低）。纳入 BetterCPT 意味着 v1 零自建模块——场景图、动画系统、渲染循环全部由 Qt 提供。但以下问题需要在 Phase 0 中用数据回答：

- Qt 子进程 + QApplication 的基础内存开销
- 共享 vs 独立 Qt 进程在 10 个 Widget 时的内存差异
- 构建链：CMake + Qt SDK 进入项目的维护成本
- Qt 的 C++ 代码和 Rust Host 的 IPC 通信延迟

## 与 Phase 1/2 的关系

Phase 1（Host 骨架）和 Phase 2（Script Runtime）**不依赖任何渲染能力**，因此可以与 Phase 0 并行推进。Phase 3（Widget Runtime）必须在 Phase 0 通过验收后才能启动。

```
Phase 0（Qt Spike）       ──→  通过?  ──→  Phase 3（Widget Runtime）
Phase 1（Host 骨架）      ──────────────────────────────→ Phase 2 → Phase 4
Phase 2（Script Runtime）
```

## 验收标准（硬指标）

- **Qt Overlay 窗口**：QWidget 无边框 + WindowStaysOnTopHint + WA_TransparentForMouseEvents 稳定创建，Click-through 在非交互区域正确生效
- **IPC 渲染指令**：Qt 进程接收 JSON-RPC 消息（{"method":"createRect"...}），正确创建并渲染 QWidget
- **动画帧率**：QPropertyAnimation 的缩放动画在 144Hz 显示器上不掉帧，≥ 120FPS
- **内存基准（共享进程）**：1 个 Qt 进程 + 10 个简单 Widget（每个是一个 QWidget 子窗口），Private Working Set < 60MB
- **内存基准（独立进程）**：10 个独立 Qt 进程，每个 1 个 Widget，总 Private Working Set 的实际数据（用于和共享模式对比决策）
- **多显示器**：在双屏/三屏配置上，窗口吸附和 DPI 切换不崩溃
- **崩溃恢复**：手动 kill Qt 进程，Rust Host 在 3 秒内检测到并通过 IPC 重启 Qt 进程
- **CPU 占用**：空闲状态下 Qt 进程 CPU < 1%

## 技术步骤

### 1. 搭建 Qt 构建环境

- 安装 Qt 6.5 LTS（MinGW 或 MSVC 版本，取决于 Rust Host 的 ABI）
- 配置 CMakeLists.txt
- 验证 Qt 项目编译通过

### 2. 创建最小 Qt 验证应用

独立的 Qt C++ 项目，不依赖 BetterCPT Host：

- QApplication 启动
- 创建无边框 QWidget（FramelessWindowHint | WindowStaysOnTopHint | Tool）
- 设置 WA_TransparentForMouseEvents 实现非交互区域穿透
- 吸附屏幕底部居中

### 3. 实现 JSON-RPC IPC 接收端

Qt 侧实现一个简单的 JSON-RPC 接收器（通过 stdin/stdout 或命名管道）：

```cpp
// Qt 侧接收渲染指令
// {"jsonrpc":"2.0","method":"createRect","params":{...},"id":1}
void RenderServer::onMessage(const QJsonObject& msg) {
    if (msg["method"] == "createRect") {
        auto* rect = new QWidget(parent);
        rect->setStyleSheet("background: rgba(255,255,255,0.8); border-radius: 16px;");
        // ...
    }
}
```

### 4. Rust Host 侧启动 Qt 子进程

独立的 Rust 测试项目：

- 使用 `std::process::Command` 启动 Qt exe
- 建立 stdin/stdout pipe 或命名管道用于 IPC
- 发送 JSON-RPC 消息，验证 Qt 进程收到并正确渲染

### 5. 实现缩放动画

使用 QPropertyAnimation（不是手写插值）：

```cpp
auto* anim = new QPropertyAnimation(widget, "geometry");
anim->setDuration(200);
anim->setStartValue(original_rect);
anim->setEndValue(scaled_rect);
anim->setEasingCurve(QEasingCurve::OutCubic);
anim->start();
```

验证：鼠标 Hover 触发动画，帧率 ≥ 120FPS。

### 6. 测试共享进程 vs 独立进程的内存

这是 Phase 0 最重要的数据产出：

- **共享模式**：1 个 Qt 进程，在其中创建 10 个独立 QWidget 窗口。用 Task Manager 测量 Private Working Set
- **独立模式**：启动 10 个独立的 Qt 进程，每个 1 个 QWidget 窗口。测量 10 个进程总计的 Private Working Set
- 记录数据，作为 Phase 3 选择部署模式的决策依据

### 7. 实现崩溃恢复验证

- Rust Host 监控 Qt 子进程（通过监控 pipe 断开或进程退出信号）
- 手动 kill Qt 进程
- 验证 Rust Host 在 3 秒内检测到
- 验证 Rust Host 自动重启 Qt 进程

### 8. 多显示器和 DPI 测试

- 窗口在显示器间拖拽
- 不同 DPI 设置（100%、125%、150%、200%）
- 验证不崩溃，Qt 自动处理 DPI 缩放

### 9. 基本稳定性

- Qt 进程运行 1 小时
- 内存不持续增长
- Explorer 重启后窗口恢复

## 任务清单

- [ ] 安装 Qt 6.5 LTS，配置 CMake 构建环境
- [ ] 创建最小 Qt C++ 项目，验证编译通过
- [ ] 实现无边框 Overlay QWidget 窗口
- [ ] 实现 WA_TransparentForMouseEvents Click-through
- [ ] 实现 WindowStaysOnTopHint 始终置顶
- [ ] 实现窗口吸附屏幕底部居中
- [ ] 实现 QPropertyAnimation 缩放动画
- [ ] 实现鼠标 Hover 触发动画
- [ ] 实现 JSON-RPC IPC 接收端（Qt 侧）
- [ ] 创建 Rust 测试项目，实现子进程启动
- [ ] 实现 Rust → Qt IPC 通信（stdin/stdout pipe）
- [ ] 验证 IPC 渲染指令正确渲染
- [ ] 测量共享 Qt 进程：10 Widget 的内存
- [ ] 测量独立 Qt 进程：10 Widget 的内存
- [ ] 实现崩溃检测和自动重启
- [ ] 多显示器测试
- [ ] 多 DPI 测试
- [ ] 1 小时常驻稳定性测试
- [ ] Explorer 重启恢复测试
- [ ] **验收**：以上所有验收标准通过，根据内存数据决定共享/独立模式 → 进入 Phase 3

## 如果未通过验收

| 失败项 | 排查方向 | 备选方案 |
|---|---|---|
| Qt 独立进程内存过高 | 每个 QApplication 有基础开销 ~5-8MB | 采用共享 Qt 进程模式 |
| Qt 共享进程崩溃影响面大 | QWidget 之间缺乏进程级隔离 | 混合模式：Dock 共享进程，第三方 Widget 独立进程 |
| JSON-RPC IPC 延迟影响动画 | 检查 pipe 缓冲策略 | 渲染指令使用共享内存，JSON-RPC 只用于控制指令 |
| Qt SDK 构建依赖过重 | 评估 CI/CD 影响 | 预先编译 Qt 静态库纳入仓库，或考虑 vcpkg 管理依赖 |
| 多项指标均不理想 | — | 回退到 D2D+DXGI 手写渲染方案（已有完整的 D2D+DXGI Phase 0 设计作为后备） |

---

---

# 第一阶段：Host 核心骨架

## 目标

让 BetterCPT Host 作为后台进程跑起来，管理自身生命周期，为后续所有 Runtime 打地基。

没有这个骨架，Dock 只是一个孤立程序，不是 Runtime 的一部分。

## 验收标准

- Host 静默启动，驻留后台
- 系统托盘图标正常显示
- Host 正常退出，资源完整释放
- 基础 IPC 消息总线可以收发消息
- 日志系统正常写入文件

---

## 技术步骤

### 1. 初始化 Rust 项目结构

```
bettercpt/
  Cargo.toml
  src/
    main.rs             ← Host 入口
    host/
      mod.rs
      lifecycle.rs      ← 启动 / 退出管理
      ipc.rs            ← IPC 消息总线
      logger.rs         ← 日志系统
      tray.rs           ← 系统托盘
    runtime/
      mod.rs            ← Runtime 统一接口（暂时空着）
```

**Cargo.toml 依赖**：

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Shell",
    "Win32_System_Threading",
] }
```

### 2. 实现系统托盘图标

Host 后台运行，系统托盘是唯一入口：

- 右键菜单：打开 Dashboard / 退出
- 托盘图标点击：显示快速菜单

技术要点：
- 使用 `Shell_NotifyIconW` 创建托盘图标
- 创建隐藏消息窗口处理托盘事件（`WM_USER` 自定义消息）

### 3. 实现单例锁

防止 Host 重复启动：
- 使用 Windows Named Mutex
- 启动时检测，已有实例则退出

### 4. 实现基础 IPC 消息总线

Host 内部各模块通信的基础，后续所有 Runtime 都接入这里：

```rust
#[derive(Serialize, Deserialize)]
pub struct IpcMessage {
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}
```

使用 `tokio::sync::mpsc` 作为内部消息总线。

### 5. 实现日志系统

- `tracing` + `tracing-subscriber`
- 日志写入：`%AppData%/BetterCPT/logs/host.log`
- Debug 模式同时输出到控制台

---

## 任务清单

- [ ] 初始化 Cargo 项目，配置依赖
- [ ] 实现 main.rs 入口，初始化 tokio 运行时
- [ ] 实现日志系统（文件 + 控制台）
- [ ] 实现单例锁（Named Mutex）
- [ ] 实现隐藏消息窗口
- [ ] 实现系统托盘图标显示
- [ ] 实现托盘右键菜单（退出功能）
- [ ] 实现 IpcMessage 数据结构
- [ ] 实现内部消息总线（tokio mpsc）
- [ ] 实现优雅退出（清理资源后关闭）
- [ ] **验收**：Host 静默启动 → 托盘图标出现 → 右键退出 → 进程正常关闭

---

---

# 第二阶段：Script Runtime

## 目标

嵌入 Deno Core，验证插件完整生命周期：安装 → 启动 → 运行 → 崩溃恢复 → 停止。

**验收插件**：文件行数统计工具（text-line-counter）。选择这个而不是 PDF 合并器的原因：PDF 处理需要额外的第三方 JS 库（pdf-lib 等），增加了与"生命周期管控"无关的技术复杂度。行数统计只需 fs:read 权限即可验证所有核心机制。

这一阶段解决的核心问题：生命周期管控机制跑通了，第三阶段的 Widget Runtime 直接复用同一套 Host 框架。

## 验收标准

- Host 可以加载 Script 插件
- 插件可以读写文件（经过权限校验）
- 插件通过 IPC 和 Host 通信正常
- 插件崩溃后 Host 自动重启（最多 5 次）
- 插件正常停止并释放资源

---

## 技术步骤

### 1. 添加 Deno Core 依赖

```toml
[dependencies]
deno_core = "0.290"   # 版本以实际最新为准
```

### 2. 实现 manifest 解析器

```rust
#[derive(Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub runtime: RuntimeType,
    pub permissions: Vec<String>,
    pub entry: String,
}

#[derive(Serialize, Deserialize)]
pub enum RuntimeType {
    #[serde(rename = "widget")]  Widget,
    #[serde(rename = "script")]  Script,
    #[serde(rename = "native")]  Native,
}
```

### 3. 实现权限系统

权限定义：

```
fs:read          → 读取文件
fs:write         → 写入文件
network:fetch    → 网络请求
dialog:open      → 打开文件对话框
dialog:save      → 保存文件对话框
system:info      → 读取系统信息
```

实现：
- 解析 manifest permissions 字段
- 在 Deno Core 中拦截对应 API
- 未声明权限的调用直接返回错误

### 4. 实现 Script Runtime 管理器

```rust
pub struct ScriptRuntime {
    plugin_id: String,
    js_runtime: deno_core::JsRuntime,
    permissions: PermissionSet,
}

impl ScriptRuntime {
    pub async fn start(&mut self, entry: &str) -> Result<()>
    pub async fn stop(&mut self) -> Result<()>
    pub async fn send_message(&mut self, msg: IpcMessage) -> Result<()>
}
```

### 5. 实现插件 IPC 桥接

插件侧调用方式：

```typescript
// 插件 JS 代码
const data = await BetterCPT.invoke("fs.readFile", { path: "C:/a.pdf" })
await BetterCPT.invoke("fs.writeFile", { path: "C:/out.pdf", data })
```

Host 侧注册 op：

```rust
#[deno_core::op2::op]
async fn op_invoke(method: String, params: Value) -> Result<Value>
```

### 6. 实现崩溃监控和自动重启

- 每个 Script Runtime 跑在独立 tokio task
- task panic 时 Host 捕获，等待 3 秒后自动重启
- 重启超过 5 次则停止重启并通知用户

### 7. 编写验收插件：文件行数统计工具（text-line-counter）

选择这个插件的原因：只需要 fs:read 权限即可验证核心机制（IPC 通信、权限拦截、崩溃恢复），不需要引入第三方 JS 库依赖。等生命周期管控验证通过后，PDF 处理等复杂插件可以随时添加。

manifest.json：

```json
{
  "id": "text-line-counter",
  "name": "文件行数统计工具",
  "version": "1.0.0",
  "runtime": "script",
  "permissions": ["fs:read", "dialog:open"],
  "entry": "index.ts"
}
```

index.ts：

```typescript
// 验收插件：统计选中文本文件的行数
const file = await BetterCPT.dialog.openSingle({ filter: "*.txt" })
const content = await BetterCPT.fs.readTextFile(file)
const lines = content.split("\n").length
await BetterCPT.notification.show("统计完成", `文件共 ${lines} 行`)
```

---

## 任务清单

- [ ] 添加 deno_core 依赖，验证编译通过
- [ ] 实现 PluginManifest 数据结构和 JSON 解析
- [ ] 实现 RuntimeType 枚举和分发逻辑
- [ ] 实现权限列表定义和 PermissionSet 结构
- [ ] 实现 ScriptRuntime 结构体
- [ ] 实现 Deno JsRuntime 初始化
- [ ] 实现权限拦截（未授权调用返回错误）
- [ ] 实现 `BetterCPT.invoke` 全局 API 注入
- [ ] 实现 `fs.readFile` / `fs.writeFile` op
- [ ] 实现 `dialog.open` / `dialog.save` op
- [ ] 实现插件启动流程（manifest → Runtime → entry）
- [ ] 实现插件停止流程（信号 → 等待 → 强制终止）
- [ ] 实现崩溃监控（task panic 捕获）
- [ ] 实现自动重启（3秒延迟，最多 5 次）
- [ ] 编写 text-line-counter 验收插件
- [ ] **验收**：text-line-counter 正常统计文件行数，崩溃后自动重启；未声明权限（如 fs:write）的调用被拦截

---

---

# 第三阶段：Widget Runtime（v1 — Qt 渲染后端）

## 目标

这是整个项目最核心的验证阶段。

实现 Widget Runtime v1，用 Qt（C++ 子进程 + JSON-RPC IPC）渲染第一个桌面 Widget。所有场景图、动画系统、渲染循环均由 Qt 提供，v1 零自建模块。

**验收目标：一个流畅的 macOS 风格 Dock**

要求：
- 120FPS 流畅动画（QPropertyAnimation，GPU 加速由 Qt 自动管理）
- 毛玻璃背景（Qt::WA_TranslucentBackground + 手动模糊）
- Hover 放大动画（QEasingCurve::OutCubic，框架自带）
- 内存根据 Phase 0 实测数据决定（共享模式预期 < 50MB，独立模式预期 < 120MB）
- Widget SDK 命令式 API 正常控制渲染（TS → Deno → Rust Host → Qt IPC）

只有 Dock 真正成功，才能证明 BetterCPT Runtime 本身成立。

**v1 不使用 DirectComposition**。DComp 会在 Runtime 成立后作为 v2+ 增强层引入——Qt 渲染到离屏 Surface → DComp 合成。

## 验收标准

- Dock 常驻桌面底部，始终置顶
- 毛玻璃背景效果正常
- 鼠标悬停时图标平滑放大，动画无撕裂
- 动画由 GPU 驱动（Qt 自动选择 OpenGL/D3D 后端），CPU 占用极低
- 内存：共享 Qt 进程模式 < 60MB（含 10 个 Widget）；Dock 单独 < 25MB
- Widget SDK 命令式 API 正常控制渲染
- Qt 子进程崩溃后，Rust Host 自动检测并重启，Widget 恢复显示

### 验收测量方法

| 指标 | 测量工具 | 方法 |
|---|---|---|
| 帧率 | PresentMon 或 Qt 内置 QElapsedTimer | 录制动画期间帧间隔 |
| 内存 | Task Manager 详情 | Dock 单独 Private Working Set；10 Widget 总计 Private Working Set |
| CPU | Task Manager | 空闲 Qt 进程 CPU%；动画期间 Host+Qt 合计 CPU% |
| 崩溃恢复 | 手动 kill Qt 进程 | 计时从 kill 到 Widget 恢复显示的时间 |

---

## 技术步骤

### 1. 搭建 Qt + Rust Host 项目结构

```
bettercpt/
  host/                    ← Rust Host（已有的 Phase 1 代码）
  widget-qt/               ← Qt C++ 项目（新增）
    CMakeLists.txt
    main.cpp               ← QApplication + IPC 接收端
    RenderServer.cpp/h     ← JSON-RPC 渲染指令处理
    WidgetManager.cpp/h    ← QWidget 生命周期管理
```

### 2. 实现 Qt Overlay 窗口

```cpp
// Qt 无边框 Overlay 窗口
auto* dock = new QWidget(nullptr);
dock->setWindowFlags(
    Qt::FramelessWindowHint |
    Qt::WindowStaysOnTopHint |
    Qt::Tool
);
dock->setAttribute(Qt::WA_TranslucentBackground);  // 毛玻璃背景
dock->setAttribute(Qt::WA_TransparentForMouseEvents, false);  // 图标区域不透
// 非图标区域通过 setMask() 裁剪，实现部分穿透
```

### 3. 实现 JSON-RPC IPC 接收端

Qt 侧通过 stdin/stdout pipe 接收 Rust Host 的 JSON-RPC 指令：

```cpp
void RenderServer::processMessage(const QJsonObject& msg) {
    QString method = msg["method"].toString();
    if (method == "createRect") {
        auto params = msg["params"].toObject();
        auto* w = WidgetManager::createRect(params);
        sendResponse(msg["id"].toInt(), {{"widgetId", w->id()}});
    } else if (method == "setScale") {
        WidgetManager::getWidget(params["id"])->setScale(
            params["scale"].toDouble(), params["duration"].toInt());
    }
}
```

渲染原语映射：

| SDK API (TS) | JSON-RPC | Qt 操作 |
|---|---|---|
| `createRect({blur,radius,w,h})` | `{"method":"createRect",...}` | `new QWidget()` + stylesheet |
| `createIcon({src,size})` | `{"method":"createIcon",...}` | `new QLabel()` + `QPixmap` |
| `setScale(1.2, 200)` | `{"method":"setScale",...}` | `QPropertyAnimation("geometry")` |
| `setBlur(true)` | `{"method":"setBlur",...}` | `setAttribute(WA_TranslucentBackground)` |

### 4. 实现 Hover 动画（QPropertyAnimation）

Qt 自带动画系统，不需要手写插值：

```cpp
void WidgetController::animateScale(QWidget* w, double target, int durationMs) {
    auto* anim = new QPropertyAnimation(w, "geometry");
    anim->setDuration(durationMs);
    anim->setStartValue(w->geometry());
    QRect targetRect = /* 基于 target scale 计算 */;
    anim->setEndValue(targetRect);
    anim->setEasingCurve(QEasingCurve::OutCubic);
    anim->start(QAbstractAnimation::DeleteWhenStopped);
}
```

### 5. 实现毛玻璃背景

```cpp
// Qt 毛玻璃：半透明背景 + 模糊效果
dock->setStyleSheet("background: rgba(30, 30, 30, 0.85); border-radius: 16px;");
// 如需更高级的模糊，使用 QGraphicsBlurEffect
auto* blur = new QGraphicsBlurEffect();
blur->setBlurRadius(20);
dock->setGraphicsEffect(blur);
```

### 6. 实现 Rust Host 侧的 Qt 进程管理

```rust
pub struct QtWidgetRuntime {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl QtWidgetRuntime {
    pub fn start(qt_exe_path: &str) -> Result<Self> {
        let mut child = Command::new(qt_exe_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        // ...
    }

    pub fn send_render_cmd(&mut self, msg: IpcMessage) -> Result<()> {
        serde_json::to_writer(&mut self.stdin, &msg)?;
        self.stdin.write_all(b"\n")?;
        Ok(())
    }

    pub fn check_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))  // None = still running
    }
}
```

### 7. 实现崩溃恢复

- Rust Host 每 5 秒检查 Qt 子进程存活（`try_wait`）
- IPC pipe 断开时标记 Qt 进程为 dead
- 等待 3 秒后自动重启 Qt 进程
- 重启后重放当前已注册 Widget 的渲染指令，恢复显示

### 8. 实现 Widget SDK 命令式 API

SDK 开发者 API 保持不变。内部流程：

```typescript
// TS 侧（不变）
icon.onHover(() => icon.setScale(1.2, 200))
  ↓
// Deno Core → IPC → Rust Host → JSON-RPC → Qt 进程
{"method":"setScale","params":{"id":"icon-3","scale":1.2,"duration":200}}
  ↓
// Qt 侧：QPropertyAnimation 执行缩放
```

### 9. 编写 Dock 验收插件

```typescript
import { createRect, createIcon } from "bettercpt-sdk"
import { system } from "bettercpt-sdk/system"

const dock = createRect({ blur: true, radius: 16, width: 400, height: 64 })

const apps = [
  { icon: "chrome.png", path: "C:/..." },
  { icon: "vscode.png", path: "C:/..." },
]

for (const app of apps) {
  const icon = createIcon({ src: app.icon, size: 48 })
  dock.addChild(icon)
  icon.onHover(() => icon.setScale(1.2, 200))
  icon.onLeave(() => icon.setScale(1.0, 200))
  icon.onClick(() => system.launch(app.path))
}
```

---

## 任务清单（v1 — Qt 渲染后端）

- [ ] 搭建 Qt 6.5 C++ 项目结构（CMakeLists.txt + main.cpp）
- [ ] 实现 QApplication + 事件循环启动
- [ ] 实现无边框 Overlay QWidget（FramelessWindowHint | StaysOnTopHint | Tool）
- [ ] 实现 WA_TranslucentBackground 毛玻璃背景
- [ ] 实现 WA_TransparentForMouseEvents + setMask 部分点击穿透
- [ ] 实现窗口吸附屏幕底部居中
- [ ] 实现 JSON-RPC IPC 接收端（stdin/stdout pipe）
- [ ] 实现 RenderServer：createRect / createIcon / createText 指令
- [ ] 实现 QPropertyAnimation 缩放动画（OutCubic easing）
- [ ] 实现鼠标 Hover 检测和动画触发
- [ ] Rust Host 侧：实现 Qt 子进程启动（std::process::Command）
- [ ] Rust Host 侧：实现 IPC 消息发送（stdin pipe）
- [ ] Rust Host 侧：实现 IPC 响应接收（stdout pipe）
- [ ] 实现 Widget Deno Core 单实例初始化
- [ ] 实现 SDK IPC 协议（TS → Deno → Rust Host → Qt IPC）
- [ ] 实现 createRect / createText / createIcon SDK API
- [ ] 实现 setScale / setBlur / onHover / onLeave / onClick SDK API
- [ ] 实现 Qt 进程崩溃检测（try_wait + pipe broken）
- [ ] 实现 Qt 进程自动重启 + Widget 状态恢复
- [ ] 编写 Dock 验收插件
- [ ] **验收**：Dock 常驻底部，毛玻璃背景，Hover 流畅放大，内存达标，崩溃恢复正常

---

---

# 第四阶段：Dashboard

## 目标

用 Tauri v2 + React 实现管理后台，让用户通过图形界面管理插件。

Dashboard 只在用户主动打开时启动，关闭后完整释放内存。

## 验收标准

- 点击托盘图标可以打开 Dashboard
- 显示已安装插件列表，状态实时同步
- 可以启动 / 停止单个插件
- 可以从本地文件夹安装新插件
- 关闭后内存完全释放

---

## 技术步骤

### 1. 初始化 Tauri v2 项目

在 workspace 里添加：

```
bettercpt/
  host/             ← 已有的 Host
  dashboard/
    src-tauri/      ← Tauri 后端（Rust）
    src/            ← React 前端
    package.json
```

### 2. 实现 Host ↔ Dashboard IPC

```rust
#[tauri::command]
async fn get_plugins() -> Vec<PluginInfo>

#[tauri::command]
async fn start_plugin(id: String) -> Result<()>

#[tauri::command]
async fn stop_plugin(id: String) -> Result<()>

#[tauri::command]
async fn install_plugin(path: String) -> Result<()>
```

### 3. 实现 React 前端页面

```
Dashboard
  ├── 侧边栏
  │     ├── 已安装插件
  │     ├── 插件市场（占位）
  │     └── 设置（占位）
  └── 主内容区
        ├── 插件列表（图标 / 名称 / 版本 / 状态 / 启停按钮）
        └── 从本地安装按钮
```

### 4. 实现插件状态实时同步

Host 推送状态变化事件：

```rust
app.emit("plugin:status-changed", PluginStatusEvent {
    id: "pdf-merger",
    status: PluginStatus::Running,
})?;
```

React 侧监听：

```typescript
await listen("plugin:status-changed", (event) => {
  updatePluginStatus(event.payload)
})
```

### 5. 实现本地插件安装

- 用户选择插件文件夹
- Host 验证 manifest.json 格式
- 复制到 `%AppData%/BetterCPT/plugins/`
- 刷新插件列表

---

## 任务清单

- [ ] 初始化 Tauri v2 项目，配置 workspace
- [ ] 配置 Tauri 窗口（无边框 / 圆角 / 固定尺寸）
- [ ] 实现 get_plugins / start_plugin / stop_plugin / install_plugin Command
- [ ] 实现插件状态变化事件推送
- [ ] 搭建 React 项目（Vite + TypeScript + Tailwind）
- [ ] 实现侧边栏导航
- [ ] 实现插件列表页面和插件卡片组件
- [ ] 实现实时状态监听（plugin:status-changed）
- [ ] 实现本地安装入口（文件夹选择 + manifest 验证）
- [ ] 实现托盘菜单点击打开 Dashboard
- [ ] 实现 Dashboard 关闭时销毁 WebView 释放内存
- [ ] **验收**：完整走通 安装 → 启动 → 停止 → 卸载 插件流程

---

---

## 各阶段难度参考

| 阶段 | 难度 | 主要风险 |
|---|---|---|
| Phase 0：Qt Render Spike | ★★☆☆☆ | Qt 本身成熟，主要验证 Rust↔Qt IPC 通信和共享/独立进程的内存数据 |
| 第一阶段：Host 骨架 | ★★☆☆☆ | Win32 托盘 API 细节多 |
| 第二阶段：Script Runtime | ★★★☆☆ | Deno Core 嵌入文档少，AI 帮助有限 |
| 第三阶段：Widget Runtime (v1) | ★★★☆☆ | Qt 提供场景图+动画+渲染循环，主要工作在 IPC 协议和进程管理 |
| 第四阶段：Dashboard | ★★☆☆☆ | Tauri + React 生态成熟，AI 帮助很好 |
| 第五阶段：DComp 增强层 | ★★★★☆ | DComp 引入难度高，但已有 v1 Runtime 基础，Qt→离屏→DComp 逐步替换 |

**第三阶段特别提示**：v1 使用 Qt 作为渲染后端，场景图、动画系统、渲染循环全部由 Qt 提供。主要开发量在 Rust↔Qt 的 JSON-RPC IPC 通信和 Qt 进程的生命周期管理上。这比手写 D2D+DXGI 渲染管线的工作量少一个数量级。

---

## v2+ 增强路径：DirectComposition 渐进引入

v1 用 Qt 子进程跑通 Runtime 后，v2+ 可逐步引入 DComp 作为增强层。在 Qt 方案下，引入路径是 Qt 从独立窗口变成 GPU Surface 生产者：

```
v1:   Qt 独立窗口              ← 每个 Widget 是独立 QWidget
v2+:  Qt → 离屏渲染            ← Qt 渲染到离屏 Surface（QOpenGLWidget / QOffscreenSurface）
      + DComp Visual 树        ← Rust Host 将 Surface 绑定到 DComp Visual
      + DComp Animation        ← DComp 接管动画插值（替代 QPropertyAnimation）
      + DComp Blur             ← DWM 系统级毛玻璃替代 Qt::WA_TranslucentBackground
```

### 引入前提

- v1 Runtime 已稳定运行，有真实用户使用
- 核心架构（IPC、生命周期、权限系统、Deno 嵌入）已被充分验证
- DComp 引入不影响现有 Widget 的运行（检测 + 回退机制：DComp 不可用则保持 Qt 独立窗口模式）
- Widget SDK 开发者 API 保持兼容（底层从 QWidget 窗口 → DComp Visual，SDK 不变）

### 各步收益

| 步骤 | 引入内容 | 对 v1 的改善 |
|---|---|---|
| v2: Qt 离屏渲染 | QOffscreenSurface → DXGI Shared Handle | Qt 窗口不再独立存在，由 DComp 统一合成 |
| v3: DComp Visual Tree | Visual 树替代 QWidget 层级 | 系统级合成优化；所有 Widget 在一个合成树中 |
| v4: DComp Animation | CreateAnimation/AddCubic 替代 QPropertyAnimation | GPU 自动插值，CPU 更低 |
| v5: DComp Blur | DWM Blur/Acrylic 替代 Qt 手动模糊 | 原生毛玻璃，视觉效果提升 |

---

## 测试策略

### Phase 1（Host 骨架）

- manifest 解析器单元测试（合法 JSON / 非法 JSON / 缺失字段 / 错误 runtime 类型）
- IpcMessage 序列化往返测试
- 单例锁获取和释放测试
- Host 启动 → 托盘图标 → 退出 的端到端手动验收

### Phase 2（Script Runtime）

- 权限系统单元测试：声明权限通过、未声明权限拦截、空权限列表
- Deno op 注册和调用测试
- Script Runtime 崩溃恢复测试（在插件 TS 代码中主动 throw / 死循环）
- 重启次数限制测试（连续崩溃 5 次后停止重启）

### Phase 3（Widget Runtime — Qt）

- 帧率：PresentMon 录制 QPropertyAnimation 动画帧间隔
- 内存：启动 10 分钟和 2 小时后的 Qt 进程 Private Working Set 对比
- 崩溃恢复：手动 kill Qt 进程，验证 Rust Host 检测 + 重启 + Widget 恢复
- Widget Deno 实例崩溃：Deno 重启，Qt 进程不受影响，Widget 重新注册
- 多显示器：物理插拔显示器测试
- IPC 延迟：测量 TS 调用到 Qt 渲染完成的端到端延迟

### 不做的事

- Widget 视觉效果的像素级自动化截图对比（成本极高，收益低）
- 跨 Windows 版本全矩阵测试（MVP 阶段只测 Win10 21H2+ / Win11）

---

---

# 第五阶段：DComp 增强层（远期，v2+）

## 目标

在 v1 Runtime 稳定运行后，按能力逐步引入 DirectComposition，替代 v1 中自建的对应模块。每一步都是增量替换，出问题可回退。

## 前提条件

以下条件必须全部满足才能启动任一 DComp 增强步骤：

- v1 Runtime 已发布并稳定运行 ≥ 3 个月
- 有活跃用户基础，能获得真实的兼容性反馈
- Host 骨架、Script Runtime、Dashboard 均已完成并通过验收
- DComp 引入配备了能力检测 + 自动回退机制（DComp 不可用时回退到 v1 的 D2D 实现）

## 子阶段

### 5a. DComp Blur 增强

**目标**：用 DWM Blur 或 Acrylic 替代 v1 的 D2D Gaussian Blur Effect。

**验收标准**：
- 在支持 DWM Blur / Acrylic 的系统上，毛玻璃效果与系统原生一致
- 在不支持的系统上，自动回退到 v1 的 D2D Gaussian Blur
- Widget SDK 的 `{ blur: true }` 参数行为不变

**复杂度**：★★☆☆☆（DWM Blur API 简单，Acrylic 稍复杂但文档充分）

### 5b. DComp Animation 增强

**目标**：用 DComp CreateAnimation / AddCubic 替代 v1 的自建插值系统。

**验收标准**：
- Hover 动画使用 DComp Animation，CPU 参与度显著降低
- 回退检测：DComp Animation 创建失败时回退到 v1 插值
- Widget SDK 的 `setScale(x, duration)` 行为不变

**复杂度**：★★★★☆（需要理解 DComp 动画时钟与 DWM 帧同步）

### 5c. DComp Visual Tree 增强

**目标**：将 Qt 的 QWidget 层级迁移到 DComp Visual 树（Qt → 离屏渲染 → DComp 合成）。

**验收标准**：
- Widget 层级关系、渲染顺序、裁剪行为与 v1 一致
- Dirty Rect 由 DComp 自动管理，不需要 Qt 的 QWidget::update()
- 内存进一步降低（DComp Visual 比 QWidget 窗口更轻）

**复杂度**：★★★★★（涉及整个渲染层的重构，是最复杂的增强步骤）

### 不做的事

- 不会一次性全量切换 DComp。任何一步出问题都回退到 v1 的 Qt 独立窗口实现。
- 不会因 DComp 而提高最低 Windows 版本要求。不支持 DComp 的系统沿用 v1 的 Qt 渲染。

---

## 文件目录约定

```
%AppData%/BetterCPT/
  plugins/
    text-line-counter/
      manifest.json
      index.ts
    dock/
      manifest.json
      index.ts
  logs/
    host.log
  config/
    settings.json
```
