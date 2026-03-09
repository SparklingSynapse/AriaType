import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import logoLight from "../../../assets/ariatype-light.png";
import logoDark from "../../../assets/ariatype-dark.png";
import {
  ChevronRight,
  ChevronLeft,
  X,
  Mic,
  Accessibility,
  Check,
  Loader2,
  Keyboard,
  Shield,
  Zap,
  Eye,
  Languages,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import {
  systemCommands,
  modelCommands,
  events,
  audioCommands,
  type ModelInfo,
} from "@/lib/tauri";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { HotkeyInput, formatHotkey } from "@/components/ui/hotkey-input";
import { Select } from "@/components/ui/select";

const DEFAULT_HOTKEY = "Shift+Space";

// Common languages for onboarding
const COMMON_LANGUAGES = [
  { code: "auto", label: "Auto" },
  { code: "en", label: "English" },
  { code: "zh", label: "Chinese (Simplified)" },
  { code: "zh-TW", label: "Chinese (Traditional)" },
  { code: "yue", label: "Cantonese" },
  { code: "es", label: "Spanish" },
  { code: "fr", label: "French" },
  { code: "de", label: "German" },
  { code: "ja", label: "Japanese" },
  { code: "ko", label: "Korean" },
  { code: "pt", label: "Portuguese" },
  { code: "ru", label: "Russian" },
  { code: "ar", label: "Arabic" },
  { code: "hi", label: "Hindi" },
  { code: "it", label: "Italian" },
  { code: "nl", label: "Dutch" },
  { code: "pl", label: "Polish" },
  { code: "tr", label: "Turkish" },
  { code: "vi", label: "Vietnamese" },
  { code: "th", label: "Thai" },
  { code: "id", label: "Indonesian" },
];

type StepId = "permissions" | "model" | "language" | "hotkey" | "practice" | "done";

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
  const [micLoading, setMicLoading] = useState(false);
  const [axLoading, setAxLoading] = useState(false);

  const checkPermissions = useCallback(() => {
    systemCommands
      .checkPermission("microphone")
      .then((s) => setMicStatus(s as typeof micStatus))
      .catch(console.error);
    systemCommands
      .checkPermission("accessibility")
      .then((s) => setAxStatus(s === "granted"))
      .catch(console.error);
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
      console.error("Failed to request microphone permission:", err);
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
      console.error("Failed to request accessibility permission:", err);
    } finally {
      setAxLoading(false);
    }
  };

  return (
    <div className="space-y-4 w-full">
      <div className="flex items-center justify-between p-3 rounded-lg border border-border bg-card">
        <div className="flex items-center gap-3">
          <div
            className={cn(
              "w-8 h-8 rounded-full flex items-center justify-center",
              micStatus === "granted" ? "bg-green-500/20" : "bg-muted",
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

      <div className="flex items-center justify-between p-3 rounded-lg border border-border bg-card">
        <div className="flex items-center gap-3">
          <div
            className={cn(
              "w-8 h-8 rounded-full flex items-center justify-center",
              axStatus === true ? "bg-green-500/20" : "bg-muted",
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
    </div>
  );
}

function ModelStep() {
  const { t } = useTranslation();
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    modelCommands.getModels().then(setModels).catch(console.error);
  }, []);

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenComplete: (() => void) | undefined;

    const setup = async () => {
      unlistenProgress = await events.onModelDownloadProgress((data) => {
        setProgress(data.progress);
      });
      unlistenComplete = await events.onModelDownloadComplete(() => {
        setDownloading(null);
        setProgress(0);
        modelCommands.getModels().then(setModels).catch(console.error);
      });
    };
    setup();

    return () => {
      unlistenProgress?.();
      unlistenComplete?.();
    };
  }, []);

  const handleDownload = async (modelName: string) => {
    setDownloading(modelName);
    setProgress(0);
    try {
      await modelCommands.downloadModel(modelName);
    } catch (err) {
      console.error("Failed to download model:", err);
      setDownloading(null);
    }
  };

  const downloadedModels = models.filter((m) => m.downloaded);
  const recommendedModel = models.find((m) => m.name === "base") || models[0];

  if (downloadedModels.length > 0) {
    return (
      <div className="flex flex-col items-center gap-3 text-center">
        <div className="w-12 h-12 rounded-full bg-green-500/20 flex items-center justify-center">
          <Check className="w-10 h-10 text-green-500" />
        </div>
        <p className="mt-2 text-sm">
          {t("onboarding.model.downloaded", {
            model: downloadedModels[0].display_name,
          })}
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-3 w-full">
      <p className="text-sm text-muted-foreground text-center mb-2">
        {t("onboarding.model.selectHint")}
      </p>
      {models.slice(0, 3).map((model) => {
        const isRecommended = model.name === recommendedModel?.name;
        return (
          <div
            key={model.name}
            className={cn(
              "flex items-center justify-between p-3 rounded-lg border bg-card",
              isRecommended
                ? "border-primary/50 bg-primary/5"
                : "border-border",
            )}
          >
            <div className="flex items-center gap-3">
              <div>
                <div className="flex items-center gap-2">
                  <p className="text-sm font-medium">{model.display_name}</p>
                  {isRecommended && (
                    <div className="flex items-center gap-1 text-xs text-primary">
                      <Check className="w-3 h-3" />
                      <span>Recommended</span>
                    </div>
                  )}
                </div>
                <p className="text-xs text-muted-foreground">
                  {model.size_mb}MB · {t("model.available.accuracy")}:{" "}
                  {model.accuracy_score}/10
                </p>
              </div>
            </div>
            {downloading === model.name ? (
              <div className="flex items-center gap-2 w-24">
                <div className="flex-1 h-1.5 bg-secondary rounded-full overflow-hidden">
                  <div
                    className="h-full bg-primary transition-all"
                    style={{ width: `${progress}%` }}
                  />
                </div>
                <span className="text-xs text-muted-foreground w-8">
                  {progress}%
                </span>
              </div>
            ) : (
              <Button
                size="sm"
                variant="outline"
                onClick={() => handleDownload(model.name)}
                disabled={downloading !== null}
              >
                {t("model.available.download")}
              </Button>
            )}
          </div>
        );
      })}
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
    <div className="space-y-4 w-3/4">
      <div className="flex items-center justify-center gap-2 mb-4">
        <Languages className="w-5 h-5 text-primary" />
        <span className="text-sm font-medium">
          {t("onboarding.language.current")}
        </span>
      </div>

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
  );
}

function HotkeyStep() {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();

  if (!settings) return null;

  const saveHotkey = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, {
      setting: "hotkey",
      value,
    });
    await updateSetting("hotkey", value);
  };

  return (
    <div className="space-y-4 w-full">
      <div className="flex items-center justify-center gap-2">
        <Keyboard className="w-5 h-5 text-primary" />
        <span className="text-sm font-medium">
          {t("onboarding.hotkey.current")}
        </span>
      </div>

      <div className="flex justify-center">
        <HotkeyInput
          value={settings.hotkey}
          onChange={saveHotkey}
          placeholder={t("hotkey.recording.pressKeys")}
          className="w-48 px-4 py-3 text-center text-lg rounded-lg border bg-background"
        />
      </div>

      <p className="text-xs text-muted-foreground text-center">
        {t("onboarding.hotkey.hint")}
      </p>
    </div>
  );
}

