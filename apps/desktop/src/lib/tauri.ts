import { invoke } from "@tauri-apps/api/core";
import { listen, emit } from "@tauri-apps/api/event";
import { logger } from "./logger";

/** Wrapped invoke that logs command name, params (debug), timing (debug), and errors (error) */
function invokeWithLogging<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const start = performance.now();
  logger.debug(`ipc_request`, { command, args });

  return invoke<T>(command, args)
    .then((result) => {
      const duration_ms = Math.round(performance.now() - start);
      logger.debug(`ipc_response`, { command, duration_ms });
      return result;
    })
    .catch((error: unknown) => {
      const duration_ms = Math.round(performance.now() - start);
      logger.error(`ipc_error`, { command, duration_ms, error: String(error) });
      throw error;
    });
}

export interface Position {
  x: number;
  y: number;
}

export interface RecordingState {
  is_recording: boolean;
  is_transcribing: boolean;
  audio_level: number;
  output_path: string | null;
}

export interface RecordingStateEvent {
  status: string;
  task_id: number;
}

export interface TranscriptionCompleteEvent {
  text: string;
  task_id: number;
}

export interface CloudProviderConfig {
  enabled: boolean;
  provider_type: string;
  api_key: string;
  base_url: string;
  model: string;
  enable_thinking: boolean;
}

export interface CloudSttConfig {
  enabled: boolean;
  provider_type: string;
  api_key: string;
  app_id: string;
  base_url: string;
  model: string;
  language: string;
}

export interface ProviderFieldSchema {
  name: string;
  key: string;
  required: boolean;
  default_value: string;
  example: string;
  secret: boolean;
}

export interface ProviderSchema {
  id: string;
  name: string;
  fields: ProviderFieldSchema[];
}

export interface CloudProviderSchemas {
  stt: ProviderSchema[];
  polish: ProviderSchema[];
}

export interface AppSettings {
  hotkey: string;
  recording_mode: "hold" | "toggle";
  model: string;
  stt_engine: string;
  pill_position: string;
  pill_indicator_mode: string;
  auto_start: boolean;
  gpu_acceleration: boolean;
  language: string;
  stt_engine_language: string;
  beep_on_record: boolean;
  audio_device: string;
  polish_enabled: boolean;
  polish_system_prompt: string;
  polish_model: string;
  theme_mode: "system" | "light" | "dark";
  stt_engine_initial_prompt: string;
  model_resident: boolean;
  idle_unload_minutes: number;
  denoise_mode: string;
  stt_engine_work_domain: string;
  stt_engine_work_domain_prompt: string;
  stt_engine_work_subdomain: string;
  stt_engine_user_glossary: string;
  analytics_opt_in: boolean;
  cloud_stt_enabled: boolean;
  active_cloud_stt_provider: string;
  cloud_stt_configs: Record<string, CloudSttConfig>;
  cloud_polish_enabled: boolean;
  active_cloud_polish_provider: string;
  cloud_polish_configs: Record<string, CloudProviderConfig>;
  vad_enabled: boolean;
  stay_in_tray: boolean;
  polish_selected_template: string;
  polish_custom_templates: CustomPolishTemplate[];
}

export interface ModelInfo {
  name: string;
  display_name: string;
  size_mb: number;
  url: string;
  downloaded: boolean;
  speed_score: number;
  accuracy_score: number;
}

export interface PolishModelInfo {
  id: string;
  name: string;
  size: string;
  downloaded: boolean;
}

export interface RecommendedModel {
  engine_type: string;
  model_name: string;
  display_name: string;
  size_mb: number;
  speed_score: number;
  accuracy_score: number;
  downloaded: boolean;
}

export const windowCommands = {
  showMain: () => invokeWithLogging("show_main_window"),
  hideMain: () => invokeWithLogging("hide_main_window"),
  showPill: () => invokeWithLogging("show_pill_window"),
  hidePill: () => invokeWithLogging("hide_pill_window"),
  updatePillPosition: (x: number, y: number) =>
    invokeWithLogging("update_pill_position", { x, y }),
  getPillPosition: () => invokeWithLogging<Position | null>("get_pill_position"),
};

