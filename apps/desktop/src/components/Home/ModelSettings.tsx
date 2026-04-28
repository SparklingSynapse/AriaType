import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useEventListeners } from "@/hooks/useEventListeners";
import {
  settingsCommands,
  modelCommands,
  events,
  type ModelInfo,
  type PolishModelInfo,
} from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { SettingsPageLayout } from "./SettingsPageLayout";
import { confirm } from "@/components/ui/confirm";
import { VoiceInputSection } from "./model/VoiceInputSection";
import { PolishSection } from "./model/PolishSection";
import { PerformanceSection } from "./model/PerformanceSection";

import { cn } from "@/lib/utils";

export function ModelSettings() {
  const { t } = useTranslation();
  const { settings } = useSettingsContext();
  const [activeTab, setActiveTab] = useState<"voice" | "polish" | "performance">("voice");

  const [models, setModels] = useState<ModelInfo[]>([]);
  const [downloading, setDownloading] = useState<Set<string>>(new Set());
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({});
  
  const [polishModels, setPolishModels] = useState<PolishModelInfo[]>([]);
  const [selectedPolishModel, setSelectedPolishModel] = useState<string>("");
  const [polishDownloadingId, setPolishDownloadingId] = useState<string | null>(null);
  const [polishProgress, setPolishProgress] = useState<number | null>(null);

  const loadModels = useCallback(async () => {
    try {
      const list = await modelCommands.getModels();
      setModels(list);
    } catch (err) {
      logger.error("failed_to_load_models", { error: String(err) });
    }
  }, []);

  const loadPolishModels = useCallback(async () => {
    try {
      const models = await modelCommands.getPolishModels();
      setPolishModels(models);
    } catch (err) {
      logger.error("failed_to_load_polish_models", { error: String(err) });
    }
  }, []);

  useEffect(() => {
    if (!settings) return;

    loadModels();

    loadPolishModels().then(() => {
      if (settings.polish_model) setSelectedPolishModel(settings.polish_model);
    });
  }, [settings === null, loadModels, loadPolishModels]);

  useEventListeners(async () => {
    return [
      await events.onModelDownloadComplete((model) => {
        setDownloading((prev) => {
          const next = new Set(prev);
          next.delete(model);
          return next;
        });
        setDownloadProgress((prev) => {
          const next = { ...prev };
          delete next[model];
          return next;
        });
        loadModels();
      }),
      await events.onModelDownloadCancelled((model) => {
        setDownloading((prev) => {
          const next = new Set(prev);
          next.delete(model);
          return next;
        });
        setDownloadProgress((prev) => {
          const next = { ...prev };
          delete next[model];
          return next;
        });
      }),
      await events.onModelDownloadProgress((data) => {
        setDownloadProgress((prev) => ({
          ...prev,
          [data.model]: data.progress,
        }));
      }),
      await events.onModelDeleted(() => loadModels()),
      await events.onPolishModelDownloadProgress((data) => {
        if (data.model_id === polishDownloadingId) {
          setPolishProgress(data.progress);
        }
      }),
      await events.onPolishModelDownloadComplete((modelId) => {
        if (modelId === polishDownloadingId) {
          setPolishDownloadingId(null);
          setPolishProgress(null);
          loadPolishModels();
          setSelectedPolishModel((prev) => {
            if (!prev) {
              settingsCommands
                .updateSettings("polish_model", modelId)
                .catch((err: unknown) => logger.error("failed_to_update_polish_model", { error: String(err) }));
              return modelId;
            }
            return prev;
          });
        }
      }),
      await events.onPolishModelDownloadCancelled((modelId) => {
        if (modelId === polishDownloadingId) {
          setPolishDownloadingId(null);
          setPolishProgress(null);
        }
      }),
      await events.onPolishModelDeleted(() => loadPolishModels()),
    ];
  }, [loadModels, loadPolishModels, polishDownloadingId]);

  const handleDownload = async (modelName: string) => {
    if (downloading.has(modelName)) return;
    setDownloading((prev) => new Set(prev).add(modelName));
    try {
      await modelCommands.downloadModel(modelName);
    } catch (err) {
      logger.error("failed_to_download_model", { error: String(err) });
      setDownloading((prev) => {
        const next = new Set(prev);
        next.delete(modelName);
        return next;
      });
      setDownloadProgress((prev) => {
        const next = { ...prev };
        delete next[modelName];
        return next;
      });
    }
  };

  const handleCancel = async (modelName: string) => {
    try {
      await modelCommands.cancelDownload(modelName);
    } catch (err) {
      logger.error("failed_to_cancel_download", { error: String(err) });
    }
  };

  const handleDelete = async (modelName: string) => {
    const confirmed = await confirm({
      title: "Delete Model",
      description: `Are you sure you want to delete the "${modelName}" model? This action cannot be undone.`,
      confirmText: "Delete",
      cancelText: "Cancel",
      variant: "danger",
    });
    if (!confirmed) return;

    try {
      await modelCommands.deleteModel(modelName);
      await loadModels();
    } catch (err) {
      logger.error("failed_to_delete_model", { error: String(err) });
    }
  };

  const handlePolishDownload = async (modelId: string) => {
    setPolishDownloadingId(modelId);
    setPolishProgress(0);
    try {
      await modelCommands.downloadPolishModelById(modelId);
    } catch (err) {
      logger.error("failed_to_download_polish_model", { error: String(err) });
      setPolishDownloadingId(null);
      setPolishProgress(null);
    }
  };

  const handlePolishCancel = async (modelId: string) => {
    try {
      await modelCommands.cancelPolishDownload(modelId);
    } catch (err) {
      logger.error("failed_to_cancel_polish_download", { error: String(err) });
    }
  };

  const handlePolishDelete = async (modelId: string) => {
    const confirmed = await confirm({
      title: "Delete Polish Model",
      description: `Are you sure you want to delete this polish model? This action cannot be undone.`,
      confirmText: "Delete",
      cancelText: "Cancel",
      variant: "danger",
    });
    if (!confirmed) return;

    try {
      await modelCommands.deletePolishModelById(modelId);
      setPolishModels((prev) =>
        prev.map((m) => (m.id === modelId ? { ...m, downloaded: false } : m)),
      );
      if (selectedPolishModel === modelId) {
        setSelectedPolishModel("");
      }
    } catch (err) {
      logger.error("failed_to_delete_polish_model", { error: String(err) });
    }
  };

  if (!settings) return null;

  return (
    <SettingsPageLayout
      title={t("model.title")}
      description={t("model.description")}
      testId="model-page"
    >
      <div className="inline-flex h-11 items-center justify-center rounded-full bg-secondary p-1.5 text-muted-foreground ">
        <button
          onClick={() => setActiveTab("voice")}
          className={cn(
            "inline-flex h-full items-center justify-center whitespace-nowrap rounded-full px-5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
            activeTab === "voice"
              ? "bg-background text-foreground shadow-sm"
              : "hover:text-foreground hover:bg-background/40"
          )}
        >
          {t("model.tabs.voice", "Voice Input")}
        </button>
        <button
          onClick={() => setActiveTab("polish")}
          className={cn(
            "inline-flex h-full items-center justify-center whitespace-nowrap rounded-full px-5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
            activeTab === "polish"
              ? "bg-background text-foreground shadow-sm"
              : "hover:text-foreground hover:bg-background/40"
          )}
        >
          {t("model.tabs.polish", "Polish")}
        </button>
        <button
          onClick={() => setActiveTab("performance")}
          className={cn(
            "inline-flex h-full items-center justify-center whitespace-nowrap rounded-full px-5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
            activeTab === "performance"
              ? "bg-background text-foreground shadow-sm"
              : "hover:text-foreground hover:bg-background/40"
          )}
        >
          {t("model.tabs.performance", "Performance")}
        </button>
      </div>

      <div >
        {activeTab === "voice" && (
          <VoiceInputSection
            models={models}
            downloading={downloading}
            downloadProgress={downloadProgress}
            onDownload={handleDownload}
            onCancel={handleCancel}
            onDelete={handleDelete}
          />
        )}
        
        {activeTab === "polish" && (
          <PolishSection
            polishModels={polishModels}
            selectedPolishModel={selectedPolishModel}
            setSelectedPolishModel={setSelectedPolishModel}
            polishDownloadingId={polishDownloadingId}
            polishProgress={polishProgress}
            onDownload={handlePolishDownload}
            onCancel={handlePolishCancel}
            onDelete={handlePolishDelete}
          />
        )}

        {activeTab === "performance" && (
          <PerformanceSection />
        )}
      </div>
    </SettingsPageLayout>
  );
}