function PracticeStep({ hotkey }: { hotkey: string }) {
  const { t } = useTranslation();
  const formattedHotkey = formatHotkey(hotkey).replace(/\+/g, " + ");
  const [isRecording, setIsRecording] = useState(false);
  const [audioLevel, setAudioLevel] = useState(0);
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
        } else if (event.status === "transcribing") {
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
        console.error("Transcription error:", error);
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
    if (
      ![
        "MetaLeft",
        "MetaRight",
        "ControlLeft",
        "ControlRight",
        "AltLeft",
        "AltRight",
        "ShiftLeft",
        "ShiftRight",
      ].includes(e.code)
    ) {
      actualParts.push(keyName);
    }

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
      className="flex flex-col items-center gap-6 w-full"
      tabIndex={0}
      onKeyDown={handleKeyDown}
      onKeyUp={handleKeyUp}
    >
      <div
        className={cn(
          "w-14 h-14 rounded-full flex items-center justify-center border transition-colors duration-100",
          isRecording ? "border-green-500/50" : "border-border",
        )}
        style={
          isRecording
            ? {
                borderColor: `rgba(34, 197, 94, ${0.3 + audioLevel * 0.7})`,
              }
            : undefined
        }
      >
        <Mic
          className={cn(
            "w-6 h-6 transition-colors duration-100",
            isRecording ? "text-green-500" : "text-muted-foreground",
          )}
        />
      </div>

      <p className="text-sm text-muted-foreground text-center">
        {isTranscribing
          ? t("onboarding.practice.transcribing")
          : t("onboarding.practice.instruction", { hotkey: formattedHotkey })}
      </p>

      <div className="w-full h-[100px] p-3 rounded-lg bg-card border border-border">
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
    <div className="flex flex-col items-center gap-8 text-center w-full max-w-sm">
      <div className="w-12 h-12 rounded-full bg-green-500/20 flex items-center justify-center">
        <Check className="w-10 h-10 text-green-500" />
      </div>
      <div>
        <h3 className="text-xl font-medium mb-1">
          {t("onboarding.done.congrats")}
        </h3>
        <p className="text-sm text-muted-foreground">
          {t("onboarding.done.ready")}
        </p>
      </div>

      <div className="w-full space-y-3">
        {features.map((feature, index) => (
          <div
            key={index}
            className="flex items-center justify-center gap-3 p-3 rounded-lg bg-muted/50 border border-border bg-card"
          >
            <feature.icon className="w-5 h-5 text-muted-foreground shrink-0" />
            <div className="text-center">
              <span className="text-sm font-medium">{feature.label}</span>
              <span className="text-sm text-muted-foreground ml-1">
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
  const { settings } = useSettingsContext();
  const [currentStep, setCurrentStep] = useState(0);

  const steps: Step[] = [
    {
      id: "permissions",
      title: t("onboarding.permissions.title"),
      description: t("onboarding.permissions.description"),
    },
    {
      id: "model",
      title: t("onboarding.model.title"),
      description: t("onboarding.model.description"),
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
      id: "practice",
      title: t("onboarding.practice.title"),
      description: t("onboarding.practice.description"),
    },
    {
      id: "done",
      title: t("onboarding.done.title"),
      description: t("onboarding.done.description"),
    },
  ];

  const handleNext = () => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1);
    } else {
      analytics.track(AnalyticsEvents.ONBOARDING_COMPLETED);
      onClose();
    }
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
    if (isOpen) {
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
      case "model":
        return <ModelStep />;
      case "language":
        return <LanguageStep />;
      case "hotkey":
        return <HotkeyStep />;
      case "practice":
        return (
          <PracticeStep
            hotkey={settings?.hotkey || DEFAULT_HOTKEY.toLowerCase()}
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

      <div className="relative z-10 w-[560px] min-h-[520px] mx-4 bg-background rounded-xl border border-border shadow-2xl overflow-hidden flex flex-col">
        <div className="flex flex-col items-center gap-3 pt-6 shrink-0">
          <img src={logoLight} alt="AriaType" className="h-7 dark:hidden" />
          <img src={logoDark} alt="AriaType" className="h-7 hidden dark:block" />
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
                    : "w-1.5 bg-muted",
              )}
            />
          ))}
          </div>
        </div>

        <div className="flex-1 px-14 py-6 flex flex-col">
          {current.id !== "done" && (
            <>
              <h2 className="text-lg font-semibold text-center mb-2">
                {current.title}
              </h2>
              <p className="text-sm text-muted-foreground text-center mb-4">
                {current.description}
              </p>
            </>
          )}

          <div className="flex-1 flex items-center justify-center">
            {renderStepContent()}
          </div>
        </div>

        <div className="flex items-center justify-between space-x-4 p-4 border-t border-border bg-muted/30 shrink-0">
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
          <Button size="sm" onClick={handleNext} className="gap-2">
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
