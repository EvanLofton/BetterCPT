export interface SliderProps {
  label: string;
  icon?: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  suffix?: string;
  minLabel?: string;
  maxLabel?: string;
  /** 自定义当前值显示，不传则显示 `${value}${suffix}` */
  formatValue?: (value: number) => string;
  /** 拖动过程中实时回调（本地状态更新，不发 IPC） */
  onChange: (value: number) => void;
  /** 松手时回调（发 IPC，可选） */
  onChangeEnd?: (value: number) => void;
}

export default function Slider({
  label,
  icon,
  value,
  min,
  max,
  step = 1,
  suffix = "",
  minLabel,
  maxLabel,
  formatValue,
  onChange,
  onChangeEnd,
}: SliderProps) {
  const pct = ((value - min) / (max - min)) * 100;
  const display = formatValue ? formatValue(value) : `${value}${suffix}`;

  return (
    <div className="bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-6 shadow-sm">
      <div className="flex items-center gap-3 mb-4">
        {icon && (
          <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center shrink-0">
            <span className="material-symbols-outlined text-primary text-[20px]">{icon}</span>
          </div>
        )}
        <div className="flex-1">
          <h3 className="text-body-sm font-semibold text-on-surface">{label}</h3>
        </div>
        <span className="text-body-sm font-semibold text-primary tabular-nums min-w-[2.5rem] text-right">
          {display}
        </span>
      </div>
      <div className="relative h-8 flex items-center">
        <div className="absolute inset-x-0 h-1.5 rounded-full bg-surface-container-high overflow-hidden">
          <div
            className="h-full rounded-full bg-primary transition-all duration-150 ease-out"
            style={{ width: `${pct}%` }}
          />
        </div>
        <input
          type="range"
          className="w-full h-full opacity-0 cursor-pointer relative z-10"
          value={value}
          min={min}
          max={max}
          step={step}
          onChange={(e) => onChange(parseInt(e.target.value))}
          onMouseUp={(e) => onChangeEnd?.(parseInt((e.target as HTMLInputElement).value))}
        />
      </div>
      {(minLabel || maxLabel) && (
        <div className="flex justify-between mt-2">
          <span className="text-[10px] text-on-surface-variant/50 font-mono">{minLabel || min + suffix}</span>
          <span className="text-[10px] text-on-surface-variant/50 font-mono">{maxLabel || max + suffix}</span>
        </div>
      )}
    </div>
  );
}
