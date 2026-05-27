import { useState, useEffect } from "react";
import { Routes, Route, NavLink, useLocation, useNavigate } from "react-router-dom";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { useTheme } from "./ThemeContext";
import type { PluginInfo, SnapshotResponse, RuntimeEvent } from "./types";
import Plugins from "./pages/Plugins";
import PluginDetail from "./pages/PluginDetail";
import CrashLog from "./pages/CrashLog";
import Settings from "./pages/Settings";
import Home from "./pages/Home";
import Marketplace from "./pages/Marketplace";
import MarketplaceDetail from "./pages/MarketplaceDetail";

export default function App() {
  const { theme, setTheme } = useTheme();
  const [disconnected, _setDisconnected] = useState(false);
  const [_events, _setEvents] = useState<RuntimeEvent[]>([]);
  const [appWindow, setAppWindow] = useState<Record<string, ()=>void> | null>(null);
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [pluginSubOpen, setPluginSubOpen] = useState(true);
  const [selectedSubItem, setSelectedSubItem] = useState<string>("all");
  const location = useLocation();
  const navigate = useNavigate();

  // 根据当前路径同步子菜单选中状态
  useEffect(() => {
    if (location.pathname === "/plugins") {
      setSelectedSubItem("all");
      setPluginSubOpen(true);
    } else if (location.pathname.startsWith("/plugins/")) {
      const id = decodeURIComponent(location.pathname.replace("/plugins/", ""));
      setSelectedSubItem(id);
      setPluginSubOpen(true);
    } else {
      setSelectedSubItem("all");
    }
  }, [location.pathname]);

  useEffect(() => {
    try {
      setAppWindow(getCurrentWindow() as unknown as Record<string, ()=>void>);
    } catch (e) { console.warn(e); }

    let unlisten: UnlistenFn | undefined;
    (async () => {
      unlisten = await listen<RuntimeEvent>("runtime_event", (evt: { payload: RuntimeEvent }) => {
        _setEvents((prev: RuntimeEvent[]) => [...prev.slice(-99), evt.payload]);
      });
    })();

    // 加载插件列表供侧边栏使用
    invoke<string>("get_runtime_snapshot")
      .then((json) => {
        const snap: SnapshotResponse = JSON.parse(json);
        setPlugins(snap.plugins || []);
      })
      .catch(console.error);

    return () => { unlisten?.(); };
  }, []);

  // 侧边栏插件列表随 runtime_event 刷新
  useEffect(() => {
    let unlistenEvent: UnlistenFn | undefined;
    (async () => {
      unlistenEvent = await listen<RuntimeEvent>("runtime_event", () => {
        invoke<string>("get_runtime_snapshot")
          .then((json) => {
            const snap: SnapshotResponse = JSON.parse(json);
            setPlugins(snap.plugins || []);
          })
          .catch(console.error);
      });
    })();
    return () => { unlistenEvent?.(); };
  }, []);

  const navClass = (isActive: boolean) =>
    `flex items-center gap-2.5 px-3 py-1.5 rounded-md active:scale-95 transition-all duration-150 ${
      isActive
        ? "bg-secondary-container text-on-secondary-container border-l-[3px] border-primary font-medium"
        : "text-on-surface-variant hover:bg-surface-container-low border-l-[3px] border-l-transparent"
    }`;

  const subNavClass = (isActive: boolean) =>
    `flex items-center gap-2.5 px-3 py-1.5 rounded-md active:scale-95 transition-all duration-150 text-[11px] ${
      isActive
        ? "bg-secondary-container/60 text-on-secondary-container font-medium"
        : "text-on-surface-variant hover:bg-surface-container-low"
    }`;

  const handleWindow = (action: string) => {
    if (!appWindow) return;
    if (action === "minimize") appWindow.minimize?.();
    if (action === "maximize") appWindow.toggleMaximize?.();
    if (action === "close") appWindow.close?.();
  };

  const isPluginsActive = location.pathname.startsWith("/plugins");
  const totalCrashes = plugins.filter((p) => p.crash_count > 0).reduce((s, p) => s + p.crash_count, 0);

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-background">
      {/* Title bar */}
      <div
        className="h-[50px] flex flex-shrink-0 select-none bg-panel"
        onMouseDown={(e) => { const t = e.target as HTMLElement; if (t.tagName !== "BUTTON" && t.closest("button") === null) invoke("start_drag"); }}
      >
        <div className="w-[170px] flex-shrink-0 bg-panel flex items-end gap-3 px-4 pb-1.5">
          <div className="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center">
            <img src={new URL('/load_icon.png', import.meta.url).href} className="w-7 h-7" alt="" />
          </div>
          <div className="flex flex-col leading-tight">
            <h1 className="font-display text-[17px] font-bold text-primary tracking-tight">BetterCPT</h1>
            <span className="text-[10px] text-on-surface-variant/60 font-medium">v1.0.0</span>
          </div>
        </div>
        <div className="flex-1 flex items-center justify-end px-6 gap-4">
          <div className="flex items-center gap-1 bg-surface-container rounded-md p-0.5">
            <button
              onClick={() => setTheme("light")}
              className={`w-7 h-5 rounded flex items-center justify-center transition-all ${theme === "light" ? "bg-white shadow-sm" : ""}`}
              title="浅色"
            >
              <span className={`material-symbols-outlined text-[14px] ${theme === "light" ? "text-amber-500" : "text-on-surface-variant"}`}>light_mode</span>
            </button>
            <button
              onClick={() => setTheme("dark")}
              className={`w-7 h-5 rounded flex items-center justify-center transition-all ${theme === "dark" ? "bg-white shadow-sm" : ""}`}
              title="暗色"
            >
              <span className={`material-symbols-outlined text-[14px] ${theme === "dark" ? "text-amber-500" : "text-on-surface-variant"}`}>dark_mode</span>
            </button>
          </div>
          <button className="relative p-2 rounded-full hover:bg-surface-container-low transition-colors translate-y-[3px]">
            <span className="material-symbols-outlined text-[18px] text-on-surface-variant">notifications</span>
            <span className="absolute top-0.5 right-0.5 w-2 h-2 bg-primary rounded-full border border-surface" />
          </button>
          <div className="flex items-center gap-1.5 pl-3 border-l border-outline-variant/30">
            <button className="w-3 h-3 rounded-full bg-amber-400 hover:bg-amber-300 transition-colors" title="最小化" onClick={() => handleWindow("minimize")} />
            <button className="w-3 h-3 rounded-full bg-emerald-400 hover:bg-emerald-300 transition-colors" title="最大化" onClick={() => handleWindow("maximize")} />
            <button className="w-3 h-3 rounded-full bg-red-400 hover:bg-red-500 transition-colors" title="关闭" onClick={() => handleWindow("close")} />
          </div>
        </div>
      </div>

      <div className="flex flex-1 overflow-hidden">
        <aside className="w-[170px] flex-shrink-0 bg-panel flex flex-col">

          <nav className="flex-1 flex flex-col gap-0 px-2 mt-[25px] overflow-y-auto">
            {/* 首页 */}
            <NavLink to="/" end className={({ isActive }) => navClass(isActive)}>
              <span className="material-symbols-outlined text-[18px]">home</span>
              <span className="text-[11px] font-medium">首页</span>
            </NavLink>

            {/* 插件管理 — 可展开 */}
            <div>
              <button
                onClick={() => setPluginSubOpen(!pluginSubOpen)}
                className={`w-full flex items-center gap-2 px-3 py-1.5 rounded-md active:scale-95 transition-all duration-150 text-[11px] font-medium ${
                  isPluginsActive
                    ? "bg-secondary-container text-on-secondary-container border-l-[3px] border-primary font-medium"
                    : "text-on-surface-variant hover:bg-surface-container-low border-l-[3px] border-l-transparent"
                }`}
              >
                <span className="material-symbols-outlined transition-transform duration-200 text-[18px]"
                  style={{ transform: pluginSubOpen ? 'rotate(90deg)' : 'rotate(0deg)' }}>
                  chevron_right
                </span>
                <span className="material-symbols-outlined text-[18px]">extension</span>
                <span className="flex-1 text-left">插件管理</span>
                <span className="text-[10px] text-on-surface-variant tabular-nums">{plugins.length}</span>
              </button>
              {pluginSubOpen && (
                <div className="ml-6 mt-0.5 flex flex-col gap-0.5">
                  <button
                    onClick={() => { setSelectedSubItem("all"); navigate("/plugins"); }}
                    className={subNavClass(selectedSubItem === "all" && isPluginsActive)}
                  >
                    <span className="w-1.5 h-1.5 rounded-full bg-primary" />
                    <span>全部插件</span>
                  </button>
                  {plugins.map((p) => {
                    const name = p.id.split("/").pop() || p.id;
                    const dotColor = p.status === "running" ? "bg-primary"
                      : p.status === "crashed" ? "bg-status-error"
                      : p.status === "stopped" ? "bg-outline"
                      : "bg-status-warning";
                    return (
                      <button
                        key={p.id}
                        onClick={() => { setSelectedSubItem(p.id); navigate(`/plugins/${encodeURIComponent(p.id)}`); }}
                        className={subNavClass(selectedSubItem === p.id && isPluginsActive)}
                      >
                        <span className={`w-1.5 h-1.5 rounded-full ${dotColor} shrink-0`} />
                        <span>{name}</span>
                      </button>
                    );
                  })}
                </div>
              )}
            </div>

            {/* 插件市场 */}
            <NavLink to="/marketplace" className={({ isActive }) => navClass(isActive)}>
              <span className="material-symbols-outlined text-[18px]">storefront</span>
              <span className="text-[11px] font-medium">插件市场</span>
            </NavLink>

            {/* 崩溃日志 — 带角标 */}
            <NavLink to="/crashes" className={({ isActive }) => navClass(isActive)}>
              <span className="material-symbols-outlined text-[18px]">error</span>
              <span className="flex-1 text-[11px] font-medium">崩溃日志</span>
              {totalCrashes > 0 && (
                <span className="bg-error text-white text-[10px] font-bold rounded-full min-w-[18px] h-[18px] flex items-center justify-center px-1">
                  {totalCrashes}
                </span>
              )}
            </NavLink>
          </nav>

          {/* Footer */}
          <div className="px-2 pb-2 border-t border-outline-variant/20 mt-auto pt-2">
            <NavLink to="/settings" className={({ isActive }) => navClass(isActive)}>
              <span className="material-symbols-outlined text-[18px]">settings</span>
              <span className="text-[11px] font-medium">设置</span>
            </NavLink>
          </div>
        </aside>

        {/* Main content */}
        <main className="flex-1 bg-background overflow-y-auto">
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/plugins" element={<Plugins />} />
            <Route path="/plugins/:pluginId" element={<PluginDetail />} />
            <Route path="/marketplace" element={<Marketplace />} />
            <Route path="/marketplace/:pluginId" element={<MarketplaceDetail />} />
            <Route path="/crashes" element={<CrashLog />} />
            <Route path="/settings" element={<Settings />} />
          </Routes>
        </main>
      </div>

      {/* Disconnect overlay */}
      {disconnected && (
        <div className="fixed inset-0 bg-white/80 backdrop-blur-sm z-50 flex items-center justify-center">
          <div className="text-center bg-white rounded-2xl shadow-xl p-8 border border-slate-200">
            <p className="font-semibold text-on-surface">连接已断开</p>
            <p className="text-sm text-on-surface-variant mt-1">正在尝试重新连接...</p>
          </div>
        </div>
      )}
    </div>
  );
}
