import { invoke } from "@tauri-apps/api/core";
import { listen, emit } from "@tauri-apps/api/event";

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

export interface AppSettings {
  hotkey: string;
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

export const windowCommands = {
  showMain: () => invoke("show_main_window"),
  hideMain: () => invoke("hide_main_window"),
  showPill: () => invoke("show_pill_window"),
  hidePill: () => invoke("hide_pill_window"),
  showToast: (message: string) => invoke("show_toast", { message }),
  hideToast: () => invoke("hide_toast"),
  updatePillPosition: (x: number, y: number) =>
    invoke("update_pill_position", { x, y }),
  getPillPosition: () => invoke<Position | null>("get_pill_position"),
};

export const audioCommands = {
  startRecording: () => invoke<string>("start_recording"),
  stopRecording: () => invoke<string | null>("stop_recording"),
  getAudioLevel: () => invoke<number>("get_audio_level"),
  getRecordingState: () => invoke<RecordingState>("get_recording_state"),
};

export const textCommands = {
  insertText: (text: string) => invoke("insert_text", { text }),
  copyToClipboard: (text: string) => invoke("copy_to_clipboard", { text }),
  restoreClipboard: (text: string) => invoke("restore_clipboard", { text }),
};

export const settingsCommands = {
  getSettings: () => invoke<AppSettings>("get_settings"),
  updateSettings: (key: string, value: unknown) =>
    invoke("update_settings", { key, value }),
  setHotkeyCaptureMode: (enabled: boolean) =>
    invoke("set_hotkey_capture_mode", { enabled }),
  getGlossaryContent: (subdomain: string) =>
    invoke<string>("get_glossary_content", { subdomain }),
  getAvailableSubdomains: (domain: string) =>
    invoke<string[]>("get_available_subdomains", { domain }),
};

export const systemCommands = {
  getAudioDevices: () => invoke<string[]>("get_audio_devices"),
  checkPermission: (kind: "accessibility" | "input_monitoring" | "microphone") =>
    invoke<string>("check_permission", { kind }),
  applyPermission: (kind: "accessibility" | "input_monitoring" | "microphone") =>
    invoke<void>("apply_permission", { kind }),
  getLogContent: (lines: number) => invoke<string>("get_log_content", { lines }),
  openLogFolder: () => invoke("open_log_folder"),
};

export const transcribeCommands = {
  transcribeAudio: (audioPath: string) =>
    invoke<string>("transcribe_audio", { audioPath }),
  getSTTEngines: () => invoke<string[]>("get_stt_engines"),
};

export const modelCommands = {
  getModels: () => invoke<ModelInfo[]>("get_models"),
  isModelDownloaded: (modelName: string) =>
    invoke<boolean>("is_model_downloaded", { modelName }),
  downloadModel: (modelName: string) =>
    invoke<void>("download_model", { modelName }),
  cancelDownload: (modelName: string) =>
    invoke<void>("cancel_download", { modelName }),
  deleteModel: (modelName: string) =>
    invoke<void>("delete_model", { modelName }),
  getPolishModels: () =>
    invoke<PolishModelInfo[]>("get_polish_models"),
  getCurrentPolishModel: () =>
    invoke<string>("get_current_polish_model"),
  isPolishModelDownloaded: () =>
    invoke<boolean>("is_polish_model_downloaded"),
  isPolishModelDownloadedForModel: (modelId: string) =>
    invoke<boolean>("is_polish_model_downloaded_for_model", { modelId }),
  downloadPolishModel: () =>
    invoke<void>("download_polish_model"),
  downloadPolishModelById: (modelId: string) =>
    invoke<void>("download_polish_model_by_id", { modelId }),
  cancelPolishDownload: (modelId: string) =>
    invoke<void>("cancel_polish_download", { modelId }),
  deletePolishModel: () =>
    invoke<void>("delete_polish_model"),
  deletePolishModelById: (modelId: string) =>
    invoke<void>("delete_polish_model_by_id", { modelId }),
  getPolishTemplates: () =>
    invoke<PolishTemplate[]>("get_polish_templates"),
  getPolishTemplatePrompt: (templateId: string) =>
    invoke<string>("get_polish_template_prompt", { templateId }),
};

export interface PolishTemplate {
  id: string;
  name: string;
  description: string;
}

export const events = {
  onRecordingStateChanged: (callback: (event: RecordingStateEvent) => void) => {
    return listen<RecordingStateEvent>("recording-state-changed", (event) => {
      callback(event.payload);
    });
  },
  onAudioLevel: (callback: (level: number) => void) => {
    return listen<number>("audio-level", (event) => {
      callback(event.payload);
    });
  },
  onTranscriptionComplete: (callback: (event: TranscriptionCompleteEvent) => void) => {
    return listen<TranscriptionCompleteEvent>("transcription-complete", (event) => {
      callback(event.payload);
    });
  },
  onTranscriptionError: (callback: (error: string) => void) => {
    return listen<string>("transcription-error", (event) => {
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
      callback(event.payload);
    });
  },
  onModelDownloadComplete: (callback: (model: string) => void) => {
    return listen<{ model: string }>("model-download-complete", (event) => {
      callback(event.payload.model);
    });
  },
  onModelDownloadCancelled: (callback: (model: string) => void) => {
    return listen<{ model: string }>("model-download-cancelled", (event) => {
      callback(event.payload.model);
    });
  },
  onModelDeleted: (callback: (model: string) => void) => {
    return listen<{ model: string }>("model-deleted", (event) => {
      callback(event.payload.model);
    });
  },
  onPolishModelDownloadProgress: (
    callback: (data: { model_id: string; downloaded: number; total: number; progress: number }) => void
  ) => {
    return listen<{ model_id: string; downloaded: number; total: number; progress: number }>(
      "polish-model-download-progress",
      (event) => callback(event.payload)
    );
  },
  onPolishModelDownloadComplete: (callback: (model_id: string) => void) => {
    return listen<{ model_id: string }>("polish-model-download-complete", (event) =>
      callback(event.payload.model_id)
    );
  },
  onPolishModelDownloadCancelled: (callback: (model_id: string) => void) => {
    return listen<{ model_id: string }>("polish-model-download-cancelled", (event) =>
      callback(event.payload.model_id)
    );
  },
  onPolishModelDeleted: (callback: () => void) => {
    return listen("polish-model-deleted", () => callback());
  },
  onToastMessage: (callback: (message: string) => void) => {
    return listen<string>("toast-message", (event) => {
      callback(event.payload);
    });
  },
  onShortcutRegistrationFailed: (callback: (error: string) => void) => {
    return listen<string>("shortcut-registration-failed", (event) => {
      callback(event.payload);
    });
  },
  onSettingsChanged: (callback: (settings: AppSettings) => void) => {
    return listen<AppSettings>("settings-changed", (event) => {
      callback(event.payload);
    });
  },
  emit: (event: string, payload?: unknown) => emit(event, payload),
};
