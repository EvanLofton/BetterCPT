# BetterCPT 安全模型

> 定义权限模型、Runtime 隔离、信任边界。BetterCPT 是插件执行平台，安全不是附属功能。

---

## 一、信任模型

**核心原则：Widget Runtime 默认不可信。Script Runtime 有限可信。Native Runtime 需显式授权。**

| Runtime | 信任级别 | 隔离方式 |
|---|---|---|
| Widget | 不可信 | Qt 子进程 + Deno 沙箱 |
| Script | 有限可信 | Deno 沙箱 + 权限拦截 |
| Native | 需显式授权 | 独立子进程 + 签名 + 二次确认 |
| Dashboard | 可信（内置） | Tauri WebView，不可被第三方插件替代 |

### Runtime / Capability / Permission 三层分离

manifest.json 不混淆三层概念：

| 字段 | 含义 | 举例 |
|---|---|---|
| `runtime` | 代码的执行环境和生命周期模型 | `"widget"` / `"script"` |
| `capabilities` | 可使用的系统能力 | `"widget:overlay"` / `"window:create"` / `"notification"` |
| `permissions` | 可访问的系统资源 | `"fs:read"` / `"network:fetch"` / `"storage"` |

一个插件只有一个 runtime（执行环境），可以有多个 capabilities（系统能力）和多个 permissions（资源访问）。

---

## 二、Capability 模型

### Capability 列表

系统能力——插件"能调用什么系统服务"。

| Capability | 含义 | Widget | Script | Native |
|---|---|---|---|---|
| `widget:overlay` | Overlay 桌面窗口渲染 | ✅ | ❌ | ❌ |

> **v1 简化说明**：`widget:overlay` 在 v1 包含了原 `widget:render`（创建和管理 Widget 元素）的语义。理论上"能渲染 Widget"和"能置顶 Overlay 窗口"是可分的（一个插件可能只需要渲染能力而不需要桌面置顶），但 v1 只有 Overlay 这一种渲染载体，两者始终共存，故合并。v2 如需支持非 Overlay 的 Widget 渲染（如嵌入 Dashboard 的预览卡片），可重新拆分出独立的 `widget:render` Capability。
| `window:create` | 创建功能窗口（Utility Window） | ✅ | ✅ | ✅ |
| `notification` | 系统通知推送 | ✅ | ✅ | ✅ |

### Permission 列表

资源访问——插件"能触碰什么系统资源"。

| Permission | 含义 | Widget | Script | Native |
|---|---|---|---|---|
| `fs:read` | 读取文件 | ❌ | ✅ | ✅ |
| `fs:write` | 写入文件 | ❌ | ✅ | ✅ |
| `network:fetch` | 网络请求 | ⚠️ 仅白名单域名 | ✅ | ✅ |
| `storage` | KV 数据持久化（Host 管理，上限 1MB） | ✅ | ✅ | ✅ |
| `dialog:open` | 打开文件对话框 | ❌ | ✅ | ✅ |
| `dialog:save` | 保存文件对话框 | ❌ | ✅ | ✅ |
| `system:info` | 系统信息读取 | ❌ | ✅ | ✅ |

### 内置能力（无需声明）

以下能力 Widget 和 Script 均可直接使用，不需要在 manifest 中声明：

- `dialog.alert` / `dialog.confirm` / `dialog.prompt` — 系统消息弹窗
- `system.memory()` — 性能指标
- `setTimeout` / `setInterval` — 定时器

### 检查流程

```
安装插件
  → 解析 manifest.json
  → 提取 runtime 字段 → 确定 Capability 上限表 + Permission 上限表
  → 逐一检查 capabilities：
      ✅ 在 Capability 上限内 → 通过
      ❌ 超出上限 → 拒绝安装
  → 逐一检查 permissions：
      ✅ 在 Permission 上限内 → 通过
      ❌ 超出上限 → 拒绝安装
  → Native Runtime：额外弹出二次确认
```

### Widget 网络限制

Widget 仅允许 `network:fetch`，且受域名白名单限制：

```json
{
  "permissions": ["network:fetch"],
  "network": {
    "allowedDomains": ["api.weather.com", "api.calendar.com"]
  }
}
```

超出白名单的请求被 Host 拦截。

### 四层架构

```
manifest.json（插件声明）
  ├── runtime        → 执行环境 + 生命周期
  ├── capabilities   → 系统能力声明
  └── permissions    → 资源访问声明
        ↓
Host Ceiling（硬编码上限，按 runtime 类型）
  ├── Capability Ceiling
  └── Permission Ceiling
        ↓
Capability Registry（Host 内部注册表，capability/permission → 实现绑定）
        ↓
Runtime Binding（Deno op / SDK API / IPC command）
```

---

## 三、Runtime 隔离

| 隔离维度 | Widget | Script | Native |
|---|---|---|---|
| 进程隔离 | ✅ Qt 子进程 | ❌ (Deno 在 Host 内) | ✅ 独立子进程 |
| 文件系统 | ❌ (不可访问) | ✅ (仅声明路径) | ✅ (全量) |
| 网络 | ⚠️ 白名单域名 | ✅ | ✅ |
| GPU 上下文 | 独立（Qt 管理） | 无 | 独立 |
| 崩溃影响 | 单个或全部 Widget 渲染 | 单个插件 | 自身 |

### Widget Deno 沙箱

Deno Core 默认不暴露 `Deno.readFile`、`Deno.writeFile`、`Deno.run` 等危险 API。Widget 的 Deno 实例注入 SDK 对象（`createRect` 等）以及 `BetterCPT.storage`、`BetterCPT.dialog`（alert/confirm/prompt）、`BetterCPT.notification`，不注入 `BetterCPT.invoke`。

### Script Deno 沙箱

Script 的 Deno 实例注入 `BetterCPT.invoke`，每次调用都经过 Capability 和 Permission 检查。未在 manifest 中声明的 capabilities 和 permissions 在运行时被拦截。

---

## 四、Native Runtime 安全

**v1 默认关闭，MVP 不实现。**

开启条件（必须同时满足）：
- 用户手动开启开发者模式
- 插件经过签名验证（后期）
- 用户手动二次确认

已知风险（用户必须知晓）：
- 可进行键盘监听、进程注入
- 可读取敏感文件或 Token
- 可绕过权限系统

---

## 五、插件签名（远期）

v1 不实现插件签名验证。v2+ 需补充：
- 插件包签名机制
- 官方审核流程
- 已签名插件白名单
- 未签名插件需要开发者模式

---

## 六、崩溃隔离安全性

- Widget Qt 进程崩溃 → 不影响 Script Runtime 和 Host
- Widget Deno 崩溃 → 不影响 Qt 渲染和 Script Runtime
- Script 崩溃 → 不影响 Widget Runtime 和其他 Script
- Native 崩溃 → 不影响任何其他 Runtime

崩溃不会导致权限提升。恢复后权限不变。

---

## 七、DevTools 安全

- Inspector 端点仅 `127.0.0.1` 监听
- Production 构建中默认禁用（`--devtools` flag 开启）
- 只读操作（GET）无需认证
- 修改操作（POST）需用户在 DevTools 中确认
