# BetterCPT 窗口系统

> 定义窗口的所有权、生命周期、类型、Backend。窗口是 Host 级能力，不是 Runtime 特性。

---

## 一、核心原则

**Host 拥有窗口所有权。**

插件不直接创建窗口。插件声明窗口意图，Host 决定是否创建、用什么技术创建、何时销毁。

```
插件 → Window Service API → Host → Window Backend → 实际窗口
```

## 二、窗口类型

| 类型 | 定位 | 常驻 | 需要 Overlay | 需要高性能渲染 |
|---|---|---|---|---|
| Overlay Widget | 桌面常驻组件 | ✅ | ✅ | ✅ |
| Utility Window | 功能型插件界面 | ❌ | ❌ | ❌ |
| Dashboard | 管理界面 | ❌ | ❌ | ❌ |
| Dialog | 系统对话框 | ❌ | ❌ | ❌ |

### Overlay Widget

- 归属：Widget Runtime
- Backend：Qt
- 特征：无边框、置顶、点击穿透、毛玻璃、120FPS 动画
- 详见 `WIDGET_RUNTIME.md`

### Utility Window

- 归属：Host Window Service
- Backend（v1）：WebView / Tauri
- 特征：标准窗口、有标题栏、可缩放、不常驻
- 典型场景：PDF 工具、文件转换器、自动化面板

### Dashboard

- 归属：Host（内置）
- Backend：Tauri v2 + React
- 特征：管理界面，不可被第三方插件替代
- 详见 `PROJECT_OVERVIEW.md`

### Dialog

- 归属：Host Window Service
- Backend：系统原生（Win32 `MessageBoxW` / `TaskDialog` / `GetOpenFileNameW` / `GetSaveFileNameW`）
- 类型：

| Dialog 类型 | API | 说明 |
|---|---|---|
| 文件打开 | `dialog.open({ filter })` | 系统原生文件选择 |
| 文件保存 | `dialog.save({ filter })` | 系统原生保存对话框 |
| 消息提示 | `dialog.alert(msg)` | 仅确定按钮 |
| 确认框 | `dialog.confirm(msg)` | 确定/取消，返回 boolean |
| 输入框 | `dialog.prompt(msg, default?)` | 输入框+确定/取消，返回 string 或 null |

- Widget 和 Script 均可调用，不需要额外权限声明
- 底层是同步阻塞的系统调用，不影响 Deno event loop

---

## 三、Window Service API

v1 最小集：

```typescript
// 创建 Utility Window
const win = await BetterCPT.window.create({
  type: "utility",
  title: "PDF Toolkit",
  width: 1000,
  height: 700,
  resizable: true
})

// 关闭窗口
await BetterCPT.window.close(win.id)

// 窗口大小变化事件
BetterCPT.window.onResize(win.id, (size) => {
  // { width, height }
})

// 设置最小尺寸
BetterCPT.window.setMinSize(win.id, { width: 600, height: 400 })
```

- Backend 字段不暴露给插件开发者
- Host 根据窗口类型和当前环境自动选择 Backend
- Host 自动记住上次窗口位置和大小，下次 `create()` 同名插件时恢复

---

## 四、窗口生命周期

```
插件请求创建 → Host 权限检查 → 创建 Backend 窗口 → 加载插件内容
                                                      ↓
                                              用户关闭 / 插件停止
                                                      ↓
                                              窗口销毁
```

规则：

- Utility Window 随插件生命周期：插件停止 → 窗口关闭
- 窗口关闭不等同于插件停止（用户先关窗口，插件可以不立即停止）
- Host 崩溃恢复时：Utility Window 不恢复（非关键），Overlay Widget 恢复

---

## 五、Backend 设计

**Window 是 Capability，Backend 是实现。两者不绑定。**

| Backend | v1 用途 | 特点 |
|---|---|---|
| Qt | Overlay Widget | GPU 渲染、高性能动画 |
| WebView (Tauri) | Utility Window + Dashboard | 低门槛、高生产力、适合应用界面 |
| 系统原生 | Dialog | 零成本 |

### WebView 生命周期

```
Host 按需启动 Tauri WebView → 加载插件 HTML/JS
                              ↓
                        用户关闭窗口
                              ↓
                        WebView 销毁，释放内存
```

关键约束：

- WebView 不常驻，不共享（每个 Utility Window 独立实例）
- 窗口关闭即释放，内存问题不累积

---

## 六、Capability 要求

| 窗口类型 | 需要的 Capability | 说明 |
|---|---|---|
| Utility Window | `window:create` | 在 manifest 的 `capabilities` 中声明 |
| Overlay Widget | `widget:overlay` | Widget Runtime 专属 |

`window:create` 纳入 `SECURITY_MODEL.md` Capability 表，遵循 Capability Ceiling 检查流程。

---

## 七、v1 不做的事

- ❌ 多 Backend 动态切换
- ❌ 自定义窗口外观（无边框、异形窗口）
- ❌ 多窗口管理 / 窗口组
- ❌ 窗口状态持久化
- ❌ Utility Window 崩溃恢复
