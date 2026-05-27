# BetterCPT Widget Runtime 规范

> 定义 Widget Runtime 的渲染管线、Qt 集成、部署模式、动画模型、多显示器行为。
> 
> 不包含通用 Runtime 生命周期（见 RUNTIME_SPEC.md）和 SDK API（见 SDK_SPEC.md）。

---

## 一、渲染管线

```
TS SDK（Deno Core）
  │  调用 createRect / setScale / onHover
  ↓
Host — Command Queue (mpsc)
  ↓
Render Scheduler（16ms flush, 帧级 dedup, batch）
  ↓
Host State Tree（更新 + 版本号递增）
  ↓
IPC Batch（JSON-RPC 2.0，stdin/stdout pipe）
  ↓
Qt 子进程 — Render Server
  │  解析 JSON-RPC → 创建/更新 QWidget
  │  QPropertyAnimation 执行动画
  ↓
Qt 事件循环（QApplication::exec()）
  ↓
GPU 渲染（OpenGL / Direct3D，Qt 自动选择）
  ↓
DWM 合成到桌面
```

### 关键设计

- Qt 事件循环与 Rust tokio 不协同——各自独立运行，仅通过 IPC 通信
- GPU Context 不跨进程共享——每个 Qt 进程有独立的渲染上下文
- TS 代码不直接接触 Qt——所有渲染必须经过 Host → IPC → Qt 管道

---

## 二、Qt 集成

### 进程模型

Qt 作为独立子进程运行。Rust Host 通过 `std::process::Command` 启动 Qt exe，建立 stdin/stdout pipe 用于 IPC。

### 两种部署模式

| 模式 | 内存（10 Widget） | 崩溃隔离 | 适用 |
|---|---|---|---|
| 共享 Qt 进程 | ~30-50MB | 一个崩全崩 | 默认推荐 |
| 独立 Qt 进程 | ~100-120MB | 单个 Widget 隔离 | 高稳定性要求的第三方 Widget |

**v1 默认使用共享 Qt 进程模式。** 架构保持可随时切为独立进程的能力。

### 渲染原语

Qt 侧向 Host 暴露以下渲染原语：

| SDK API (TS) | JSON-RPC Method | Qt 实现 |
|---|---|---|
| createRect({blur,radius,w,h}) | createRect | new QWidget() + stylesheet |
| createText(text,{size,weight}) | createText | new QLabel() |
| createIcon({src,size}) | createIcon | QLabel + QPixmap |
| setScale(s, duration) | setScale | QPropertyAnimation("geometry") |
| setOpacity(o, duration) | setOpacity | QPropertyAnimation("windowOpacity") |
| setBlur(enabled) | setBlur | setAttribute(WA_TranslucentBackground) |
| addChild(parent, child) | addChild | child->setParent(parent) |
| removeChild(parent, child) | removeChild | child->setParent(nullptr) |

### 毛玻璃背景

```cpp
// 方案 A：半透明背景（所有系统通用）
widget->setAttribute(Qt::WA_TranslucentBackground);
widget->setStyleSheet("background: rgba(30,30,30,0.85); border-radius: 16px;");

// 方案 B：DWM Blur（Windows 特定，需额外检测）
// v1 使用方案 A，v2+ DComp 引入后切换到 DWM Blur
```

### 点击穿透

```cpp
// 非图标区域允许鼠标穿透
widget->setAttribute(Qt::WA_TransparentForMouseEvents, false);  // 默认不透
// 通过 setMask() 裁剪非交互区域
widget->setMask(QRegion(interactive_rects));
```

---

## 三、动画模型

v1 使用 QPropertyAnimation（Qt 内置），不需要手写插值。

```cpp
auto* anim = new QPropertyAnimation(widget, "geometry");
anim->setDuration(durationMs);
anim->setStartValue(widget->geometry());
anim->setEndValue(targetRect);
anim->setEasingCurve(QEasingCurve::OutCubic);
anim->start(QAbstractAnimation::DeleteWhenStopped);
```

### 动画状态

- 动画中间帧由 Qt 本地管理，不同步回 Host（每秒 120 次变更，同步无意义且浪费 IPC 带宽）
- 动画结束时 Qt 发送 `event.animationCompleted` 到 Host
- Qt 崩溃时动画中间帧丢失，Host replay 时从起始值恢复

### v2+ DComp 替换

