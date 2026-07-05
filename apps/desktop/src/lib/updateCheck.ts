import { getVersion } from "@tauri-apps/api/app";
import { updateCommands, type AppUpdateInfo } from "@/lib/tauri";

export interface AppUpdateCheckResult {
  update: AppUpdateInfo | null;
  currentVersion: string;
  autoInstallAvailable: boolean;
  source: "updater";
}

export async function checkAppUpdate(): Promise<AppUpdateCheckResult> {
  const appVersion = await getVersion();
  const update = await updateCommands.check();
  return {
    update,
    currentVersion: update?.currentVersion || appVersion,
    autoInstallAvailable: Boolean(update),
    source: "updater",
  };
}