export const audioCommands = {
  startRecording: () => invokeWithLogging<string>("start_recording"),
  stopRecording: () => invokeWithLogging<string | null>("stop_recording"),
  cancelRecording: () => invokeWithLogging<void>("cancel_recording"),
  getAudioLevel: () => invokeWithLogging<number>("get_audio_level"),
  getRecordingState: () => invokeWithLogging<RecordingState>("get_recording_state"),
};

export const textCommands = {
  insertText: (text: string) => invokeWithLogging("insert_text", { text }),
  copyToClipboard: (text: string) => invokeWithLogging("copy_to_clipboard", { text }),
  restoreClipboard: (text: string) => invokeWithLogging("restore_clipboard", { text }),
};

export const settingsCommands = {
  getSettings: () => invokeWithLogging<AppSettings>("get_settings"),
  updateSettings: (key: string, value: unknown) =>
    invokeWithLogging("update_settings", { key, value }),
  getGlossaryContent: (subdomain: string) =>
    invokeWithLogging<string>("get_glossary_content", { subdomain }),
  getAvailableSubdomains: (domain: string) =>
    invokeWithLogging<string[]>("get_available_subdomains", { domain }),
  getCloudProviderSchemas: () =>
    invokeWithLogging<CloudProviderSchemas>("get_cloud_provider_schemas"),
};

export const hotkeyCommands = {
  startRecording: () => invokeWithLogging<void>("start_hotkey_recording"),
  stopRecording: () => invokeWithLogging<string | null>("stop_hotkey_recording"),
  cancelRecording: () => invokeWithLogging<void>("cancel_hotkey_recording"),
  peekRecording: () => invokeWithLogging<string | null>("peek_hotkey_recording"),
};

export const systemCommands = {
  getAudioDevices: () => invokeWithLogging<string[]>("get_audio_devices"),
  checkPermission: (kind: "accessibility" | "input_monitoring" | "microphone") =>
    invokeWithLogging<string>("check_permission", { kind }),
  applyPermission: (kind: "accessibility" | "input_monitoring" | "microphone") =>
    invokeWithLogging<void>("apply_permission", { kind }),
  getLogContent: (lines: number) => invokeWithLogging<string>("get_log_content", { lines }),
  openLogFolder: () => invokeWithLogging("open_log_folder"),
  getPlatform: () => invokeWithLogging<"macos" | "windows" | "linux" | "unknown">("get_platform"),
};

export const transcribeCommands = {
  transcribeAudio: (audioPath: string) =>
    invokeWithLogging<string>("transcribe_audio", { audioPath }),
  getSTTEngines: () => invokeWithLogging<string[]>("get_stt_engines"),
};

export const modelCommands = {
  getModels: () => invokeWithLogging<ModelInfo[]>("get_models"),
  isModelDownloaded: (modelName: string) =>
    invokeWithLogging<boolean>("is_model_downloaded", { modelName }),
  downloadModel: (modelName: string) =>
    invokeWithLogging<void>("download_model", { modelName }),
  cancelDownload: (modelName: string) =>
    invokeWithLogging<void>("cancel_download", { modelName }),
  deleteModel: (modelName: string) =>
    invokeWithLogging<void>("delete_model", { modelName }),
  recommendModelsByLanguage: (language: string) =>
    invokeWithLogging<RecommendedModel[]>("recommend_models_by_language", { language }),
  getPolishModels: () =>
    invokeWithLogging<PolishModelInfo[]>("get_polish_models"),
  getCurrentPolishModel: () =>
    invokeWithLogging<string>("get_current_polish_model"),
  isPolishModelDownloaded: () =>
    invokeWithLogging<boolean>("is_polish_model_downloaded"),
  isPolishModelDownloadedForModel: (modelId: string) =>
    invokeWithLogging<boolean>("is_polish_model_downloaded_for_model", { modelId }),
  downloadPolishModel: () =>
    invokeWithLogging<void>("download_polish_model"),
  downloadPolishModelById: (modelId: string) =>
    invokeWithLogging<void>("download_polish_model_by_id", { modelId }),
  cancelPolishDownload: (modelId: string) =>
    invokeWithLogging<void>("cancel_polish_download", { modelId }),
  deletePolishModel: () =>
    invokeWithLogging<void>("delete_polish_model"),
  deletePolishModelById: (modelId: string) =>
    invokeWithLogging<void>("delete_polish_model_by_id", { modelId }),
  getPolishTemplates: () =>
    invokeWithLogging<PolishTemplate[]>("get_polish_templates"),
  getPolishTemplatePrompt: (templateId: string) =>
    invokeWithLogging<string>("get_polish_template_prompt", { templateId }),
  createPolishCustomTemplate: (name: string, systemPrompt: string) =>
    invokeWithLogging<CustomPolishTemplate>("create_polish_custom_template", { name, systemPrompt }),
  updatePolishCustomTemplate: (id: string, name: string, systemPrompt: string) =>
    invokeWithLogging<void>("update_polish_custom_template", { id, name, systemPrompt }),
  deletePolishCustomTemplate: (id: string) =>
    invokeWithLogging<void>("delete_polish_custom_template", { id }),
  selectPolishTemplate: (templateId: string) =>
    invokeWithLogging<void>("select_polish_template", { templateId }),
  getPolishSelectedTemplate: () =>
    invokeWithLogging<string>("get_polish_selected_template"),
  getPolishCustomTemplates: () =>
    invokeWithLogging<CustomPolishTemplate[]>("get_polish_custom_templates"),
};

