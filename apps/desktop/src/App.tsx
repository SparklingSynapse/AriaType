import { memo } from "react";
import { Routes, Route } from "react-router-dom";
import { Toaster } from "sonner";
import { HomeLayout } from "./components/Home/HomeLayout";
import { Dashboard } from "./components/Home/Dashboard";
import { HistoryPage } from "./components/Home/HistoryPage";
import { GeneralSettings } from "./components/Home/GeneralSettings";
import { HotkeySettings } from "./components/Home/HotkeySettings";
import { ModelSettings } from "./components/Home/ModelSettings";
import { CloudService } from "./components/Home/CloudService";
import { PermissionSettings } from "./components/Home/PermissionSettings";
import { About } from "./components/Home/About";
import { LogViewer } from "./components/Home/LogViewer";
import { PolishTemplatesPage } from "./components/Home/PolishTemplatesPage";

function App() {
  return (
    <>
      {/* Toaster at root level - never re-mounts, stable across all route changes */}
      <Toaster
        position="top-center"
        expand={false}
        richColors={false}
        closeButton={false}
        duration={2000}
        offset={16}
        style={{
          left: "calc(50% + 112px)",
        }}
        toastOptions={{
          unstyled: true,
          classNames: {
            toast: "toast-base",
            title: "toast-title",
            description: "toast-description",
            success: "toast-success",
            error: "toast-error",
            warning: "toast-warning",
            info: "toast-info",
          },
        }}
      />
      {/* Drag region for overlay title bar - must be at root level with highest z-index */}
      <div
        className="fixed top-0 left-0 right-0 h-7 z-[9999]"
        data-tauri-drag-region
      />
      <Routes>
        <Route path="/" element={<HomeLayout />}>
          <Route index element={<Dashboard />} />
          <Route path="history" element={<HistoryPage />} />
          <Route path="settings" element={<GeneralSettings />} />
          <Route path="hotkey" element={<HotkeySettings />} />
          <Route path="private-ai" element={<ModelSettings />} />
          <Route path="cloud" element={<CloudService />} />
          <Route path="polish-templates" element={<PolishTemplatesPage />} />
          <Route path="permission" element={<PermissionSettings />} />
          <Route path="logs" element={<LogViewer />} />
          <Route path="about" element={<About />} />
        </Route>
      </Routes>
    </>
  );
}

export default memo(App);
