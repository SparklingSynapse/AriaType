import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PolishSection } from "../model/PolishSection";
import type { AppSettings, PolishModelInfo, PolishModelStatus } from "@/lib/tauri";

const {
  checkLocalPolishRuntimeConfigMock,
  getPolishModelStatusMock,
  updateSettingMock,
} = vi.hoisted(() => ({
  checkLocalPolishRuntimeConfigMock: vi.fn(),
  getPolishModelStatusMock: vi.fn(),
  updateSettingMock: vi.fn(),
}));

vi.mock("@/contexts/SettingsContext", () => ({
  useSettingsContext: () => ({
    settings: testSettings,
    loading: false,
    polishAvailable: false,
    updateSetting: updateSettingMock,
  }),
}));

vi.mock("@/lib/tauri", () => {
  return {
    modelCommands: {
      getPolishModelStatus: getPolishModelStatusMock,
    },
    settingsCommands: {
      checkLocalPolishRuntimeConfig: checkLocalPolishRuntimeConfigMock,
    },
  };
});

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
  polish_model: "qwen3.5-0.8b",
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
  custom_dictionary: "",
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
  downloaded: true,
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

function renderPolishSection(status: PolishModelStatus) {
  getPolishModelStatusMock.mockResolvedValue(status);
  render(
    <PolishSection
      polishModels={[polishModel]}
      selectedPolishModel="qwen3.5-0.8b"
      setSelectedPolishModel={vi.fn()}
      polishDownloadingId={null}
      polishProgress={null}
      onDownload={vi.fn()}
      onCancel={vi.fn()}
      onDelete={vi.fn()}
    />,
  );
}

describe("PolishSection local runtime readiness", () => {
  beforeEach(() => {
    getPolishModelStatusMock.mockReset();
    checkLocalPolishRuntimeConfigMock.mockReset();
    updateSettingMock.mockReset();
  });

  it("warns when the selected polish model is downloaded but the runtime is not ready", async () => {
    renderPolishSection({
      is_loaded: false,
      is_downloaded: true,
      runtime_ready: false,
      current_model: "qwen3.5-0.8b",
      engine_type: "qwen",
    });

    expect(
      await screen.findByText("model.polish.localRuntime.notReadyStatus"),
    ).toBeInTheDocument();
    expect(screen.getByText("model.polish.localRuntime.notReadyHelp")).toBeInTheDocument();
  });

  it("shows ready status only when both model file and runtime are ready", async () => {
    renderPolishSection({
      is_loaded: true,
      is_downloaded: true,
      runtime_ready: true,
      current_model: "qwen3.5-0.8b",
      engine_type: "qwen",
    });

    await waitFor(() => {
      expect(screen.getByText("model.polish.localRuntime.readyStatus")).toBeInTheDocument();
    });
    expect(screen.queryByText("model.polish.localRuntime.notReadyHelp")).not.toBeInTheDocument();
  });
});