QPropertyAnimation → DComp CreateAnimation/AddCubic。GPU 自动插值，CPU 参与度更低。SDK API 不变。

---

## 四、窗口管理

### Overlay 窗口属性

```cpp
widget->setWindowFlags(
    Qt::FramelessWindowHint       // 无边框
    | Qt::WindowStaysOnTopHint    // 始终置顶
    // Qt::Tool 已移除，见 Phase 0 验收报告问题 6
    // 任务栏隐藏通过 Win32 SetWindowLong 设置 WS_EX_TOOLWINDOW 实现
);

// 窗口显示后执行：
HWND hwnd = (HWND)widget->winId();
SetWindowLong(hwnd, GWL_EXSTYLE,
    GetWindowLong(hwnd, GWL_EXSTYLE) | WS_EX_TOOLWINDOW);
```

> ⚠️ **Phase 3 已知问题**：`SetWindowLongPtrW` + `WS_EX_TOOLWINDOW` 在部分 widget 上调用时机不正确会导致 Qt 进程崩溃。Phase 4 需研究安全的调用时机（可能需要在 `QWidget::create()` 窗口创建回调中执行，Qt6 未公开此 hook）。当前每个 widget 会显示在任务栏。

### 多显示器

- Qt 自动处理 DPI 缩放（`AA_EnableHighDpiScaling`）
- 窗口吸附：Host 计算目标显示器底部居中坐标，通过 IPC 发送 setBounds
- 显示器热插拔：监听 `QScreen::geometryChanged`，通知 Host 重新计算布局

### Z-Order

- 多个 Widget 窗口之间：Host 维护 z-order 表，通过 IPC 通知 Qt 调用 `raise()` / `lower()`
- Widget 与普通窗口之间：`WindowStaysOnTopHint` 保证 Widget 始终在普通窗口之上

---

## 五、内存基线

| 组件 | Private Working Set |
|---|---|
| QApplication 基础 | ~5-8MB |
| 单个空 QWidget (Overlay) | ~2-4MB |
| Dock（含 6 个图标 + 毛玻璃背景） | ~20-25MB |
| 10 个简单卡片（共享 Qt 进程） | ~30-50MB |

具体数据以 Phase 0 Spike 实测为准。

---

## 六、GPU 资源生命周期

所有 GPU 资源（texture、pixmap、blur surface 等）的生命周期由 **Qt 子进程独立管理**。

- Widget 创建时 Qt 分配资源
- Widget 销毁时 Qt 释放资源（`DeleteWhenStopped` / parent-child 析构链）
- Host 和插件**不跨进程管理 GPU 资源**
- Qt 崩溃时所有 GPU 资源随进程一并释放，新 Qt 进程重新分配

---

## 八、事件路由模型

v1 采用简化的 hit-testing 规则：

1. **z-order 最高的 Widget 优先接收事件**（Qt `raise()`/`lower()` 决定的窗口层级，非 DOM 树深度）
2. **穿透区域**：通过 `setMask(QRegion)` 挖空的部分，事件透传到桌面
3. **不做 bubble / capture** — 那是 DOM 的事件模型。事件由目标 Widget 直接处理，不向上/向下传递
4. **focus 管理**：最后被点击的 Widget 持有焦点；Qt 崩溃恢复后焦点统一重置

鼠标事件原语（`onMouseDown` / `onMouseMove` / `onMouseUp`）遵循相同路由规则，从 Qt 子进程转发到 Host，再路由到对应的 Isolate。

---

## 九、Render Transaction 说明

Widget 的多条渲染指令（如 `setWidth` + `setHeight` + `setOpacity`）按帧自动打包为单个 IPC batch（见 `RUNTIME_SPEC.md` Render Scheduler）。不会出现逐条 IPC 导致的 layout thrash / repaint thrash / 闪烁问题。

插件开发者不需要手动调用 `batch()`——Scheduler 在 16ms flush 时自动合并。

---

## 七、与 Script Runtime 的区别

| | Widget Runtime | Script Runtime |
|---|---|---|
| 渲染引擎 | Qt (C++ 子进程) | 无（纯逻辑） |
| 生命周期 | 常驻后台 | 按需启动，用完释放 |
| Capability | widget:overlay | window:create |
| 典型插件 | Dock, 天气卡片, 系统监控 | PDF 工具, 文件转换, 自动化 |
