# BetterCPT 测试策略

> 测试计划、验收指标、不做的事。

---

## 一、测试层次

### 单元测试（Rust）

| 模块 | 测试内容 |
|---|---|
| manifest 解析器 | 合法/非法/边界 JSON，错误 runtime 类型 |
| 权限系统 | 声明权限通过、超 Permission Ceiling 拒绝、空权限列表 |
| Capability Registry | capability → runtime binding 映射正确性 |
| IpcMessage | 序列化/反序列化往返 |
| Render Scheduler | 帧级 dedup（duration=0 合并、duration>0 保留顺序执行、跨 Widget 不合并、create 不参与） |
| 日志系统 | 写入和轮转 |
| 单例锁 | 获取和释放 |

### 集成测试

| 场景 | 测试方法 |
|---|---|
| Host 启动 → 托盘 → 退出 | 完整生命周期手动验证 |
| Script Runtime | 加载插件 → 调用 Deno op → 返回结果 → 停止 |
| 权限拦截 | 未声明权限的 Deno op 调用返回 Permission Denied |
| IPC 消息 | 超时、格式错误、大 payload 处理 |
| Qt 崩溃恢复 | 手动 kill Qt 进程 → 验证 Host 检测 + 重启 + replay + 交互恢复 |
| Deno 崩溃恢复 | 插件代码主动 throw / 死循环 → 验证 Watchdog 检测 + 重启 |
| 重启次数限制 | 连续崩溃 5 次后停止重启 |

### 性能测试（手动）

| 指标 | 测量工具 | 方法 |
|---|---|---|
| 帧率 | PresentMon | 录制 QPropertyAnimation 动画帧间隔 |
| 内存（Dock 单独） | Task Manager 详情 | Private Working Set，空闲采样 |
| 内存（10 Widget） | Task Manager 详情 | 共享/独立模式分别采样 |
| CPU 空闲 | Task Manager | 无动画状态下 Qt + Host 合计 CPU% |
| 崩溃恢复时间 | 手动计时 | kill Qt → 窗口恢复显示的时间 |
| IPC 延迟 | 自定义计时 | TS setScale 调用 → Qt 渲染完成的端到端 |

### 长时间测试（Soak Test）

- 10 个 Widget 常驻运行 24 小时
- 内存增长不超过基线的 10%
- 无崩溃

---

## 二、验收指标

| 指标 | v1 目标 | 测量方式 |
|---|---|---|
| Dock 空闲 CPU | < 1% | Task Manager |
| Dock 动画帧率 | ≥ 120FPS（144Hz 显示器） | PresentMon |
| Dock 内存 | < 25MB | Private Working Set |
| 10 Widget 内存（共享 Qt） | < 60MB | Private Working Set |
| Qt 崩溃恢复时间 | < 3 秒 | 手动计时 |
| Deno 崩溃恢复时间 | < 2 秒 | 手动计时 |

---

## 三、不做的事

- ❌ Widget 视觉效果的像素级自动化截图对比（成本极高，收益低）
- ❌ 跨 Windows 版本全矩阵测试（v1 只测 Win10 21H2+ / Win11）
- ❌ GPU Device Lost 自动化测试（手动模拟即可）
- ❌ 压力测试（v1 不做，等 Runtime 成立后）
