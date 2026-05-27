export interface ToggleProps {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** Material icon name, auto-switches based on checked state if iconOff provided */
  icon?: string;
  iconOff?: string;
}

export default function Toggle({
  label,
  description,
  checked,
  onChange,
  icon,
  iconOff,
}: ToggleProps) {
  const currentIcon = iconOff && !checked ? iconOff : icon;

  return (
    <div className="bg-surface-container-lowest/70 backdrop-blur-xl rounded-xl border border-outline-variant/30 p-6 shadow-sm">
      <div className="flex items-center gap-3">
        {currentIcon && (
          <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center shrink-0">
            <span className="material-symbols-outlined text-primary text-[20px]">{currentIcon}</span>
          </div>
        )}
        <div className="flex-1">
          <h3 className="text-body-sm font-semibold text-on-surface">{label}</h3>
          {description && (
            <p className="text-[11px] text-on-surface-variant">{description}</p>
          )}
        </div>
        <button
          role="switch"
          aria-checked={checked}
          onClick={() => onChange(!checked)}
          className={`relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-all duration-300 focus-visible:outline-2 focus-visible:outline-primary ${
            checked
              ? "bg-primary shadow-[0_0_8px_rgb(var(--color-primary-rgb)/0.4)]"
              : "bg-surface-container-high"
          }`}
        >
          <span
            className={`inline-block h-5 w-5 rounded-full bg-white shadow-sm transition-all duration-300 ${
              checked ? "translate-x-[22px]" : "translate-x-[2px]"
            }`}
          />
        </button>
      </div>
    </div>
  );
}
