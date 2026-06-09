import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ModelSettings } from "../ModelSettings";
import type { AppSettings, PolishModelInfo, PolishModelStatus } from "@/lib/tauri";

const {
  downloadPolishModelByIdMock,
  getModelsMock,
  getPolishModelsMock,
  getPolishModelStatusMock,
  onModelDeletedMock,
  onModelDownloadCancelledMock,
  onModelDownloadCompleteMock,
  onModelDownloadProgressMock,
  onPolishModelDeletedMock,
  onPolishModelDownloadCancelledMock,
  onPolishModelDownloadCompleteMock,
  onPolishModelDownloadProgressMock,
  preloadPolishModelMock,
  updateSettingContextMock,
  updateSettingsCommandMock,
} = vi.hoisted(() => ({
  downloadPolishModelByIdMock: vi.fn(),
  getModelsMock: vi.fn(),
  getPolishModelsMock: vi.fn(),
  getPolishModelStatusMock: vi.fn(),
  onModelDeletedMock: vi.fn(),
  onModelDownloadCancelledMock: vi.fn(),
  onModelDownloadCompleteMock: vi.fn(),
  onModelDownloadProgressMock: vi.fn(),
  onPolishModelDeletedMock: vi.fn(),
  onPolishModelDownloadCancelledMock: vi.fn(),
  onPolishModelDownloadCompleteMock: vi.fn(),
  onPolishModelDownloadProgressMock: vi.fn(),
  preloadPolishModelMock: vi.fn(),
  updateSettingContextMock: vi.fn(),
  updateSettingsCommandMock: vi.fn(),
}));

vi.mock("@/contexts/SettingsContext", () => ({
  useSettingsContext: () => ({
    settings: testSettings,
    loading: false,
    polishAvailable: false,
    updateSetting: updateSettingContextMock,
  }),
}));

vi.mock("@/lib/tauri", () => ({
  modelCommands: {
    downloadPolishModelById: downloadPolishModelByIdMock,
    getModels: getModelsMock,
    getPolishModels: getPolishModelsMock,
    getPolishModelStatus: getPolishModelStatusMock,
    preloadPolishModel: preloadPolishModelMock,
  },
  settingsCommands: {
    checkLocalPolishRuntimeConfig: vi.fn(async () => ({
      ok: true,
      kind: "ok",
      message: "ok",
      duration_ms: 1,
    })),
    updateSettings: updateSettingsCommandMock,
  },
  events: {
    onModelDeleted: onModelDeletedMock,
    onModelDownloadCancelled: onModelDownloadCancelledMock,
    onModelDownloadComplete: onModelDownloadCompleteMock,
    onModelDownloadProgress: onModelDownloadProgressMock,
    onPolishModelDeleted: onPolishModelDeletedMock,
    onPolishModelDownloadCancelled: onPolishModelDownloadCancelledMock,
    onPolishModelDownloadComplete: onPolishModelDownloadCompleteMock,
    onPolishModelDownloadProgress: onPolishModelDownloadProgressMock,
  },
}));

vi.mock("@/lib/analytics", () => ({
  analytics: {
    track: vi.fn(),
  },
}));

vi.mock("@/lib/logger", () => ({
  logger: {
    error: vi.fn(),
  },
}));

vi.mock("react-i18next", () => ({
  initReactI18next: {
    init: vi.fn(),
    type: "3rdParty",
  },
  useTranslation: () => ({
    i18n: {
      changeLanguage: vi.fn(async () => undefined),
    },
    t: (key: string, values?: Record<string, unknown>) => {
      if (values?.templates) return `${key}:${values.templates}`;
      return key;
    },
  }),
}));

