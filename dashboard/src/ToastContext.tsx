import { createContext, useContext, useState, useCallback, type ReactNode } from "react";

interface ToastItem {
  id: number;
  type: "info" | "success" | "error";
  message: string;
}

interface ToastContextType {
  addToast: (type: ToastItem["type"], message: string) => void;
}

const ToastContext = createContext<ToastContextType>({ addToast: () => {} });

let toastId = 0;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const addToast = useCallback((type: ToastItem["type"], message: string) => {
    const id = ++toastId;
    setToasts((prev) => [...prev, { id, type, message }]);
    setTimeout(() => setToasts((prev) => prev.filter((t) => t.id !== id)), 4000);
  }, []);

  const toastIcon = (type: string) => {
    switch (type) {
      case "success": return "check_circle";
      case "error": return "error";
      default: return "info";
    }
  };
  const toastColor = (type: string) => {
    switch (type) {
      case "success": return "border-l-status-success";
      case "error": return "border-l-status-error";
      default: return "border-l-status-warning";
    }
  };

  return (
    <ToastContext.Provider value={{ addToast }}>
      {children}
      <div className="fixed bottom-6 right-6 z-[100] flex flex-col gap-2 pointer-events-none">
        {toasts.map((t) => (
          <div
            key={t.id}
            className={`pointer-events-auto bg-surface-container-lowest/90 backdrop-blur-xl border shadow-lg rounded-xl overflow-hidden flex items-stretch w-80 animate-slide-in-right border-l-4 ${toastColor(t.type)}`}
          >
            <div className="p-4 flex gap-3 w-full items-center">
              <span className={`material-symbols-outlined mt-0.5 ${t.type === "success" ? "text-status-success" : t.type === "error" ? "text-status-error" : "text-status-warning"}`}>
                {toastIcon(t.type)}
              </span>
              <div className="flex-1">
                <h4 className="text-body-sm font-semibold text-on-surface">{t.message}</h4>
              </div>
              <button
                className="text-on-surface-variant hover:text-on-surface self-start"
                onClick={() => setToasts((prev) => prev.filter((x) => x.id !== t.id))}
              >
                <span className="material-symbols-outlined text-[18px]">close</span>
              </button>
            </div>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  return useContext(ToastContext);
}
