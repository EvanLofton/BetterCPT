import { useState, useRef, useEffect, useCallback } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { useTheme } from "../ThemeContext";
import { useToast } from "../ToastContext";

interface AppSettings {
  autoStart: boolean;
  minimizeToTray: boolean;
  devMode: boolean;
  theme: "light" | "dark";
  autoEnableNew: boolean;
  autoRestartCrash: boolean;
  maxCrashRestart: number;
  storageQuota: string;
  logLevel: string;
}

export default function Settings() {
  const { theme, setTheme } = useTheme();
  const { addToast } = useToast();
  const [loaded, setLoaded] = useState(false);

  // ---- 常规 ----
  const [autoStart, setAutoStart] = useState(true);
  const [minimizeToTray, setMinimizeToTray] = useState(false);
  const [devMode, setDevMode] = useState(false);

  // ---- 插件 ----
  const [autoEnableNew, setAutoEnableNew] = useState(true);
  const [autoRestartCrash, setAutoRestartCrash] = useState(true);
  const [maxCrashRestart, setMaxCrashRestart] = useState(5);
  const [storageQuota, setStorageQuota] = useState("1 MB");

  // ---- 日志 ----
  const [logLevel, setLogLevel] = useState("Info");

  // 加载设置
  useEffect(() => {
    (async () => {
      try {
        const raw = await invoke<string>("get_settings");
        const s: AppSettings = JSON.parse(raw);
        setAutoStart(s.autoStart);
        setMinimizeToTray(s.minimizeToTray);
        setDevMode(s.devMode);
        setTheme(s.theme);
        setAutoEnableNew(s.autoEnableNew);
        setAutoRestartCrash(s.autoRestartCrash);
        setMaxCrashRestart(s.maxCrashRestart);
        setStorageQuota(s.storageQuota);
        setLogLevel(s.logLevel);
      } catch {
        // 首次启动没有文件，使用默认值
      }
      setLoaded(true);
    })();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // 保存设置到后端
  const save = useCallback(async (patch: Partial<AppSettings>) => {
    if (!loaded) return;
    const current: AppSettings = {
      autoStart, minimizeToTray, devMode, theme,
      autoEnableNew, autoRestartCrash, maxCrashRestart,
      storageQuota, logLevel,
      ...patch,
    };
    try {
      await invoke("update_settings", { settingsJson: JSON.stringify(current, null, 2) });
      addToast("success", "设置已保存");
    } catch (e) {
      addToast("error", `保存失败: ${e}`);
    }
  }, [autoStart, minimizeToTray, devMode, theme, autoEnableNew, autoRestartCrash, maxCrashRestart, storageQuota, logLevel, loaded, addToast]);

  const upd = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    const patch = { [key]: value } as Partial<AppSettings>;
    switch (key) {
      case "autoStart": setAutoStart(value as boolean); break;
      case "minimizeToTray": setMinimizeToTray(value as boolean); break;
      case "devMode": setDevMode(value as boolean); break;
      case "theme": setTheme(value as "light" | "dark"); break;
      case "autoEnableNew": setAutoEnableNew(value as boolean); break;
      case "autoRestartCrash": setAutoRestartCrash(value as boolean); break;
      case "maxCrashRestart": setMaxCrashRestart(value as number); break;
      case "storageQuota": setStorageQuota(value as string); break;
      case "logLevel": setLogLevel(value as string); break;
    }
    save(patch);
  };

  const Toggle = ({ checked, onChange }: { checked: boolean; onChange: () => void }) => (
    <label className="relative inline-flex items-center cursor-pointer">
      <input checked={checked} onChange={onChange} className="sr-only peer" type="checkbox" />
      <div className="w-11 h-6 bg-surface-variant peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary" />
    </label>
  );

  const SelectRow = ({ label, desc, value, options, onChange }: {
    label: string; desc: string; value: string; options: string[]; onChange: (v: string) => void;
  }) => {
    const [open, setOpen] = useState(false);
    const [pos, setPos] = useState({ top: 0, left: 0 });
    const btnRef = useRef<HTMLButtonElement>(null);

    useEffect(() => {
      if (open && btnRef.current) {
        const r = btnRef.current.getBoundingClientRect();
        setPos({ top: r.bottom + 4, left: r.left });
      }
    }, [open]);

    return (
      <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors">
        <div>
          <h3 className="text-body-sm font-semibold text-on-surface">{label}</h3>
          <p className="text-[11px] text-on-surface-variant mt-1">{desc}</p>
        </div>
        <div>
          <button
            ref={btnRef}
            onClick={() => setOpen(!open)}
            className="flex items-center justify-between gap-3 bg-surface-container-lowest border border-outline-variant rounded-lg px-3 py-1.5 text-body-sm text-on-surface hover:border-primary transition-colors min-w-[110px]"
          >
            <span>{value}</span>
            <span className={`material-symbols-outlined text-on-surface-variant/40 text-[16px] transition-transform ${open ? "rotate-180" : ""}`}>arrow_drop_down</span>
          </button>
          {open && createPortal(
            <div className="fixed z-[9999] bg-surface-container-lowest border border-outline-variant rounded-lg shadow-lg overflow-hidden min-w-[110px] animate-fade-in-up"
              style={{ top: pos.top, left: pos.left }}
              onClick={() => setOpen(false)}
            >
              {options.map((opt) => (
                <button
                  key={opt}
                  onClick={() => { onChange(opt); setOpen(false); }}
                  className={`w-full px-3 py-1.5 text-body-sm transition-colors text-center ${
                    opt === value ? "bg-primary-container/20 text-primary font-medium" : "text-on-surface hover:bg-surface-container-low"
                  }`}
                >
                  {opt}
                </button>
              ))}
            </div>,
            document.body
          )}
        </div>
      </div>
    );
  };

  return (
    <div className="max-w-4xl mx-auto space-y-8 pb-16 p-6">
      <div className="mb-8">
        <h1 className="font-display text-display-lg text-on-surface">设置</h1>
        <p className="text-body-base text-on-surface-variant mt-2">
          管理 BetterCPT 应用程序偏好和配置。
        </p>
      </div>

      {/* ====== 常规 ====== */}
      <section className="bg-surface-container-lowest border border-outline-variant rounded-xl shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-outline-variant bg-surface/50">
          <h2 className="text-body-sm font-semibold text-on-surface flex items-center gap-2">
            <span className="material-symbols-outlined text-primary text-[18px]">tune</span>常规
          </h2>
        </div>
        <div className="divide-y divide-outline-variant">
          <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors">
            <div>
              <h3 className="text-body-sm font-semibold text-on-surface">开机自启动</h3>
              <p className="text-[11px] text-on-surface-variant mt-1">登录计算机时自动启动 BetterCPT。</p>
            </div>
            <Toggle checked={autoStart} onChange={() => upd("autoStart", !autoStart)} />
          </div>
          <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors">
            <div>
              <h3 className="text-body-sm font-semibold text-on-surface">最小化到托盘</h3>
              <p className="text-[11px] text-on-surface-variant mt-1">关闭窗口时保持后台运行。</p>
            </div>
            <Toggle checked={minimizeToTray} onChange={() => upd("minimizeToTray", !minimizeToTray)} />
          </div>
          <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors">
            <div>
              <h3 className="text-body-sm font-semibold text-on-surface">开发者模式</h3>
              <p className="text-[11px] text-on-surface-variant mt-1">启用插件开发工具，允许加载未签名插件和 Native Runtime。</p>
            </div>
            <Toggle checked={devMode} onChange={() => upd("devMode", !devMode)} />
          </div>
        </div>
      </section>

      {/* ====== 外观 ====== */}
      <section className="bg-surface-container-lowest border border-outline-variant rounded-xl shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-outline-variant bg-surface/50">
          <h2 className="text-body-sm font-semibold text-on-surface flex items-center gap-2">
            <span className="material-symbols-outlined text-primary text-[18px]">palette</span>外观
          </h2>
        </div>
        <div className="divide-y divide-outline-variant">
          <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors">
            <div>
              <h3 className="text-body-sm font-semibold text-on-surface">主题</h3>
              <p className="text-[11px] text-on-surface-variant mt-1">切换浅色或深色外观。</p>
            </div>
            <div className="flex items-center gap-1 bg-surface-container rounded-md p-0.5">
              <button
                onClick={() => upd("theme", "light")}
                className={`flex items-center gap-1.5 px-3 py-1 rounded text-[11px] font-medium transition-all ${
                  theme === "light" ? "bg-white shadow-sm text-on-surface" : "text-on-surface-variant"
                }`}
              >
                <span className="material-symbols-outlined text-[14px]">light_mode</span>浅色
              </button>
              <button
                onClick={() => upd("theme", "dark")}
                className={`flex items-center gap-1.5 px-3 py-1 rounded text-[11px] font-medium transition-all ${
                  theme === "dark" ? "bg-white shadow-sm text-on-surface" : "text-on-surface-variant"
                }`}
              >
                <span className="material-symbols-outlined text-[14px]">dark_mode</span>深色
              </button>
            </div>
          </div>
        </div>
      </section>

      {/* ====== 插件 ====== */}
      <section className="bg-surface-container-lowest border border-outline-variant rounded-xl shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-outline-variant bg-surface/50">
          <h2 className="text-body-sm font-semibold text-on-surface flex items-center gap-2">
            <span className="material-symbols-outlined text-primary text-[18px]">extension</span>插件
          </h2>
        </div>
        <div className="divide-y divide-outline-variant">
          <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors">
            <div>
              <h3 className="text-body-sm font-semibold text-on-surface">新插件默认启动</h3>
              <p className="text-[11px] text-on-surface-variant mt-1">从市场安装的新插件自动启用。</p>
            </div>
            <Toggle checked={autoEnableNew} onChange={() => upd("autoEnableNew", !autoEnableNew)} />
          </div>
          <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors">
            <div>
              <h3 className="text-body-sm font-semibold text-on-surface">崩溃自动重启</h3>
              <p className="text-[11px] text-on-surface-variant mt-1">插件异常退出时自动尝试重启。</p>
            </div>
            <Toggle checked={autoRestartCrash} onChange={() => upd("autoRestartCrash", !autoRestartCrash)} />
          </div>
          <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors">
            <div>
              <h3 className="text-body-sm font-semibold text-on-surface">最大崩溃重启次数</h3>
              <p className="text-[11px] text-on-surface-variant mt-1">单个插件崩溃后自动重启的上限，超限后停止重启。</p>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => upd("maxCrashRestart", Math.max(1, maxCrashRestart - 1))}
                className="w-7 h-7 rounded-md border border-outline-variant flex items-center justify-center text-on-surface-variant hover:border-primary hover:text-primary transition-colors text-[14px] font-medium"
              >
                −
              </button>
              <span className="w-8 text-center text-body-sm font-medium text-on-surface tabular-nums">{maxCrashRestart}</span>
              <button
                onClick={() => upd("maxCrashRestart", Math.min(10, maxCrashRestart + 1))}
                className="w-7 h-7 rounded-md border border-outline-variant flex items-center justify-center text-on-surface-variant hover:border-primary hover:text-primary transition-colors text-[14px] font-medium"
              >
                +
              </button>
            </div>
          </div>
          <SelectRow
            label="存储配额"
            desc="每个插件的 KV 存储上限。"
            value={storageQuota}
            options={["256 KB", "512 KB", "1 MB", "5 MB", "10 MB"]}
            onChange={(v) => upd("storageQuota", v)}
          />
        </div>
      </section>

      {/* ====== 日志 ====== */}
      <section className="bg-surface-container-lowest border border-outline-variant rounded-xl shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-outline-variant bg-surface/50">
          <h2 className="text-body-sm font-semibold text-on-surface flex items-center gap-2">
            <span className="material-symbols-outlined text-primary text-[18px]">terminal</span>日志
          </h2>
        </div>
        <div className="divide-y divide-outline-variant">
          <SelectRow
            label="日志级别"
            desc="控制写入日志文件的详细程度。Error 最少，Trace 最详细。"
            value={logLevel}
            options={["Error", "Warn", "Info", "Debug", "Trace"]}
            onChange={(v) => upd("logLevel", v)}
          />
        </div>
      </section>

      {/* ====== 关于 ====== */}
      <section className="bg-surface-container-lowest border border-outline-variant rounded-xl shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-outline-variant bg-surface/50">
          <h2 className="text-body-sm font-semibold text-on-surface flex items-center gap-2">
            <span className="material-symbols-outlined text-primary text-[18px]">info</span>关于
          </h2>
        </div>
        <div className="divide-y divide-outline-variant">
          <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors">
            <div>
              <h3 className="text-body-sm font-semibold text-on-surface">版本</h3>
              <p className="text-[11px] text-on-surface-variant mt-1">当前 BetterCPT 应用程序版本。</p>
            </div>
            <div className="font-mono text-[12px] text-on-surface-variant bg-surface-container px-3 py-1 rounded-md border border-outline-variant">v1.0.0</div>
          </div>
          <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors">
            <div>
              <h3 className="text-body-sm font-semibold text-on-surface">Host 版本</h3>
              <p className="text-[11px] text-on-surface-variant mt-1">底层系统 Host 版本。</p>
            </div>
            <div className="font-mono text-[12px] text-on-surface-variant bg-surface-container px-3 py-1 rounded-md border border-outline-variant">Windows 11 · Rust 0.1.0</div>
          </div>
          <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors">
            <div>
              <h3 className="text-body-sm font-semibold text-on-surface">Deno Core</h3>
              <p className="text-[11px] text-on-surface-variant mt-1">JavaScript/TypeScript 运行时引擎版本。</p>
            </div>
            <div className="font-mono text-[12px] text-on-surface-variant bg-surface-container px-3 py-1 rounded-md border border-outline-variant">v0.290</div>
          </div>
          <div className="px-6 py-5 flex items-center justify-between hover:bg-surface-container-low transition-colors group cursor-pointer">
            <div>
              <h3 className="text-body-sm font-semibold text-on-surface">数据目录</h3>
              <p className="text-[11px] text-on-surface-variant mt-1">插件和配置文件的存储位置。</p>
            </div>
            <div className="flex items-center gap-2">
              <span className="font-mono text-[12px] text-primary truncate max-w-[200px] md:max-w-xs group-hover:underline">%AppData%/BetterCPT</span>
              <span className="material-symbols-outlined text-on-surface-variant group-hover:text-primary transition-colors text-[16px]">open_in_new</span>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