export interface PolishTemplate {
  id: string;
  name: string;
  description: string;
}

export interface CustomPolishTemplate {
  id: string;
  name: string;
  system_prompt: string;
}

export interface TranscriptionEntry {
  id: string;
  created_at: number;
  raw_text: string;
  final_text: string;
  stt_engine: string;
  stt_model: string | null;
  language: string | null;
  audio_duration_ms: number | null;
  stt_duration_ms: number | null;
  polish_duration_ms: number | null;
  total_duration_ms: number | null;
  polish_applied: boolean;
  polish_engine: string | null;
  is_cloud: boolean;
  /** Path to the saved audio file (for retry functionality). */
  audio_path: string | null;
  /** Status of the entry: "success" or "error". */
  status: string;
  /** Error message if transcription failed. */
  error: string | null;
}

export interface DashboardStats {
  total_count: number;
  today_count: number;
  total_chars: number;
  total_output_units: number;
  total_audio_ms: number;
  avg_stt_ms: number | null;
  avg_audio_ms: number | null;
  avg_output_units: number | null;
  local_count: number;
  cloud_count: number;
  polish_count: number;
  active_days: number;
  current_streak_days: number;
  longest_streak_days: number;
  last_7_days_count: number;
  last_7_days_audio_ms: number;
  last_7_days_output_units: number;
}

export interface DailyUsage {
  date: string;
  count: number;
  audio_ms: number;
  output_units: number;
}

export interface EngineUsage {
  engine: string;
  count: number;
  avg_stt_ms: number | null;
}

export interface HistoryFilter {
  search?: string;
  engine?: string;
  /** Filter by status: "success", "error", or undefined for all. */
  status?: string;
  date_from?: number;
  date_to?: number;
  limit?: number;
  offset?: number;
}

export const historyCommands = {
  getHistory: (filter: HistoryFilter) =>
    invokeWithLogging<TranscriptionEntry[]>("get_transcription_history", { filter }),
  getEntry: (id: string) =>
    invokeWithLogging<TranscriptionEntry | null>("get_transcription_entry", { id }),
  getDashboardStats: () =>
    invokeWithLogging<DashboardStats>("get_dashboard_stats"),
  getDailyUsage: (days: number) =>
    invokeWithLogging<DailyUsage[]>("get_daily_usage", { days }),
  getEngineUsage: () =>
    invokeWithLogging<EngineUsage[]>("get_engine_usage"),
  deleteEntry: (id: string) =>
    invokeWithLogging<void>("delete_transcription_entry", { id }),
  clearAll: () =>
    invokeWithLogging<void>("clear_transcription_history"),
  retryTranscription: (id: string) =>
    invokeWithLogging<string>("retry_transcription", { id }),
};

