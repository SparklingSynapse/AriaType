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
import { ChangelogPage } from "./components/Home/ChangelogPage";
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
        offset={16}
        icons={{
          success: (
            <span
              className="flex items-center justify-center w-[22px] h-[22px] rounded-full bg-green-500 shrink-0"
              style={{
                backgroundSize: "12px",
                backgroundRepeat: "no-repeat",
                backgroundPosition: "center",
                backgroundImage:
                  "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='white' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M20 6 9 17l-5-5'/%3E%3C/svg%3E\")",
              }}
            />
          ),
          error: (
            <span
              className="flex items-center justify-center w-[22px] h-[22px] rounded-full bg-red-500 shrink-0"
              style={{
                backgroundSize: "12px",
                backgroundRepeat: "no-repeat",
                backgroundPosition: "center",
                backgroundImage:
                  "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='white' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M18 6 6 18'/%3E%3Cpath d='m6 6 12 12'/%3E%3C/svg%3E\")",
              }}
            />
          ),
          warning: (
            <span
              className="flex items-center justify-center w-[22px] h-[22px] rounded-full bg-amber-500 shrink-0"
              style={{
                backgroundSize: "12px",
                backgroundRepeat: "no-repeat",
                backgroundPosition: "center",
                backgroundImage:
                  "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='white' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 2.29 21h16.14a2 2 0 0 0 1.72-3Z'/%3E%3Cpath d='M12 9v4'/%3E%3Cpath d='M12 17h.01'/%3E%3C/svg%3E\")",
              }}
            />
          ),
          info: (
            <span
              className="flex items-center justify-center w-[22px] h-[22px] rounded-full bg-blue-500 shrink-0"
              style={{
                backgroundSize: "12px",
                backgroundRepeat: "no-repeat",
                backgroundPosition: "center",
                backgroundImage:
                  "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='white' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='12' cy='12' r='10'/%3E%3Cpath d='M12 16v-4'/%3E%3Cpath d='M12 8h.01'/%3E%3C/svg%3E\")",
              }}
            />
          ),
        }}
        toastOptions={{
          unstyled: true,
          style: {
            left: "50%",
            transform: "translateX(-50%)",
          },
          classNames: {
            toast: "toast-base",
            title: "toast-title",
            description: "toast-description",
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
          <Route path="changelog" element={<ChangelogPage />} />
          <Route path="about" element={<About />} />
        </Route>
      </Routes>
    </>
  );
}

export default memo(App);
