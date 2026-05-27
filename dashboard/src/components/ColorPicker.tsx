import { useId } from "react";

export interface ColorPickerProps {
  label: string;
  description?: string;
  /** 当前 rgba 颜色，如 "rgba(20,20,20,0.7)" */
  value: string;
  /** 预设颜色，每个元素为 "r, g, b" 的 rgb 字符串 */
  presets: { rgb: string }[];
  /** 颜色变化回调，传入完整 rgba 字符串（自动保持当前 opacity） */
  onChange: (rgbaColor: string) => void;
  /** 提取当前颜色的 rgb 部分 */
  rgbFromRgba: (rgba: string) => string;
  /** rgb 字符串转 hex，如 "20,20,20" → "#141414" */
  rgbToHex: (rgb: string) => string;
  /** hex 转 rgb 回调 */
  onHexChange: (rgb: string) => void;
}

export default function ColorPicker({
  label,
  description,
  value,
  presets,
  onChange,
  rgbFromRgba,
  rgbToHex,
  onHexChange,
}: ColorPickerProps) {
  const id = useId();
  const currentRgb = rgbFromRgba(value);

  return (
    <div className="bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-6 shadow-sm">
      <div className="flex items-center gap-3 mb-5">
        <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center shrink-0">
          <span className="material-symbols-outlined text-primary text-[20px]">palette</span>
        </div>
        <div>
          <h3 className="text-body-sm font-semibold text-on-surface">{label}</h3>
          {description && <p className="text-[11px] text-on-surface-variant">{description}</p>}
        </div>
        <div
          className="ml-auto w-9 h-9 rounded-lg border-2 border-outline-variant shadow-sm transition-colors duration-300"
          style={{ background: value }}
        />
      </div>
      <div className="flex flex-wrap gap-2.5">
        {presets.map((c) => (
          <button
            key={c.rgb}
            onClick={() => onChange(c.rgb)}
            className={`w-7 h-7 rounded-full border-2 transition-all duration-200 hover:scale-110 ${
              currentRgb === c.rgb
                ? "border-primary scale-110 shadow-[0_0_0_2px_rgb(var(--color-primary-rgb)/0.25)]"
                : "border-outline-variant/40 hover:border-primary/60"
            }`}
            style={{ background: `rgb(${c.rgb})` }}
          />
        ))}
        <div className="relative">
          <input
            type="color"
            value={rgbToHex(currentRgb)}
            onChange={(e) => {
              const h = e.target.value;
              onHexChange(`${parseInt(h.slice(1, 3), 16)}, ${parseInt(h.slice(3, 5), 16)}, ${parseInt(h.slice(5, 7), 16)}`);
            }}
            className="sr-only"
            id={id}
          />
          <label
            htmlFor={id}
            className="w-7 h-7 rounded-full border-2 border-dashed border-outline-variant/40 hover:border-primary/60 flex items-center justify-center cursor-pointer transition-all duration-200 hover:scale-110"
          >
            <span className="material-symbols-outlined text-[14px] text-on-surface-variant">add</span>
          </label>
        </div>
      </div>
    </div>
  );
}
