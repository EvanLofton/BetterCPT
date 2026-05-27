import { useState, useEffect, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PluginInfo, SnapshotResponse, RuntimeEvent } from "../types";

function formatMemory(bytes: number): string {
  if (bytes === 0) return "--";
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

interface RecentEvent {
  label: string;
  plugin_id: string;
  time: string;
  type: "start" | "stop" | "crash" | "info";
}

export default function Home() {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [recentEvents, setRecentEvents] = useState<RecentEvent[]>([]);
  const [uptime, setUptime] = useState(0);
  const navigate = useNavigate();

  // 快照加载
  const refreshSnapshot = useCallback(() => {
    invoke<string>("get_runtime_snapshot")
      .then((json) => {
        const snap: SnapshotResponse = JSON.parse(json);
        setPlugins(snap.plugins || []);
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    refreshSnapshot();
  }, [refreshSnapshot]);

  // 运行时间自增
  useEffect(() => {
    const tick = setInterval(() => setUptime((prev) => prev + 1), 1000);
    return () => clearInterval(tick);
  }, []);

  // 事件监听
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    (async () => {
      unlisten = await listen<RuntimeEvent>("runtime_event", (evt) => {
        const { event, plugin_id, crash_count } = evt.payload;
        const label = event === "plugin_started" ? "已启动"
          : event === "plugin_stopped" ? "已停止"
          : event === "plugin_crashed" ? "检测到崩溃"
          : event;
        const type = event.includes("crashed") ? "crash"
          : event.includes("started") ? "start"
          : event.includes("stopped") ? "stop"
          : "info";
        const name = plugin_id.split("/").pop() || plugin_id;
        const suffix = crash_count ? ` (崩溃 #${crash_count})` : "";
        const now = new Date();
        const time = `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
        setRecentEvents((prev) => [
          { label: `${name} ${label}${suffix}`, plugin_id, time, type },
          ...prev.slice(0, 19),
        ]);
        // 事件后刷新快照
        refreshSnapshot();
      });
    })();
    return () => { unlisten?.(); };
  }, [refreshSnapshot]);

  // 操作按钮
  const handleAction = async (pluginId: string, action: "start" | "stop") => {
    try {
      await invoke(action === "start" ? "start_plugin" : "stop_plugin", { pluginId });
      setTimeout(refreshSnapshot, 500);
    } catch (e) {
      console.error(e);
    }
  };

  const running = plugins.filter((p) => p.status === "running").length;
  const totalCrashes = plugins.reduce((s, p) => s + p.crash_count, 0);
  const totalMem = plugins.reduce((s, p) => s + (p.memory_bytes || 0), 0);
  const totalMemMB = totalMem / (1024 * 1024);
  const maxMemMB = 50;

  const eventTypeClass = (type: string) => {
    switch (type) {
      case "crash": return "bg-error-container/80 border-status-error";
      case "start": return "bg-primary/10 border-primary";
      case "stop": return "bg-surface-container-high border-outline";
      default: return "bg-surface-container-high border-outline";
    }
  };
  const eventDotClass = (type: string) => {
    switch (type) {
      case "crash": return "bg-status-error";
      case "start": return "bg-primary";
      case "stop": return "bg-outline";
      default: return "bg-outline";
    }
  };

  return (
    <div className="p-6">
      <div className="mb-8 animate-fade-in-up">
        <h2 className="font-display text-display-lg text-on-surface mb-2 flex items-center gap-3">
          BetterCPT 插件平台
          <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-primary-container/20 text-on-primary-container border border-primary-container/30">
            <span className={`w-1.5 h-1.5 rounded-full mr-1.5 ${running > 0 ? "bg-primary animate-pulse" : "bg-outline-variant"}`} />
            {running > 0 ? "运行中" : "待机"}
          </span>
        </h2>
        <p className="text-body-base text-on-surface-variant">
          Windows 桌面的下一个形态。
        </p>
      </div>

      {/* Stat cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
        {/* 已安装插件 */}
        <div
          className="bg-surface-container-lowest rounded-xl border border-outline-variant/30 p-5 shadow-sm hover:shadow-[0_12px_24px_rgba(78,222,163,0.08)] hover:border-primary/30 transition-all duration-300 hover:-translate-y-1 group relative overflow-hidden cursor-pointer"
          onClick={() => navigate("/plugins")}
        >
          <div className="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
            <span className="material-symbols-outlined text-4xl text-primary">extension</span>
          </div>
          <p className="font-label-caps text-label-caps text-on-surface-variant mb-1">已安装插件</p>
          <div className="flex items-baseline gap-2">
            <h3 className="font-display text-display-lg text-on-surface">{plugins.length}</h3>
            <span className="text-status-success text-body-sm flex items-center gap-0.5">
              <span className="material-symbols-outlined text-[14px]">play_arrow</span> {running}
            </span>
          </div>
        </div>

        {/* 总内存 */}
        <div className="bg-surface-container-lowest rounded-xl border border-outline-variant/30 p-5 shadow-sm hover:shadow-[0_12px_24px_rgba(78,222,163,0.08)] hover:border-primary/30 transition-all duration-300 hover:-translate-y-1 group relative overflow-hidden">
          <div className="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
            <span className="material-symbols-outlined text-4xl text-primary">memory</span>
          </div>
          <p className="font-label-caps text-label-caps text-on-surface-variant mb-1">总内存</p>
          <div className="flex items-baseline gap-2">
            <h3 className="font-display text-display-lg text-on-surface">{totalMemMB > 0 ? totalMemMB.toFixed(1) : "--"}</h3>
            {totalMemMB > 0 && <span className="text-body-base text-on-surface-variant">MB</span>}
          </div>
          <div className="w-full bg-surface-container-high rounded-full h-1.5 mt-3 overflow-hidden">
            <div
              className="bg-primary h-1.5 rounded-full transition-all duration-500"
              style={{ width: `${Math.min((totalMemMB / maxMemMB) * 100, 100)}%` }}
            />
          </div>
        </div>

        {/* 崩溃次数 */}
        <div
          className="bg-surface-container-lowest rounded-xl border border-outline-variant/30 p-5 shadow-sm hover:shadow-[0_12px_24px_rgba(239,68,68,0.08)] hover:border-status-error/30 transition-all duration-300 hover:-translate-y-1 group relative overflow-hidden cursor-pointer"
          onClick={() => navigate("/crashes")}
        >
          <div className="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
            <span className="material-symbols-outlined text-4xl text-status-error">error</span>
          </div>
          <p className="font-label-caps text-label-caps text-on-surface-variant mb-1">崩溃次数</p>
          <div className="flex items-baseline gap-2">
            <h3 className="font-display text-display-lg text-on-surface">{totalCrashes}</h3>
            {totalCrashes > 0 && (
              <span className="text-status-error text-body-sm">次记录</span>
            )}
          </div>
        </div>

        {/* 运行时间 */}
        <div className="bg-surface-container-lowest rounded-xl border border-outline-variant/30 p-5 shadow-sm hover:shadow-[0_12px_24px_rgba(78,222,163,0.08)] hover:border-primary/30 transition-all duration-300 hover:-translate-y-1 group relative overflow-hidden">
          <div className="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
            <span className="material-symbols-outlined text-4xl text-primary">timer</span>
          </div>
          <p className="font-label-caps text-label-caps text-on-surface-variant mb-1">运行时间</p>
          <div className="flex items-baseline gap-1">
            <h3 className="font-display text-display-lg text-on-surface">
              {formatUptime(uptime)}
            </h3>
          </div>
        </div>
      </div>

      {/* Content Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Runtime 状态表 */}
        <div className="md:col-span-2 bg-surface-container-lowest rounded-xl border border-outline-variant/30 shadow-sm overflow-hidden flex flex-col">
          <div className="p-5 border-b border-outline-variant/30 flex justify-between items-center bg-surface/50">
            <h3 className="font-headline-md text-headline-md text-on-surface flex items-center gap-2">
              <span className="material-symbols-outlined text-primary">dns</span>Runtime 状态
            </h3>
            <button
              className="text-primary hover:text-primary-container text-body-sm font-medium transition-colors"
              onClick={() => navigate("/plugins")}
            >
              管理全部
            </button>
          </div>
          <div className="overflow-y-auto flex-1">
            {plugins.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-16 text-on-surface-variant/50">
                <span className="material-symbols-outlined text-[40px] mb-3">extension_off</span>
                <p className="text-body-sm">暂无已安装插件</p>
              </div>
            ) : (
              <table className="w-full text-left border-collapse">
                <thead className="bg-surface-container-low/50 sticky top-0">
                  <tr>
                    <th className="py-3 px-5 font-label-caps text-label-caps text-on-surface-variant">插件名称</th>
                    <th className="py-3 px-5 font-label-caps text-label-caps text-on-surface-variant">状态</th>
                    <th className="py-3 px-5 font-label-caps text-label-caps text-on-surface-variant">内存</th>
                    <th className="py-3 px-5 font-label-caps text-label-caps text-on-surface-variant text-right">操作</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-outline-variant/30 text-body-sm text-on-surface">
                  {plugins.map((p) => {
                    const name = p.id.split("/").pop() || p.id;
                    const icon = p.runtime_type === "widget" ? "dock" : "notes";
                    return (
                      <tr key={p.id} className={`hover:bg-surface-container-lowest/80 transition-colors group ${p.status === "crashed" ? "bg-error-container/10" : ""}`}>
                        <td className="py-3 px-5 font-medium flex items-center gap-3">
                          <div className={`w-8 h-8 rounded-lg flex items-center justify-center shrink-0 ${
                            p.status === "running" ? "bg-primary/10 text-primary" :
                            p.status === "crashed" ? "bg-status-error/10 text-status-error" :
                            "bg-surface-container-high text-on-surface-variant"
                          }`}>
                            <span className="material-symbols-outlined text-[18px]">{icon}</span>
                          </div>
                          {name}
                        </td>
                        <td className="py-3 px-5">
                          <span className="inline-flex items-center gap-1.5">
                            <span className={`w-2 h-2 rounded-full ${
                              p.status === "running" ? "bg-primary" :
                              p.status === "crashed" ? "bg-status-error animate-pulse" :
                              "bg-outline-variant"
                            }`} />
                            {p.status === "running" ? "运行中" :
                             p.status === "crashed" ? "已崩溃" :
                             p.status === "stopped" ? "已停止" : p.status}
                          </span>
                        </td>
                        <td className="py-3 px-5 font-mono text-sm text-on-surface-variant">
                          {p.status === "running" ? formatMemory(p.memory_bytes) : "--"}
                        </td>
                        <td className="py-3 px-5 text-right">
                          <div className="flex items-center justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                            {p.status === "running" ? (
                              <button
                                onClick={() => handleAction(p.id, "stop")}
                                className="p-1.5 rounded-md hover:bg-surface-container-high text-on-surface-variant hover:text-status-error transition-colors inline-flex items-center justify-center"
                                title="停止"
                              >
                                <span className="material-symbols-outlined text-[18px]">stop_circle</span>
                              </button>
                            ) : (
                              <button
                                onClick={() => handleAction(p.id, "start")}
                                className="p-1.5 rounded-md hover:bg-surface-container-high text-on-surface-variant hover:text-primary transition-colors inline-flex items-center justify-center"
                                title={p.status === "crashed" ? "重启" : "启动"}
                              >
                                <span className="material-symbols-outlined text-[18px]">
                                  {p.status === "crashed" ? "restart_alt" : "play_circle"}
                                </span>
                              </button>
                            )}
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            )}
          </div>
        </div>

        {/* 最近事件 */}
        <div className="bg-surface-container-lowest rounded-xl border border-outline-variant/30 p-5 shadow-sm flex flex-col">
          <div className="flex justify-between items-center mb-6">
            <h3 className="font-headline-md text-headline-md text-on-surface flex items-center gap-2">
              <span className="material-symbols-outlined text-primary">history</span>最近事件
            </h3>
          </div>
          <div className="relative flex-1 pl-4">
            {recentEvents.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-on-surface-variant/40">
                <span className="material-symbols-outlined text-[32px] mb-2">list_alt</span>
                <p className="text-body-sm">等待事件...</p>
              </div>
            ) : (
              <>
                <div className="absolute left-[11px] top-2 bottom-0 w-px bg-outline-variant/30" />
                <div className="flex flex-col gap-6 relative">
                  {recentEvents.map((item, i) => (
                    <div key={i} className="flex gap-4 group">
                      <div className={`w-6 h-6 rounded-full border-2 border-surface-container-lowest flex items-center justify-center shrink-0 z-10 -ml-[7px] mt-0.5 ${eventTypeClass(item.type)}`}>
                        <div className={`w-2 h-2 rounded-full ${eventDotClass(item.type)}`} />
                      </div>
                      <div className="flex-1 pb-1">
                        <p className="text-body-sm text-on-surface font-medium">{item.label}</p>
                        <p className="font-label-caps text-label-caps text-on-surface-variant mt-1">{item.time}</p>
                      </div>
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>
          <button
            className="w-full mt-4 py-2 border border-outline-variant/50 rounded-lg text-on-surface text-body-sm hover:bg-surface-container transition-colors"
            onClick={() => navigate("/crashes")}
          >
            查看全部日志
          </button>
        </div>
      </div>
    </div>
  );
}
