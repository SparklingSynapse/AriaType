export type RecordingStatus = "idle" | "recording" | "transcribing" | "processing" | "polishing" | "error";

export interface PillPosition {
  x: number;
  y: number;
}

export type PillIndicatorMode = "always" | "when_recording" | "never";

export type PresetPosition =
  | "top-left"
  | "top-center"
  | "top-right"
  | "bottom-left"
  | "bottom-center"
  | "bottom-right";

export type LocalSttModel = "sense-voice-small" | "whisper-base" | "whisper-small";
