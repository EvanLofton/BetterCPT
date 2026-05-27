import { useState } from "react";
import { useNavigate } from "react-router-dom";

export interface PluginCard {
  id: string;
  name: string;
  scope: string;
  description: string;
  category: string;
  rating: number;
  version: string;
  icon: string;
  installed: boolean;
}

export const DEMO_PLUGINS: PluginCard[] = [
  {
    id: "advanced-terminal",
    name: "Advanced Terminal",
    scope: "@community/terminal",
    description: "高性能终端模拟器，支持多路复用和深度系统集成。",
    category: "tool", rating: 4.9, version: "v3.1.0", icon: "terminal", installed: false,
  },
  {
    id: "json-formatter-pro",
    name: "JSON Formatter Pro",
    scope: "@community/json-fmt",
    description: "实时验证、格式化和可视化复杂 JSON 结构。",
    category: "tool", rating: 4.7, version: "v1.2.4", icon: "data_object", installed: false,
  },
  {
    id: "aero-glass-theme",
    name: "Aero Glass Theme",
    scope: "@community/aero-glass",
    description: "经典半透明窗口边框，带现代性能优化。",
    category: "beauty", rating: 4.5, version: "v2.0.1", icon: "palette", installed: false,
  },
  {
    id: "resource-monitor",
    name: "Resource Monitor",
    scope: "@community/sys-mon",
    description: "精致 Widget，直接在桌面追踪 CPU、RAM 和网络使用。",
    category: "widget", rating: 4.8, version: "v1.1.0", icon: "memory", installed: false,
  },
  {
    id: "weather-widget",
    name: "Weather Widget",
    scope: "@community/weather",
    description: "实时天气显示，支持多城市切换和降水预报。",
    category: "widget", rating: 4.6, version: "v2.0.0", icon: "cloud", installed: false,
  },
  {
    id: "code-snippet-mgr",
    name: "Code Snippet Manager",
    scope: "@community/snippets",
    description: "快速保存、搜索和插入常用代码片段，支持语法高亮。",
    category: "script", rating: 4.4, version: "v1.0.3", icon: "code", installed: false,
  },
];

const CATEGORIES = [
  { key: "all", label: "全部" },
  { key: "widget", label: "Widget" },
  { key: "script", label: "Script" },
  { key: "tool", label: "工具" },
  { key: "beauty", label: "美化" },
];

const SORTS = ["精选", "最多下载", "最高评分", "最新上架"];

