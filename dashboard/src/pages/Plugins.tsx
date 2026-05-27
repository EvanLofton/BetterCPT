import { useState, useEffect, useRef, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../ToastContext";
import type { PluginInfo, SnapshotResponse } from "../types";

function statusBadge(status: string) {
  switch (status) {
    case "running": return { dot: "bg-primary shadow-[0_0_8px_rgba(0,108,74,0.4)]", text: "text-primary", label: "运行中" };
    case "stopped": return { dot: "bg-outline", text: "text-on-surface-variant", label: "已停止" };
    case "crashed": return { dot: "bg-status-error", text: "text-status-error", label: "已崩溃" };
    default: return { dot: "bg-status-warning", text: "text-status-warning", label: status };
  }
}

function formatMemory(bytes: number): string {
  if (bytes === 0) return "--";
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

type SortKey = "name" | "status";
type ViewMode = "table" | "card";

export default function Plugins() {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [searchKey, setSearchKey] = useState<SortKey>("name");
  const [sortAsc, setSortAsc] = useState(true);
  const [viewMode, setViewMode] = useState<ViewMode>("table");
  const [dragOver, setDragOver] = useState(false);
  const [confirmingUninstall, setConfirmingUninstall] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const { addToast } = useToast();
  const navigate = useNavigate();

  const loadSnapshot = async () => {
    try {
      const json = await invoke<string>("get_runtime_snapshot");
      const snapshot: SnapshotResponse = JSON.parse(json);
      setPlugins(snapshot.plugins || []);
    } catch (e) {
      console.error("Failed to load snapshot:", e);
    } finally {
      setLoading(false);
    }
  };

  const installPlugin = async (filePath: string) => {
    const fileName = filePath.split("\\").pop() || filePath;
    if (!filePath.endsWith(".bcpkg")) {
      addToast("error", `${fileName} 不是有效的 .bcpkg 插件文件`);
      return;
    }
    addToast("info", `正在识别检验 ${fileName} ...`);
    try {
      await invoke("install_plugin", { path: filePath });
      await loadSnapshot();
      addToast("success", `${fileName} 安装成功`);
    } catch (e) {
      addToast("error", `安装失败: ${String(e)}`);
    }
  };

  useEffect(() => {
    loadSnapshot();

    let unlistenDrop: (() => void) | undefined;
    let unlistenEnter: (() => void) | undefined;
    let unlistenLeave: (() => void) | undefined;
    let unlistenEvent: (() => void) | undefined;
    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      unlistenEnter = await listen("tauri://drag-enter", () => setDragOver(true));
      unlistenLeave = await listen("tauri://drag-leave", () => setDragOver(false));
      unlistenDrop = await listen<{ paths: string[] }>("tauri://drag-drop", (evt) => {
        setDragOver(false);
        if (evt.payload.paths?.length > 0) installPlugin(evt.payload.paths[0]);
      });
      // runtime_event 驱动：任何插件状态变化 → 刷新
      unlistenEvent = await listen("runtime_event", () => loadSnapshot());
    })();
    return () => {
      unlistenDrop?.();
      unlistenEnter?.();
      unlistenLeave?.();
      unlistenEvent?.();
    };
  }, []);

  const toggleSort = (key: SortKey) => {
    if (searchKey === key) setSortAsc(!sortAsc);
    else { setSearchKey(key); setSortAsc(true); }
  };

  const sortedFiltered = useMemo(() => {
    let list = plugins
      .filter((p) => {
        const q = search.toLowerCase();
        if (!q) return true;
        if (searchKey === "name") return p.id.toLowerCase().includes(q);
        return p.status.toLowerCase().includes(q);
      })
      .sort((a, b) => {
        const _a = searchKey === "name" ? a.id : a.status;
        const _b = searchKey === "name" ? b.id : b.status;
        return sortAsc ? _a.localeCompare(_b) : _b.localeCompare(_a);
      });
    return list;
  }, [plugins, search, searchKey, sortAsc]);

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    if (f) installPlugin((f as any).path || f.name);
    e.target.value = "";
  };

  return (
    <div className="p-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-2">
        <div>
          <h2 className="font-display text-display-lg text-on-surface mb-1">全部插件</h2>
          <p className="text-body-md text-on-surface-variant">
            已安装 {plugins.length} 个插件，{plugins.filter(p => p.status === "running").length} 个运行中
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => fileInputRef.current?.click()}
            className="group flex items-center gap-2 bg-surface border border-dashed border-outline-variant hover:border-primary rounded-xl px-4 py-2 transition-all shadow-sm hover:shadow-md hover:-translate-y-0.5"
          >
            <div className="flex items-center justify-center w-7 h-7 rounded-lg bg-surface-container-low group-hover:bg-primary-container/20 transition-colors">
              <span className="material-symbols-outlined text-on-surface-variant group-hover:text-primary text-[18px] transition-colors">upload_file</span>
            </div>
            <div className="flex flex-col items-start">
              <span className="font-label-md text-on-surface font-medium group-hover:text-primary transition-colors">安装 .bcpkg 插件</span>
              <span className="font-label-sm uppercase tracking-widest text-on-surface-variant">拖拽或点击选择</span>
            </div>
          </button>
          <input ref={fileInputRef} type="file" accept=".bcpkg" className="hidden" onChange={handleFileSelect} />
        </div>
      </div>

      {/* Toolbar */}
      <div className="bg-surface-container-lowest border border-outline-variant rounded-xl shadow-sm overflow-hidden flex flex-col mt-4">
        <div className="border-b border-outline-variant px-4 py-3 flex items-center justify-between bg-surface/50">
          <div className="flex items-center gap-2">
            <div className="relative">
              <span className="material-symbols-outlined absolute left-3 top-1/2 -translate-y-1/2 text-on-surface-variant text-[16px]">search</span>
              <input
                className="w-48 bg-surface-container-lowest border border-outline-variant rounded-lg pl-9 pr-4 py-1.5 text-body-sm focus:outline-none focus:border-primary transition-colors shadow-sm text-on-surface"
                placeholder={searchKey === "name" ? "按名称搜索..." : "按状态搜索..."}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>
            <button
              onClick={() => toggleSort("name")}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-body-sm transition-colors border ${
                searchKey === "name"
                  ? "bg-primary-container/10 text-primary border-primary font-medium"
                  : "text-on-surface-variant border-transparent hover:bg-surface-container-low hover:border-outline-variant"
              }`}
            >
              <span className="material-symbols-outlined text-[16px]">sort</span>
              名称
              {searchKey === "name" && (
                <span className="material-symbols-outlined text-[14px]">{sortAsc ? "arrow_upward" : "arrow_downward"}</span>
              )}
            </button>
            <button
              onClick={() => toggleSort("status")}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-body-sm transition-colors border ${
                searchKey === "status"
                  ? "bg-primary-container/10 text-primary border-primary font-medium"
                  : "text-on-surface-variant border-transparent hover:bg-surface-container-low hover:border-outline-variant"
              }`}
            >
              <span className="material-symbols-outlined text-[16px]">sort</span>
              状态
              {searchKey === "status" && (
                <span className="material-symbols-outlined text-[14px]">{sortAsc ? "arrow_upward" : "arrow_downward"}</span>
              )}
            </button>
          </div>
          <div className="flex items-center gap-1">
            <button
              onClick={() => setViewMode("table")}
              className={`p-1.5 rounded-md transition-colors ${viewMode === "table" ? "bg-surface-container border border-outline-variant shadow-sm text-on-surface" : "text-on-surface-variant hover:bg-surface-container-low border border-transparent"}`}
            >
              <span className="material-symbols-outlined text-[18px]">list</span>
            </button>
            <button
              onClick={() => setViewMode("card")}
              className={`p-1.5 rounded-md transition-colors ${viewMode === "card" ? "bg-surface-container border border-outline-variant shadow-sm text-on-surface" : "text-on-surface-variant hover:bg-surface-container-low border border-transparent"}`}
            >
              <span className="material-symbols-outlined text-[18px]">grid_view</span>
            </button>
          </div>
        </div>

        {/* Table view */}
        {viewMode === "table" && (
          <div className="overflow-x-auto">
            {loading ? (
              <div className="py-16 text-center text-on-surface-variant">加载中...</div>
            ) : sortedFiltered.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-16 text-on-surface-variant/50">
                <span className="material-symbols-outlined text-[40px] mb-3">{search ? "search_off" : "extension_off"}</span>
                <p className="text-body-sm">{search ? "未找到匹配的插件" : "暂无已安装插件"}</p>
              </div>
            ) : (
              <table className="w-full text-left border-collapse table-fixed">
                <thead>
                  <tr className="border-b border-outline-variant bg-surface-container-low/50">
                    <th className="pl-[10px] px-0 pr-2 py-3 text-[11px] uppercase tracking-widest text-on-surface-variant w-[18%]">ID</th>
                    <th className="pl-[10px] px-0 pr-2 py-3 text-[11px] uppercase tracking-widest text-on-surface-variant w-[25%]">名称</th>
                    <th className="pl-[10px] px-0 pr-2 py-3 text-[11px] uppercase tracking-widest text-on-surface-variant w-[12%]">Runtime</th>
                    <th className="pl-[10px] px-0 pr-2 py-3 text-[11px] uppercase tracking-widest text-on-surface-variant w-[12%]">状态</th>
                    <th className="pl-[10px] px-0 pr-2 py-3 text-[11px] uppercase tracking-widest text-on-surface-variant w-[10%]">内存</th>
                    <th className="pl-[10px] px-0 pr-2 py-3 text-[11px] uppercase tracking-widest text-on-surface-variant w-[10%] text-center">崩溃</th>
                    <th className="pl-[10px] px-0 pr-2 py-3 text-[11px] uppercase tracking-widest text-on-surface-variant w-[13%] text-right">操作</th>
                  </tr>
                </thead>
                <tbody className="text-body-sm divide-y divide-outline-variant/50">
                  {sortedFiltered.map((p) => {
                    const name = p.id.split("/").pop() || p.id;
                    const badge = statusBadge(p.status);
                    const icon = p.runtime_type === "widget" ? "dock" : "notes";
                    return (
                      <tr
                        key={p.id}
                        onClick={() => navigate(`/plugins/${encodeURIComponent(p.id)}`)}
                        className={`hover:bg-primary-container/10 transition-colors group cursor-pointer ${p.status === "crashed" ? "bg-error/5 border-l-2 border-error" : ""}`}
                      >
                        <td className="pl-[10px] px-0 pr-2 py-4 font-mono text-sm text-on-surface-variant truncate">{p.id}</td>
                        <td className="pl-[10px] px-0 pr-2 py-4">
                          <div className="flex items-center gap-3">
                            <div className={`w-8 h-8 rounded flex items-center justify-center border shrink-0 ${
                              p.status === "running" ? "bg-primary/10 text-primary border-primary/20" :
                              p.status === "crashed" ? "bg-error/10 text-error border-error/20" :
                              "bg-surface-container-high text-on-surface-variant border-outline-variant"
                            }`}>
                              <span className="material-symbols-outlined text-[18px]">{icon}</span>
                            </div>
                            <span className="font-medium text-on-surface group-hover:text-primary transition-colors">{name}</span>
                          </div>
                        </td>
                        <td className="pl-[10px] px-0 pr-2 py-4 text-on-surface-variant">{p.runtime_type === "widget" ? "Widget" : "Script"}</td>
                        <td className="pl-[10px] px-0 pr-2 py-4">
                          <div className="flex items-center gap-2">
                            <div className={`w-2 h-2 rounded-full ${badge.dot}`} />
                            <span className={`font-medium ${badge.text}`}>{badge.label}</span>
                          </div>
                        </td>
                        <td className="pl-[10px] px-0 pr-2 py-4 text-on-surface-variant font-mono text-sm">
                          {p.status === "running" ? formatMemory(p.memory_bytes) : "--"}
                        </td>
                        <td className="pl-[10px] px-0 pr-2 py-4 text-center text-on-surface-variant">
                          {p.crash_count > 0 ? <span className="text-status-error font-bold">{p.crash_count}</span> : "0"}
                        </td>
                        <td className="pl-[10px] px-0 pr-2 py-4">
                          <div className="flex items-center justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                            {p.status === "running" ? (
                              <button className="p-1.5 rounded text-on-surface-variant hover:text-on-surface hover:bg-surface-container-high transition-colors inline-flex items-center justify-center" title="停止"
                                onClick={(e) => { e.stopPropagation(); invoke("stop_plugin", { pluginId: p.id }).then(loadSnapshot); }}>
                                <span className="material-symbols-outlined text-[18px]">stop</span>
                              </button>
                            ) : (
                              <button className="p-1.5 rounded text-on-surface-variant hover:text-primary hover:bg-primary/10 transition-colors inline-flex items-center justify-center" title="启动"
                                onClick={(e) => { e.stopPropagation(); invoke("start_plugin", { pluginId: p.id }).then(loadSnapshot); }}>
                                <span className="material-symbols-outlined text-[18px]">play_arrow</span>
                              </button>
                            )}
                            <button className="p-1.5 rounded text-on-surface-variant hover:text-error hover:bg-error/10 transition-colors inline-flex items-center justify-center" title="卸载"
                              onClick={(e) => { e.stopPropagation(); setConfirmingUninstall(p.id); }}>
                              <span className="material-symbols-outlined text-[18px]">delete</span>
                            </button>
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            )}
          </div>
        )}

        {/* Card view */}
        {viewMode === "card" && (
          <div className="p-4">
            {loading ? (
              <div className="py-16 text-center text-on-surface-variant">加载中...</div>
            ) : sortedFiltered.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-16 text-on-surface-variant/50">
                <span className="material-symbols-outlined text-[40px] mb-3">{search ? "search_off" : "extension_off"}</span>
                <p className="text-body-sm">{search ? "未找到匹配的插件" : "暂无已安装插件"}</p>
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {sortedFiltered.map((p) => {
                  const name = p.id.split("/").pop() || p.id;
                  const badge = statusBadge(p.status);
                  const icon = p.runtime_type === "widget" ? "dock" : "notes";
                  return (
                    <div
                      key={p.id}
                      onClick={() => navigate(`/plugins/${encodeURIComponent(p.id)}`)}
                      className="bg-surface-container-lowest rounded-xl border border-outline-variant/30 p-4 flex flex-col gap-3 shadow-sm hover:-translate-y-1 hover:shadow-md hover:border-primary/30 transition-all duration-200 cursor-pointer"
                    >
                      <div className="flex justify-between items-start">
                        <div className={`w-10 h-10 rounded-lg flex items-center justify-center shrink-0 ${
                          p.status === "running" ? "bg-primary/10 text-primary" :
                          p.status === "crashed" ? "bg-error/10 text-error" :
                          "bg-surface-container-high text-on-surface-variant"
                        }`}>
                          <span className="material-symbols-outlined text-[20px]">{icon}</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <div className={`w-2 h-2 rounded-full ${badge.dot}`} />
                          <span className={`text-[10px] font-medium ${badge.text}`}>{badge.label}</span>
                        </div>
                      </div>
                      <div>
                        <h3 className="text-body-sm text-on-surface font-semibold mb-0.5">{name}</h3>
                        <p className="text-[11px] text-on-surface-variant font-mono">{p.id}</p>
                      </div>
                      <div className="mt-auto pt-3 flex items-center justify-between border-t border-outline-variant/30 text-on-surface-variant">
                        <div className="flex items-center gap-3 text-[11px]">
                          <span>{p.runtime_type === "widget" ? "Widget" : "Script"}</span>
                          <span>{(p.memory_bytes / (1024 * 1024)).toFixed(1)} MB</span>
                          <span>crash: {p.crash_count}</span>
                        </div>
                        <div className="flex gap-1">
                          {p.status === "running" ? (
                            <button className="p-1 rounded text-on-surface-variant hover:text-on-surface hover:bg-surface-container-high transition-colors" title="停止"
                              onClick={(e) => { e.stopPropagation(); invoke("stop_plugin", { pluginId: p.id }).then(loadSnapshot); }}>
                              <span className="material-symbols-outlined text-[16px]">stop</span>
                            </button>
                          ) : (
                            <button className="p-1 rounded text-on-surface-variant hover:text-primary hover:bg-primary/10 transition-colors" title="启动"
                              onClick={(e) => { e.stopPropagation(); invoke("start_plugin", { pluginId: p.id }).then(loadSnapshot); }}>
                              <span className="material-symbols-outlined text-[16px]">play_arrow</span>
                            </button>
                          )}
                          <button className="p-1 rounded text-on-surface-variant hover:text-error hover:bg-error/10 transition-colors" title="卸载"
                            onClick={(e) => { e.stopPropagation(); setConfirmingUninstall(p.id); }}>
                            <span className="material-symbols-outlined text-[16px]">delete</span>
                          </button>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Drag overlay */}
      {dragOver && (
        <div className="fixed inset-0 z-50 bg-primary/10 border-2 border-dashed border-primary rounded-xl flex items-center justify-center pointer-events-none">
          <div className="flex flex-col items-center gap-3 p-8 bg-surface-container-lowest/90 backdrop-blur-xl rounded-xl shadow-xl">
            <span className="material-symbols-outlined text-[48px] text-primary">upload_file</span>
            <p className="text-on-surface font-medium">释放以安装</p>
          </div>
        </div>
      )}

      {/* 卸载确认弹窗 */}
      {confirmingUninstall && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm" onClick={() => setConfirmingUninstall(null)}>
          <div className="bg-surface-container-lowest rounded-xl border border-outline-variant/30 shadow-xl p-6 max-w-sm w-full mx-4" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 rounded-full bg-error-container flex items-center justify-center shrink-0">
                <span className="material-symbols-outlined text-error text-[20px]">warning</span>
              </div>
              <div>
                <h3 className="text-body-sm font-semibold text-on-surface">确认卸载</h3>
                <p className="text-[11px] text-on-surface-variant mt-0.5">
                  确定要卸载 <span className="font-mono text-primary">{confirmingUninstall.split("/").pop()}</span> 吗？此操作不可撤销。
                </p>
              </div>
            </div>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setConfirmingUninstall(null)}
                className="px-4 py-2 rounded-lg text-body-sm font-medium text-on-surface-variant hover:bg-surface-container-high transition-colors"
              >
                取消
              </button>
              <button
                onClick={async () => {
                  await invoke("uninstall_plugin", { pluginId: confirmingUninstall });
                  setConfirmingUninstall(null);
                  addToast("success", "已卸载");
                  loadSnapshot();
                }}
                className="px-4 py-2 rounded-lg text-body-sm font-medium bg-error text-white hover:brightness-110 transition-colors"
              >
                确认卸载
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
