import { useState, useEffect, useCallback } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PluginInfo, SnapshotResponse, RuntimeEvent } from "../types";
import { ColorPicker, Slider, Toggle } from "../components";

const TABS = [
  { key: "info", label: "基本信息" },
  { key: "config", label: "配置" },
  { key: "permissions", label: "权限" },
  { key: "logs", label: "日志" },
] as const;

type TabKey = (typeof TABS)[number]["key"];

function statusBadge(status: string) {
  switch (status) {
    case "running": return { dot: "bg-primary", text: "text-primary", label: "运行中" };
    case "stopped": return { dot: "bg-outline", text: "text-on-surface-variant", label: "已停止" };
    case "crashed": return { dot: "bg-status-error", text: "text-status-error", label: "已崩溃" };
    default: return { dot: "bg-status-warning", text: "text-status-warning", label: status };
  }
}

export default function PluginDetail() {
  const { pluginId } = useParams<{ pluginId: string }>();
  const navigate = useNavigate();
  const [plugin, setPlugin] = useState<PluginInfo | null>(null);
  const [activeTab, setActiveTab] = useState<TabKey>("config");
  const [loading, setLoading] = useState(true);
  const [showUninstallConfirm, setShowUninstallConfirm] = useState(false);

  const loadPlugin = useCallback(async () => {
    try {
      const json = await invoke<string>("get_runtime_snapshot");
      const snap: SnapshotResponse = JSON.parse(json);
      const found = (snap.plugins || []).find((p) => p.id === pluginId) || null;
      setPlugin(found);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, [pluginId]);

  useEffect(() => {
    setLoading(true);
    loadPlugin();
  }, [loadPlugin]);

  // 监听 runtime_event，当前插件状态变化时即时刷新
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    (async () => {
      unlisten = await listen<RuntimeEvent>("runtime_event", (evt) => {
        if (evt.payload.plugin_id === pluginId) {
          loadPlugin();
        }
      });
    })();
    return () => { unlisten?.(); };
  }, [pluginId, loadPlugin]);

  const handleAction = async (action: string) => {
    if (!plugin) return;
    try {
      await invoke(`${action}_plugin`, { pluginId: plugin.id });
      await loadPlugin();
    } catch (e) {
      console.error(e);
    }
  };

  const handleUninstall = async () => {
    if (!plugin) return;
    try {
      await invoke("uninstall_plugin", { pluginId: plugin.id });
      navigate("/plugins");
    } catch (e) {
      console.error(e);
    }
    setShowUninstallConfirm(false);
  };

  // ── 配置状态 ──
  // 必须在所有条件 return 之前调用（React Hooks 规则）
  const [bgColor, setBgColor] = useState("rgba(20, 20, 20, 0.70)");
  const [bgOpacity, setBgOpacity] = useState(30); // 透明度%，30%透明=alpha 0.70
  const [bgRadius, setBgRadius] = useState(16);
  const [dockName, setDockName] = useState("Dock");
  const [dockVisible, setDockVisible] = useState(true);
  const [iconSize, setIconSize] = useState(48);
  const [iconTopMargin, setIconTopMargin] = useState(8);
  const [iconLeftMargin, setIconLeftMargin] = useState(16);
  const [iconGap, setIconGap] = useState(16);
  const sendWidgetCmd = async (method: string, params: Record<string, unknown>) => {
    try {
      await invoke("send_widget_command", { method, paramsJson: JSON.stringify(params) });
    } catch (e) {
      console.error("send_widget_command:", e);
    }
  };

  // 同步配置到插件 JS 的 config 全局对象
  const syncConfig = () => {
    if (!plugin) return;
    const cfg = { bgColor, bgOpacity, bgRadius, iconSize, iconTopMargin, iconLeftMargin, iconGap, dockVisible };
    invoke("update_plugin_config", { pluginId: plugin.id, configJson: JSON.stringify(cfg) }).catch((e: unknown) => {
      console.error("update_plugin_config:", e);
    });
  };

  // v1 实时生效：颜色/透明度/圆角 → sendWidgetCmd
  // v1 延迟生效：图标布局 (无 setGeometry) → syncConfig 写入 config，下次启动生效
  const applyLayoutConfig = () => {
    syncConfig();
  };
  const rgbFromRgba = (rgba: string) => {
    const m = rgba.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
    return m ? `${m[1]}, ${m[2]}, ${m[3]}` : "20, 20, 20";
  };
  const rgbToHex = (rgb: string) => {
    const parts = rgb.split(",").map((s) =>
      parseInt(s.trim()).toString(16).padStart(2, "0")
    );
    return `#${parts.join("")}`;
  };
  const updateColor = (rgb: string) => {
    const newColor = `rgba(${rgb}, ${bgOpacity / 100})`;
    setBgColor(newColor);
    sendWidgetCmd("setColor", { id: "widget_1", color: newColor, radius: bgRadius });
  };
  const updateOpacityLocal = (opacity: number) => {
    setBgOpacity(opacity);
    setBgColor(`rgba(${rgbFromRgba(bgColor)}, ${1 - opacity / 100})`);
  };
  const updateOpacityCommit = (opacity: number) => {
    const c = `rgba(${rgbFromRgba(bgColor)}, ${1 - opacity / 100})`;
    sendWidgetCmd("setColor", { id: "widget_1", color: c, radius: bgRadius });
    syncConfig();
  };
  const updateRadiusLocal = (r: number) => setBgRadius(r);
  const updateRadiusCommit = (r: number) => {
    sendWidgetCmd("setColor", { id: "widget_1", color: bgColor, radius: r });
    syncConfig();
  };
  const toggleVisibility = () => {
    const next = !dockVisible;
    setDockVisible(next);
    if (next) {
      sendWidgetCmd("setColor", { id: "widget_1", color: bgColor, radius: bgRadius });
    } else {
      sendWidgetCmd("setColor", { id: "widget_1", color: "rgba(0,0,0,0)", radius: bgRadius });
    }
    syncConfig();
  };
  const PRESET_COLORS = [
    { rgb: "20, 20, 20" },
    { rgb: "80, 80, 80" },
    { rgb: "240, 240, 240" },
    { rgb: "30, 80, 180" },
    { rgb: "0, 108, 74" },
    { rgb: "180, 40, 40" },
    { rgb: "200, 130, 40" },
    { rgb: "120, 40, 140" },
  ];

  if (loading) {
    return <div className="p-6 text-on-surface-variant">加载中...</div>;
  }

  if (!plugin) {
    return (
      <div className="p-6 text-center">
        <p className="text-on-surface-variant mb-4">未找到插件 "{pluginId}"</p>
        <button onClick={() => navigate("/plugins")} className="text-primary hover:underline">
          返回插件管理
        </button>
      </div>
    );
  }

  const name = plugin.id.split("/").pop() || plugin.id;
  const badge = statusBadge(plugin.status);
  const runtimeIcon = plugin.runtime_type === "widget" ? "dock" : "notes";
  const memoryMB = plugin.memory_bytes / (1024 * 1024);

  return (
    <div className="p-6">
      {/* Header card */}
      <div className="flex flex-col md:flex-row gap-6 items-start md:items-center justify-between bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-6 shadow-sm">
        <div className="flex items-center gap-6">
          <div className="w-20 h-20 rounded-xl bg-gradient-to-br from-surface to-surface-variant border border-outline-variant flex items-center justify-center shadow-sm shrink-0">
            <span className="material-symbols-outlined text-[40px] text-primary fill">{runtimeIcon}</span>
          </div>
          <div>
            <div className="flex items-center gap-3 mb-1">
              <h2 className="font-display text-display-lg text-on-surface">{name}</h2>
              <span className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-primary-container/10 border border-primary/20 font-label-caps text-label-caps ${badge.text}`}>
                <span className={`w-1.5 h-1.5 rounded-full ${badge.dot}`} />
                {badge.label}
              </span>
            </div>
            <p className="font-mono text-[12px] text-on-surface-variant">{plugin.id} · v1.0.0</p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          {plugin.status === "running" ? (
            <button
              onClick={() => handleAction("stop")}
              className="px-4 py-2 rounded-lg bg-surface border border-outline-variant text-on-surface text-body-sm font-medium hover:bg-surface-variant transition-colors shadow-sm flex items-center gap-2"
            >
              <span className="material-symbols-outlined text-[18px]">stop_circle</span>停止
            </button>
          ) : (
            <button
              onClick={() => handleAction("start")}
              className="px-4 py-2 rounded-lg bg-primary text-on-primary text-body-sm font-medium hover:brightness-110 transition-colors shadow-sm flex items-center gap-2"
            >
              <span className="material-symbols-outlined text-[18px]">play_arrow</span>启动
            </button>
          )}
          <button
            onClick={() => setShowUninstallConfirm(true)}
            className="px-4 py-2 rounded-lg bg-surface border border-error/30 text-error text-body-sm font-medium hover:bg-error-container hover:border-error/50 transition-colors shadow-sm flex items-center gap-2"
          >
            <span className="material-symbols-outlined text-[18px]">delete</span>卸载
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-6 items-start mt-6">
        {/* Main: tabs + content — 全宽 */}
        <div className="flex flex-col gap-6">
          {/* Tabs */}
          <div className="border-b border-outline-variant">
            <nav className="flex space-x-8">
              {TABS.map((tab) => (
                <button
                  key={tab.key}
                  onClick={() => setActiveTab(tab.key)}
                  className={`whitespace-nowrap py-4 px-1 border-b-2 text-body-base font-medium transition-colors ${
                    activeTab === tab.key
                      ? "border-primary text-primary"
                      : "border-transparent text-on-surface-variant hover:text-on-surface hover:border-outline-variant"
                  }`}
                >
                  {tab.label}
                </button>
              ))}
            </nav>
          </div>

          {/* Tab: 基本信息 — 左侧文档区 + 右侧数据 */}
          {activeTab === "info" && (
            <div className="grid grid-cols-1 md:grid-cols-12 gap-4">
              {/* 左侧：插件介绍（未来从 info.md 加载） */}
              <div className="md:col-span-8">
                <div className="bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-6 shadow-sm min-h-[300px]">
                  <h2 className="font-headline-md text-headline-md text-on-surface mb-4">关于 {name}</h2>
                  <div className="prose prose-sm max-w-none text-on-surface-variant space-y-4">
                    <p>
                      {name} 是一个 {plugin.runtime_type === "widget" ? "Widget" : "Script"} 类型的 BetterCPT 插件，
                      提供 {' '}
                      {plugin.runtime_type === "widget"
                        ? "常驻桌面的视觉组件，通过 GPU 加速渲染实现流畅交互体验。"
                        : "按需启动的功能型工具，在独立的运行时环境中执行。"}
                    </p>
                    <h3 className="font-label-caps text-label-caps text-on-surface mt-4">功能特性</h3>
                    <ul className="space-y-1.5 list-disc pl-4 text-body-sm">
                      <li>轻量级运行，内存占用极低</li>
                      <li>支持热更新，修改代码后自动生效</li>
                      <li>崩溃自动恢复，不影响其他插件运行</li>
                      <li>完整的 TypeScript SDK 开发支持</li>
                    </ul>
                    <h3 className="font-label-caps text-label-caps text-on-surface mt-4">使用方法</h3>
                    <p className="text-body-sm">
                      在 BetterCPT Dashboard 中启动插件后，{plugin.runtime_type === "widget" ? "Widget 将立即渲染到桌面指定位置。可通过配置面板调整外观和行为。" : "插件将在后台运行，可通过 SDK API 调用其功能。"}
                    </p>
                    <div className="mt-4 p-3 bg-surface-container rounded-lg border border-outline-variant/30">
                      <p className="text-[11px] text-on-surface-variant">
                        <span className="material-symbols-outlined text-[14px] align-middle mr-1">info</span>
                        以上为占位内容。正式版本将从插件目录中的 <code className="font-mono text-[11px] bg-surface-container-high px-1 rounded">info.md</code> 文件加载完整的介绍、截图和使用文档。
                      </p>
                    </div>
                  </div>
                </div>
              </div>
              {/* 右侧：基本信息 + 运行数据/统计合并 */}
              <div className="md:col-span-4 flex flex-col gap-4">
                <div className="bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-5 shadow-sm">
                  <div className="flex items-center gap-2 mb-4">
                    <span className="material-symbols-outlined text-[18px] text-primary/60">info</span>
                    <h3 className="font-label-caps text-label-caps text-on-surface-variant">基本信息</h3>
                  </div>
                  <div className="space-y-3">
                    <div className="flex justify-between text-body-sm"><span className="text-on-surface-variant">Runtime</span><span className="text-primary font-medium">{plugin.runtime_type === "widget" ? "Widget" : "Script"}</span></div>
                    <div className="flex justify-between text-body-sm"><span className="text-on-surface-variant">版本</span><span className="font-mono text-[12px]">v1.0.0</span></div>
                    <div className="flex justify-between text-body-sm"><span className="text-on-surface-variant">入口文件</span><span className="font-mono text-[12px]">index.ts</span></div>
                    <div className="flex justify-between text-body-sm"><span className="text-on-surface-variant">引擎要求</span><span className="font-mono text-[12px]">&gt;=0.1.0</span></div>
                  </div>
                </div>
                <div className="bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-5 shadow-sm flex flex-col gap-4">
                  <div className="flex items-center gap-2">
                    <span className="material-symbols-outlined text-[18px] text-primary/60">monitoring</span>
                    <h3 className="font-label-caps text-label-caps text-on-surface-variant">运行数据</h3>
                  </div>
                  <div className="flex justify-between text-body-sm"><span className="text-on-surface-variant">崩溃次数</span><span className="font-medium">{plugin.crash_count}</span></div>
                  <div className="flex justify-between text-body-sm"><span className="text-on-surface-variant">状态</span><span className="font-medium">{badge.label}</span></div>
                  <div className="border-t border-outline-variant/30 pt-3 mt-1">
                    <div className="flex items-center gap-3">
                      <div className="w-9 h-9 rounded-lg bg-primary/10 flex items-center justify-center shrink-0">
                        <span className="material-symbols-outlined text-primary text-[18px]">memory</span>
                      </div>
                      <div className="flex-1">
                        <div className="flex justify-between items-end mb-1">
                          <p className="text-[11px] text-on-surface-variant">内存使用率</p>
                          <p className="font-mono text-[11px] text-on-surface font-medium">{Math.round((memoryMB / 50) * 100)}%</p>
                        </div>
                        <div className="w-full bg-surface-container-high rounded-full h-1">
                          <div className="bg-primary h-1 rounded-full transition-all duration-500" style={{ width: `${Math.min((memoryMB / 50) * 100, 100)}%` }} />
                        </div>
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-3">
                    <div className="w-9 h-9 rounded-lg bg-surface-container flex items-center justify-center shrink-0">
                      <span className="material-symbols-outlined text-on-surface-variant text-[18px]">schedule</span>
                    </div>
                    <div>
                      <p className="text-[11px] text-on-surface-variant">运行时长</p>
                      <p className="text-body-sm text-on-surface font-medium">{plugin.status === "running" ? "--" : "未运行"}</p>
                    </div>
                  </div>
                </div>
                <div className="px-2 text-on-surface-variant text-body-sm space-y-2">
                  <p className="flex justify-between"><span className="text-[11px]">开发者:</span><span className="text-on-surface text-[11px]">BetterCPT Team</span></p>
                  <p className="flex justify-between"><span className="text-[11px]">最近更新:</span><span className="text-on-surface text-[11px]">2026-05-19</span></p>
                  <p className="flex justify-between"><span className="text-[11px]">路径:</span><span className="font-mono text-[10px] truncate ml-2 max-w-[120px]">{plugin.id}</span></p>
                </div>
              </div>
            </div>
          )}

          {/* Tab: 配置 */}
          {activeTab === "config" && (
            <div className="space-y-4">
              <ColorPicker
                label="背景颜色"
                description="调整 Dock 栏填充色"
                value={bgColor}
                presets={PRESET_COLORS}
                onChange={updateColor}
                rgbFromRgba={rgbFromRgba}
                rgbToHex={rgbToHex}
                onHexChange={updateColor}
              />

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <Slider
                  label="透明度"
                  icon="opacity"
                  value={bgOpacity}
                  min={10}
                  max={100}
                  step={5}
                  suffix="%"
                  onChange={updateOpacityLocal}
                  onChangeEnd={updateOpacityCommit}
                />
                <Slider
                  label="圆角"
                  icon="rounded_corner"
                  value={bgRadius}
                  min={0}
                  max={40}
                  step={2}
                  suffix="px"
                  onChange={updateRadiusLocal}
                  onChangeEnd={updateRadiusCommit}
                />
              </div>

              {/* ====== 图标布局 ====== */}
              <div className="bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-6 shadow-sm">
                <div className="flex items-center gap-3 mb-5">
                  <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center shrink-0">
                    <span className="material-symbols-outlined text-primary text-[20px]">dashboard</span>
                  </div>
                  <div>
                    <h3 className="text-body-sm font-semibold text-on-surface">图标布局</h3>
                    <p className="text-[11px] text-on-surface-variant">调整图标在 Dock 栏中的位置和间距</p>
                  </div>
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <Slider label="图标大小" icon="aspect_ratio" value={iconSize} min={24} max={96} step={4} suffix="px" onChange={setIconSize} onChangeEnd={() => applyLayoutConfig()} />
                  <Slider label="上边距" icon="vertical_align_top" value={iconTopMargin} min={0} max={28} step={2} suffix="px" onChange={setIconTopMargin} onChangeEnd={() => applyLayoutConfig()} />
                  <Slider label="左边距" icon="format_align_left" value={iconLeftMargin} min={0} max={48} step={2} suffix="px" onChange={setIconLeftMargin} onChangeEnd={() => applyLayoutConfig()} />
                  <Slider label="图标间距" icon="space_bar" value={iconGap} min={0} max={40} step={2} suffix="px" onChange={setIconGap} onChangeEnd={() => applyLayoutConfig()} />
                </div>
                <div className="flex items-center gap-2 mt-4 text-[11px] text-on-surface-variant/60">
                  <span className="material-symbols-outlined text-[14px]">info</span>
                  图标布局变更将在插件下次启动时生效（停止 → 启动）
                </div>
              </div>

              <div className="bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-6 shadow-sm">
                <div className="flex items-center gap-3 mb-4">
                  <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center shrink-0">
                    <span className="material-symbols-outlined text-primary text-[20px]">badge</span>
                  </div>
                  <div className="flex-1">
                    <h3 className="text-body-sm font-semibold text-on-surface">显示名称</h3>
                    <p className="text-[11px] text-on-surface-variant">Dashboard 中显示的名称</p>
                  </div>
                </div>
                <input
                  type="text"
                  value={dockName}
                  onChange={(e) => setDockName(e.target.value)}
                  className="w-full bg-surface-container-lowest border border-outline-variant rounded-lg px-3 py-2 text-body-sm text-on-surface focus:outline-none focus:border-primary transition-colors"
                  placeholder="Dock"
                />
              </div>

              <Toggle
                label="显示在桌面"
                description={dockVisible ? "Dock 栏正在桌面显示" : "Dock 栏已隐藏"}
                checked={dockVisible}
                onChange={toggleVisibility}
                icon="visibility"
                iconOff="visibility_off"
              />
            </div>
          )}

          {/* Tab: 权限 */}
          {activeTab === "permissions" && (
            <div className="bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-6 shadow-sm max-w-lg">
              <div className="flex items-start justify-between mb-4">
                <div>
                  <h3 className="font-headline-md text-headline-md text-on-surface mb-1">权限声明</h3>
                  <p className="text-body-sm text-on-surface-variant">此插件的 Capability 和 Permission 清单。</p>
                </div>
                <span className="material-symbols-outlined text-primary/60">shield</span>
              </div>
              <div className="space-y-3">
                <div className="flex items-center gap-2 text-body-sm">
                  <span className="text-primary font-bold">✓</span>
                  <span className="font-mono text-[12px]">widget:overlay</span>
                  <span className="text-on-surface-variant">— 桌面覆盖层渲染</span>
                </div>
                <div className="flex items-center gap-2 text-body-sm">
                  <span className="text-primary font-bold">✓</span>
                  <span className="font-mono text-[12px]">system:launch</span>
                  <span className="text-on-surface-variant">— 启动外部应用</span>
                </div>
                <div className="flex items-center gap-2 text-body-sm">
                  <span className="text-primary font-bold">✓</span>
                  <span className="font-mono text-[12px]">storage</span>
                  <span className="text-on-surface-variant">— KV 持久化存储</span>
                </div>
                <div className="flex items-center gap-2 text-body-sm">
                  <span className="text-outline-variant">✗</span>
                  <span className="font-mono text-[12px] text-on-surface-variant">fs:write</span>
                  <span className="text-on-surface-variant">— 未授权</span>
                </div>
              </div>
            </div>
          )}

          {/* Tab: 日志 */}
          {activeTab === "logs" && (
            <div className="bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-6 shadow-sm">
              <div className="flex items-start justify-between mb-4">
                <div>
                  <h3 className="font-headline-md text-headline-md text-on-surface mb-1">运行日志</h3>
                  <p className="text-body-sm text-on-surface-variant">最近 20 条。</p>
                </div>
                <span className="material-symbols-outlined text-primary/60">terminal</span>
              </div>
              <pre className="font-mono text-[11px] text-on-surface-variant leading-relaxed bg-surface-container-low rounded-lg p-4 border border-outline-variant max-h-64 overflow-y-auto">
                <code>{`[10:32:15] INFO  ${name} plugin started
              [10:32:15] INFO  Initializing runtime environment
              [10:32:15] INFO  Loading configuration
              [10:32:16] INFO  Registering handlers
              [10:32:16] INFO  Plugin ready`}</code>
              </pre>
            </div>
          )}
        </div>
      </div>

      {/* 卸载确认弹窗 */}
      {showUninstallConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm" onClick={() => setShowUninstallConfirm(false)}>
          <div className="bg-surface-container-lowest rounded-xl border border-outline-variant/30 shadow-xl p-6 max-w-sm w-full mx-4" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 rounded-full bg-error-container flex items-center justify-center shrink-0">
                <span className="material-symbols-outlined text-error text-[20px]">warning</span>
              </div>
              <div>
                <h3 className="text-body-sm font-semibold text-on-surface">确认卸载</h3>
                <p className="text-[11px] text-on-surface-variant mt-0.5">确定要卸载 <span className="font-mono text-primary">{name}</span> 吗？此操作不可撤销。</p>
              </div>
            </div>
            <div className="flex justify-end gap-2">
              <button onClick={() => setShowUninstallConfirm(false)} className="px-4 py-2 rounded-lg text-body-sm font-medium text-on-surface-variant hover:bg-surface-container-high transition-colors">取消</button>
              <button onClick={handleUninstall} className="px-4 py-2 rounded-lg text-body-sm font-medium bg-error text-white hover:brightness-110 transition-colors">确认卸载</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
