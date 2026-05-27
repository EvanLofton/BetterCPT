// v1: 占位描述页。v2 将从插件 info.md 动态加载内容，开发者自行维护。
import { useParams, useNavigate } from "react-router-dom";
import { DEMO_PLUGINS } from "./Marketplace";

export default function MarketplaceDetail() {
  const { pluginId } = useParams<{ pluginId: string }>();
  const navigate = useNavigate();
  const plugin = DEMO_PLUGINS.find((p) => p.id === pluginId);

  if (!plugin) {
    return (
      <div className="p-6 text-center">
        <p className="text-on-surface-variant mb-4">未找到该插件</p>
        <button onClick={() => navigate("/marketplace")} className="text-primary hover:underline">返回市场</button>
      </div>
    );
  }

  return (
    <div className="p-6">
      {/* Back arrow */}
      <button
        onClick={() => navigate("/marketplace")}
        className="inline-flex items-center gap-1.5 text-on-surface-variant hover:text-primary transition-colors mb-6 text-body-sm"
      >
        <span className="material-symbols-outlined text-[20px]">arrow_back</span>
        返回插件市场
      </button>

      {/* Header card */}
      <div className="flex flex-col md:flex-row gap-6 items-start md:items-center justify-between bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-6 shadow-sm mb-6">
        <div className="flex items-center gap-5">
          <div className="w-16 h-16 rounded-xl bg-gradient-to-br from-primary to-primary-container flex items-center justify-center text-on-primary shadow-sm shrink-0">
            <span className="material-symbols-outlined text-[32px]">{plugin.icon}</span>
          </div>
          <div>
            <div className="flex items-center gap-3 mb-1">
              <h2 className="font-display text-display-lg text-on-surface">{plugin.name}</h2>
              <span className="px-2.5 py-0.5 rounded-full bg-primary-container/10 text-primary border border-primary/20 text-[10px] font-medium">
                {plugin.category === "widget" ? "Widget" : plugin.category === "script" ? "Script" : plugin.category === "tool" ? "工具" : "美化"}
              </span>
            </div>
            <p className="font-mono text-[12px] text-on-surface-variant">{plugin.scope} · {plugin.version}</p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1 text-amber-500">
            <span className="material-symbols-outlined text-[20px] fill">star</span>
            <span className="text-body-lg text-on-surface font-semibold">{plugin.rating}</span>
          </div>
          <button className={`px-5 py-2 rounded-lg text-body-sm font-medium active:scale-95 transition-all shadow-sm ${
            plugin.installed
              ? "bg-surface-container-high text-on-surface-variant border border-outline-variant cursor-default"
              : "bg-primary text-on-primary hover:brightness-110"
          }`} disabled={plugin.installed}>
            {plugin.installed ? "已安装" : "安装"}
          </button>
        </div>
      </div>

      {/* Description */}
      <div className="bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-6 shadow-sm">
        <h3 className="font-headline-md text-headline-md text-on-surface mb-4 flex items-center gap-2">
          <span className="material-symbols-outlined text-primary/60">description</span>关于此插件
        </h3>
        <p className="text-body-base text-on-surface-variant leading-relaxed mb-6">
          {plugin.description}
        </p>

        <h4 className="font-label-caps text-label-caps text-on-surface-variant mb-3">功能亮点</h4>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mb-6">
          {[
            { icon: "bolt", label: "高性能", desc: "优化的运行效率，资源占用极低" },
            { icon: "update", label: "持续更新", desc: "社区活跃维护，及时修复问题" },
            { icon: "shield", label: "安全可靠", desc: "通过 BetterCPT 安全审核" },
            { icon: "integration_instructions", label: "易于集成", desc: "标准 SDK API，快速上手" },
          ].map((item, i) => (
            <div key={i} className="flex items-start gap-3 p-3 rounded-lg bg-surface-container-low">
              <span className="material-symbols-outlined text-primary/60 mt-0.5">{item.icon}</span>
              <div>
                <p className="text-body-sm text-on-surface font-medium">{item.label}</p>
                <p className="text-[11px] text-on-surface-variant">{item.desc}</p>
              </div>
            </div>
          ))}
        </div>

        <h4 className="font-label-caps text-label-caps text-on-surface-variant mb-3">技术信息</h4>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-body-sm">
          <div className="p-3 rounded-lg bg-surface-container-low">
            <p className="text-on-surface-variant text-[11px] mb-0.5">版本</p>
            <p className="text-on-surface font-medium font-mono text-[13px]">{plugin.version}</p>
          </div>
          <div className="p-3 rounded-lg bg-surface-container-low">
            <p className="text-on-surface-variant text-[11px] mb-0.5">Scope</p>
            <p className="text-on-surface font-medium font-mono text-[11px] truncate">{plugin.scope}</p>
          </div>
          <div className="p-3 rounded-lg bg-surface-container-low">
            <p className="text-on-surface-variant text-[11px] mb-0.5">类型</p>
            <p className="text-on-surface font-medium">{plugin.category === "widget" ? "Widget" : plugin.category === "script" ? "Script" : plugin.category === "tool" ? "工具" : "美化"}</p>
          </div>
          <div className="p-3 rounded-lg bg-surface-container-low">
            <p className="text-on-surface-variant text-[11px] mb-0.5">评分</p>
            <p className="text-on-surface font-medium flex items-center gap-1">
              <span className="material-symbols-outlined text-amber-500 text-[14px] fill">star</span>
              {plugin.rating}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
