import { Routes, Route } from "react-router-dom";
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
  );
}

export default App;
