import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { GeneralSettings } from "../GeneralSettings";
import type { AppSettings } from "@/lib/tauri";

const {
  getAudioDevicesMock,
  getIdentifierMock,
  getPlatformMock,
  updateSettingMock,
} = vi.hoisted(() => ({
  getAudioDevicesMock: vi.fn(),
  getIdentifierMock: vi.fn(),
  getPlatformMock: vi.fn(),
  updateSettingMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/app", () => ({
  getIdentifier: getIdentifierMock,
}));

vi.mock("@/contexts/SettingsContext", () => ({
  useSettingsContext: () => ({
    settings: testSettings,
    loading: false,
    polishAvailable: false,
    updateSetting: updateSettingMock,
  }),
}));

vi.mock("@/lib/tauri", () => ({
  settingsCommands: {
    getAvailableSubdomains: vi.fn(async () => []),
    clearCorrectionMemory: vi.fn(async () => undefined),
    openCorrectionMemoryDirectory: vi.fn(async () => undefined),
  },
  systemCommands: {
    getAudioDevices: getAudioDevicesMock,
    getPlatform: getPlatformMock,
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

vi.mock("@/lib/toast", () => ({
  showErrorToast: vi.fn(),
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
    t: (key: string) => key,
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
  model: "tiny",
  model_resident: false,
  pill_background_color: "#1d1d1d",
  pill_background_opacity: 1,
  pill_indicator_mode: "always",
  pill_position: "bottom-right",
  pill_size: 2,
  polish_custom_templates: [],
  polish_model: "",
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

describe("GeneralSettings correction memory directory entry", () => {
  beforeEach(() => {
    getAudioDevicesMock.mockResolvedValue(["default"]);
    getPlatformMock.mockResolvedValue("macos");
    getIdentifierMock.mockReset();
    updateSettingMock.mockReset();
  });

  it("hides the correction memory folder button outside in-house builds", async () => {
    getIdentifierMock.mockResolvedValue("com.ariatype.voicetotext");

    render(<GeneralSettings />);

    await waitFor(() => {
      expect(getIdentifierMock).toHaveBeenCalled();
    });

    expect(screen.queryByText("general.privacy.correctionMemoryOpenAction")).not.toBeInTheDocument();
  });

  it("shows the correction memory folder button in in-house builds", async () => {
    getIdentifierMock.mockResolvedValue("com.ariatype.voicetotext.inhouse");

    render(<GeneralSettings />);

    expect(await screen.findByText("general.privacy.correctionMemoryOpenAction")).toBeInTheDocument();
  });
});