const testSettings: AppSettings = {
  active_cloud_polish_provider: "openai",
  active_cloud_stt_provider: "volcengine",
  analytics_opt_in: false,
  audio_device: "default",
  auto_start: false,
  beep_on_record: true,
  cloud_polish_configs: {},
  cloud_polish_enabled: false,
  cloud_stt_configs: {},
  cloud_stt_enabled: false,
  correction_memory_enabled: true,
  denoise_mode: "off",
  gpu_acceleration: false,
  hotkey: "Cmd+Slash",
  idle_unload_minutes: 5,
  language: "auto",
  local_polish_runtime: {
    provider_type: "llama-server",
    base_url: "http://127.0.0.1:8000/v1",
    api_key: "",
    server_command: "",
    server_args_json: "",
    ready_timeout_secs: 20,
  },
  model: "tiny",
  model_resident: false,
  pill_background_color: "#1d1d1d",
  pill_background_opacity: 1,
  pill_indicator_mode: "always",
  pill_position: "bottom-right",
  pill_size: 2,
  polish_custom_templates: [],
  polish_model: "qwen3.5-2b",
  polish_stream_direct_typing_enabled: false,
  polish_system_prompt: "",
  recording_mode: "hold",
  shortcut_profiles: {
    dictate: {
      hotkey: "Cmd+Slash",
      trigger_mode: "hold",
      action: { Record: { polish_template_id: null } },
    },
    riff: {
      hotkey: "Opt+Slash",
      trigger_mode: "toggle",
      action: { Record: { polish_template_id: null } },
    },
  },
  stay_in_tray: false,
  stt_engine: "sherpa",
  stt_engine_initial_prompt: "",
  stt_engine_language: "en",
  stt_engine_user_glossary: "",
  stt_engine_work_domain: "general",
  stt_engine_work_domain_prompt: "",
  stt_engine_work_subdomain: "general",
  theme_mode: "system",
  vad_enabled: false,
  window_context_enabled: false,
};

const polishModel: PolishModelInfo = {
  id: "qwen3.5-0.8b",
  name: "Qwen 3.5 0.8B",
  size: "600MB",
  downloaded: false,
  compatibility: {
    level: "smooth",
    code: "smooth",
    minimum_memory_mb: 2048,
    recommended_memory_mb: 4096,
    device_memory_mb: 16384,
    logical_cpu_count: 10,
  },
  latency_profile: {
    class: "fast",
    code: "fast_transcript_preserving",
    recommended_templates: ["filler"],
    caution_templates: [],
  },
};

const polishStatus: PolishModelStatus = {
  is_loaded: false,
  is_downloaded: false,
  runtime_ready: false,
  current_model: "qwen3.5-2b",
  engine_type: "qwen",
};

function registerEventMocks() {
  const cleanup = vi.fn();
  onModelDeletedMock.mockImplementation(async () => cleanup);
  onModelDownloadCancelledMock.mockImplementation(async () => cleanup);
  onModelDownloadCompleteMock.mockImplementation(async () => cleanup);
  onModelDownloadProgressMock.mockImplementation(async () => cleanup);
  onPolishModelDeletedMock.mockImplementation(async () => cleanup);
  onPolishModelDownloadCancelledMock.mockImplementation(async () => cleanup);
  onPolishModelDownloadCompleteMock.mockImplementation(async () => cleanup);
  onPolishModelDownloadProgressMock.mockImplementation(async () => cleanup);
}

describe("ModelSettings polish model activation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    registerEventMocks();
    getModelsMock.mockResolvedValue([]);
    getPolishModelsMock.mockResolvedValue([polishModel]);
    getPolishModelStatusMock.mockResolvedValue(polishStatus);
    downloadPolishModelByIdMock.mockResolvedValue(undefined);
    preloadPolishModelMock.mockResolvedValue(undefined);
    updateSettingContextMock.mockResolvedValue(undefined);
    updateSettingsCommandMock.mockResolvedValue(undefined);
  });

  it("selects and preloads a polish model after its download completes", async () => {
    render(<ModelSettings />);

    fireEvent.click(screen.getByRole("button", { name: "model.tabs.polish" }));
    fireEvent.click(await screen.findByRole("button", { name: "model.available.download" }));

    await waitFor(() => {
      expect(downloadPolishModelByIdMock).toHaveBeenCalledWith("qwen3.5-0.8b");
    });
    await waitFor(() => {
      expect(onPolishModelDownloadCompleteMock).toHaveBeenCalledTimes(1);
    });

    const callback = onPolishModelDownloadCompleteMock.mock.calls[0]?.[0];
    expect(callback).toBeTypeOf("function");
    await act(async () => {
      callback("qwen3.5-0.8b");
    });

    await waitFor(() => {
      expect(updateSettingsCommandMock).toHaveBeenCalledWith("polish_model", "qwen3.5-0.8b");
      expect(preloadPolishModelMock).toHaveBeenCalledTimes(1);
    });
  });
});
