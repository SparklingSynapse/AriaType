import { Routes, Route } from "react-router-dom";
import { HomeLayout } from "./components/Home/HomeLayout";
import { GeneralSettings } from "./components/Home/GeneralSettings";
import { HotkeySettings } from "./components/Home/HotkeySettings";
import { ModelSettings } from "./components/Home/ModelSettings";
import { PermissionSettings } from "./components/Home/PermissionSettings";
import { About } from "./components/Home/About";
import { LogViewer } from "./components/Home/LogViewer";

function App() {
  return (
    <Routes>
      <Route path="/" element={<HomeLayout />}>
        <Route index element={<GeneralSettings />} />
        <Route path="hotkey" element={<HotkeySettings />} />
        <Route path="private-ai" element={<ModelSettings />} />
        <Route path="permission" element={<PermissionSettings />} />
        <Route path="logs" element={<LogViewer />} />
        <Route path="about" element={<About />} />
      </Route>
    </Routes>
  );
}

export default App;
