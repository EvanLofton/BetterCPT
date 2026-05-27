import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../ToastContext";

function syntaxHighlight(json: string): string {
  return json
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"([^"]+)":/g, '<span class="text-primary font-medium">"$1"</span>:')
    .replace(/: "([^"]*)"/g, ': <span class="text-on-surface-variant">"$1"</span>')
    .replace(/: (\d+)/g, ': <span class="text-status-warning">$1</span>')
    .replace(/: (true|false|null)/g, ': <span class="text-primary-container">$1</span>');
}

interface CrashMeta {
  filename: string;
  time: string;
  date: string;
  errorType: string;
  severity: "error" | "warning";
  pluginId?: string;
}

function formatTimestamp(ts: string): { time: string; date: string } {
  try {
    const d = new Date(ts);
    return {
      time: d.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }),
      date: `${String(d.getDate()).padStart(2, "0")}/${String(d.getMonth() + 1).padStart(2, "0")}`,
    };
  } catch { return { time: "--:--", date: "--/--" }; }
}

export default function CrashLog() {
  const [files, setFiles] = useState<string[]>([]);
  const [metaList, setMetaList] = useState<CrashMeta[]>([]);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [fileContent, setFileContent] = useState<string | null>(null);
  const [parsedJson, setParsedJson] = useState<Record<string, any> | null>(null);
  const [showClearConfirm, setShowClearConfirm] = useState(false);
  const { addToast } = useToast();

  const refreshFiles = async () => {
    try {
      const json = await invoke<string>("list_crash_logs");
      const list = JSON.parse(json) as string[];
      setFiles(list);
      const metas: CrashMeta[] = [];
      for (const f of list) {
        try {
          const raw = await invoke<string>("read_crash_log", { filename: f });
          const obj = JSON.parse(raw);
          const exc = obj.exception || {};
          const ts = obj.metadata?.timestamp ? formatTimestamp(obj.metadata.timestamp) : { time: "--:--", date: "--/--" };
          metas.push({
            filename: f,
            time: ts.time,
            date: ts.date,
            errorType: exc.type || "Unknown",
            severity: exc.type?.toLowerCase().includes("null") || exc.type?.toLowerCase().includes("sigsegv") ? "error" : "warning",
            pluginId: obj.pluginId,
          });
        } catch {
          metas.push({ filename: f, time: "--:--", date: "--/--", errorType: "Unknown", severity: "warning" });
        }
      }
      setMetaList(metas);
    } catch (e) { console.error(e); }
  };

  useEffect(() => { refreshFiles(); }, []);

  const handleClearAll = async () => {
    try {
      await invoke("clear_crash_logs");
      setSelectedFile(null);
      setFileContent(null);
      setParsedJson(null);
      await refreshFiles();
      addToast("success", "已清除全部崩溃日志");
    } catch (e) {
      addToast("error", `清除失败: ${e}`);
    }
    setShowClearConfirm(false);
  };

  const selectFile = (filename: string) => {
    setSelectedFile(filename);
    invoke<string>("read_crash_log", { filename })
      .then((raw) => {
        setFileContent(raw);
        try {
          setParsedJson(JSON.parse(raw));
        } catch { setParsedJson(null); }
      })
      .catch((e) => {
        setFileContent(String(e));
        setParsedJson(null);
      });
  };

  const copyContent = () => {
    if (!fileContent) return;
    navigator.clipboard.writeText(fileContent).then(() => {
      addToast("success", "已复制到剪贴板");
    }).catch(() => {
      addToast("error", "复制失败");
    });
  };

  const exportFile = async () => {
    if (!fileContent || !selectedFile) return;
    try {
      // WebView2 原生保存对话框
      const handle = await (window as any).showSaveFilePicker({
        suggestedName: selectedFile,
        types: [{ description: "JSON", accept: { "application/json": [".json"] } }],
      });
      const writable = await handle.createWritable();
      await writable.write(fileContent);
      await writable.close();
      addToast("success", `已保存: ${selectedFile}`);
    } catch (e: any) {
      if (e?.name === "AbortError") return; // 用户取消
      addToast("error", "保存失败，请重试");
    }
  };

  return (
    <div className="p-6 flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between mb-4 flex-shrink-0">
        <div>
          <h1 className="font-display text-display-lg text-on-surface mb-1">崩溃日志</h1>
          <p className="text-body-md text-on-surface-variant">
            查看和管理插件崩溃记录{files.length > 0 ? `（${files.length}）` : ""}
          </p>
        </div>
        {files.length > 0 && (
          <button
            onClick={() => setShowClearConfirm(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-body-sm font-medium text-on-surface-variant hover:text-error hover:bg-error/5 border border-transparent hover:border-error/20 transition-colors"
          >
            <span className="material-symbols-outlined text-[16px]">delete_sweep</span>
            清除全部
          </button>
        )}
      </div>

      <div className="flex gap-4 flex-1 min-h-0">
        {/* File list */}
        <aside className="w-72 flex-shrink-0 flex flex-col bg-surface-container-lowest rounded-xl border border-outline-variant/30 shadow-sm overflow-hidden">
          <div className="p-4 border-b border-outline-variant/30 flex justify-between items-center bg-surface/50">
            <span className="text-[11px] font-semibold text-on-surface-variant uppercase tracking-wider">崩溃记录</span>
            <span className="text-[10px] text-on-surface-variant">{files.length} 条</span>
          </div>
          <div className="overflow-y-auto flex-1 p-2 space-y-1">
            {files.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-16 text-on-surface-variant/50">
                <span className="material-symbols-outlined text-[40px] mb-3">check_circle</span>
                <p className="text-body-sm">暂无崩溃记录</p>
              </div>
            ) : (
              metaList.map((m) => {
                const isActive = selectedFile === m.filename;
                return (
                  <button
                    key={m.filename}
                    onClick={() => selectFile(m.filename)}
                    className={`w-full text-left p-3 rounded-lg transition-colors border-l-[3px] group ${
                      isActive
                        ? "bg-primary-container/20 border-primary shadow-sm"
                        : "border-transparent hover:bg-surface-container-high"
                    }`}
                  >
                    <div className="flex justify-between items-start mb-1">
                      <span className={`text-[12px] font-medium truncate pr-2 ${
                        isActive ? "text-on-surface" : "text-on-surface-variant group-hover:text-on-surface"
                      }`}>
                        {m.filename}
                      </span>
                      <div className="text-right shrink-0">
                        <div className="text-[10px] text-on-surface-variant tabular-nums">{m.time}</div>
                        <div className="text-[9px] text-on-surface-variant/50 tabular-nums">{m.date}</div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className={`w-1.5 h-1.5 rounded-full ${
                        m.severity === "error" ? "bg-status-error" : "bg-status-warning"
                      }`} />
                      <span className={`text-[10px] uppercase tracking-wider ${
                        m.severity === "error" ? "text-status-error font-medium" : "text-status-warning"
                      }`}>
                        {m.errorType}
                      </span>
                      {m.pluginId && (
                        <span className="text-[10px] text-on-surface-variant/50 truncate ml-auto">
                          {m.pluginId.split("/").pop()}
                        </span>
                      )}
                    </div>
                  </button>
                );
              })
            )}
          </div>
        </aside>

        {/* Detail panel */}
        <section className="flex-1 flex flex-col bg-surface-container-lowest rounded-xl border border-outline-variant/30 shadow-sm overflow-hidden">
          <div className="p-4 border-b border-outline-variant/30 bg-surface/50 flex justify-between items-center">
            <div className="flex items-center gap-3 min-w-0">
              <span className="material-symbols-outlined text-on-surface-variant shrink-0">description</span>
              <h3 className="text-body-sm font-semibold text-on-surface truncate">
                {selectedFile || "选择左侧文件查看详情"}
              </h3>
            </div>
            {selectedFile && (
              <div className="flex gap-2 shrink-0">
                <button
                  onClick={copyContent}
                  className="px-3 py-1.5 rounded-md text-[11px] font-medium bg-primary-container/10 text-primary hover:bg-primary-container/20 transition-colors flex items-center gap-1"
                >
                  <span className="material-symbols-outlined text-[14px]">content_copy</span>复制
                </button>
                <button
                  onClick={exportFile}
                  className="px-3 py-1.5 rounded-md text-[11px] font-medium bg-surface-container-high text-on-surface-variant hover:bg-surface-variant transition-colors flex items-center gap-1"
                >
                  <span className="material-symbols-outlined text-[14px]">download</span>导出
                </button>
              </div>
            )}
          </div>
          <div className="flex-1 overflow-auto p-4 bg-surface">
            {parsedJson ? (
              <div className="space-y-3">
                {/* Crash summary */}
                {parsedJson.metadata && (
                  <div className="flex gap-3 text-[11px] text-on-surface-variant mb-4 flex-wrap">
                    {parsedJson.metadata.timestamp && (
                      <span className="px-2 py-0.5 rounded bg-surface-container-high">
                        {parsedJson.metadata.timestamp}
                      </span>
                    )}
                    {parsedJson.metadata.app_version && (
                      <span className="px-2 py-0.5 rounded bg-surface-container-high">
                        {parsedJson.metadata.app_version}
                      </span>
                    )}
                    {parsedJson.metadata.os && (
                      <span className="px-2 py-0.5 rounded bg-surface-container-high">
                        {parsedJson.metadata.os}
                      </span>
                    )}
                  </div>
                )}
                {parsedJson.exception && (
                  <div className="p-3 rounded-lg bg-error/5 border border-error/20 mb-4">
                    <p className="text-[11px] text-status-error font-semibold uppercase tracking-wider mb-1">
                      {parsedJson.exception.type}
                    </p>
                    <p className="text-body-sm text-on-surface">
                      {parsedJson.exception.message}
                    </p>
                  </div>
                )}
                {parsedJson.pluginId && (
                  <div className="flex items-center gap-2 text-body-sm text-on-surface-variant mb-4">
                    <span>插件:</span>
                    <span className="font-mono text-[12px] text-primary">{parsedJson.pluginId}</span>
                    {parsedJson.crashCount != null && (
                      <span className="text-status-error font-medium">(崩溃 #{parsedJson.crashCount})</span>
                    )}
                  </div>
                )}
                {/* Raw JSON */}
                <details open className="mt-4">
                  <summary className="text-[11px] text-on-surface-variant cursor-pointer hover:text-on-surface font-medium mb-2">
                    原始 JSON
                  </summary>
                  <pre className="font-mono text-[11px] leading-relaxed bg-surface-container-low rounded-lg p-4 border border-outline-variant/30 overflow-x-auto">
                    <code dangerouslySetInnerHTML={{ __html: syntaxHighlight(fileContent || "") }} />
                  </pre>
                </details>
              </div>
            ) : fileContent ? (
              <pre className="font-mono text-[12px] leading-relaxed whitespace-pre-wrap text-on-surface-variant">{fileContent}</pre>
            ) : (
              <div className="flex flex-col items-center justify-center h-full text-on-surface-variant/40">
                <span className="material-symbols-outlined text-[48px] mb-3">list_alt</span>
                <p className="text-body-sm">选择左侧崩溃记录查看详情</p>
              </div>
            )}
          </div>
        </section>
      </div>

      {/* 清除确认弹窗 */}
      {showClearConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm" onClick={() => setShowClearConfirm(false)}>
          <div className="bg-surface-container-lowest rounded-xl border border-outline-variant/30 shadow-xl p-6 max-w-sm w-full mx-4" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 rounded-full bg-error-container flex items-center justify-center shrink-0">
                <span className="material-symbols-outlined text-error text-[20px]">warning</span>
              </div>
              <div>
                <h3 className="text-body-sm font-semibold text-on-surface">清除全部崩溃日志</h3>
                <p className="text-[11px] text-on-surface-variant mt-0.5">此操作不可撤销，所有 {files.length} 条记录将被永久删除。</p>
              </div>
            </div>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setShowClearConfirm(false)}
                className="px-4 py-2 rounded-lg text-body-sm font-medium text-on-surface-variant hover:bg-surface-container-high transition-colors"
              >
                取消
              </button>
              <button
                onClick={handleClearAll}
                className="px-4 py-2 rounded-lg text-body-sm font-medium bg-error text-white hover:brightness-110 transition-colors"
              >
                确认清除
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
