import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useEventListeners } from "@/hooks/useEventListeners";
import { Button } from "@/components/ui/button";
import {
  ChevronRight,
  ChevronLeft,
  X,
  Mic,
  Accessibility,
  Monitor,
  Check,
  Loader2,
  Shield,
  Zap,
  Eye,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { showToast } from "@/lib/toast";
import {
  systemCommands,
  modelCommands,
  events,
  audioCommands,
  type RecommendedModel,
} from "@/lib/tauri";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { HotkeyInput, formatHotkey } from "@/components/ui/hotkey-input";
import { Select } from "@/components/ui/select";
import permissionsSvg from "@/assets/illustrations/onboarding/permissions.png";
import languageSvg from "@/assets/illustrations/onboarding/language.png";
import modelSvg from "@/assets/illustrations/onboarding/model.png";
import hotkeySvg from "@/assets/illustrations/onboarding/hotkey.png";
import practiceSvg from "@/assets/illustrations/onboarding/practice.png";
import doneSvg from "@/assets/illustrations/onboarding/done.png";
import {
  resolveOnboardingModelProgress,
  resolveOnboardingModelReady,
} from "./onboarding-model";

const DEFAULT_HOTKEY = "Shift+Space";
const ONBOARDING_RESET_EVENT = "ariatype:onboarding-reset";

const SENSEVOICE_PREFERRED_ONBOARDING = ["zh-CN", "zh-TW", "yue-CN", "ja-JP", "ko-KR", "en-US"];

function isSenseVoicePreferred(lang: string | undefined): boolean {
  if (!lang || lang === "auto") return false;
  return SENSEVOICE_PREFERRED_ONBOARDING.some((l) => lang.startsWith(l.split("-")[0]));
}

function getRecommendedModelName(lang: string): string {
  if (isSenseVoicePreferred(lang)) {
    return "sense-voice-small";
  }
  return "whisper-base";
}

const SUPPORTED_LANGUAGES = [
  "en-US", "zh-CN", "zh-TW", "yue-CN", "es-ES", "fr-FR", "de-DE",
  "ja-JP", "ko-KR", "pt-BR", "ru-RU", "ar-SA", "hi-IN", "it-IT",
  "nl-NL", "pl-PL", "tr-TR", "vi-VN", "th-TH", "id-ID",
];

function detectSystemLanguage(): string {
  const browserLang = navigator.language || navigator.languages?.[0] || "";
  if (!browserLang) return "auto";

  const baseLang = browserLang.split("-")[0];

  const mappings: Record<string, string> = {
    zh: browserLang.includes("Hant") || browserLang.includes("TW") || browserLang.includes("HK") || browserLang.includes("MO") ? "zh-TW" : "zh-CN",
    yue: "yue-CN",
    ja: "ja-JP",
    ko: "ko-KR",
    en: "en-US",
    es: "es-ES",
    fr: "fr-FR",
    de: "de-DE",
    pt: "pt-BR",
    ru: "ru-RU",
    ar: "ar-SA",
    hi: "hi-IN",
    it: "it-IT",
    nl: "nl-NL",
    pl: "pl-PL",
    tr: "tr-TR",
    vi: "vi-VN",
    th: "th-TH",
    id: "id-ID",
  };

  const mapped = mappings[baseLang];
  if (mapped && SUPPORTED_LANGUAGES.includes(mapped)) {
    return mapped;
  }

  return "auto";
}

const COMMON_LANGUAGES = [
  { code: "en-US", label: "English" },
  { code: "zh-CN", label: "Chinese (Simplified)" },
  { code: "zh-TW", label: "Chinese (Traditional)" },
  { code: "yue-CN", label: "Cantonese" },
  { code: "es-ES", label: "Spanish" },
  { code: "fr-FR", label: "French" },
  { code: "de-DE", label: "German" },
  { code: "ja-JP", label: "Japanese" },
  { code: "ko-KR", label: "Korean" },
  { code: "pt-BR", label: "Portuguese" },
  { code: "ru-RU", label: "Russian" },
  { code: "ar-SA", label: "Arabic" },
  { code: "hi-IN", label: "Hindi" },
  { code: "it-IT", label: "Italian" },
  { code: "nl-NL", label: "Dutch" },
  { code: "pl-PL", label: "Polish" },
  { code: "tr-TR", label: "Turkish" },
  { code: "vi-VN", label: "Vietnamese" },
  { code: "th-TH", label: "Thai" },
  { code: "id-ID", label: "Indonesian" },
  { code: "auto", label: "Auto" },
];

type StepId =
  | "permissions"
  | "model"
  | "language"
  | "hotkey"
  | "practice"
  | "done";

// --- SVG Icons Components ---
// Removed PracticeSvgIcon in favor of imported practiceSvg asset
// --- End SVG Icons ---

interface Step {
  id: StepId;
  title: string;
  description: string;
}

function PermissionStep() {
  const { t } = useTranslation();
  const [micStatus, setMicStatus] = useState<
    "granted" | "denied" | "not_determined" | null
  >(null);
  const [axStatus, setAxStatus] = useState<boolean | null>(null);
  const [screenStatus, setScreenStatus] = useState<
    "granted" | "denied" | "not_determined" | null
  >(null);
  const [micLoading, setMicLoading] = useState(false);
  const [axLoading, setAxLoading] = useState(false);
  const [screenLoading, setScreenLoading] = useState(false);

  const checkPermissions = useCallback(() => {
    systemCommands
      .checkPermission("microphone")
      .then((s) => setMicStatus(s as typeof micStatus))
      .catch((err: unknown) => logger.error("check_microphone_permission_failed", { error: String(err) }));
    systemCommands
      .checkPermission("accessibility")
      .then((s) => setAxStatus(s === "granted"))
      .catch((err: unknown) => logger.error("check_accessibility_permission_failed", { error: String(err) }));
    systemCommands
      .checkPermission("screen_recording")
      .then((s) => setScreenStatus(s as typeof screenStatus))
      .catch((err: unknown) => logger.error("check_screen_recording_permission_failed", { error: String(err) }));
  }, []);

  useEffect(() => {
    checkPermissions();
    const onFocus = () => checkPermissions();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [checkPermissions]);

  const handleMicPermission = async () => {
    setMicLoading(true);
    try {
      await systemCommands.applyPermission("microphone");
      setTimeout(checkPermissions, 500);
    } catch (err) {
      logger.error("failed_to_request_microphone_permission", { error: String(err) });
    } finally {
      setMicLoading(false);
    }
  };

  const handleAxPermission = async () => {
    setAxLoading(true);
    try {
      await systemCommands.applyPermission("accessibility");
      setTimeout(checkPermissions, 500);
    } catch (err) {
      logger.error("failed_to_request_accessibility_permission", { error: String(err) });
    } finally {
      setAxLoading(false);
    }
  };

  const handleScreenPermission = async () => {
    setScreenLoading(true);
    try {
      await systemCommands.applyPermission("screen_recording");
      setTimeout(checkPermissions, 500);
    } catch (err) {
      logger.error("failed_to_request_screen_recording_permission", { error: String(err) });
    } finally {
      setScreenLoading(false);
    }
  };

  return (
    <div className="flex flex-col items-center gap-4 w-full max-w-sm mx-auto">
      <img
        src={permissionsSvg}
        alt="Permissions"
        className="w-full max-w-[180px] max-h-[140px] object-contain"
      />
      <div className="space-y-4 w-full">
        <div
          className="flex items-center justify-between p-4 rounded-2xl border border-border bg-card"
          data-testid="onboarding-permission-microphone"
          data-status={micStatus ?? "pending"}
        >
          <div className="flex items-center gap-3">
            <div
              className={cn(
                "w-8 h-8 rounded-full flex items-center justify-center border",
                micStatus === "granted"
                  ? "bg-green-500/10 border-green-500/20 text-green-600"
                  : "bg-transparent border-border text-muted-foreground",
              )}
            >
              {micStatus === "granted" ? (
                <Check className="w-4 h-4 text-green-500" />
              ) : (
                <Mic className="w-4 h-4 text-muted-foreground" />
              )}
            </div>
            <div>
              <p className="text-sm font-medium">
                {t("onboarding.permissions.microphone")}
              </p>
              <p className="text-xs text-muted-foreground">
                {t("onboarding.permissions.microphoneDesc")}
              </p>
            </div>
          </div>
          <Button
            size="sm"
            variant={micStatus === "granted" ? "outline" : "default"}
            onClick={handleMicPermission}
            disabled={micLoading || micStatus === "granted"}
            className="w-20"
          >
            {micLoading ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : micStatus === "granted" ? (
              t("onboarding.permissions.granted")
            ) : (
              t("onboarding.permissions.grant")
            )}
          </Button>
        </div>

        <div
          className="flex items-center justify-between p-4 rounded-2xl border border-border bg-card"
          data-testid="onboarding-permission-accessibility"
          data-status={axStatus === null ? "pending" : axStatus ? "granted" : "denied"}
        >
          <div className="flex items-center gap-3">
            <div
              className={cn(
                "w-8 h-8 rounded-full flex items-center justify-center border",
                axStatus === true
                  ? "bg-green-500/10 border-green-500/20 text-green-600"
                  : "bg-transparent border-border text-muted-foreground",
              )}
            >
              {axStatus === true ? (
                <Check className="w-4 h-4 text-green-500" />
              ) : (
                <Accessibility className="w-4 h-4 text-muted-foreground" />
              )}
            </div>
            <div>
              <p className="text-sm font-medium">
                {t("onboarding.permissions.accessibility")}
              </p>
              <p className="text-xs text-muted-foreground">
                {t("onboarding.permissions.accessibilityDesc")}
              </p>
            </div>
          </div>
          <Button
            size="sm"
            variant={axStatus === true ? "outline" : "default"}
            onClick={handleAxPermission}
            disabled={axLoading || axStatus === true}
            className="w-20"
          >
            {axLoading ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : axStatus === true ? (
              t("onboarding.permissions.granted")
            ) : (
              t("onboarding.permissions.grant")
            )}
          </Button>
        </div>

        <div
          className="flex items-center justify-between p-4 rounded-2xl border border-border bg-card"
          data-testid="onboarding-permission-screen-recording"
          data-status={screenStatus === null ? "pending" : screenStatus}
        >
          <div className="flex items-center gap-3">
            <div
              className={cn(
                "w-8 h-8 rounded-full flex items-center justify-center border",
                screenStatus === "granted"
                  ? "bg-green-500/10 border-green-500/20 text-green-600"
                  : "bg-transparent border-border text-muted-foreground",
              )}
            >
              {screenStatus === "granted" ? (
                <Check className="w-4 h-4 text-green-500" />
              ) : (
                <Monitor className="w-4 h-4 text-muted-foreground" />
              )}
            </div>
            <div>
              <p className="text-sm font-medium">
                {t("onboarding.permissions.screenRecording")}
              </p>
              <p className="text-xs text-muted-foreground">
                {t("onboarding.permissions.screenRecordingDesc")}
              </p>
            </div>
          </div>
          <Button
            size="sm"
            variant={screenStatus === "granted" ? "outline" : "default"}
            onClick={handleScreenPermission}
            disabled={screenLoading || screenStatus === "granted"}
            className="w-20"
          >
            {screenLoading ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : screenStatus === "granted" ? (
              t("onboarding.permissions.granted")
            ) : (
              t("onboarding.permissions.grant")
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}

function CircularProgress({
  progress,
  size = 16,
  strokeWidth = 2,
}: {
  progress: number;
  size?: number;
  strokeWidth?: number;
}) {
  const radius = (size - strokeWidth) / 2;
  const circumference = radius * 2 * Math.PI;
  const offset = circumference - (progress / 100) * circumference;

  return (
    <svg width={size} height={size} className="text-green-500">
      <circle
        className="text-green-500/20"
        strokeWidth={strokeWidth}
        stroke="currentColor"
        fill="transparent"
        r={radius}
        cx={size / 2}
        cy={size / 2}
      />
      <circle
        className="transition-all duration-300"
        strokeWidth={strokeWidth}
        strokeDasharray={circumference}
        strokeDashoffset={offset}
        strokeLinecap="round"
        stroke="currentColor"
        fill="transparent"
        r={radius}
        cx={size / 2}
        cy={size / 2}
        transform={`rotate(-90 ${size / 2} ${size / 2})`}
      />
    </svg>
  );
}

function ModelStep({
  language,
  selectedModel,
  onSelectModel,
  onModelReadyChange,
}: {
  language: string;
  selectedModel: string | null;
  onSelectModel: (modelName: string) => void;
  onModelReadyChange: (isReady: boolean) => void;
}) {
  const { t } = useTranslation();
  const [models, setModels] = useState<RecommendedModel[]>([]);
  const [progressMap, setProgressMap] = useState<Record<string, number>>({});
  const [downloadedMap, setDownloadedMap] = useState<Record<string, boolean>>({});

  const recommendedModelName = getRecommendedModelName(language);
  const recommendedModelProgress = resolveOnboardingModelProgress({
    selectedModel: recommendedModelName,
    models,
    progressMap,
    downloadedMap,
  });
  const recommendedModelReady = recommendedModelProgress >= 100;

  // Fetch recommendations when component mounts or language changes
  useEffect(() => {
    modelCommands
      .recommendModelsByLanguage(language || "auto")
      .then(setModels)
      .catch((err: unknown) => logger.error("failed_to_recommend_models", { error: String(err) }));
  }, [language]);

  // Check if recommended model is already downloaded and start download if not
  useEffect(() => {
    let mounted = true;
    const modelName = getRecommendedModelName(language || "auto");

    const checkAndDownload = async () => {
      try {
        const isDownloaded = await modelCommands.isModelDownloaded(modelName);
        if (!mounted) return;

        if (isDownloaded) {
          setDownloadedMap((prev) => ({ ...prev, [modelName]: true }));
          setProgressMap((prev) => ({ ...prev, [modelName]: 100 }));
        } else {
          setDownloadedMap((prev) => ({ ...prev, [modelName]: false }));
          setProgressMap((prev) => ({ ...prev, [modelName]: 0 }));
          await modelCommands.downloadModel(modelName);
        }
      } catch (err: unknown) {
        // Backend returns error if already downloading - that's expected
        const errorMsg = String(err);
        if (!errorMsg.includes("already downloading") && mounted) {
          logger.error("failed_to_start_model_download", { modelName, error: errorMsg });
        }
      }
    };

    checkAndDownload();

    return () => {
      mounted = false;
    };
  }, [language]);

  useEventListeners(async () => {
    return [
      await events.onModelDownloadProgress((data) => {
        setProgressMap((prev) => ({ ...prev, [data.model]: data.progress }));
      }),
      await events.onModelDownloadComplete(async (modelName) => {
        setDownloadedMap((prev) => ({ ...prev, [modelName]: true }));
        setProgressMap((prev) => ({ ...prev, [modelName]: 100 }));
        modelCommands
          .recommendModelsByLanguage(language || "auto")
          .then(setModels)
          .catch((err: unknown) => logger.error("failed_to_refresh_models_after_download", { error: String(err) }));
        const displayName = modelName === "sense-voice-small" ? "SenseVoice Small"
          : modelName === "whisper-base" ? "Whisper Base"
          : modelName === "whisper-small" ? "Whisper Small"
          : modelName;
        showToast(`${displayName} download complete`);
      }),
    ];
  }, [language]);

  useEffect(() => {
    onSelectModel(recommendedModelName);
  }, [recommendedModelName, onSelectModel]);

  // Only allow next step when the currently selected model is downloaded
  useEffect(() => {
    onModelReadyChange(
      resolveOnboardingModelReady({
        selectedModel,
        models,
        progressMap,
        downloadedMap,
      }),
    );
  }, [selectedModel, models, progressMap, downloadedMap, onModelReadyChange]);

  return (
    <div className="flex flex-col items-center gap-4 w-full max-w-sm mx-auto">
      <img src={modelSvg} alt="Model" className="w-full max-w-[220px] max-h-[120px] object-contain" />
      <div className="space-y-3 w-full">
        <div className="flex items-center justify-between p-4 rounded-2xl border border-border bg-card">
          <div className="flex items-center gap-3">
            <div>
              <p className="text-sm font-medium">
                {recommendedModelName === "sense-voice-small"
                  ? "SenseVoice Small"
                  : recommendedModelName === "whisper-base"
                    ? "Whisper Base"
                    : recommendedModelName}
              </p>
              <p className="text-xs text-muted-foreground">
                {models.find((m) => m.model_name === recommendedModelName)?.size_mb ?? "..."}MB
              </p>
            </div>
          </div>
          <div className="flex items-center justify-center w-[18px] h-[18px]">
            {recommendedModelReady ? (
              <Check className="w-[18px] h-[18px] text-green-500" />
            ) : (
              <CircularProgress
                progress={recommendedModelProgress}
                size={18}
                strokeWidth={2}
              />
            )}
          </div>
        </div>
        {recommendedModelProgress > 0 && recommendedModelProgress < 100 && (
          <p className="text-xs text-muted-foreground text-center">
            {t("onboarding.model.downloading")}
          </p>
        )}
      </div>
    </div>
  );
}

function LanguageStep() {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();

  if (!settings) return null;

  const handleLanguageChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, {
      setting: "stt_engine_language",
      value,
    });
    await updateSetting("stt_engine_language", value);
  };

  return (
    <div className="flex flex-col items-center gap-4 w-full max-w-sm mx-auto">
      <img
        src={languageSvg}
        alt="Language"
        className="w-full max-w-[180px] max-h-[140px] object-contain"
      />
      <div className="space-y-4 w-full">
        <div className="space-y-2">
          <Select
            value={settings.stt_engine_language ?? "auto"}
            onChange={(e) => handleLanguageChange(e.target.value)}
            options={COMMON_LANGUAGES.map((lang) => ({
              value: lang.code,
              label: lang.label,
            }))}
            className="w-full"
          />
        </div>

        <p className="text-xs text-muted-foreground text-center">
          {t("onboarding.language.hint")}
        </p>
      </div>
    </div>
  );
}

function HotkeyStep() {
  const { t } = useTranslation();
  const { settings } = useSettingsContext();

  if (!settings) return null;

  const defaultHotkey =
    settings?.shortcut_profiles?.dictate?.hotkey || "Shift+Space";

  // Note: HotkeyInput's onChange is called after backend has already
  // registered the hotkey. Backend emits SETTINGS_CHANGED which auto-refreshes UI.
  // We only track analytics, no updateSetting call.
  const handleHotkeyChange = (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, {
      setting: "hotkey",
      value,
    });
  };

  return (
    <div className="flex flex-col items-center gap-4 w-full max-w-sm mx-auto">
      <img
        src={hotkeySvg}
        alt="Hotkey"
        className="w-full max-w-[180px] max-h-[140px] object-contain"
      />
      <div className="space-y-4 w-full">
        <div className="flex justify-center">
          <HotkeyInput
            profileKey="dictate"
            value={defaultHotkey}
            onChange={handleHotkeyChange}
            placeholder={t("hotkey.recording.pressKeys")}
            className="w-48"
          />
        </div>

        <p className="text-xs text-muted-foreground text-center">
          {t("onboarding.hotkey.hint")}
        </p>
      </div>
    </div>
  );
}

function PracticeStep({ hotkey }: { hotkey: string }) {
  const { t } = useTranslation();
  const formattedHotkey = formatHotkey(hotkey).replace(/\+/g, " + ");
  const [isRecording, setIsRecording] = useState(false);
  const [, setAudioLevel] = useState(0);
  const [transcript, setTranscript] = useState<string | null>(null);
  const [isTranscribing, setIsTranscribing] = useState(false);

  useEffect(() => {
    let unlistenState: (() => void) | undefined;
    let unlistenLevel: (() => void) | undefined;
    let unlistenTranscript: (() => void) | undefined;
    let unlistenError: (() => void) | undefined;

    const setup = async () => {
      unlistenState = await events.onRecordingStateChanged((event) => {
        if (event.status === "recording") {
          setIsRecording(true);
          setIsTranscribing(false);
        } else if (
          event.status === "transcribing" ||
          event.status === "processing" ||
          event.status === "polishing"
        ) {
          setIsRecording(false);
          setIsTranscribing(true);
        } else if (event.status === "idle" || event.status === "error") {
          setIsRecording(false);
          setIsTranscribing(false);
        }
      });

      unlistenLevel = await events.onAudioLevel((level) => {
        setAudioLevel(level);
      });

      unlistenTranscript = await events.onTranscriptionComplete((event) => {
        setTranscript(event.text);
        setIsTranscribing(false);
      });

      unlistenError = await events.onTranscriptionError((error) => {
        logger.error("transcription_error", { error });
        setIsTranscribing(false);
      });
    };

    setup();

    return () => {
      unlistenState?.();
      unlistenLevel?.();
      unlistenTranscript?.();
      unlistenError?.();
    };
  }, []);

  const handleKeyDown = async (e: React.KeyboardEvent) => {
    const modifierCodes = new Set([
      "MetaLeft",
      "MetaRight",
      "ControlLeft",
      "ControlRight",
      "AltLeft",
      "AltRight",
      "ShiftLeft",
      "ShiftRight",
    ]);

    if (modifierCodes.has(e.code)) {
      return;
    }

    const expectedParts = hotkey.toLowerCase().split("+").sort();
    const actualParts: string[] = [];

    if (e.metaKey) actualParts.push("cmd");
    if (e.ctrlKey) actualParts.push("ctrl");
    if (e.altKey) actualParts.push("alt");
    if (e.shiftKey) actualParts.push("shift");

    const keyName = e.code.startsWith("Key")
      ? e.code.slice(3).toLowerCase()
      : e.code.startsWith("Digit")
        ? e.code.slice(5)
        : e.code.toLowerCase();
    actualParts.push(keyName);

    actualParts.sort();

    const isMatch =
      expectedParts.length === actualParts.length &&
      expectedParts.every((p) => actualParts.includes(p));

    if (isMatch && !isRecording && !isTranscribing) {
      e.preventDefault();
      setTranscript(null);
      await audioCommands.startRecording();
    }
  };

  const handleKeyUp = async () => {
    if (isRecording) {
      await audioCommands.stopRecording();
    }
  };

  return (
    <div
      className="flex flex-col items-center gap-4 w-full max-w-sm mx-auto"
      tabIndex={0}
      onKeyDown={handleKeyDown}
      onKeyUp={handleKeyUp}
    >
      <div className="relative flex justify-center items-center w-full max-w-[160px]">
        <div className="relative z-10 w-full max-h-[120px] flex justify-center items-center">
          <img
            src={practiceSvg}
            alt="Practice"
            className={cn(
              "w-full h-full object-contain transition-all duration-300",
              isRecording ? "scale-110 drop-shadow-[0_0_15px_rgba(34,197,94,0.5)]" : "",
            )}
          />
        </div>
      </div>

      <p className="text-sm text-muted-foreground text-center mt-2">
        {isTranscribing
          ? t("onboarding.practice.transcribing")
          : t("onboarding.practice.instruction", { hotkey: formattedHotkey })}
      </p>

      <div className="w-full h-[100px] p-4 rounded-2xl bg-card border border-border">
        {transcript ? (
          <div className="h-full overflow-auto">
            <p className="text-xs text-muted-foreground mb-2">
              {t("onboarding.practice.result")}:
            </p>
            <p className="text-sm leading-relaxed">{transcript}</p>
          </div>
        ) : (
          <div className="h-full flex items-center justify-center">
            <p className="text-xs text-muted-foreground text-center">
              {t("onboarding.practice.tip")}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

function DoneStep() {
  const { t } = useTranslation();

  const features = [
    {
      icon: Shield,
      label: t("onboarding.done.feature1"),
      highlight: t("onboarding.done.feature1Highlight"),
    },
    {
      icon: Zap,
      label: t("onboarding.done.feature2"),
      highlight: t("onboarding.done.feature2Highlight"),
    },
    {
      icon: Eye,
      label: t("onboarding.done.feature3"),
      highlight: t("onboarding.done.feature3Highlight"),
    },
  ];

  return (
    <div className="flex flex-col items-center gap-4 text-center w-full max-w-lg mx-auto">
      <img src={doneSvg} alt="Done" className="w-full max-w-[160px] max-h-[120px] object-contain" />

      <div className="w-full grid grid-cols-3 gap-3">
        {features.map((feature, index) => (
          <div
            key={index}
            className="flex flex-col items-center justify-center gap-2 p-4 rounded-2xl border border-border bg-card"
          >
            <feature.icon className="w-5 h-5 text-muted-foreground shrink-0" />
            <div className="text-center">
              <span className="text-sm font-medium block leading-tight">{feature.label}</span>
              <span className="text-xs text-muted-foreground block leading-tight mt-0.5">
                {feature.highlight}
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

interface OnboardingGuideProps {
  isOpen: boolean;
  onClose: () => void;
}

export function OnboardingGuide({ isOpen, onClose }: OnboardingGuideProps) {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();
  const [currentStep, setCurrentStep] = useState(0);
  const [selectedModel, setSelectedModel] = useState<string | null>(null);
  const [isModelReady, setIsModelReady] = useState(false);
  const allSteps: Step[] = [
    {
      id: "permissions",
      title: t("onboarding.permissions.title"),
      description: t("onboarding.permissions.description"),
    },
    {
      id: "language",
      title: t("onboarding.language.title"),
      description: t("onboarding.language.description"),
    },
    {
      id: "hotkey",
      title: t("onboarding.hotkey.title"),
      description: t("onboarding.hotkey.description"),
    },
    {
      id: "model",
      title: t("onboarding.model.title"),
      description: t("onboarding.model.description"),
    },
    {
      id: "practice",
      title: t("onboarding.practice.title"),
      description: t("onboarding.practice.description"),
    },
    {
      id: "done",
      title: t("onboarding.done.congrats"),
      description: t("onboarding.done.ready"),
    },
  ];

  const steps = allSteps;

  const handleNext = async () => {
    if (current.id === "language") {
      const language = settings?.stt_engine_language || "auto";
      const modelName = getRecommendedModelName(language);
      modelCommands.isModelDownloaded(modelName).then((isDownloaded) => {
        if (!isDownloaded) {
          modelCommands.downloadModel(modelName).catch((err: unknown) => {
            const errorMsg = String(err);
            if (!errorMsg.includes("already downloading")) {
              logger.error("failed_to_start_model_download", { modelName, error: errorMsg });
            }
          });
        }
      });
    }
    if (current.id === "model" && selectedModel) {
      const engineType = selectedModel?.startsWith("sense-voice")
        ? "sensevoice"
        : "whisper";
      await updateSetting("model", selectedModel);
      await updateSetting("stt_engine", engineType);
    }
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1);
    } else {
      analytics.track(AnalyticsEvents.ONBOARDING_COMPLETED);
      onClose();
    }
  };

  const canProceed = () => {
    if (current.id === "model") return !!selectedModel && isModelReady;
    return true;
  };

  const handlePrev = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const handleSkip = () => {
    analytics.track(AnalyticsEvents.ONBOARDING_SKIPPED, { step: currentStep });
    onClose();
  };

  useEffect(() => {
    if (isOpen) {
      setCurrentStep(0);
      analytics.track(AnalyticsEvents.ONBOARDING_STARTED);
    }
  }, [isOpen]);

  useEffect(() => {
    const handleReset = () => {
      setCurrentStep(0);
      setSelectedModel(null);
      setIsModelReady(false);
    };

    window.addEventListener(ONBOARDING_RESET_EVENT, handleReset);
    return () => window.removeEventListener(ONBOARDING_RESET_EVENT, handleReset);
  }, []);

  useEffect(() => {
    if (isOpen && (!settings?.stt_engine_language || settings.stt_engine_language === "auto")) {
      const detected = detectSystemLanguage();
      if (detected !== "auto") {
        updateSetting("stt_engine_language", detected)
          .catch((err: unknown) => logger.error("failed_to_set_detected_language", { error: String(err) }));
      }
    }
  }, [isOpen, settings?.stt_engine_language, updateSetting]);

  useEffect(() => {
    if (currentStep >= steps.length) {
      setCurrentStep(Math.max(0, steps.length - 1));
    }
  }, [steps.length, currentStep]);

  useEffect(() => {
    if (isOpen && currentStep < steps.length) {
      analytics.track(AnalyticsEvents.ONBOARDING_STEP_VIEWED, {
        step: currentStep,
        step_id: steps[currentStep].id,
      });
    }
  }, [isOpen, currentStep]);

  if (!isOpen) return null;

  const current = steps[currentStep];
  const isLastStep = currentStep === steps.length - 1;

  const renderStepContent = () => {
    switch (current.id) {
      case "permissions":
        return <PermissionStep />;
      case "language":
        return <LanguageStep />;
      case "hotkey":
        return <HotkeyStep />;
      case "model":
        return (
          <ModelStep
            language={settings?.stt_engine_language || "auto"}
            selectedModel={selectedModel}
            onSelectModel={setSelectedModel}
            onModelReadyChange={setIsModelReady}
          />
        );
      case "practice":
        return (
          <PracticeStep
            hotkey={settings?.shortcut_profiles?.dictate?.hotkey || DEFAULT_HOTKEY.toLowerCase()}
          />
        );
      case "done":
        return <DoneStep />;
      default:
        return null;
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={handleSkip}
      />

      <div
        className="relative z-10 w-[640px] h-[620px] mx-4 bg-background rounded-3xl border border-border shadow-2xl flex flex-col"
        data-testid="onboarding-modal"
        data-step-id={current.id}
      >
        <div className="flex flex-col items-center gap-3 pt-6 shrink-0">
          <div className="flex justify-center gap-2">
            {steps.map((_, index) => (
              <button
                key={index}
                onClick={() => setCurrentStep(index)}
                className={cn(
                  "h-1.5 rounded-full transition-all duration-300 cursor-pointer",
                  index === currentStep
                    ? "w-6 bg-primary"
                    : index < currentStep
                      ? "w-1.5 bg-primary"
                      : "w-1.5 bg-input",
                )}
              />
            ))}
          </div>
        </div>

        <div className="flex-1 flex flex-col">
          <div className="pt-6 px-14 shrink-0">
            <h2 className="text-lg font-semibold text-center mb-2">
              {current.title}
            </h2>
            <p className="text-sm text-muted-foreground text-center">
              {current.description}
            </p>
          </div>

          <div className="flex-1 flex items-center justify-center px-14 pb-10">
            {renderStepContent()}
          </div>
        </div>

        <div className="flex items-center justify-between space-x-4 p-4 border-t border-border shrink-0 rounded-b-xl">
          {currentStep > 0 ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={handlePrev}
              className="gap-2"
            >
              <ChevronLeft className="w-4 h-4" />
              {t("onboarding.prev")}
            </Button>
          ) : (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleSkip}
              className="text-muted-foreground"
            >
              {t("onboarding.skip")}
            </Button>
          )}
          <Button
            size="sm"
            onClick={handleNext}
            disabled={!canProceed()}
            className="gap-2"
            data-testid="onboarding-primary-action"
          >
            {isLastStep ? t("onboarding.finish") : t("onboarding.next")}
            {!isLastStep && <ChevronRight className="w-4 h-4" />}
          </Button>
        </div>

        <Button
          variant="ghost"
          size="icon"
          className="absolute top-3 right-3 h-8 w-8 text-muted-foreground hover:text-foreground"
          onClick={handleSkip}
        >
          <X className="w-4 h-4" />
        </Button>
      </div>
    </div>
  );
}
