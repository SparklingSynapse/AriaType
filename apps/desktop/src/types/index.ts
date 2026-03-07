export type RecordingStatus = "idle" | "recording" | "transcribing" | "processing" | "error";

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

export type WhisperModel = "tiny" | "base" | "small" | "medium" | "large";
