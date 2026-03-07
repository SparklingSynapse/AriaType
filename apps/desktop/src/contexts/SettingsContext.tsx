import { createContext, useContext, type ReactNode } from "react";
import { useSettings } from "@/hooks/useSettings";
import type { AppSettings } from "@/lib/tauri";

interface SettingsContextType {
  settings: AppSettings | null;
  loading: boolean;
  updateSetting: (key: string, value: unknown) => Promise<void>;
}

const SettingsContext = createContext<SettingsContextType | undefined>(undefined);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const { settings, loading, updateSetting } = useSettings();

  return (
    <SettingsContext.Provider value={{ settings, loading, updateSetting }}>
      {children}
    </SettingsContext.Provider>
  );
}

export function useSettingsContext() {
  const context = useContext(SettingsContext);
  if (context === undefined) {
    throw new Error("useSettingsContext must be used within a SettingsProvider");
  }
  return context;
}
