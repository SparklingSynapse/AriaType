import { Settings } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

interface SettingsButtonProps {}

export function SettingsButton({}: SettingsButtonProps) {
  const handleClick = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await invoke("show_main_window");
    } catch (err) {
      console.error("Failed to show main window:", err);
    }
  };

  return (
    <button
      onClick={handleClick}
      className="ml-2 flex h-4 w-4 items-center justify-center rounded-full bg-zinc-300 text-primary dark:bg-zinc-700 dark:text-white opacity-60 hover:opacity-100 transition-opacity duration-100"
    >
      <Settings className="h-2.5 w-2.5" />
    </button>
  );
}
