import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter } from "react-router-dom";
import { ToastProvider } from "./ToastContext";
import { ThemeProvider } from "./ThemeContext";
import App from "./App";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ThemeProvider>
      <ToastProvider>
        <HashRouter>
          <App />
        </HashRouter>
      </ToastProvider>
    </ThemeProvider>
  </React.StrictMode>
);

// 移除 loading 页（React 已挂载）
if (typeof (window as any).__bccRemoveLoading === "function") {
  (window as any).__bccRemoveLoading();
}
