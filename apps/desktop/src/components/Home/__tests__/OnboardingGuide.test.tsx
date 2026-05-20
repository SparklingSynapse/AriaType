import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { OnboardingGuide } from "../OnboardingGuide";

const mocks = vi.hoisted(() => {
  const onModelDownloadProgressMock = vi.fn(async () => () => undefined);
  const onModelDownloadCompleteMock = vi.fn(async () => () => undefined);

  return {
    applyPermissionMock: vi.fn(async () => undefined),
    checkPermissionMock: vi.fn(async () => "granted"),
    downloadModelMock: vi.fn(async () => undefined),
    isModelDownloadedMock: vi.fn(async () => false),
    onAudioLevelMock: vi.fn(async () => () => undefined),
    onHotkeyCapturedMock: vi.fn(async () => () => undefined),
    onModelDownloadCompleteMock,
    onModelDownloadProgressMock,
    onRecordingStateChangedMock: vi.fn(async () => () => undefined),
    onTranscriptionCompleteMock: vi.fn(async () => () => undefined),
    onTranscriptionErrorMock: vi.fn(async () => () => undefined),
    recommendModelsByLanguageMock: vi.fn(async () => [
      {
        engine_type: "sensevoice",
        model_name: "sense-voice-small",
        display_name: "SenseVoice Small",
        size_mb: 229,
        speed_score: 4,
        accuracy_score: 4,
        downloaded: false,
      },
    ]),
    startRecordingMock: vi.fn(async () => undefined),
    stopRecordingMock: vi.fn(async () => undefined),
    trackMock: vi.fn(),
    updateSettingMock: vi.fn(async () => undefined),
  };
});

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string>) => (
      params?.hotkey ? `${key} ${params.hotkey}` : key
    ),
  }),
}));

vi.mock("@/contexts/SettingsContext", () => ({
  useSettingsContext: () => ({
    loading: false,
    polishAvailable: false,
    settings: {
      shortcut_profiles: {
        dictate: {
          hotkey: "Shift+Space",
        },
      },
      stt_engine_language: "zh-CN",
    },
    updateSetting: mocks.updateSettingMock,
  }),
}));

vi.mock("@/lib/analytics", () => ({
  analytics: {
    track: mocks.trackMock,
  },
}));

vi.mock("@/lib/logger", () => ({
  logger: {
    error: vi.fn(),
  },
}));

vi.mock("@/lib/toast", () => ({
  showToast: vi.fn(),
}));

vi.mock("@/components/ui/hotkey-input", () => ({
  HotkeyInput: () => <button type="button">hotkey</button>,
  formatHotkey: (hotkey: string) => hotkey,
}));

vi.mock("@/lib/tauri", () => ({
  audioCommands: {
    startRecording: mocks.startRecordingMock,
    stopRecording: mocks.stopRecordingMock,
  },
  events: {
    onAudioLevel: mocks.onAudioLevelMock,
    onHotkeyCaptured: mocks.onHotkeyCapturedMock,
    onModelDownloadComplete: mocks.onModelDownloadCompleteMock,
    onModelDownloadProgress: mocks.onModelDownloadProgressMock,
    onRecordingStateChanged: mocks.onRecordingStateChangedMock,
    onTranscriptionComplete: mocks.onTranscriptionCompleteMock,
    onTranscriptionError: mocks.onTranscriptionErrorMock,
  },
  modelCommands: {
    downloadModel: mocks.downloadModelMock,
    isModelDownloaded: mocks.isModelDownloadedMock,
    recommendModelsByLanguage: mocks.recommendModelsByLanguageMock,
  },
  systemCommands: {
    applyPermission: mocks.applyPermissionMock,
    checkPermission: mocks.checkPermissionMock,
  },
}));

describe("OnboardingGuide model download flow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.downloadModelMock.mockImplementation(async () => undefined);
    mocks.checkPermissionMock.mockResolvedValue("granted");
    mocks.isModelDownloadedMock.mockResolvedValue(false);
    mocks.onModelDownloadCompleteMock.mockImplementation(async () => () => undefined);
    mocks.onModelDownloadProgressMock.mockImplementation(async () => () => undefined);
  });

  it("starts the model download from the model step after progress listeners are mounted", async () => {
    let progressListenerReady = false;
    let releaseProgressListener: (() => void) | undefined;
    mocks.onModelDownloadProgressMock.mockImplementationOnce(() => (
      new Promise<() => void>((resolve) => {
        releaseProgressListener = () => {
          progressListenerReady = true;
          resolve(() => undefined);
        };
      })
    ));
    mocks.downloadModelMock.mockImplementationOnce(async () => {
      expect(progressListenerReady).toBe(true);
    });

    render(<OnboardingGuide isOpen onClose={vi.fn()} />);

    const modal = screen.getByTestId("onboarding-modal");
    const nextButton = screen.getByTestId("onboarding-primary-action");

    expect(modal).toHaveAttribute("data-step-id", "permissions");

    fireEvent.click(nextButton);
    await waitFor(() => expect(modal).toHaveAttribute("data-step-id", "language"));

    fireEvent.click(nextButton);
    await waitFor(() => expect(modal).toHaveAttribute("data-step-id", "hotkey"));
    await Promise.resolve();

    expect(mocks.isModelDownloadedMock).not.toHaveBeenCalled();
    expect(mocks.downloadModelMock).not.toHaveBeenCalled();

    fireEvent.click(nextButton);
    await waitFor(() => expect(modal).toHaveAttribute("data-step-id", "model"));
    await Promise.resolve();

    expect(mocks.isModelDownloadedMock).not.toHaveBeenCalled();
    expect(mocks.downloadModelMock).not.toHaveBeenCalled();
    expect(releaseProgressListener).toBeDefined();

    releaseProgressListener?.();

    await waitFor(() => {
      expect(mocks.downloadModelMock).toHaveBeenCalledWith("sense-voice-small");
    });

    expect(mocks.onModelDownloadProgressMock).toHaveBeenCalledTimes(1);
    expect(mocks.onModelDownloadCompleteMock).toHaveBeenCalledTimes(1);
  });
});