export default function Marketplace() {
  const [activeCategory, setActiveCategory] = useState("all");
  const [sort, setSort] = useState(SORTS[0]);
  const [search, setSearch] = useState("");
  const [sortOpen, setSortOpen] = useState(false);
  const navigate = useNavigate();

  const filtered = DEMO_PLUGINS.filter((p) => {
    if (activeCategory !== "all" && p.category !== activeCategory) return false;
    if (search) {
      const q = search.toLowerCase();
      return p.name.toLowerCase().includes(q) || p.description.toLowerCase().includes(q) || p.scope.toLowerCase().includes(q);
    }
    return true;
  });

  return (
    <div className="p-6">
      {/* Header */}
      <div className="mb-6">
        <h2 className="font-display text-display-lg text-on-surface tracking-tight">插件市场</h2>
        <p className="text-body-md text-on-surface-variant mt-1">
          发现和安装优质插件、Widget 和工具，提升你的工作流。
        </p>
      </div>

      {/* Toolbar: 分类 + 搜索 + 排序 同一行 */}
      <div className="flex items-center justify-between gap-4 mb-6">
        {/* Category tags */}
        <div className="flex items-center gap-2 overflow-x-auto scrollbar-hide">
          {CATEGORIES.map((cat) => (
            <button
              key={cat.key}
              onClick={() => setActiveCategory(cat.key)}
              className={`px-4 py-1.5 rounded-full text-body-sm font-medium transition-all active:scale-95 whitespace-nowrap ${
                activeCategory === cat.key
                  ? "bg-primary-container text-on-primary-container shadow-sm"
                  : "bg-surface-container-lowest text-on-surface border border-outline-variant hover:border-primary hover:text-primary"
              }`}
            >
              {cat.label}
            </button>
          ))}
        </div>
        {/* Search + Sort */}
        <div className="flex items-center gap-3 shrink-0">
          <div className="relative">
            <span className="material-symbols-outlined absolute left-3 top-1/2 -translate-y-1/2 text-on-surface-variant text-[18px]">search</span>
            <input
              className="w-48 bg-surface-container-lowest border border-outline-variant rounded-lg pl-10 pr-4 py-1 text-body-sm focus:outline-none focus:border-primary transition-colors shadow-sm text-on-surface"
              placeholder="搜索插件..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          <div className="flex items-center gap-2">
            <span className="text-body-sm text-outline">排序:</span>
            <div className="relative">
              <button
                onClick={() => setSortOpen(!sortOpen)}
                className="flex items-center justify-center gap-1 bg-surface-container-lowest border border-outline-variant rounded-lg px-2 py-1 text-body-sm shadow-sm text-on-surface hover:border-primary transition-colors w-[90px]"
              >
                {sort}
                <span className={`material-symbols-outlined text-on-surface-variant/40 text-[16px] transition-transform ${sortOpen ? "rotate-180" : ""}`}>arrow_drop_down</span>
              </button>
              <div className={`absolute top-full left-0 mt-1 bg-surface-container-lowest border border-outline-variant rounded-lg shadow-lg z-20 overflow-hidden transition-all duration-200 origin-top w-[90px] ${
                sortOpen ? "opacity-100 scale-y-100 translate-y-0" : "opacity-0 scale-y-0 -translate-y-1 pointer-events-none"
              }`}>
                {SORTS.map((s) => (
                  <button
                    key={s}
                    onClick={() => { setSort(s); setSortOpen(false); }}
                    className={`w-full px-3 py-1.5 text-body-sm transition-colors whitespace-nowrap text-center ${
                      s === sort ? "bg-primary-container/20 text-primary font-medium" : "text-on-surface hover:bg-surface-container-low"
                    }`}
                  >
                    {s}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Plugin cards grid */}
      <div className="grid grid-cols-3 gap-4">
        {filtered.map((p) => (
          <article
            key={p.id}
            onClick={() => navigate(`/marketplace/${p.id}`)}
            className="bg-surface-container-lowest rounded-xl border border-outline-variant/30 p-4 flex flex-col gap-3 shadow-sm hover:-translate-y-1 hover:shadow-md hover:border-primary/30 transition-all duration-200 cursor-pointer"
          >
            <div className="flex justify-between items-start">
              <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-primary to-primary-container flex items-center justify-center text-on-primary shadow-sm shrink-0">
                <span className="material-symbols-outlined text-[20px]">{p.icon}</span>
              </div>
              <span className="px-2 py-0.5 bg-surface-container-high text-on-surface-variant rounded font-mono text-[10px]">{p.scope}</span>
            </div>
            <div>
              <h3 className="text-body-sm text-on-surface font-semibold mb-0.5">{p.name}</h3>
              <p className="text-[11px] text-on-surface-variant line-clamp-2">{p.description}</p>
            </div>
            <div className="mt-auto pt-3 flex items-center justify-between border-t border-outline-variant/30">
              <div className="flex items-center gap-2">
                <div className="flex items-center gap-0.5 text-amber-500">
                  <span className="material-symbols-outlined text-[14px] fill">star</span>
                  <span className="text-[12px] text-on-surface font-medium">{p.rating}</span>
                </div>
                <span className="text-[10px] uppercase tracking-wider text-outline">{p.version}</span>
              </div>
              <button
                onClick={(e) => { e.stopPropagation(); }}
                className={`px-3 py-1 rounded-md text-[11px] font-medium active:scale-95 transition-all shadow-sm ${
                  p.installed
                    ? "bg-surface-container-high text-on-surface-variant border border-outline-variant cursor-default"
                    : "bg-primary-container text-on-primary-container hover:brightness-110"
                }`}
                disabled={p.installed}
              >
                {p.installed ? "已安装" : "安装"}
              </button>
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}