export const events = {
  onRecordingStateChanged: (callback: (event: RecordingStateEvent) => void) => {
    return listen<RecordingStateEvent>("recording-state-changed", (event) => {
      const { task_id, status } = event.payload;
      logger.info("event_received-recording_state_changed", { task_id, status });
      callback(event.payload);
    });
  },
  onAudioLevel: (callback: (level: number) => void) => {
    return listen<number>("audio-level", (event) => {
      const level = event.payload;
      logger.debug("event_received-audio_level", { level });
      callback(event.payload);
    });
  },
  onTranscriptionComplete: (callback: (event: TranscriptionCompleteEvent) => void) => {
    return listen<TranscriptionCompleteEvent>("transcription-complete", (event) => {
      const { task_id, text } = event.payload;
      logger.info("event_received-transcription_complete", { task_id, text_len: text.length });
      callback(event.payload);
    });
  },
  onTranscriptionError: (callback: (error: string) => void) => {
    return listen<string>("transcription-error", (event) => {
      const error = event.payload;
      logger.error("event_received-transcription_error", { error });
      callback(event.payload);
    });
  },
  onModelDownloadProgress: (
    callback: (data: {
      model: string;
      downloaded: number;
      total: number;
      progress: number;
    }) => void
  ) => {
    return listen<{
      model: string;
      downloaded: number;
      total: number;
      progress: number;
    }>("model-download-progress", (event) => {
      const { model, downloaded, total, progress } = event.payload;
      logger.debug("event_received-model_download_progress", { model, downloaded, total, progress });
      callback(event.payload);
    });
  },
  onModelDownloadComplete: (callback: (model: string) => void) => {
    return listen<{ model: string }>("model-download-complete", (event) => {
      const model = event.payload.model;
      logger.info("event_received-model_download_complete", { model });
      callback(event.payload.model);
    });
  },
  onModelDownloadCancelled: (callback: (model: string) => void) => {
    return listen<{ model: string }>("model-download-cancelled", (event) => {
      const model = event.payload.model;
      logger.info("event_received-model_download_cancelled", { model });
      callback(event.payload.model);
    });
  },
  onModelDeleted: (callback: (model: string) => void) => {
    return listen<{ model: string }>("model-deleted", (event) => {
      const model = event.payload.model;
      logger.info("event_received-model_deleted", { model });
      callback(event.payload.model);
    });
  },
  onPolishModelDownloadProgress: (
    callback: (data: { model_id: string; downloaded: number; total: number; progress: number }) => void
  ) => {
    return listen<{ model_id: string; downloaded: number; total: number; progress: number }>(
      "polish-model-download-progress",
      (event) => {
        const { model_id, downloaded, total, progress } = event.payload;
        logger.debug("event_received-polish_model_download_progress", { model_id, downloaded, total, progress });
        callback(event.payload);
      }
    );
  },
  onPolishModelDownloadComplete: (callback: (model_id: string) => void) => {
    return listen<{ model_id: string }>("polish-model-download-complete", (event) => {
      const model_id = event.payload.model_id;
      logger.info("event_received-polish_model_download_complete", { model_id });
      callback(event.payload.model_id);
    });
  },
  onPolishModelDownloadCancelled: (callback: (model_id: string) => void) => {
    return listen<{ model_id: string }>("polish-model-download-cancelled", (event) => {
      const model_id = event.payload.model_id;
      logger.info("event_received-polish_model_download_cancelled", { model_id });
      callback(event.payload.model_id);
    });
  },
  onPolishModelDeleted: (callback: () => void) => {
    return listen("polish-model-deleted", () => {
      logger.info("event_received-polish_model_deleted");
      callback();
    });
  },
  onToastMessage: (callback: (message: string) => void) => {
    return listen<string>("toast-message", (event) => {
      logger.debug("event_received-toast_message", { message: event.payload });
      callback(event.payload);
    });
  },
  onShortcutRegistrationFailed: (callback: (error: string) => void) => {
    return listen<string>("shortcut-registration-failed", (event) => {
      const error = event.payload;
      logger.error("event_received-shortcut_registration_failed", { error });
      callback(event.payload);
    });
  },
  onHotkeyCaptured: (callback: (hotkey: string) => void) => {
    return listen<string>("hotkey-captured", (event) => {
      const hotkey = event.payload;
      logger.info("event_received-hotkey_captured", { hotkey });
      callback(event.payload);
    });
  },
  onSettingsChanged: (callback: (settings: AppSettings) => void) => {
    return listen<AppSettings>("settings-changed", (event) => {
      logger.info("event_received-settings_changed");
      callback(event.payload);
    });
  },
  emit: (event: string, payload?: unknown) => emit(event, payload),
};
