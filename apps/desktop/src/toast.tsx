import React from "react";
import ReactDOM from "react-dom/client";
import { ToastWindow } from "./components/Toast/ToastWindow";
import "./index.css";

// Toast window has no ThemeProvider — follow system preference directly
const applySystemTheme = () => {
  document.documentElement.classList.toggle(
    "dark",
    window.matchMedia("(prefers-color-scheme: dark)").matches
  );
};
applySystemTheme();
window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applySystemTheme);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ToastWindow />
  </React.StrictMode>
);
