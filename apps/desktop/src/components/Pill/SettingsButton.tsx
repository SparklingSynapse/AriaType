import { Settings } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { logger } from "@/lib/logger";

interface SettingsButtonProps {}

export function SettingsButton({}: SettingsButtonProps) {
  const handleClick = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await invoke("show_main_window");
    } catch (err) {
      logger.error("failed_to_show_main_window", { error: String(err) });
    }
  };

  return (
    <button
      onClick={handleClick}
      className="ml-2 flex h-4 w-4 items-center justify-center rounded-full bg-zinc-700 text-zinc-400 opacity-60 hover:opacity-100 transition-opacity duration-100"
    >
      <Settings className="h-2.5 w-2.5" />
    </button>
  );
}
